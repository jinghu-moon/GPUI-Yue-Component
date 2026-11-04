//! Sparse explicit spans. Geometry never feeds back into track sizing.
use super::*;
use std::{collections::BTreeMap, ops::Range};

#[derive(Clone, Debug)]
pub struct VirtualGridSpan {
    origin: VirtualGridCellKey,
    rows: usize,
    columns: usize,
}
impl VirtualGridSpan {
    pub fn new(origin: VirtualGridCellKey, rows: usize, columns: usize) -> Self {
        Self {
            origin,
            rows,
            columns,
        }
    }
    pub fn origin(&self) -> &VirtualGridCellKey {
        &self.origin
    }
    pub fn row_span(&self) -> usize {
        self.rows
    }
    pub fn column_span(&self) -> usize {
        self.columns
    }
}

pub(super) struct ResolvedSpan {
    pub rows: Range<usize>,
    pub columns: Range<usize>,
}
pub(super) struct GridSpans {
    pub entries: Vec<ResolvedSpan>,
    pub generations: (u64, u64),
    pub frozen: (usize, usize),
    max_end: Vec<usize>,
}
impl GridSpans {
    pub fn new(
        rows: &VirtualizerState,
        columns: &VirtualizerState,
        descriptors: &[VirtualGridSpan],
        frozen: (usize, usize),
    ) -> Result<Self, VirtualizerError> {
        VirtualGrid::validate_axes(rows, columns)?;
        let rs = rows.source_for_render();
        let cs = columns.source_for_render();
        let mut entries = Vec::with_capacity(descriptors.len());
        for descriptor in descriptors {
            if descriptor.rows == 0 || descriptor.columns == 0 {
                return Err(VirtualizerError::InvalidGeometry("span count"));
            }
            let r = rs
                .index_of(descriptor.origin.row())
                .ok_or(VirtualizerError::MissingKey)?;
            let c = cs
                .index_of(descriptor.origin.column())
                .ok_or(VirtualizerError::MissingKey)?;
            let re = r
                .checked_add(descriptor.rows)
                .filter(|end| *end <= rs.len())
                .ok_or(VirtualizerError::InvalidGeometry("span bounds"))?;
            let ce = c
                .checked_add(descriptor.columns)
                .filter(|end| *end <= cs.len())
                .ok_or(VirtualizerError::InvalidGeometry("span bounds"))?;
            if r < frozen.0 && re > frozen.0 || c < frozen.1 && ce > frozen.1 {
                return Err(VirtualizerError::Unsupported(
                    "span crosses frozen boundary",
                ));
            }
            entries.push(ResolvedSpan {
                rows: r..re,
                columns: c..ce,
            });
        }
        entries.sort_unstable_by_key(|span| (span.rows.start, span.columns.start));
        // End events precede starts: adjacent rectangles do not overlap.
        let mut events = Vec::with_capacity(entries.len() * 2);
        for (i, span) in entries.iter().enumerate() {
            events.push((span.rows.start, true, i));
            events.push((span.rows.end, false, i));
        }
        events.sort_unstable();
        let mut active: BTreeMap<usize, usize> = BTreeMap::new();
        for (_, start, i) in events {
            let span = &entries[i];
            if !start {
                active.remove(&span.columns.start);
                continue;
            }
            if active
                .range(..span.columns.end)
                .next_back()
                .is_some_and(|(_, end)| *end > span.columns.start)
            {
                return Err(VirtualizerError::InvalidGeometry("overlapping spans"));
            }
            active.insert(span.columns.start, span.columns.end);
        }
        let mut result = Self {
            max_end: vec![0; entries.len().saturating_mul(4)],
            entries,
            generations: (rows.generation(), columns.generation()),
            frozen,
        };
        if !result.entries.is_empty() {
            result.build(1, 0, result.entries.len());
        }
        Ok(result)
    }
    fn build(&mut self, node: usize, lo: usize, hi: usize) -> usize {
        let end = if hi - lo == 1 {
            self.entries[lo].rows.end
        } else {
            let mid = (lo + hi) / 2;
            let a = self.build(node * 2, lo, mid);
            let b = self.build(node * 2 + 1, mid, hi);
            a.max(b)
        };
        self.max_end[node] = end;
        end
    }
    pub fn covering(&self, row: usize, column: usize) -> Option<usize> {
        if self.entries.is_empty() {
            None
        } else {
            self.find(1, 0, self.entries.len(), row, column)
        }
    }
    fn find(&self, node: usize, lo: usize, hi: usize, row: usize, column: usize) -> Option<usize> {
        if self.max_end[node] <= row || self.entries[lo].rows.start > row {
            return None;
        }
        if hi - lo == 1 {
            return self.entries[lo].columns.contains(&column).then_some(lo);
        }
        let mid = (lo + hi) / 2;
        self.find(node * 2, lo, mid, row, column)
            .or_else(|| self.find(node * 2 + 1, mid, hi, row, column))
    }
}
