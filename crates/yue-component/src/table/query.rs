use super::*;
use std::{any::Any, cmp::Ordering, collections::BTreeMap, marker::PhantomData};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableDataMode {
    Local,
    Remote,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableTotal {
    Known(usize),
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableSortDirection {
    Ascending,
    Descending,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableSort {
    column: TableColumnKey,
    direction: TableSortDirection,
}
impl TableSort {
    pub fn new(column: TableColumnKey, direction: TableSortDirection) -> Self {
        Self { column, direction }
    }
    pub fn column(&self) -> &TableColumnKey {
        &self.column
    }
    pub fn direction(&self) -> TableSortDirection {
        self.direction
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TablePage {
    number: usize,
    size: usize,
}
impl TablePage {
    pub fn new(number: usize, size: usize) -> Result<Self, TableError> {
        if number == 0
            || size == 0
            || number
                .checked_sub(1)
                .and_then(|n| n.checked_mul(size))
                .is_none()
        {
            Err(TableError::InvalidQuery("invalid page range".into()))
        } else {
            Ok(Self { number, size })
        }
    }
    pub fn number(self) -> usize {
        self.number
    }
    pub fn size(self) -> usize {
        self.size
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TableFilterValue<V> {
    Scalar(V),
    Set(Vec<V>),
    Range { start: V, end: V },
    Missing,
}
#[derive(Clone)]
pub struct TableFilterKey<V> {
    column: TableColumnKey,
    rule: u64,
    _value: PhantomData<fn() -> V>,
}
impl<V> TableFilterKey<V> {
    pub fn column(&self) -> &TableColumnKey {
        &self.column
    }
}
type EqualFilter = fn(&dyn Any, &dyn Any) -> bool;
#[derive(Clone)]
pub(super) struct Filter {
    rule: u64,
    value: Rc<dyn Any>,
    equals: EqualFilter,
}
impl PartialEq for Filter {
    fn eq(&self, rhs: &Self) -> bool {
        self.rule == rhs.rule && (self.equals)(self.value.as_ref(), rhs.value.as_ref())
    }
}
#[derive(Clone, Default, PartialEq)]
pub struct TableQuery {
    sorts: Vec<TableSort>,
    filters: BTreeMap<TableColumnKey, Filter>,
    page: Option<TablePage>,
}
impl std::fmt::Debug for TableQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TableQuery")
            .field("sorts", &self.sorts)
            .field("filter_columns", &self.filters.keys().collect::<Vec<_>>())
            .field("page", &self.page)
            .finish()
    }
}
impl TableQuery {
    pub fn paged(number: usize, size: usize) -> Result<Self, TableError> {
        Self::default().with_page(number, size)
    }
    pub fn with_page(mut self, number: usize, size: usize) -> Result<Self, TableError> {
        self.page = Some(TablePage::new(number, size)?);
        Ok(self)
    }
    pub fn without_page(mut self) -> Self {
        self.page = None;
        self
    }
    pub fn page(&self) -> Option<TablePage> {
        self.page
    }
    pub fn sorts(&self) -> &[TableSort] {
        &self.sorts
    }
    pub fn with_sorts(mut self, sorts: Vec<TableSort>) -> Self {
        self.sorts = sorts;
        self.first_page();
        self
    }
    pub fn with_filter<V: Clone + PartialEq + 'static>(
        mut self,
        key: &TableFilterKey<V>,
        value: Option<TableFilterValue<V>>,
    ) -> Result<Self, TableError> {
        if let Some(value) = value {
            self.filters.insert(
                key.column.clone(),
                Filter {
                    rule: key.rule,
                    value: Rc::new(value),
                    equals: |a, b| {
                        a.downcast_ref::<TableFilterValue<V>>()
                            == b.downcast_ref::<TableFilterValue<V>>()
                    },
                },
            );
        } else {
            if self
                .filters
                .get(&key.column)
                .is_some_and(|filter| filter.rule != key.rule)
            {
                return Err(TableError::InvalidQuery("stale filter key".into()));
            }
            self.filters.remove(&key.column);
        }
        self.first_page();
        Ok(self)
    }
    pub fn filter<V: 'static>(&self, key: &TableFilterKey<V>) -> Option<&TableFilterValue<V>> {
        self.filters
            .get(&key.column)
            .filter(|f| f.rule == key.rule)?
            .value
            .downcast_ref()
    }
    pub fn has_filter(&self, column: &TableColumnKey) -> bool {
        self.filters.contains_key(column)
    }
    pub fn clear_filters(mut self) -> Self {
        self.filters.clear();
        self.first_page();
        self
    }
    fn first_page(&mut self) {
        if let Some(page) = &mut self.page {
            page.number = 1;
        }
    }
    pub(super) fn validate<R: 'static>(
        &self,
        columns: &[TableColumn<R>],
    ) -> Result<(), TableError> {
        let mut seen = std::collections::HashSet::new();
        for sort in &self.sorts {
            if !seen.insert(&sort.column)
                || !columns.iter().any(|c| {
                    c.key() == &sort.column && c.query.as_ref().is_some_and(|r| r.compare.is_some())
                })
            {
                return Err(TableError::InvalidQuery(
                    "unsupported or duplicate sort".into(),
                ));
            }
        }
        for (key, filter) in &self.filters {
            let rule = columns
                .iter()
                .find(|c| c.key() == key)
                .and_then(|c| c.query.as_ref())
                .ok_or_else(|| TableError::InvalidQuery("filter column removed".into()))?;
            if rule.id != filter.rule || !(rule.validate)(filter.value.as_ref()) {
                return Err(TableError::InvalidQuery(
                    "stale or invalid filter value".into(),
                ));
            }
        }
        Ok(())
    }
    pub(super) fn same_rules<R: 'static>(
        &self,
        before: &[TableColumn<R>],
        after: &[TableColumn<R>],
    ) -> bool {
        self.sorts
            .iter()
            .map(|s| &s.column)
            .chain(self.filters.keys())
            .all(|key| {
                let rule = |columns: &[TableColumn<R>]| {
                    columns
                        .iter()
                        .find(|c| c.key() == key)
                        .and_then(|c| c.query.as_ref())
                        .map(|rule| rule.id)
                };
                rule(before).is_some() && rule(before) == rule(after)
            })
    }
}
type Compare<R> = Rc<dyn Fn(&R, &R) -> Ordering>;
type Predicate<R> = Rc<dyn Fn(&R, &dyn Any) -> bool>;
type ValueCompare<V> = Rc<dyn Fn(&V, &V) -> Ordering>;
type ValidateFilter = Rc<dyn Fn(&dyn Any) -> bool>;
type ValuePredicate<V> = Rc<dyn Fn(&V) -> bool>;
type ScalarPredicate<V> = Rc<dyn Fn(&V, &V) -> bool>;
pub(super) struct QueryRule<R> {
    id: u64,
    compare: Option<Compare<R>>,
    predicate: Predicate<R>,
    validate: ValidateFilter,
}
pub struct TableValueColumn<R: 'static, V: Clone + PartialEq + PartialOrd + 'static> {
    column: TableColumn<R>,
    read: Rc<dyn Fn(&R) -> V>,
    scalar: ScalarPredicate<V>,
    sortable: bool,
    filterable: bool,
    missing: Option<ValuePredicate<V>>,
    rule: u64,
    has_display: bool,
    compare: Option<ValueCompare<V>>,
}
static NEXT_RULE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
impl<R: 'static> TableColumn<R> {
    pub fn value<V: Clone + PartialEq + PartialOrd + 'static>(
        key: impl Into<SharedString>,
        title: impl Into<SharedString>,
        read: impl Fn(&R) -> V + 'static,
    ) -> TableValueColumn<R, V> {
        TableValueColumn {
            column: Self::text(key, title, |_| ""),
            read: Rc::new(read),
            scalar: Rc::new(|a, b| a == b),
            sortable: false,
            filterable: false,
            missing: None,
            has_display: false,
            compare: None,
            rule: NEXT_RULE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        }
    }
    pub fn is_sortable(&self) -> bool {
        self.query.as_ref().is_some_and(|r| r.compare.is_some())
    }
}
impl<R: 'static, V: Clone + PartialEq + PartialOrd + 'static> TableValueColumn<R, V> {
    pub fn display_with(mut self, display: impl Fn(&V) -> SharedString + 'static) -> Self {
        let read = self.read.clone();
        self.column.text = Some(Rc::new(move |r| display(&read(r))));
        self.has_display = true;
        self
    }
    pub fn render_with<E: IntoElement>(
        mut self,
        render: impl Fn(&R, &V, &TableCellContext, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        let read = self.read.clone();
        self.column.text = None;
        self.column.renderer = Some(Rc::new(move |row, cell, w, cx| {
            render(row, &read(row), cell, w, cx).into_any_element()
        }));
        self.has_display = true;
        self
    }
    pub fn compare_by(mut self, compare: impl Fn(&V, &V) -> Ordering + 'static) -> Self {
        self.compare = Some(Rc::new(compare));
        self.sortable = true;
        self
    }
    pub fn sortable(mut self, enabled: bool) -> Self
    where
        V: Ord,
    {
        self.sortable = enabled;
        if enabled && self.compare.is_none() {
            self.compare = Some(Rc::new(Ord::cmp));
        }
        self
    }
    pub fn filter_scalar(mut self, predicate: impl Fn(&V, &V) -> bool + 'static) -> Self {
        self.scalar = Rc::new(predicate);
        self.filterable = true;
        self
    }
    pub fn filterable(mut self, enabled: bool) -> Self {
        self.filterable = enabled;
        self
    }
    pub fn missing_when(mut self, predicate: impl Fn(&V) -> bool + 'static) -> Self {
        self.missing = Some(Rc::new(predicate));
        self.filterable = true;
        self
    }
    pub fn filter_key(&self) -> TableFilterKey<V> {
        TableFilterKey {
            column: self.column.key.clone(),
            rule: self.rule,
            _value: PhantomData,
        }
    }
    pub fn build(mut self) -> Result<TableColumn<R>, TableError> {
        if self.column.key.as_str().trim().is_empty() {
            return Err(TableError::EmptyKey);
        }
        if !self.has_display {
            return Err(TableError::InvalidQuery(
                "value column requires a display renderer".into(),
            ));
        }
        let compare = self.sortable.then(|| {
            let read = self.read.clone();
            let compare = self
                .compare
                .expect("sortable columns install a total comparator");
            Rc::new(move |a: &R, b: &R| {
                let a = read(a);
                let b = read(b);
                compare(&a, &b)
            }) as Compare<R>
        });
        let read = self.read;
        let filterable = self.filterable;
        let missing_allowed = self.missing.is_some();
        let missing = self.missing;
        let scalar = self.scalar;
        self.column.query = Some(Rc::new(QueryRule {
            id: self.rule,
            compare,
            predicate: Rc::new(move |r, value| {
                let Some(filter) = value.downcast_ref::<TableFilterValue<V>>() else {
                    return false;
                };
                let actual = read(r);
                match filter {
                    TableFilterValue::Scalar(v) => scalar(&actual, v),
                    TableFilterValue::Set(v) => v.iter().any(|v| scalar(&actual, v)),
                    TableFilterValue::Range { start, end } => actual >= *start && actual <= *end,
                    TableFilterValue::Missing => missing.as_ref().is_some_and(|p| p(&actual)),
                }
            }),
            validate: Rc::new(move |value| {
                filterable
                    && value
                        .downcast_ref::<TableFilterValue<V>>()
                        .is_some_and(|v| match v {
                            TableFilterValue::Range { start, end } => start
                                .partial_cmp(end)
                                .is_some_and(|o| o != Ordering::Greater),
                            TableFilterValue::Missing => missing_allowed,
                            _ => true,
                        })
            }),
        }));
        Ok(self.column)
    }
}

/// Read-only page/order projection; borrowing a record never clones it.
pub struct TableProjection<S: TableSource> {
    source: Rc<S>,
    revision: u64,
    keys: Vec<TableRowKey>,
    positions: std::collections::HashMap<TableRowKey, usize>,
}
impl<S: TableSource> TableProjection<S> {
    pub fn source(&self) -> &Rc<S> {
        &self.source
    }
    pub fn try_new(
        source: Rc<S>,
        revision: u64,
        keys: Vec<TableRowKey>,
    ) -> Result<Self, TableError> {
        let mut positions = std::collections::HashMap::new();
        for (ix, key) in keys.iter().enumerate() {
            if source.row(key).is_none() {
                return Err(TableError::MissingKey);
            }
            if positions.insert(key.clone(), ix).is_some() {
                return Err(TableError::DuplicateRowKey(key.clone()));
            }
        }
        Ok(Self {
            source,
            revision,
            keys,
            positions,
        })
    }
}
impl<S: TableSource> TableSource for TableProjection<S> {
    type Row = S::Row;
    fn scope(&self) -> &SharedString {
        self.source.scope()
    }
    fn revision(&self) -> u64 {
        self.revision
    }
    fn len(&self) -> usize {
        self.keys.len()
    }
    fn row_key(&self, ix: usize) -> Option<&TableRowKey> {
        self.keys.get(ix)
    }
    fn position_of(&self, key: &TableRowKey) -> Option<usize> {
        self.positions.get(key).copied()
    }
    fn row(&self, key: &TableRowKey) -> Option<&Self::Row> {
        self.source.row(key)
    }
    fn contains_key(&self, key: &TableRowKey) -> bool {
        self.source.contains_key(key)
    }
}

/// A returned page plus explicit known selectable keys. Records from other pages
/// are not fabricated or retained; batch operations consume selected keys.
pub struct TableRemotePage<S: TableSource> {
    page: Rc<S>,
    domain: std::collections::BTreeSet<TableRowKey>,
}
impl<S: TableSource> TableRemotePage<S> {
    pub fn new(page: Rc<S>, known_keys: impl IntoIterator<Item = TableRowKey>) -> Self {
        let mut domain: std::collections::BTreeSet<_> = known_keys.into_iter().collect();
        domain.extend((0..page.len()).filter_map(|i| page.row_key(i).cloned()));
        Self { page, domain }
    }
}
impl<S: TableSource> TableSource for TableRemotePage<S> {
    type Row = S::Row;
    fn scope(&self) -> &SharedString {
        self.page.scope()
    }
    fn revision(&self) -> u64 {
        self.page.revision()
    }
    fn len(&self) -> usize {
        self.page.len()
    }
    fn row_key(&self, ix: usize) -> Option<&TableRowKey> {
        self.page.row_key(ix)
    }
    fn position_of(&self, key: &TableRowKey) -> Option<usize> {
        self.page.position_of(key)
    }
    fn row(&self, key: &TableRowKey) -> Option<&Self::Row> {
        self.page.row(key)
    }
    fn contains_key(&self, key: &TableRowKey) -> bool {
        self.domain.contains(key)
    }
}
pub struct TableLocalQuery;
/// Stage timings for the opt-in headless query probe. Excludes fixture/oracle
/// setup and accepting the resulting view in the host.
#[cfg(feature = "headless-tests")]
#[doc(hidden)]
#[derive(Default)]
#[non_exhaustive]
pub struct TableQueryTimings {
    pub validation: std::time::Duration,
    pub filter: std::time::Duration,
    pub sort: std::time::Duration,
    pub projection: std::time::Duration,
}
impl TableLocalQuery {
    pub fn execute<S: TableSource>(
        source: Rc<S>,
        columns: Rc<[TableColumn<S::Row>]>,
        query: TableQuery,
        revision: u64,
    ) -> Result<TableView<TableProjection<S>>, TableError> {
        Self::execute_inner(source, columns, query, revision, |_, _| {}, |_, _| {})
    }
    /// Aggregates the complete filtered order before pagination, without a
    /// second filter/sort pass or retaining additional records.
    pub fn execute_with_summary<S: TableSource, A>(
        source: Rc<S>,
        columns: Rc<[TableColumn<S::Row>]>,
        query: TableQuery,
        revision: u64,
        aggregate: impl FnOnce(&S, &[TableRowKey]) -> A,
    ) -> Result<(TableView<TableProjection<S>>, A), TableError> {
        let mut aggregate = Some(aggregate);
        let mut summary = None;
        let view = Self::execute_inner(
            source,
            columns,
            query,
            revision,
            |_, _| {},
            |source, keys| {
                summary = Some(aggregate.take().expect("one aggregate per query")(
                    source, keys,
                ));
            },
        )?;
        Ok((view, summary.expect("successful query computes summary")))
    }
    #[cfg(feature = "headless-tests")]
    #[doc(hidden)]
    pub fn execute_profiled<S: TableSource>(
        source: Rc<S>,
        columns: Rc<[TableColumn<S::Row>]>,
        query: TableQuery,
        revision: u64,
    ) -> Result<(TableView<TableProjection<S>>, TableQueryTimings), TableError> {
        let mut timings = TableQueryTimings::default();
        let view = Self::execute_inner(
            source,
            columns,
            query,
            revision,
            |stage, elapsed| match stage {
                0 => timings.validation = elapsed,
                1 => timings.filter = elapsed,
                2 => timings.sort = elapsed,
                _ => timings.projection = elapsed,
            },
            |_, _| {},
        )?;
        Ok((view, timings))
    }
    fn execute_inner<S: TableSource>(
        source: Rc<S>,
        columns: Rc<[TableColumn<S::Row>]>,
        mut query: TableQuery,
        revision: u64,
        mut observe: impl FnMut(u8, std::time::Duration),
        mut aggregate: impl FnMut(&S, &[TableRowKey]),
    ) -> Result<TableView<TableProjection<S>>, TableError> {
        let start = std::time::Instant::now();
        query.validate(&columns)?;
        observe(0, start.elapsed());
        let start = std::time::Instant::now();
        let filters: Vec<_> = query
            .filters
            .iter()
            .map(|(key, f)| {
                (
                    columns
                        .iter()
                        .find(|c| c.key() == key)
                        .unwrap()
                        .query
                        .as_ref()
                        .unwrap(),
                    f,
                )
            })
            .collect();
        let mut keys = Vec::with_capacity(source.len());
        for ix in 0..source.len() {
            let key = source.row_key(ix).ok_or(TableError::InvalidSource)?;
            let row = source.row(key).ok_or(TableError::InvalidSource)?;
            if filters
                .iter()
                .all(|(r, f)| (r.predicate)(row, f.value.as_ref()))
            {
                keys.push(key.clone());
            }
        }
        observe(1, start.elapsed());
        let start = std::time::Instant::now();
        let sorts: Vec<_> = query
            .sorts
            .iter()
            .map(|s| {
                (
                    columns
                        .iter()
                        .find(|c| c.key() == &s.column)
                        .unwrap()
                        .query
                        .as_ref()
                        .unwrap()
                        .compare
                        .as_ref()
                        .unwrap(),
                    s.direction,
                )
            })
            .collect();
        if !sorts.is_empty() {
            keys.sort_by(|a, b| {
                let a = source.row(a).unwrap();
                let b = source.row(b).unwrap();
                sorts
                    .iter()
                    .map(|(compare, direction)| {
                        let order = compare(a, b);
                        if *direction == TableSortDirection::Descending {
                            order.reverse()
                        } else {
                            order
                        }
                    })
                    .find(|o| *o != Ordering::Equal)
                    .unwrap_or(Ordering::Equal)
            });
        }
        observe(2, start.elapsed());
        let start = std::time::Instant::now();
        let total = keys.len();
        aggregate(&source, &keys);
        if let Some(page) = &mut query.page {
            page.number = page.number.min(total.div_ceil(page.size).max(1));
            let start = (page.number - 1) * page.size;
            keys = keys.into_iter().skip(start).take(page.size).collect();
        }
        let result = TableView::new(
            Rc::new(TableProjection::try_new(source, revision, keys)?),
            columns,
        )
        .with_query(query)
        .with_result(TableDataMode::Local, TableTotal::Known(total));
        observe(3, start.elapsed());
        Ok(result)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableQuerySource {
    Sort,
    Filter,
    Pagination,
    Refresh,
    External,
}
#[derive(Clone, Debug)]
pub struct TableQueryRequest {
    id: TableRequestId,
    scope: SharedString,
    base_revision: u64,
    before: TableQuery,
    target: TableQuery,
    source: TableQuerySource,
}
impl TableQueryRequest {
    pub fn id(&self) -> TableRequestId {
        self.id
    }
    pub fn before(&self) -> &TableQuery {
        &self.before
    }
    pub fn target(&self) -> &TableQuery {
        &self.target
    }
    pub fn base_revision(&self) -> u64 {
        self.base_revision
    }
    pub fn source(&self) -> TableQuerySource {
        self.source
    }
    pub fn apply_to<S: TableSource>(
        &self,
        accepted: &TableView<S>,
    ) -> Result<TableQuery, TableError> {
        if self.scope != *accepted.source.scope() {
            return Err(TableError::StaleRequest);
        }
        self.target.validate(&accepted.columns)?;
        Ok(self.target.clone())
    }
}
impl<D: TableDelegate> YueTableState<D> {
    pub fn is_current_request(&self, id: TableRequestId) -> bool {
        id.owner == self.owner && self.requests.contains(&id)
    }
    pub fn request_query(
        &mut self,
        query: TableQuery,
        source: TableQuerySource,
        cx: &mut gpui_kit::Context<Self>,
    ) -> Result<TableRequestId, TableError> {
        let view = self.view().ok_or(TableError::Unbound)?;
        if self
            .presentation
            .as_ref()
            .is_some_and(|p| p.disabled || p.readonly)
        {
            return Err(TableError::UnsupportedOperation("query is disabled"));
        }
        query.validate(&view.columns)?;
        let before = view.query.clone();
        let scope = view.source.scope().clone();
        let base_revision = view.revision;
        if self.requests.len() >= 64 && self.query_draft.is_none() {
            return Err(TableError::TooManyPendingRequests);
        }
        if let Some((id, _)) = self.query_draft.take() {
            self.requests.remove(&id);
        }
        self.sequence += 1;
        let id = TableRequestId {
            owner: self.owner,
            sequence: self.sequence,
        };
        self.requests.insert(id);
        self.pending = Some(id);
        self.query_draft = Some((id, query.clone()));
        self.cancel_resize(cx);
        cx.emit(TableEvent::QueryRequested(TableQueryRequest {
            id,
            scope,
            base_revision,
            before,
            target: query,
            source,
        }));
        cx.notify();
        Ok(id)
    }
    fn next_query(&self) -> Result<TableQuery, TableError> {
        Ok(self
            .query_draft
            .as_ref()
            .map(|(_, q)| q)
            .unwrap_or(&self.view().ok_or(TableError::Unbound)?.query)
            .clone())
    }
    pub fn request_sort(
        &mut self,
        column: &TableColumnKey,
        multiple: bool,
        cx: &mut gpui_kit::Context<Self>,
    ) -> Result<TableRequestId, TableError> {
        let mut query = self.next_query()?;
        let next = match query
            .sorts
            .iter()
            .find(|s| &s.column == column)
            .map(|s| s.direction)
        {
            None => Some(TableSortDirection::Ascending),
            Some(TableSortDirection::Ascending) => Some(TableSortDirection::Descending),
            Some(TableSortDirection::Descending) => None,
        };
        if !multiple {
            query.sorts.retain(|s| &s.column == column);
        }
        if let Some(ix) = query.sorts.iter().position(|s| &s.column == column) {
            if let Some(direction) = next {
                query.sorts[ix] = TableSort::new(column.clone(), direction);
            } else {
                query.sorts.remove(ix);
            }
        } else if let Some(direction) = next {
            query.sorts.push(TableSort::new(column.clone(), direction));
        }
        query.first_page();
        self.request_query(query, TableQuerySource::Sort, cx)
    }
    pub fn request_filter<V: Clone + PartialEq + 'static>(
        &mut self,
        key: &TableFilterKey<V>,
        value: Option<TableFilterValue<V>>,
        cx: &mut gpui_kit::Context<Self>,
    ) -> Result<TableRequestId, TableError> {
        if !self
            .view()
            .ok_or(TableError::Unbound)?
            .columns
            .iter()
            .any(|column| {
                column.key() == &key.column
                    && column
                        .query
                        .as_ref()
                        .is_some_and(|rule| rule.id == key.rule)
            })
        {
            return Err(TableError::InvalidQuery("stale filter key".into()));
        }
        let query = self.next_query()?.with_filter(key, value)?;
        self.request_query(query, TableQuerySource::Filter, cx)
    }
    pub fn request_page(
        &mut self,
        number: usize,
        size: usize,
        cx: &mut gpui_kit::Context<Self>,
    ) -> Result<TableRequestId, TableError> {
        let query = self.next_query()?.with_page(number, size)?;
        self.request_query(query, TableQuerySource::Pagination, cx)
    }
    pub fn request_refresh(
        &mut self,
        cx: &mut gpui_kit::Context<Self>,
    ) -> Result<TableRequestId, TableError> {
        self.request_query(self.next_query()?, TableQuerySource::Refresh, cx)
    }
}

/// Silent controlled adapter: unrelated table updates preserve jump-input drafts.
pub fn sync_table_pagination<S: TableSource>(
    view: &TableView<S>,
    pagination: &gpui_kit::Entity<crate::YuePaginationState>,
    window: &mut Window,
    cx: &mut App,
) -> Result<(), TableError> {
    let (TableTotal::Known(total), Some(page)) = (view.total(), view.query().page()) else {
        return Err(TableError::UnsupportedOperation(
            "pagination requires known total and a page",
        ));
    };
    let value = crate::PaginationValue::new(total, page.number(), page.size())
        .map_err(|e| TableError::InvalidQuery(e.to_string().into()))?;
    pagination.update(cx, |s, cx| {
        if s.value() != value {
            s.set_value(value, window, cx);
        }
        s.set_loading(
            matches!(
                view.load(),
                TableLoadState::Loading | TableLoadState::Refreshing
            ),
            window,
            cx,
        );
    });
    Ok(())
}
