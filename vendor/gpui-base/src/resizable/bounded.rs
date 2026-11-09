//! Opt-in adjacent-panel constraints; legacy redistribution stays in mod.rs.
use super::*;

pub(super) struct BoundedLifetime {
    pub _activation: gpui::Subscription,
}

/// Opt into bounded adjacent resizing, keyboard operation and default-size reset.
#[derive(Clone, Copy, Debug)]
pub struct ResizableInteraction {
    pub(super) step: Pixels,
    pub(super) large_step: Pixels,
    pub(super) disabled: bool,
}

impl ResizableInteraction {
    pub fn new(step: Pixels, large_step: Pixels) -> Self {
        assert!(step.as_f32().is_finite() && step > px(0.));
        assert!(large_step.as_f32().is_finite() && large_step >= step);
        Self {
            step,
            large_step,
            disabled: false,
        }
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

pub(super) struct PanelSpec {
    pub key: ElementId,
    pub initial: Option<Pixels>,
    pub range: Range<Pixels>,
    pub fixed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_minimum_size_is_not_an_unmeasured_sentinel() {
        let bounds = Bounds::new(gpui::point(px(0.), px(0.)), gpui::size(px(100.), px(160.)));
        let range = px(100.)..px(300.);
        let state = ResizableState {
            bounded: true,
            sizes: vec![px(100.)],
            panels: vec![ResizablePanelState {
                size: Some(px(100.)),
                bounds,
                size_range: range.clone(),
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(!state.panel_size_changes(0, bounds, &range, false));
    }
}

impl ResizableState {
    pub(super) fn sync_keyed_panels(
        &mut self,
        axis: Axis,
        specs: &[PanelSpec],
        cx: &mut Context<Self>,
    ) {
        let changed = !self.bounded
            || self.axis != axis
            || self.panels.len() != specs.len()
            || self.panels.iter().zip(specs).any(|(p, s)| {
                p.key.as_ref() != Some(&s.key)
                    || p.initial != s.initial
                    || p.size_range != s.range
                    || p.fixed != s.fixed
            });
        if !changed {
            return;
        }
        for (ix, spec) in specs.iter().enumerate() {
            assert!(
                !specs[..ix].iter().any(|s| s.key == spec.key),
                "duplicate resizable panel id"
            );
            assert!(
                spec.range.start >= px(0.)
                    && spec.range.start.as_f32().is_finite()
                    && spec.range.end >= spec.range.start
                    && spec.range.end.as_f32().is_finite()
            );
            assert!(
                spec.initial
                    .is_none_or(|v| v >= px(0.) && v.as_f32().is_finite())
            );
        }
        self.initial_fit_pending |= !self.bounded || self.panels.is_empty();
        let old_panels = std::mem::take(&mut self.panels);
        let old_sizes = std::mem::take(&mut self.sizes);
        for spec in specs {
            let old = old_panels
                .iter()
                .position(|p| p.key.as_ref() == Some(&spec.key));
            let mut panel = old.map(|ix| old_panels[ix].clone()).unwrap_or_default();
            let size = old
                .map(|ix| old_sizes[ix])
                .unwrap_or_else(|| spec.initial.unwrap_or(PANEL_MIN_SIZE));
            panel.key = Some(spec.key.clone());
            panel.initial = spec.initial;
            panel.size_range = spec.range.clone();
            panel.fixed = spec.fixed;
            panel.size = Some(size.clamp(spec.range.start, spec.range.end));
            self.sizes.push(panel.size.unwrap());
            self.panels.push(panel);
        }
        self.axis = axis;
        self.bounded = true;
        self.resizing_panel_ix = None;
        self.drag_origin = None;
        self.drag_changed = false;
        self.fit_bounded(cx);
        cx.notify();
    }

    pub(super) fn fit_bounded(&mut self, cx: &mut Context<Self>) {
        let total = self.container_size();
        if total <= px(0.) || self.panels.is_empty() {
            return;
        }
        if self.initial_fit_pending {
            let flexible = self.panels.iter().filter(|p| p.initial.is_none()).count();
            if flexible > 0 {
                let specified: f32 = self
                    .panels
                    .iter()
                    .filter_map(|p| p.initial)
                    .map(|v| v.as_f32())
                    .sum();
                let share = px((total.as_f32() - specified).max(0.) / flexible as f32);
                for (p, size) in self.panels.iter_mut().zip(&mut self.sizes) {
                    if p.initial.is_none() {
                        *size = share;
                        p.size = Some(share);
                    }
                }
            }
            self.initial_fit_pending = false;
        }
        let mut sizes: Vec<f64> = self
            .sizes
            .iter()
            .zip(&self.panels)
            .map(|(v, p)| (*v).clamp(p.size_range.start, p.size_range.end).as_f32() as f64)
            .collect();
        // Flexible panels share residual space proportionally; saturated panels
        // leave the active set. Impossible minima/maxima remain authoritative.
        let weights: Vec<f64> = sizes.iter().map(|v| v.max(1.)).collect();
        for _ in 0..=sizes.len() {
            let delta = total.as_f32() as f64 - sizes.iter().sum::<f64>();
            if delta.abs() < 0.001 {
                break;
            }
            let active: Vec<usize> = self
                .panels
                .iter()
                .enumerate()
                .filter_map(|(ix, p)| {
                    (!p.fixed
                        && if delta > 0. {
                            sizes[ix] < p.size_range.end.as_f32() as f64
                        } else {
                            sizes[ix] > p.size_range.start.as_f32() as f64
                        })
                    .then_some(ix)
                })
                .collect();
            let sum: f64 = active.iter().map(|ix| weights[*ix]).sum();
            if sum <= 0. {
                break;
            }
            for ix in active {
                let p = &self.panels[ix];
                sizes[ix] = (sizes[ix] + delta * weights[ix] / sum).clamp(
                    p.size_range.start.as_f32() as f64,
                    p.size_range.end.as_f32() as f64,
                );
            }
        }
        let sizes: Vec<Pixels> = sizes.into_iter().map(|v| px(v as f32)).collect();
        if self.sizes != sizes {
            self.sizes = sizes;
            for (panel, size) in self.panels.iter_mut().zip(&self.sizes) {
                panel.size = Some(*size);
            }
            cx.notify();
        }
    }

    /// Effective size limits for the panel preceding a divider. Includes both
    /// neighbours' constraints; None means there is no feasible divider motion.
    pub fn boundary_range(&self, ix: usize) -> Option<Range<Pixels>> {
        let left = self.panels.get(ix)?;
        let right = self.panels.get(ix + 1)?;
        let total = self.sizes[ix] + self.sizes[ix + 1];
        let min = left.size_range.start.max(total - right.size_range.end);
        let max = left.size_range.end.min(total - right.size_range.start);
        (min <= max).then_some(min..max)
    }

    pub(super) fn resize_bounded(
        &mut self,
        ix: usize,
        size: Pixels,
        cx: &mut Context<Self>,
    ) -> bool {
        if !size.as_f32().is_finite() {
            return false;
        }
        let Some(range) = self.boundary_range(ix) else {
            return false;
        };
        let next = size.clamp(range.start, range.end);
        let delta = next - self.sizes[ix];
        if delta.abs() < px(0.001) {
            return false;
        }
        self.sizes[ix] = next;
        self.sizes[ix + 1] -= delta;
        self.panels[ix].size = Some(next);
        self.panels[ix + 1].size = Some(self.sizes[ix + 1]);
        self.drag_changed |= self.resizing_panel_ix.is_some();
        cx.notify();
        true
    }

    pub(super) fn reset_boundary(&mut self, ix: usize, cx: &mut Context<Self>) -> bool {
        let Some(panel) = self.panels.get(ix) else {
            return false;
        };
        if ix + 1 >= self.sizes.len() {
            return false;
        }
        let size = panel
            .initial
            .unwrap_or((self.sizes[ix] + self.sizes[ix + 1]) / 2.);
        self.resize_bounded(ix, size, cx)
    }

    /// Restore all initial preferences in an opted-in bounded layout.
    /// Emits Resized only if the resulting layout changes.
    pub fn reset_layout(&mut self, cx: &mut Context<Self>) {
        if !self.bounded {
            return;
        }
        let before = self.sizes.clone();
        let flexible = self
            .panels
            .iter()
            .filter(|p| p.initial.is_none())
            .count()
            .max(1);
        let specified: Pixels = px(self
            .panels
            .iter()
            .filter_map(|p| p.initial)
            .map(|v| v.as_f32())
            .sum());
        let equal = (self.container_size() - specified).max(px(0.)) / flexible as f32;
        for (panel, size) in self.panels.iter_mut().zip(&mut self.sizes) {
            *size = panel
                .initial
                .unwrap_or(equal)
                .clamp(panel.size_range.start, panel.size_range.end);
            panel.size = Some(*size);
        }
        self.fit_bounded(cx);
        if self.sizes != before {
            self.done_resizing(cx);
            cx.notify();
        }
    }

    /// Restore a measured snapshot without emitting a user commit. Returns false
    /// for invalid input or before this state has been attached to a bounded group.
    pub fn set_sizes(&mut self, sizes: &[Pixels], cx: &mut Context<Self>) -> bool {
        if !self.bounded
            || sizes.len() != self.panels.len()
            || sizes.iter().any(|s| !s.as_f32().is_finite() || *s < px(0.))
        {
            return false;
        }
        self.sizes = sizes.to_vec();
        for (panel, size) in self.panels.iter_mut().zip(&self.sizes) {
            panel.size = Some(*size);
        }
        self.fit_bounded(cx);
        self.resizing_panel_ix = None;
        cx.notify();
        true
    }

    pub fn is_resizing(&self) -> bool {
        self.resizing_panel_ix.is_some()
    }
}
