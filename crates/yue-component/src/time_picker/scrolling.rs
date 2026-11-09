//! Finite selection positioning. Native wheel scrolling remains the only input owner.
use crate::{
    YueTheme,
    motion::{MotionDuration, MotionEase},
};
use gpui_kit::base::motion::{self, MotionStatus, Transition};
use gpui_kit::{App, ElementId, ScrollHandle, Window, point, px};
use std::{cell::RefCell, rc::Rc, time::Duration};

#[derive(Default)]
struct Position {
    pending: Option<(usize, bool)>,
    target: Option<f32>,
    geometry: Option<(f32, f32, usize)>,
    selected: usize,
}
#[derive(Clone, Default)]
pub(super) struct ColumnPosition(Rc<RefCell<Position>>);
impl ColumnPosition {
    pub(super) fn request(&self, index: usize, animate: bool) {
        self.0.borrow_mut().pending = Some((index, animate));
    }
    pub(super) fn cancel(&self) {
        let mut state = self.0.borrow_mut();
        state.pending = None;
        state.target = None;
    }
    pub(super) fn position(
        &self,
        id: ElementId,
        scroll: &ScrollHandle,
        geometry: (f32, f32, usize),
        theme: YueTheme,
        w: &mut Window,
        cx: &mut App,
    ) {
        let mut state = self.0.borrow_mut();
        let (row, rows, count) = geometry;
        let resized = state.geometry != Some(geometry);
        state.geometry = Some(geometry);
        let pending = state
            .pending
            .take()
            .or_else(|| resized.then_some((state.selected, false)));
        if let Some((index, animate)) = pending {
            state.selected = index;
            let target =
                -(index.saturating_sub(2) as f32 * row).min(((count as f32 - rows) * row).max(0.));
            if !animate || resized {
                state.target = None;
                scroll.set_offset(point(px(0.), px(target)));
            } else {
                state.target = Some(target);
            }
        }
        let current = f32::from(scroll.offset().y);
        let target = state.target.unwrap_or(current);
        // Reset the keyed channel to the actual native position while idle/cancelled.
        // A later selection transitions from that position rather than a stale target.
        let policy = if state.target.is_some() {
            crate::motion::transition(theme, MotionDuration::Interaction, MotionEase::Standard)
        } else {
            Transition::new(Duration::ZERO)
        };
        let sample = motion::transition_with_status(id, target, policy, w, cx);
        if state.target.is_some() {
            scroll.set_offset(point(px(0.), px(sample.value)));
            if matches!(sample.status, MotionStatus::Finished | MotionStatus::Idle) {
                state.target = None;
            }
        }
    }
}
