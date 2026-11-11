//! Resize geometry and gesture wiring. Base owns the actual handle and drag.
use super::*;
use crate::{
    theme::keys,
    tokens::{rems, rgba},
};
use gpui_kit::base::{HandleEdge, InteractiveElementExt as _, ResizeHandleState, resize_handle};
use gpui_kit::{
    AppContext as _, Axis, Context, InteractiveElement as _, MouseButton, MouseMoveEvent,
    MouseUpEvent, Render, Role, StatefulInteractiveElement as _, Styled as _, WeakEntity, div,
};
use std::cell::RefCell;
pub(crate) type ResizeCallback = dyn Fn(Pixels, &mut Window, &mut App);
#[derive(Clone, Default)]
pub(crate) struct Memory(Rc<RefCell<Sizes>>);
#[derive(Default)]
struct Sizes {
    source: Option<(DrawerPlacement, DefiniteLength)>,
    preferred: Option<Pixels>,
    gesture: Option<Gesture>,
}
struct Gesture {
    origin: Point<Pixels>,
    extent: Pixels,
}
impl Memory {
    pub fn resolve(&self, p: &Presentation, available: Size<Pixels>, w: &Window) -> Resolved {
        let mut sizes = self.0.borrow_mut();
        if sizes.source != Some((p.placement, p.extent)) {
            sizes.source = Some((p.placement, p.extent));
            sizes.preferred = None;
            sizes.gesture = None;
        }
        let axis = if p.placement.horizontal() {
            available.width
        } else {
            available.height
        };
        let resolve = |length: DefiniteLength| {
            let value = length.to_pixels(axis.into(), w.rem_size());
            assert!(
                value.as_f32().is_finite() && value >= px(0.),
                "drawer resize limits must be finite and nonnegative"
            );
            value
        };
        let min = resolve(p.min);
        let max = resolve(p.max);
        assert!(
            max >= min,
            "drawer resize maximum must not be less than minimum"
        );
        let max = max.min(axis).max(px(0.));
        let min = min.min(max);
        let initial = p.extent(available, w);
        let extent = if p.resizable {
            sizes.preferred.unwrap_or(initial).clamp(min, max)
        } else {
            initial
        };
        Resolved {
            memory: self.clone(),
            placement: p.placement,
            extent,
            initial: initial.clamp(min, max),
            min,
            max,
            on_resize: p.on_resize.clone(),
        }
    }
    pub fn cancel(&self) {
        self.0.borrow_mut().gesture = None;
    }
}
#[derive(Clone)]
pub(crate) struct Resolved {
    memory: Memory,
    placement: DrawerPlacement,
    pub extent: Pixels,
    initial: Pixels,
    min: Pixels,
    max: Pixels,
    on_resize: Option<Rc<ResizeCallback>>,
}
impl Resolved {
    fn set(&self, value: Pixels, w: &mut Window, cx: &mut App) {
        let value = value.clamp(self.min, self.max);
        let mut sizes = self.memory.0.borrow_mut();
        if sizes.preferred.unwrap_or(self.extent) == value {
            return;
        }
        sizes.preferred = Some(value);
        drop(sizes);
        if let Some(callback) = &self.on_resize {
            callback(value, w, cx);
        }
        w.refresh();
    }
    fn reset(&self, w: &mut Window, cx: &mut App) {
        self.set(self.initial, w, cx);
        self.memory.0.borrow_mut().preferred = None;
    }
    pub fn controls(
        &self,
        id: ElementId,
        theme: YueTheme,
        state: &Entity<YueDrawerState>,
        enabled: bool,
        w: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        if !enabled {
            self.memory.cancel();
            return div().into_any_element();
        }
        let focus = w
            .use_keyed_state((id, "drawer-resize-focus"), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let axis = if self.placement.horizontal() {
            Axis::Horizontal
        } else {
            Axis::Vertical
        };
        let edge = match self.placement {
            DrawerPlacement::Left | DrawerPlacement::Top => HandleEdge::Trailing,
            _ => HandleEdge::Leading,
        };
        let drag = self.clone();
        let keyboard = self.clone();
        let reset = self.clone();
        let captured = self.clone();
        let pointer_focus = focus.clone();
        let weak = state.downgrade();
        let owner = state.downgrade();
        let appearance = Rc::new(
            move |context: &gpui_kit::base::ResizeHandleContext, _: &mut Window, _: &mut App| {
                let active = context.state() != ResizeHandleState::Idle || context.is_focused();
                Some(
                    div()
                        .size_full()
                        .bg(if active {
                            rgba(theme, keys::ACCENT_SOLID)
                        } else {
                            gpui_kit::transparent_black().into()
                        })
                        .into_any_element(),
                )
            },
        );
        let step = rems(theme, "--resizable-keyboard-step").to_pixels(w.rem_size());
        let large = rems(theme, "--resizable-keyboard-step-large").to_pixels(w.rem_size());
        let handle = resize_handle::<ResizeDrag, ResizeGhost>("drawer-resize", axis)
            .inside(edge)
            .with_focus(&focus)
            .with_appearance(appearance)
            .hit_width(rems(theme, keys::SPACE_16).to_pixels(w.rem_size()))
            .line_width(rems(theme, "--resizable-line-width").to_pixels(w.rem_size()))
            .role(Role::Splitter)
            .aria_label("Adjust drawer size")
            .tab_stop(true)
            .on_mouse_down(MouseButton::Left, move |_, w, cx| {
                pointer_focus.focus(w, cx)
            })
            .on_key_down(move |event, w, cx| {
                let direction = match (keyboard.placement, event.keystroke.key.as_str()) {
                    (DrawerPlacement::Left, "right")
                    | (DrawerPlacement::Right, "left")
                    | (DrawerPlacement::Top, "down")
                    | (DrawerPlacement::Bottom, "up") => 1.,
                    (DrawerPlacement::Left, "left")
                    | (DrawerPlacement::Right, "right")
                    | (DrawerPlacement::Top, "up")
                    | (DrawerPlacement::Bottom, "down") => -1.,
                    _ => 0.,
                };
                let value = match event.keystroke.key.as_str() {
                    "home" => Some(keyboard.min),
                    "end" => Some(keyboard.max),
                    _ if direction != 0. => Some(
                        keyboard.extent
                            + if event.keystroke.modifiers.shift {
                                large
                            } else {
                                step
                            } * direction,
                    ),
                    _ => None,
                };
                if let Some(value) = value {
                    cx.stop_propagation();
                    keyboard.set(value, w, cx);
                }
            })
            .on_double_click(move |_, w, cx| reset.reset(w, cx))
            .on_drag(ResizeDrag, move |_, _, w, cx| {
                drag.memory.0.borrow_mut().gesture = Some(Gesture {
                    origin: w.mouse_position(),
                    extent: drag.extent,
                });
                cx.new(|_| ResizeGhost)
            });
        let capture = gpui_kit::canvas(
            |_, _, _| {},
            move |_, _, w, _| {
                let resize = captured.clone();
                let weak: WeakEntity<YueDrawerState> = weak.clone();
                w.on_mouse_event(move |event: &MouseMoveEvent, phase, w, cx| {
                    if !phase.capture() {
                        return;
                    }
                    if event.pressed_button != Some(MouseButton::Left)
                        || !w.is_window_active()
                        || !cx.has_active_drag()
                        || !weak
                            .upgrade()
                            .is_some_and(|s| s.read(cx).is_open() && !s.read(cx).is_pending())
                    {
                        resize.memory.cancel();
                        return;
                    }
                    let next = {
                        let sizes = resize.memory.0.borrow();
                        sizes.gesture.as_ref().map(|g| {
                            let delta = event.position - g.origin;
                            g.extent
                                + match resize.placement {
                                    DrawerPlacement::Left => delta.x,
                                    DrawerPlacement::Right => -delta.x,
                                    DrawerPlacement::Top => delta.y,
                                    DrawerPlacement::Bottom => -delta.y,
                                }
                        })
                    };
                    if let Some(value) = next {
                        resize.set(value, w, cx);
                    }
                });
                let memory = captured.memory.clone();
                w.on_mouse_event(move |event: &MouseUpEvent, phase, _, _| {
                    if phase.capture() && event.button == MouseButton::Left {
                        memory.cancel();
                    }
                });
                // A dropped declaration has no live capture; weak ownership avoids retaining its session.
                let _ = &owner;
            },
        )
        .absolute()
        .inset_0();
        div()
            .absolute()
            .inset_0()
            .child(handle)
            .child(capture)
            .into_any_element()
    }
}
#[derive(Clone)]
struct ResizeDrag;
struct ResizeGhost;
impl Render for ResizeGhost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_0()
    }
}
