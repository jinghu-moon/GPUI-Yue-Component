use super::*;
use gpui_kit::Context;
use std::{collections::HashMap, ops::Range};
/// A query-scoped indexed result. Slot keys identify positions within this
/// immutable query scope; changing query must create a new scope. Only loaded
/// records occupy storage. Slot keys are never business selection keys until loaded.
pub struct TableRangeRows<R: 'static> {
    scope: SharedString,
    revision: u64,
    keys: Rc<[TableRowKey]>,
    positions: Rc<HashMap<TableRowKey, usize>>,
    records: HashMap<usize, Rc<R>>,
}
impl<R: 'static> TableRangeRows<R> {
    pub fn new(
        scope: impl Into<SharedString>,
        revision: u64,
        len: usize,
    ) -> Result<Self, TableError> {
        let scope = scope.into();
        if scope.trim().is_empty() {
            return Err(TableError::EmptyKey);
        }
        let keys: Rc<[_]> = (0..len)
            .map(|i| TableRowKey::try_new(format!("slot-{i}")).unwrap())
            .collect::<Vec<_>>()
            .into();
        let positions = Rc::new(
            keys.iter()
                .enumerate()
                .map(|(i, k)| (k.clone(), i))
                .collect(),
        );
        Ok(Self {
            scope,
            revision,
            keys,
            positions,
            records: HashMap::new(),
        })
    }
    pub fn with_records(
        &self,
        revision: u64,
        records: impl IntoIterator<Item = (usize, R)>,
    ) -> Result<Self, TableError> {
        if revision <= self.revision {
            return Err(TableError::StaleSource);
        }
        let mut next = Self {
            scope: self.scope.clone(),
            revision,
            keys: self.keys.clone(),
            positions: self.positions.clone(),
            records: self.records.clone(),
        };
        for (i, row) in records {
            if i >= self.keys.len() {
                return Err(TableError::InvalidSource);
            }
            next.records.insert(i, Rc::new(row));
        }
        Ok(next)
    }
    pub fn loaded_len(&self) -> usize {
        self.records.len()
    }
}
impl<R: 'static> TableSource for TableRangeRows<R> {
    type Row = R;
    fn scope(&self) -> &SharedString {
        &self.scope
    }
    fn revision(&self) -> u64 {
        self.revision
    }
    fn len(&self) -> usize {
        self.keys.len()
    }
    fn row_key(&self, i: usize) -> Option<&TableRowKey> {
        self.keys.get(i)
    }
    fn position_of(&self, k: &TableRowKey) -> Option<usize> {
        self.positions.get(k).copied()
    }
    fn row(&self, k: &TableRowKey) -> Option<&R> {
        self.records.get(&self.position_of(k)?).map(|r| r.as_ref())
    }
    fn is_placeholder(&self, k: &TableRowKey) -> bool {
        self.position_of(k)
            .is_some_and(|i| !self.records.contains_key(&i))
    }
    fn has_placeholders(&self) -> bool {
        self.records.len() < self.keys.len()
    }
    fn record_len(&self) -> usize {
        self.records.len()
    }
}
#[derive(Clone, Debug)]
pub struct TableRangeRequest {
    pub(super) id: TableRequestId,
    pub(super) scope: SharedString,
    pub(super) source: u64,
    pub(super) query: TableQuery,
    pub(super) range: Range<usize>,
}
impl TableRangeRequest {
    pub fn id(&self) -> TableRequestId {
        self.id
    }
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }
    pub fn scope(&self) -> &SharedString {
        &self.scope
    }
    pub fn source_revision(&self) -> u64 {
        self.source
    }
    pub fn query(&self) -> &TableQuery {
        &self.query
    }
}
impl<D: TableDelegate> YueTableState<D> {
    pub(super) fn observe_range(&mut self, range: Range<usize>, cx: &mut Context<Self>) {
        let Some(p) = &self.presentation else { return };
        let Some(prefetch) = p.range_prefetch else {
            return;
        };
        let headers = cells::header_count(p);
        let Some(v) = self.view() else { return };
        let range = range.start.saturating_sub(headers).saturating_sub(prefetch)
            ..range
                .end
                .saturating_sub(headers)
                .saturating_add(prefetch)
                .min(v.source.len());
        if self.range_failure.as_ref() == Some(&range) {
            return;
        }
        let _ = self.request_range(range, cx);
    }
    /// One latest viewport demand at a time. Hosts start/cancel work; stale success,
    /// failure and retry tickets cannot settle a later viewport/query request.
    pub fn request_range(
        &mut self,
        range: Range<usize>,
        cx: &mut Context<Self>,
    ) -> Result<Option<TableRangeRequest>, TableError> {
        let view = self.view().ok_or(TableError::Unbound)?;
        if range.start > range.end || range.end > view.source.len() {
            return Err(TableError::InvalidSource);
        }
        if view.query.page().is_some() {
            return Err(TableError::UnsupportedCombination(
                "range loading and pagination are exclusive",
            ));
        }
        if !(range.clone()).any(|i| {
            view.source
                .row_key(i)
                .is_some_and(|k| view.source.is_placeholder(k))
        }) {
            return Ok(None);
        }
        if self
            .range_pending
            .as_ref()
            .is_some_and(|r| r.range.start <= range.start && r.range.end >= range.end)
        {
            return Ok(None);
        }
        let scope = view.source.scope().clone();
        let source = view.source.revision();
        let query = view.query.clone();
        if let Some(old) = self.range_pending.take() {
            self.requests.remove(&old.id);
        }
        self.sequence += 1;
        let id = TableRequestId {
            owner: self.owner,
            sequence: self.sequence,
        };
        let request = TableRangeRequest {
            id,
            scope,
            source,
            query,
            range,
        };
        self.requests.insert(id);
        self.range_pending = Some(request.clone());
        self.range_failure = None;
        cx.emit(TableEvent::RangeRequested(request.clone()));
        Ok(Some(request))
    }
    pub fn commit_range(
        &mut self,
        request: &TableRangeRequest,
        candidate: TableView<D::Source>,
        cx: &mut Context<Self>,
    ) -> Result<TableCommitOutcome, TableError> {
        let old = self.view().ok_or(TableError::Unbound)?;
        if !self
            .range_pending
            .as_ref()
            .is_some_and(|r| r.id == request.id)
            || !self.requests.contains(&request.id)
            || request.id.owner != self.owner
            || old.source.scope() != &request.scope
            || old.source.revision() != request.source
            || old.query != request.query
        {
            return Err(TableError::StaleRequest);
        }
        if candidate.source.scope() != &request.scope
            || candidate.query != request.query
            || candidate.source.len() != old.source.len()
            || request.range.clone().any(|i| {
                candidate
                    .source
                    .row_key(i)
                    .is_none_or(|k| candidate.source.is_placeholder(k))
            })
        {
            return Err(TableError::InvalidSource);
        }
        self.commit_view(candidate, Some(request.id), cx)
    }
    pub fn reject_range(
        &mut self,
        request: &TableRangeRequest,
        cx: &mut Context<Self>,
    ) -> Result<(), TableError> {
        if !self
            .range_pending
            .as_ref()
            .is_some_and(|r| r.id == request.id)
        {
            return Err(TableError::StaleRequest);
        }
        self.range_failure = Some(request.range.clone());
        self.reject_request(request.id, cx)
    }
    pub fn retry_range(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Result<Option<TableRangeRequest>, TableError> {
        let Some(range) = self.range_failure.take() else {
            return Ok(None);
        };
        self.request_range(range, cx)
    }
}
