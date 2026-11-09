//! Presentation-only row extents; selection and expansion remain in Base.
use super::*;
use gpui_kit::{Bounds, Element, GlobalElementId, InspectorElementId, LayoutId, Pixels, px};
use std::{cell::RefCell, collections::HashMap, time::Instant};

// Bound each animation frame independently of the total tree size. Larger
// subtree transactions still rotate their indicator, but settle row geometry.
const MAX_ANIMATED_ROWS: usize = 128;
const MIN_EXTENT: f32 = 0.01;
pub(super) type Snapshot = Rc<gpui_kit::PaintSnapshot>;
#[derive(Default)]
pub(super) struct PaintCache {
    rows: HashMap<SharedString, Snapshot>,
}
pub(super) type SharedPaintCache = Rc<RefCell<PaintCache>>;
struct Tween {
    key: SharedString,
    from: Pixels,
    to: Pixels,
    current: Pixels,
    snapshot: Option<Snapshot>,
}
#[derive(Default)]
pub(super) struct TreeMotion {
    pub rows: Rc<Vec<TreeRow>>,
    pub positions: Rc<HashMap<ElementId, usize>>,
    pub paint: SharedPaintCache,
    tweens: Vec<Tween>,
    started: Option<Instant>,
    timing: Option<gpui_kit::base::Timing>,
    pub requested: bool,
    pub geometry_key: String,
}
impl TreeMotion {
    pub fn running(&self) -> bool {
        self.started.is_some()
    }
    fn extent(&self, key: &SharedString, height: Pixels) -> Pixels {
        self.tweens
            .iter()
            .find(|t| &t.key == key)
            .map_or(height, |t| t.current)
    }
    pub fn ghost(&self, key: &SharedString) -> Option<Snapshot> {
        self.tweens
            .iter()
            .find(|t| &t.key == key)
            .and_then(|t| t.snapshot.clone())
    }
    pub fn reset(&mut self, rows: Rc<Vec<TreeRow>>, positions: Rc<HashMap<ElementId, usize>>) {
        self.rows = rows;
        self.positions = positions;
        self.tweens.clear();
        self.timing = None;
        self.started = None;
        self.requested = false;
    }
    /// Only a bounded changed interval is animated. Stable prefix/suffix data
    /// are shared with the virtualizer, not rebuilt by the animation frames.
    pub fn retarget(
        &mut self,
        rows: Rc<Vec<TreeRow>>,
        height: Pixels,
        now: Instant,
        theme: YueTheme,
    ) -> Option<Vec<Pixels>> {
        let mut prefix = 0;
        while prefix < rows.len().min(self.rows.len())
            && rows[prefix].key() == self.rows[prefix].key()
        {
            prefix += 1;
        }
        let mut suffix = 0;
        while suffix < rows.len().min(self.rows.len()) - prefix
            && rows[rows.len() - 1 - suffix].key() == self.rows[self.rows.len() - 1 - suffix].key()
        {
            suffix += 1;
        }
        if rows.len() + self.rows.len() - 2 * (prefix + suffix) > MAX_ANIMATED_ROWS {
            return None;
        }
        let target_keys: std::collections::HashSet<_> =
            rows.iter().map(|r| r.key().clone()).collect();
        let mut merged = rows[..prefix].to_vec();
        merged.extend(
            self.rows[prefix..self.rows.len() - suffix]
                .iter()
                .filter(|r| !target_keys.contains(r.key()))
                .cloned(),
        );
        merged.extend_from_slice(&rows[prefix..]);
        let mut extents = Vec::with_capacity(merged.len());
        let mut tweens = Vec::new();
        let mut opening = false;
        for row in &merged {
            let from = if self
                .positions
                .contains_key(&ElementId::Name(row.key().clone()))
            {
                self.extent(row.key(), height)
            } else {
                px(0.)
            };
            let to = if target_keys.contains(row.key()) {
                height
            } else {
                px(0.)
            };
            extents.push(from.max(px(MIN_EXTENT)));
            if (from - to).abs() > px(MIN_EXTENT) {
                opening |= to > from;
                tweens.push(Tween {
                    key: row.key().clone(),
                    from,
                    to,
                    current: from,
                    snapshot: self
                        .ghost(row.key())
                        .or_else(|| self.paint.borrow().rows.get(row.key()).cloned()),
                });
            }
        }
        if tweens.is_empty() || tweens.len() > MAX_ANIMATED_ROWS {
            return None;
        }
        if tweens.iter().any(|t| t.to > t.from) && tweens.iter().any(|t| t.to < t.from) {
            // Several unrelated branches changed before the next frame. Avoid
            // reordering a closing ghost across another branch's new children.
            return None;
        }
        let duration = if opening {
            crate::MotionDuration::Enter
        } else {
            crate::MotionDuration::Exit
        }
        .duration(theme);
        // A short reverse covers only the remaining distance.
        let distance = tweens
            .iter()
            .map(|t| f32::from((t.to - t.from).abs()))
            .sum::<f32>();
        let full = f32::from(height) * tweens.len() as f32;
        let duration = duration.mul_f32((distance / full).clamp(0., 1.));
        let ease = if opening {
            crate::MotionEase::Enter
        } else {
            crate::MotionEase::Stow
        };
        self.timing = Some(
            gpui_kit::base::Timing::new(duration)
                .ease(gpui_kit::base::Easing::Custom(Rc::from(ease.easing(theme)))),
        );
        self.started = Some(now);
        self.tweens = tweens;
        self.positions = Rc::new(
            merged
                .iter()
                .enumerate()
                .map(|(i, r)| (ElementId::Name(r.key().clone()), i))
                .collect(),
        );
        self.rows = Rc::new(merged);
        Some(extents)
    }
    pub fn sample(&mut self, now: Instant) -> Vec<crate::VirtualizerSizeUpdate> {
        let Some(started) = self.started else {
            return vec![];
        };
        let sample = self
            .timing
            .as_ref()
            .unwrap()
            .sample(now.saturating_duration_since(started));
        let mut updates = Vec::with_capacity(self.tweens.len());
        let reveal =
            self.tweens.iter().all(|t| t.to > t.from) || self.tweens.iter().all(|t| t.to < t.from);
        let mut remaining = if reveal {
            self.tweens
                .iter()
                .map(|t| t.from + (t.to - t.from) * sample.directed_progress)
                .fold(px(0.), |a, b| a + b)
        } else {
            px(0.)
        };
        for t in &mut self.tweens {
            if reveal {
                let full = t.from.max(t.to);
                t.current = remaining.clamp(px(0.), full);
                remaining = (remaining - full).max(px(0.));
            } else {
                t.current = t.from + (t.to - t.from) * sample.directed_progress;
            }
            updates.push(crate::VirtualizerSizeUpdate::new(
                t.key.clone(),
                t.current.max(px(MIN_EXTENT)),
            ));
        }
        if sample.finished {
            self.started = None;
        }
        updates
    }
}

/// Capture mounted rows only. Closing rows replay commands, with no input,
/// focus handles, entities or accessibility descendants kept alive.
pub(super) struct CaptureRow {
    pub child: AnyElement,
    /// None marks the viewport paint boundary, clearing last frame's cache.
    pub key: Option<SharedString>,
    pub cache: SharedPaintCache,
}
impl IntoElement for CaptureRow {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for CaptureRow {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        w: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(w, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        w: &mut Window,
        cx: &mut App,
    ) {
        self.child.prepaint(w, cx);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        b: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        w: &mut Window,
        cx: &mut App,
    ) {
        let Some(key) = &self.key else {
            self.cache.borrow_mut().rows.clear();
            self.child.paint(w, cx);
            return;
        };
        let (_, snapshot) = w.capture_paint(b.origin, |w| self.child.paint(w, cx));
        self.cache
            .borrow_mut()
            .rows
            .insert(key.clone(), Rc::new(snapshot));
    }
}
