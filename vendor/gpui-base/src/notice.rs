//! Controlled inline notice lifecycle. No presentation or motion policy.
use gpui::{App, Context, EntityId, FocusHandle, Subscription, Window};
#[cfg(not(target_family = "wasm"))]
use std::time::Instant;
use std::{rc::Rc, time::Duration};
#[cfg(target_family = "wasm")]
use web_time::Instant;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NoticeCloseReason {
    CloseButton,
    Timeout,
}
type CloseHandler = dyn Fn(NoticeCloseReason, &mut Window, &mut App);
type ClosedHandler = dyn Fn(&mut Window, &mut App);
/// Internal presentation state for a controlled, inline surface. The owner
/// commits visibility. Timed close requests are one-shot per visible generation.
pub struct NoticeController {
    focus: FocusHandle,
    visible: bool,
    initialized: bool,
    generation: u64,
    awaiting_close: bool,
    delivered: bool,
    timeout_requested: bool,
    duration: Option<Duration>,
    remaining: Option<Duration>,
    stamp: Instant,
    hovered: bool,
    focused: bool,
    exposed: bool,
    active: bool,
    owner: Option<EntityId>,
    close: Option<Rc<CloseHandler>>,
    closed: Option<Rc<ClosedHandler>>,
    _subscriptions: Vec<Subscription>,
}
impl NoticeController {
    pub fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        let subscriptions = vec![
            cx.observe_window_activation(w, |s, w, cx| s.availability(w, cx)),
            cx.observe_window_visibility(w, |s, _, w, cx| s.availability(w, cx)),
            cx.on_focus_in(&focus, w, |s, w, cx| {
                s.advance(cx.background_executor().now());
                s.focused = true;
                s.refresh(w, cx);
            }),
            cx.on_focus_out(&focus, w, |s, _, w, cx| {
                s.advance(cx.background_executor().now());
                s.focused = false;
                s.refresh(w, cx);
            }),
        ];
        Self {
            focus,
            visible: false,
            initialized: false,
            generation: 0,
            awaiting_close: false,
            delivered: false,
            timeout_requested: false,
            duration: None,
            remaining: None,
            stamp: cx.background_executor().now(),
            hovered: false,
            focused: false,
            exposed: false,
            active: w.is_visible() && w.is_window_active(),
            owner: None,
            close: None,
            closed: None,
            _subscriptions: subscriptions,
        }
    }
    pub fn focus(&self) -> FocusHandle {
        self.focus.clone()
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn sync(
        &mut self,
        visible: bool,
        duration: Option<Duration>,
        owner: EntityId,
        focus_after: Option<FocusHandle>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        assert!(
            duration.is_none_or(|d| !d.is_zero()),
            "notice duration must be nonzero"
        );
        if self.visible != visible || (visible && self.duration != duration) || !self.initialized {
            let hiding = self.initialized && self.visible && !visible;
            self.generation += 1;
            self.awaiting_close = hiding;
            self.delivered = false;
            self.visible = visible;
            self.duration = duration;
            self.remaining = duration;
            self.timeout_requested = false;
            self.stamp = cx.background_executor().now();
            self.exposed = false;
            self.hovered = false;
            if hiding && self.focus.contains_focused(w, cx) {
                // Delay until the live descendants have been detached. Do not
                // redirect focus that a host callback has already moved.
                let old = w.focused(cx);
                let focus = self.focus.clone();
                let weak = cx.weak_entity();
                let generation = self.generation;
                w.on_next_frame(move |w, cx| {
                    if weak.upgrade().is_some_and(|s| {
                        let s = s.read(cx);
                        !s.visible && s.generation == generation
                    }) && (w.focused(cx) == old || focus.contains_focused(w, cx))
                    {
                        if let Some(target) =
                            focus_after.filter(|f| crate::Root::is_focus_available(f, w))
                        {
                            target.focus(w, cx);
                        } else {
                            w.blur(cx);
                            crate::Root::focus_next(w, cx);
                        }
                    }
                });
            }
        }
        self.initialized = true;
        self.owner = Some(owner);
    }
    pub fn handlers(&mut self, close: Option<Rc<CloseHandler>>, closed: Option<Rc<ClosedHandler>>) {
        self.close = close;
        self.closed = closed;
    }
    pub fn request_close(
        &mut self,
        reason: NoticeCloseReason,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.visible {
            return;
        }
        if reason == NoticeCloseReason::Timeout {
            if self.timeout_requested {
                return;
            }
            self.timeout_requested = true;
        }
        if let Some(handler) = self.close.clone() {
            let generation = self.generation;
            let weak = cx.weak_entity();
            w.defer(cx, move |w, cx| {
                if weak.upgrade().is_some_and(|s| {
                    let s = s.read(cx);
                    s.visible && s.generation == generation
                }) {
                    handler(reason, w, cx);
                }
            });
        }
    }
    pub fn settled(&mut self, generation: u64, open: bool, w: &mut Window, cx: &mut Context<Self>) {
        if !open
            && !self.visible
            && self.awaiting_close
            && !self.delivered
            && self.generation == generation
        {
            self.delivered = true;
            self.awaiting_close = false;
            if let Some(handler) = self.closed.clone() {
                let weak = cx.weak_entity();
                w.defer(cx, move |w, cx| {
                    if weak.upgrade().is_some_and(|s| {
                        let s = s.read(cx);
                        !s.visible && s.generation == generation
                    }) {
                        handler(w, cx);
                    }
                });
            }
        }
    }
    fn running(&self) -> bool {
        self.visible
            && self.exposed
            && self.active
            && !self.hovered
            && !self.focused
            && !self.timeout_requested
    }
    fn advance(&mut self, now: Instant) {
        if self.running() {
            if let Some(remaining) = self.remaining.as_mut() {
                *remaining = remaining.saturating_sub(now.saturating_duration_since(self.stamp));
            }
        }
        self.stamp = now;
    }
    fn availability(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.advance(cx.background_executor().now());
        self.active = w.is_visible() && w.is_window_active();
        self.refresh(w, cx);
    }
    pub fn set_hovered(&mut self, value: bool, w: &mut Window, cx: &mut Context<Self>) {
        if self.hovered != value {
            self.advance(cx.background_executor().now());
            self.hovered = value;
            self.refresh(w, cx);
        }
    }
    /// Sample once per displayed frame. Returns remaining fraction and whether
    /// another frame is needed; callers own viewport visibility and painting.
    pub fn sample(&mut self, exposed: bool, w: &mut Window, cx: &mut Context<Self>) -> (f32, bool) {
        let now = cx.background_executor().now();
        // Focus must be a distinct pause reason, independent of hover state.
        let focused = self.focus.contains_focused(w, cx);
        self.advance(now);
        self.focused = focused;
        self.exposed = exposed;
        if self.running() && self.remaining == Some(Duration::ZERO) {
            self.request_close(NoticeCloseReason::Timeout, w, cx);
        }
        let progress = self
            .remaining
            .zip(self.duration)
            .map_or(1., |(left, total)| {
                (left.as_secs_f64() / total.as_secs_f64()) as f32
            });
        (
            progress,
            self.running() && !focused && self.duration.is_some(),
        )
    }
    fn refresh(&self, w: &mut Window, cx: &mut Context<Self>) {
        if let Some(owner) = self.owner {
            App::notify(cx, owner);
        }
        w.refresh();
    }
}
