use super::*;
use gpui_kit::{Context, Pixels, px};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Default)]
pub struct TableRowSizing {
    pub(super) kind: RowSizingKind,
}
#[derive(Clone, Default)]
pub(super) enum RowSizingKind {
    #[default]
    Fixed,
    Known(Rc<BTreeMap<TableRowKey, Rems>>),
    Measured(Rems),
}
impl TableRowSizing {
    pub fn fixed() -> Self {
        Self::default()
    }
    pub fn known(
        heights: impl IntoIterator<Item = (TableRowKey, Rems)>,
    ) -> Result<Self, TableError> {
        let mut values = BTreeMap::new();
        for (key, height) in heights {
            validate_height(height)?;
            if values.insert(key.clone(), height).is_some() {
                return Err(TableError::DuplicateRowKey(key));
            }
        }
        Ok(Self {
            kind: RowSizingKind::Known(Rc::new(values)),
        })
    }
    pub fn measured(estimate: Rems) -> Result<Self, TableError> {
        validate_height(estimate)?;
        Ok(Self {
            kind: RowSizingKind::Measured(estimate),
        })
    }
    pub fn is_measured(&self) -> bool {
        matches!(self.kind, RowSizingKind::Measured(_))
    }
    pub(super) fn same(&self, other: &Self) -> bool {
        match (&self.kind, &other.kind) {
            (RowSizingKind::Fixed, RowSizingKind::Fixed) => true,
            (RowSizingKind::Measured(a), RowSizingKind::Measured(b)) => a == b,
            (RowSizingKind::Known(a), RowSizingKind::Known(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
    pub(super) fn identity(&self) -> u64 {
        match &self.kind {
            RowSizingKind::Fixed => 0,
            RowSizingKind::Measured(estimate) => 1 | ((estimate.0.to_bits() as u64) << 2),
            RowSizingKind::Known(values) => (Rc::as_ptr(values) as usize as u64) | 2,
        }
    }
    pub(super) fn validate<S: TableSource>(&self, source: &S) -> Result<(), TableError> {
        if let RowSizingKind::Known(heights) = &self.kind
            && ((0..source.len()).any(|i| {
                source
                    .row_key(i)
                    .is_none_or(|key| !heights.contains_key(key))
            }) || heights
                .keys()
                .any(|key| !source.contains_key(key) && source.position_of(key).is_none()))
        {
            return Err(TableError::InvalidGeometry);
        }
        Ok(())
    }
}
fn validate_height(height: Rems) -> Result<(), TableError> {
    if height.0.is_finite() && height.0 > 0. {
        Ok(())
    } else {
        Err(TableError::InvalidGeometry)
    }
}

#[derive(PartialEq)]
pub(super) struct Geometry {
    pub source: u64,
    pub height: f32,
    pub width: f32,
    pub header: bool,
    pub depth: usize,
    pub footer: bool,
    pub sizing: u64,
    pub measurement_epoch: u64,
}
#[derive(PartialEq)]
pub(super) struct MeasurementConstraints {
    pub scope: SharedString,
    pub columns: usize,
    pub widths: Vec<(TableColumnKey, u32)>,
    pub rem: u32,
    pub minimum: u32,
    pub font: u32,
    pub line: u32,
    pub padding: u32,
    pub family: SharedString,
}
#[derive(Default)]
pub(super) struct Measurements {
    pub epoch: u64,
    pub constraints: Option<MeasurementConstraints>,
    pub sizes: HashMap<TableRowKey, (u64, Pixels, u64)>,
    pub pending: HashMap<TableRowKey, (u64, Pixels)>,
    pub scheduled: bool,
    pub clock: u64,
}
impl<D: TableDelegate> YueTableState<D> {
    pub(super) fn measure_cell(
        &mut self,
        key: TableRowKey,
        epoch: u64,
        revision: u64,
        height: Pixels,
        cx: &mut Context<Self>,
    ) {
        if epoch != self.measurements.epoch || !f32::from(height).is_finite() || height <= px(0.) {
            return;
        }
        let Some(view) = self.view() else {
            return;
        };
        if !view.sizing.is_measured()
            || view.source.position_of(&key).is_none()
            || view.source.row_revision(&key) != revision
        {
            return;
        }
        self.measurements
            .pending
            .entry(key)
            .and_modify(|(rev, h)| {
                if *rev == revision {
                    *h = (*h).max(height);
                } else {
                    *rev = revision;
                    *h = height;
                }
            })
            .or_insert((revision, height));
        if !self.measurements.scheduled {
            self.measurements.scheduled = true;
            let own = cx.weak_entity();
            cx.defer(move |cx| {
                let _ = own.update(cx, |s, cx| s.flush_measurements(cx));
            });
        }
    }
    fn flush_measurements(&mut self, cx: &mut Context<Self>) {
        self.measurements.scheduled = false;
        let Some(view) = self.accepted.clone() else {
            self.measurements.pending.clear();
            return;
        };
        if !view.sizing.is_measured() {
            self.measurements.pending.clear();
            return;
        }
        self.measurements.clock += 1;
        let clock = self.measurements.clock;
        let mut updates = Vec::new();
        let mut accepted = Vec::new();
        for (key, (revision, height)) in self.measurements.pending.drain() {
            if view.source.position_of(&key).is_none() || view.source.row_revision(&key) != revision
            {
                continue;
            }
            let id = state::row_id(view.source.scope(), &key);
            let current = self
                .measurements
                .sizes
                .get(&key)
                .map(|(_, height, _)| *height);
            if current.is_none_or(|current| (current - height).abs() > px(0.5)) {
                updates.push(crate::VirtualizerSizeUpdate::new(id, height));
            }
            accepted.push((key, (revision, height, clock)));
        }
        if !updates.is_empty()
            && crate::VirtualGrid::update_sizes(&self.rows, &self.columns, &updates, &[], cx)
                .is_err()
        {
            return;
        }
        self.measurements.sizes.extend(accepted);
        // Bound retained measurements independently of total records or prior pages.
        if self.measurements.sizes.len() > 4096 {
            let mut oldest: Vec<_> = self
                .measurements
                .sizes
                .iter()
                .map(|(key, (_, _, clock))| (key.clone(), *clock))
                .collect();
            oldest.sort();
            oldest.sort_by_key(|(_, clock)| *clock);
            for (key, _) in oldest
                .into_iter()
                .take(self.measurements.sizes.len() - 4096)
            {
                self.measurements.sizes.remove(&key);
            }
        }
        if !updates.is_empty() {
            cx.notify();
        }
    }
}
