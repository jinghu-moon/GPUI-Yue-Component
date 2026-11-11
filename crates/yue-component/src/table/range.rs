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
    tail_cursor: Option<SharedString>,
}
impl TableRangeRequest {
    pub fn tail_cursor(&self) -> Option<&SharedString> {
        self.tail_cursor.as_ref()
    }
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
        self.observe_tail(cx);
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
        #[cfg(feature = "table-profile")]
        let _perf = gpui_kit::perf_stats::scope("table.range.request_range");
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
        if self.requests.len() >= 64 && self.range_pending.is_none() {
            return Err(TableError::TooManyPendingRequests);
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
            tail_cursor: None,
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
        #[cfg(feature = "table-profile")]
        let _perf = gpui_kit::perf_stats::scope("table.range.commit_range");
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
            || if request.tail_cursor.is_some() {
                candidate.source.len() <= old.source.len()
                    || (0..old.source.len())
                        .any(|i| old.source.row_key(i) != candidate.source.row_key(i))
            } else {
                candidate.source.len() != old.source.len()
            }
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
        if let Some(cursor) = &request.tail_cursor {
            self.tail_failed = Some(cursor.clone());
        } else {
            self.range_failure = Some(request.range.clone());
        }
        self.reject_request(request.id, cx)
    }
    pub fn retry_range(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Result<Option<TableRangeRequest>, TableError> {
        if let Some(cursor) = self.tail_failed.take() {
            self.tail_seen = None;
            return self.request_tail(cursor, cx);
        }
        let Some(range) = self.range_failure.take() else {
            return Ok(None);
        };
        self.request_range(range, cx)
    }
}

impl<D: TableDelegate> YueTableState<D> {
    pub(super) fn observe_tail(&mut self, cx: &mut Context<Self>) {
        let Some(p) = &self.presentation else { return };
        let Some((cursor, distance, has_more)) = &p.tail else {
            return;
        };
        if !has_more || self.tail_failed.is_some() {
            return;
        }
        let Some(view) = self.view() else { return };
        let near = if view.source.is_empty() {
            true
        } else {
            let rows = self.rows.read(cx);
            let Some(vp) = rows.viewport() else { return };
            let Ok(snapshot) = rows.snapshot(vp) else {
                return;
            };
            let headers = cells::header_count(&Presentation {
                resolved: self.resolved.clone(),
                ..p.clone()
            });
            let Ok(last) = snapshot.item(headers + view.source.len() - 1) else {
                return;
            };
            let trailing = if p.footer_fixed && view.columns.iter().any(|c| c.footer.is_some()) {
                snapshot
                    .item(snapshot.len() - 1)
                    .map_or(0., |i| f64::from(i.extent()))
            } else {
                0.
            };
            last.start() + f64::from(last.extent())
                - (vp.offset() + f64::from(vp.extent()) - trailing)
                <= f64::from(distance.to_pixels(self.layout_rem))
        };
        if near {
            let cursor = cursor.clone();
            let _ = self.request_tail(cursor, cx);
        }
    }
    pub fn request_tail(
        &mut self,
        cursor: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Result<Option<TableRangeRequest>, TableError> {
        #[cfg(feature = "table-profile")]
        let _perf = gpui_kit::perf_stats::scope("table.range.request_tail");
        let cursor = cursor.into();
        if cursor.trim().is_empty() {
            return Err(TableError::EmptyKey);
        }
        let v = self.view().ok_or(TableError::Unbound)?;
        if v.query.page().is_some() || v.source.has_placeholders() {
            return Err(TableError::UnsupportedCombination(
                "tail loading requires an unpaged loaded result",
            ));
        }
        if self.tail_seen.as_ref() == Some(&cursor) || self.range_pending.is_some() {
            return Ok(None);
        }
        if self.requests.len() >= 64 {
            return Err(TableError::TooManyPendingRequests);
        }
        let (scope, source, query, len) = (
            v.source.scope().clone(),
            v.source.revision(),
            v.query.clone(),
            v.source.len(),
        );
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
            range: len..len,
            tail_cursor: Some(cursor.clone()),
        };
        self.tail_seen = Some(cursor);
        self.tail_failed = None;
        self.requests.insert(id);
        self.range_pending = Some(request.clone());
        cx.emit(TableEvent::RangeRequested(request.clone()));
        Ok(Some(request))
    }
}
