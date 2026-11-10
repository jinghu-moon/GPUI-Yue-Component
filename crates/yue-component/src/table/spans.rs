use super::*;
use std::collections::HashMap;
#[derive(Clone, Debug)]
pub struct TableSpan {
    origin: TableCellKey,
    rows: usize,
    columns: usize,
}
impl TableSpan {
    pub fn new(origin: TableCellKey, rows: usize, columns: usize) -> Self {
        Self {
            origin,
            rows,
            columns,
        }
    }
    pub fn origin(&self) -> &TableCellKey {
        &self.origin
    }
    pub fn row_span(&self) -> usize {
        self.rows
    }
    pub fn column_span(&self) -> usize {
        self.columns
    }
}
#[derive(Default)]
struct SpanData {
    spans: Rc<[TableSpan]>,
    by_row: HashMap<TableRowKey, Vec<usize>>,
}
#[derive(Clone, Default)]
pub struct TableSpanPlan(Rc<SpanData>);
impl TableSpanPlan {
    pub fn new(spans: impl Into<Rc<[TableSpan]>>) -> Self {
        let spans = spans.into();
        let mut by_row: HashMap<_, Vec<_>> = HashMap::new();
        for (i, span) in spans.iter().enumerate() {
            by_row.entry(span.origin.row().clone()).or_default().push(i);
        }
        Self(Rc::new(SpanData { spans, by_row }))
    }
    pub fn descriptors(&self) -> &[TableSpan] {
        &self.0.spans
    }
    pub(super) fn identity(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }
    pub(super) fn same(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
            || self.descriptors().is_empty() && other.descriptors().is_empty()
    }
    pub(super) fn row_spans(&self, row: &TableRowKey) -> impl Iterator<Item = &TableSpan> {
        self.0
            .by_row
            .get(row)
            .into_iter()
            .flatten()
            .map(|i| &self.0.spans[*i])
    }
    pub(super) fn origin_for<S: TableSource>(
        &self,
        view: &TableView<S>,
        key: &TableCellKey,
    ) -> TableCellKey {
        let Some(row) = view.source.position_of(key.row()) else {
            return key.clone();
        };
        let mut columns: Vec<_> = view.columns.iter().filter(|c| c.visible).collect();
        columns.sort_by_key(|c| c.pin);
        let Some(col) = columns.iter().position(|c| c.key() == key.column()) else {
            return key.clone();
        };
        for span in self.descriptors() {
            let Some(r) = view.source.position_of(span.origin.row()) else {
                continue;
            };
            let Some(c) = columns.iter().position(|c| c.key() == span.origin.column()) else {
                continue;
            };
            if (r..r + span.rows).contains(&row) && (c..c + span.columns).contains(&col) {
                return span.origin.clone();
            }
        }
        key.clone()
    }
    pub(super) fn validate<S: TableSource>(&self, view: &TableView<S>) -> Result<(), TableError> {
        let mut columns: Vec<_> = view.columns.iter().filter(|c| c.visible).collect();
        columns.sort_by_key(|c| c.pin);
        let positions: HashMap<_, _> = columns
            .iter()
            .enumerate()
            .map(|(i, c)| (c.key(), i))
            .collect();
        let mut rectangles = Vec::new();
        for span in self.descriptors() {
            let r = view
                .source
                .position_of(span.origin.row())
                .ok_or(TableError::MissingKey)?;
            let c = *positions
                .get(span.origin.column())
                .ok_or(TableError::MissingKey)?;
            let end_row = r
                .checked_add(span.rows)
                .filter(|end| span.rows > 0 && *end <= view.source.len())
                .ok_or(TableError::InvalidGeometry)?;
            let end_col = c
                .checked_add(span.columns)
                .filter(|end| span.columns > 0 && *end <= columns.len())
                .ok_or(TableError::InvalidGeometry)?;
            if columns[c..end_col]
                .iter()
                .any(|col| col.pin != columns[c].pin || col.responsive.is_some())
                || (r..end_row).any(|ix| view.source.is_detail(view.source.row_key(ix).unwrap()))
            {
                return Err(TableError::UnsupportedCombination(
                    "spans cannot cross pin regions, responsive columns or details",
                ));
            }
            rectangles.push((r..end_row, c..end_col));
        }
        // Sparse descriptors, never N*C cells. Sort and active-row sweep rejects
        // even a malformed completely offscreen plan before accepting the view.
        rectangles.sort_by_key(|(r, c)| (r.start, c.start));
        let mut active: Vec<(std::ops::Range<usize>, std::ops::Range<usize>)> = Vec::new();
        for (rows, cols) in rectangles {
            active.retain(|(r, _)| r.end > rows.start);
            if active
                .iter()
                .any(|(_, c)| c.start < cols.end && cols.start < c.end)
            {
                return Err(TableError::UnsupportedCombination("overlapping spans"));
            }
            active.push((rows, cols));
        }
        Ok(())
    }
}
