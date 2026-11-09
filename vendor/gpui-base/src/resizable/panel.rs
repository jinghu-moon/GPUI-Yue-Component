use std::{ops::Range, rc::Rc};

use gpui::{
    Along, AnyElement, App, AppContext, Axis, Bounds, Context, Element, ElementId, Empty, Entity,
    EventEmitter, InteractiveElement as _, IntoElement, IsZero as _, MouseMoveEvent, MouseUpEvent,
    ParentElement, Pixels, Render, RenderOnce, StatefulInteractiveElement, Style, StyleRefinement,
    Styled, Window, div, prelude::FluentBuilder,
};

use crate::{AxisExt, ElementExt, StyledExt as _, h_flex, resizable::PANEL_MIN_SIZE, v_flex};

use super::{
    ResizableInteraction, ResizableState, ResizeHandleRenderer, resizable_panel, resize_handle,
};
use crate::InteractiveElementExt as _;

pub enum ResizablePanelEvent {
    Resized,
}

#[derive(Clone)]
pub(crate) struct DragPanel;
impl Render for DragPanel {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        Empty
    }
}

/// A group of resizable panels.
#[derive(IntoElement)]
pub struct ResizablePanelGroup {
    id: ElementId,
    state: Option<Entity<ResizableState>>,
    axis: Axis,
    size: Option<Pixels>,
    children: Vec<ResizablePanel>,
    on_resize: Rc<dyn Fn(&Entity<ResizableState>, &mut Window, &mut App)>,
    handle_appearance: Option<ResizeHandleRenderer>,
    interaction: Option<ResizableInteraction>,
    hit_width: Option<Pixels>,
    line_width: Option<Pixels>,
}

impl ResizablePanelGroup {
    /// Create a new resizable panel group.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            axis: Axis::Horizontal,
            children: vec![],
            state: None,
            size: None,
            on_resize: Rc::new(|_, _, _| {}),
            handle_appearance: None,
            interaction: None,
            hit_width: None,
            line_width: None,
        }
    }

    /// Hand the painted part of every divider in this group to `appearance`.
    ///
    /// The hit area, the cursor and the drag stay here; a renderer that
    /// returns `None` for a given handle leaves the built-in line on it.
    pub fn with_handle_appearance(mut self, appearance: ResizeHandleRenderer) -> Self {
        self.handle_appearance = Some(appearance);
        self
    }
    /// Enable stable keyed panels and bounded adjacent resizing with keyboard support.
    pub fn with_interaction(mut self, interaction: ResizableInteraction) -> Self {
        self.interaction = Some(interaction);
        self
    }
    pub fn with_handle_geometry(mut self, hit: Pixels, line: Pixels) -> Self {
        assert!(
            hit.as_f32().is_finite()
                && line.as_f32().is_finite()
                && line > gpui::px(0.)
                && hit >= line
        );
        self.hit_width = Some(hit);
        self.line_width = Some(line);
        self
    }

    /// Bind yourself to a resizable state entity.
    ///
    /// If not provided, it will handle its own state internally.
    pub fn with_state(mut self, state: &Entity<ResizableState>) -> Self {
        self.state = Some(state.clone());
        self
    }

    /// Set the axis of the resizable panel group, default is horizontal.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Add a panel to the group.
    ///
    /// - The `axis` will be set to the same axis as the group.
    /// - The `initial_size` will be set to the average size of all panels if not provided.
    /// - The `group` will be set to the group entity.
    pub fn child(mut self, panel: impl Into<ResizablePanel>) -> Self {
        self.children.push(panel.into());
        self
    }

    /// Add multiple panels to the group.
    pub fn children<I>(mut self, panels: impl IntoIterator<Item = I>) -> Self
    where
        I: Into<ResizablePanel>,
    {
        self.children = panels.into_iter().map(|panel| panel.into()).collect();
        self
    }

    /// Set size of the resizable panel group
    ///
    /// - When the axis is horizontal, the size is the height of the group.
    /// - When the axis is vertical, the size is the width of the group.
    pub fn size(mut self, size: Pixels) -> Self {
        self.size = Some(size);
        self
    }

    /// Set the callback to be called when the panels are resized.
    ///
    /// ## Callback arguments
    ///
    /// - Entity<ResizableState>: The state of the ResizablePanelGroup.
    pub fn on_resize(
        mut self,
        on_resize: impl Fn(&Entity<ResizableState>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_resize = Rc::new(on_resize);
        self
    }
}

impl<T> From<T> for ResizablePanel
where
    T: Into<AnyElement>,
{
    fn from(value: T) -> Self {
        resizable_panel().child(value.into())
    }
}

impl From<ResizablePanelGroup> for ResizablePanel {
    fn from(value: ResizablePanelGroup) -> Self {
        resizable_panel().child(value)
    }
}

impl EventEmitter<ResizablePanelEvent> for ResizablePanelGroup {}

impl RenderOnce for ResizablePanelGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.unwrap_or_else(|| {
            window.use_keyed_state(self.id.clone(), cx, |_, _| ResizableState::default())
        });
        let container = if self.axis.is_horizontal() {
            h_flex().items_stretch()
        } else {
            v_flex()
        };

        // Sync panels to the state
        let panels_count = self.children.len();
        state.update(cx, |state, cx| {
            if self.interaction.is_some() {
                let specs: Vec<_> = self
                    .children
                    .iter()
                    .map(|panel| super::bounded::PanelSpec {
                        key: panel
                            .key
                            .clone()
                            .expect("bounded panels require stable ids"),
                        initial: panel.initial_size,
                        range: panel.size_range.clone(),
                        fixed: panel.fixed && panel.initial_size.is_some(),
                    })
                    .collect();
                state.sync_keyed_panels(self.axis, &specs, cx);
            } else {
                state.sync_panels_count(self.axis, panels_count, cx);
            }
        });

        if let Some(interaction) = self.interaction {
            let weak = state.downgrade();
            window.use_keyed_state((self.id.clone(), "bounded-lifetime"), cx, |window, cx| {
                let released = weak.clone();
                cx.on_release(move |_: &mut super::bounded::BoundedLifetime, cx| {
                    let _ = released.update(cx, |s, _| {
                        s.resizing_panel_ix = None;
                        s.drag_origin = None;
                        s.drag_changed = false;
                    });
                })
                .detach();
                super::bounded::BoundedLifetime {
                    _activation: cx.observe_window_activation(window, move |_, window, cx| {
                        if !window.is_window_active() {
                            let _ = weak.update(cx, |s, cx| {
                                if s.resizing_panel_ix.take().is_some() {
                                    cx.notify();
                                }
                                s.drag_origin = None;
                                s.drag_changed = false;
                            });
                        }
                    }),
                }
            });
            if interaction.disabled || !window.is_window_active() || !window.is_visible() {
                state.update(cx, |s, _| {
                    s.resizing_panel_ix = None;
                    s.drag_origin = None;
                    s.drag_changed = false;
                });
            }
        }
        container
            .id(self.id)
            .size_full()
            // The group only distributes space along its own axis, so a caller
            // supplied size can only mean the cross axis.
            .when_some(self.size, |this, size| match self.axis {
                Axis::Horizontal => this.h(size),
                Axis::Vertical => this.w(size),
            })
            .children(
                self.children
                    .into_iter()
                    .enumerate()
                    .map(|(ix, mut panel)| {
                        panel.panel_ix = ix;
                        panel.axis = self.axis;
                        panel.state = Some(state.clone());
                        panel.handle_appearance = self.handle_appearance.clone();
                        panel.interaction = self.interaction;
                        panel.hit_width = self.hit_width;
                        panel.line_width = self.line_width;
                        panel.on_resize = self.on_resize.clone();
                        if ix > 0 && panel.boundary_label.is_none() {
                            panel.boundary_label = panel.label.clone();
                        }
                        panel
                    }),
            )
            .on_prepaint({
                let state = state.clone();
                move |bounds, window, cx| {
                    // The same bounds as last frame change nothing; updating
                    // the state for them would mark it changed every frame.
                    if state.read(cx).bounds == bounds {
                        return;
                    }
                    state.update(cx, |state, cx| {
                        let size_changed =
                            state.bounds.size.along(self.axis) != bounds.size.along(self.axis);

                        state.bounds = bounds;

                        if size_changed {
                            state.adjust_to_container_size(cx);
                            // The adjustment lands after this frame's layout has
                            // already been computed, and a notify raised during a
                            // draw only records the view as dirty without scheduling
                            // a frame for it. Defer the notify so it runs once the
                            // draw has finished and can schedule the settling frame.
                            // Otherwise that frame stays pending until some later
                            // input repaints the window, and the divider appears to
                            // jump on hover.
                            let state = cx.entity();
                            window.defer(cx, move |_, cx| {
                                state.update(cx, |_, cx| cx.notify());
                            });
                        }
                    })
                }
            })
            .child(ResizePanelGroupElement {
                state: state.clone(),
                axis: self.axis,
                on_resize: self.on_resize.clone(),
                interaction: self.interaction,
            })
    }
}

/// A resizable panel inside a [`ResizablePanelGroup`].
///
/// Implements [`Styled`], so call sites can override the panel's
/// rendered styles. User overrides are applied **between** the panel's
/// flex defaults and its size management — the caller can override the
/// internal `flex_grow: 1` (e.g. via `.flex_none()`) and add their own
/// padding / colors / borders, while the panel's runtime size
/// constraints (`min_w`/`max_w`/`flex_basis` driven by `ResizableState`)
/// always win.
///
/// A common override is `.flex_none()`: the panel sets `flex_grow: 1`
/// internally, so a sized panel that should hold its width when a
/// sibling collapses needs to opt out of growth via `.flex_none()`.
///
/// To keep a panel's size when the group's container resizes, use
/// [`ResizablePanel::fixed`].
///
/// ```ignore
/// h_resizable("layout")
///     .child(resizable_panel().size(px(220.)).flex_none().child(sidebar))
///     .child(resizable_panel().child(content))                // flex
///     .child(resizable_panel().size(px(280.)).flex_none().child(metadata))
/// ```
///
/// **Reserved styles**: do not call these from outside — they fight the
/// panel's own layout management:
/// - `.flex_basis(...)` — driven by `ResizableState`, not by the caller.
/// - `.absolute()` — would remove the panel from the resizable's flex flow.
/// - `.overflow_hidden()` — may clip the resize handle, which is positioned
///   absolute at `left: -4px` of each panel after the first.
#[derive(IntoElement)]
pub struct ResizablePanel {
    key: Option<ElementId>,
    label: Option<gpui::SharedString>,
    boundary_label: Option<gpui::SharedString>,
    interaction: Option<ResizableInteraction>,
    hit_width: Option<Pixels>,
    line_width: Option<Pixels>,
    on_resize: Rc<dyn Fn(&Entity<ResizableState>, &mut Window, &mut App)>,
    axis: Axis,
    panel_ix: usize,
    state: Option<Entity<ResizableState>>,
    /// Initial size is the size that the panel has when it is created.
    initial_size: Option<Pixels>,
    /// size range limit of this panel.
    size_range: Range<Pixels>,
    children: Vec<AnyElement>,
    visible: bool,
    style: StyleRefinement,
    handle_appearance: Option<ResizeHandleRenderer>,
    fixed: bool,
}

impl ResizablePanel {
    /// Create a new resizable panel.
    pub(super) fn new() -> Self {
        Self {
            key: None,
            label: None,
            boundary_label: None,
            interaction: None,
            hit_width: None,
            line_width: None,
            on_resize: Rc::new(|_, _, _| {}),
            panel_ix: 0,
            initial_size: None,
            state: None,
            size_range: (PANEL_MIN_SIZE..Pixels::MAX),
            axis: Axis::Horizontal,
            children: vec![],
            visible: true,
            style: StyleRefinement::default(),
            handle_appearance: None,
            fixed: false,
        }
    }

    /// Set the visibility of the panel, default is true.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.key = Some(id.into());
        self
    }
    pub fn label(mut self, label: impl Into<gpui::SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the initial size of the panel.
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.initial_size = Some(size.into());
        self
    }

    /// Set the size range to limit panel resize.
    ///
    /// Default is [`PANEL_MIN_SIZE`] to [`Pixels::MAX`].
    pub fn size_range(mut self, range: impl Into<Range<Pixels>>) -> Self {
        self.size_range = range.into();
        self
    }

    /// Keep this panel's size when the group's container resizes.
    ///
    /// When the container grows or shrinks, the other panels share the
    /// difference in proportion to their sizes, so a sidebar or an inspector
    /// keeps the width the user gave it. Dragging its own handle still resizes
    /// it. It needs a [`Self::size`], and at least one panel in the group
    /// should not be fixed.
    pub fn fixed(mut self) -> Self {
        self.fixed = true;
        self
    }
}

impl Styled for ResizablePanel {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for ResizablePanel {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ResizablePanel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if !self.visible {
            return div().id(("resizable-panel", self.panel_ix));
        }

        let state = self
            .state
            .expect("BUG: The `state` in ResizablePanel should be present.");
        let panel_state = state
            .read(cx)
            .panels
            .get(self.panel_ix)
            .expect("BUG: The `index` of ResizablePanel should be one of in `state`.")
            .clone();
        let size_range = self.size_range.clone();
        // A fixed panel needs a size to keep; without one it lays out as usual.
        let fixed = self.fixed && self.initial_size.is_some();

        let panel_id = self
            .key
            .clone()
            .unwrap_or_else(|| ("resizable-panel", self.panel_ix).into());
        let focus = self
            .interaction
            .filter(|i| !i.disabled && self.panel_ix > 0)
            .map(|_| {
                window
                    .use_keyed_state(
                        (panel_id.clone(), gpui::SharedString::from("resize-focus")),
                        cx,
                        |_, cx| cx.focus_handle().tab_stop(true),
                    )
                    .read(cx)
                    .clone()
            });
        let range = state
            .read(cx)
            .boundary_range(self.panel_ix.saturating_sub(1));
        let sizes = state.read(cx).sizes();
        let value = sizes
            .get(self.panel_ix.saturating_sub(1))
            .copied()
            .unwrap_or_default();
        div()
            .id(panel_id)
            .flex()
            .flex_grow_1()
            .size_full()
            .relative()
            // Apply caller style overrides here — between the flex defaults
            // above and the size management below. This lets callers cancel
            // the unconditional `.flex_grow_1()` (via `.flex_none()`, the load-
            // bearing case for sized panels next to a collapsing sibling) and
            // add their own padding / colors / borders, while keeping the
            // panel's runtime size constraints (min/max + `flex_basis` driven
            // by `ResizableState`) authoritative.
            .refine_style(&self.style)
            .when(self.axis.is_vertical(), |this| {
                this.min_h(size_range.start).max_h(size_range.end)
            })
            .when(self.axis.is_horizontal(), |this| {
                this.min_w(size_range.start).max_w(size_range.end)
            })
            // 1. initial_size is None, to use auto size.
            // 2. initial_size is Some and size is none, to use the initial size of the panel for first time render.
            // 3. initial_size is Some and size is Some, use `size`.
            .when(self.initial_size.is_none(), |this| this.flex_shrink_1())
            .when_some(self.initial_size, |this, initial_size| {
                // The `self.size` is None, that mean the initial size for the panel,
                // so we need set `flex_shrink_0` To let it keep the initial size.
                this.when(
                    panel_state.size.is_none() && !initial_size.is_zero(),
                    |this| this.flex_none(),
                )
                .flex_basis(initial_size)
            })
            .map(|this| match panel_state.size {
                Some(size) => this.flex_basis(size.min(size_range.end).max(size_range.start)),
                None => this,
            })
            // A fixed panel neither grows nor shrinks with the container, so
            // the first frame of a resize already has its size. Not
            // `flex_none`, which also resets the basis set above.
            .when(fixed, |this| this.flex_grow_0().flex_shrink_0())
            .on_prepaint({
                let state = state.clone();
                move |bounds, _, cx| {
                    if !state.read(cx).panel_size_changes(
                        self.panel_ix,
                        bounds,
                        &self.size_range,
                        fixed,
                    ) {
                        return;
                    }
                    state.update(cx, |state, cx| {
                        state.update_panel_size(self.panel_ix, bounds, self.size_range, fixed, cx)
                    })
                }
            })
            .children(self.children)
            .when(self.panel_ix > 0, |this| {
                let ix = self.panel_ix - 1;
                let on_resize = self.on_resize.clone();
                let keyboard_state = state.clone();
                let reset_state = state.clone();
                let reset_callback = on_resize.clone();
                let press_state = state.clone();
                let accessibility_state = state.clone();
                let accessibility_callback = on_resize.clone();
                let decrement_state = state.clone();
                let decrement_callback = on_resize.clone();
                this.child(
                    resize_handle(("resizable-handle", ix), self.axis)
                        .when_some(self.hit_width, |h, width| h.hit_width(width))
                        .when_some(self.line_width, |h, width| h.line_width(width))
                        .when_some(self.handle_appearance.clone(), |handle, appearance| {
                            handle.with_appearance(appearance)
                        })
                        .when_some(self.interaction, |handle, interaction| {
                            handle
                                .disabled(interaction.disabled)
                                .role(gpui::Role::Splitter)
                                .aria_label(
                                    self.boundary_label
                                        .clone()
                                        .unwrap_or_else(|| "Resize".into()),
                                )
                                .aria_orientation(if self.axis.is_horizontal() {
                                    gpui::Orientation::Vertical
                                } else {
                                    gpui::Orientation::Horizontal
                                })
                                .aria_numeric_value(value.as_f32() as f64)
                                .when_some(range.clone(), |h, range| {
                                    h.aria_min_numeric_value(range.start.as_f32() as f64)
                                        .aria_max_numeric_value(range.end.as_f32() as f64)
                                })
                                .when_some(focus.clone(), |h, f| {
                                    h.with_focus(&f).on_mouse_down(
                                        gpui::MouseButton::Left,
                                        move |event, window, cx| {
                                            f.focus(window, cx);
                                            press_state.update(cx, |s, _| {
                                                let pointer = if self.axis.is_horizontal() {
                                                    event.position.x
                                                } else {
                                                    event.position.y
                                                };
                                                s.drag_origin = Some((pointer, s.sizes[ix]));
                                                s.drag_changed = false;
                                            });
                                        },
                                    )
                                })
                                .when(!interaction.disabled, |h| {
                                    h.on_key_down(move |event, window, cx| {
                                        let stroke = &event.keystroke;
                                        if stroke.modifiers.control
                                            || stroke.modifiers.alt
                                            || stroke.modifiers.platform
                                        {
                                            return;
                                        }
                                        let step = if stroke.modifiers.shift {
                                            interaction.large_step
                                        } else {
                                            interaction.step
                                        };
                                        let key = stroke.key.as_str();
                                        let delta = match (self.axis, key) {
                                            (Axis::Horizontal, "left") | (Axis::Vertical, "up") => {
                                                Some(-step)
                                            }
                                            (Axis::Horizontal, "right")
                                            | (Axis::Vertical, "down") => Some(step),
                                            _ => None,
                                        };
                                        if delta.is_none()
                                            && !matches!(key, "home" | "end" | "enter")
                                        {
                                            return;
                                        }
                                        window.prevent_default();
                                        cx.stop_propagation();
                                        let changed = keyboard_state.update(cx, |s, cx| {
                                            if key == "enter" {
                                                return s.reset_boundary(ix, cx);
                                            }
                                            let Some(range) = s.boundary_range(ix) else {
                                                return false;
                                            };
                                            let value = if key == "home" {
                                                range.start
                                            } else if key == "end" {
                                                range.end
                                            } else {
                                                s.sizes[ix] + delta.unwrap()
                                            };
                                            s.resize_bounded(ix, value, cx)
                                        });
                                        if changed {
                                            keyboard_state.update(cx, |s, cx| s.done_resizing(cx));
                                            on_resize(&keyboard_state, window, cx);
                                        }
                                    })
                                    .on_double_click(move |_, window, cx| {
                                        cx.stop_propagation();
                                        if reset_state.update(cx, |s, cx| s.reset_boundary(ix, cx))
                                        {
                                            reset_state.update(cx, |s, cx| s.done_resizing(cx));
                                            reset_callback(&reset_state, window, cx);
                                        }
                                    })
                                    .on_a11y_action(
                                        gpui::AccessibleAction::Increment,
                                        move |_, window, cx| {
                                            let changed =
                                                accessibility_state.update(cx, |s, cx| {
                                                    s.resize_bounded(
                                                        ix,
                                                        s.sizes[ix] + interaction.step,
                                                        cx,
                                                    )
                                                });
                                            if changed {
                                                accessibility_state
                                                    .update(cx, |s, cx| s.done_resizing(cx));
                                                accessibility_callback(
                                                    &accessibility_state,
                                                    window,
                                                    cx,
                                                );
                                            }
                                        },
                                    )
                                    .on_a11y_action(
                                        gpui::AccessibleAction::Decrement,
                                        move |_, window, cx| {
                                            let changed = decrement_state.update(cx, |s, cx| {
                                                s.resize_bounded(
                                                    ix,
                                                    s.sizes[ix] - interaction.step,
                                                    cx,
                                                )
                                            });
                                            if changed {
                                                decrement_state
                                                    .update(cx, |s, cx| s.done_resizing(cx));
                                                decrement_callback(&decrement_state, window, cx);
                                            }
                                        },
                                    )
                                })
                        })
                        .when(self.interaction.is_none_or(|i| !i.disabled), |handle| {
                            handle.on_drag(DragPanel, move |drag_panel, _, _, cx| {
                                cx.stop_propagation();
                                // Set current resizing panel ix
                                state.update(cx, |state, _| {
                                    state.resizing_panel_ix = Some(ix);
                                });
                                cx.new(|_| drag_panel.clone())
                            })
                        }),
                )
            })
    }
}

struct ResizePanelGroupElement {
    state: Entity<ResizableState>,
    on_resize: Rc<dyn Fn(&Entity<ResizableState>, &mut Window, &mut App)>,
    axis: Axis,
    interaction: Option<ResizableInteraction>,
}

impl IntoElement for ResizePanelGroupElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ResizePanelGroupElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<gpui::ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        (window.request_layout(Style::default(), None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        ()
    }

    fn paint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.on_mouse_event({
            let state = self.state.clone();
            let axis = self.axis;
            let current_ix = state.read(cx).resizing_panel_ix;
            let interaction = self.interaction;
            move |e: &MouseMoveEvent, phase, window, cx| {
                if !phase.bubble() {
                    return;
                }
                let current_ix = if interaction.is_some() {
                    state.read(cx).resizing_panel_ix
                } else {
                    current_ix
                };
                let Some(ix) = current_ix else { return };
                if interaction.is_some()
                    && (interaction.is_some_and(|i| i.disabled)
                        || !window.is_window_active()
                        || !e.dragging())
                {
                    state.update(cx, |s, _| s.resizing_panel_ix = None);
                    return;
                }

                state.update(cx, |state, cx| {
                    let panel = state.panels.get(ix).expect("BUG: invalid panel index");

                    if interaction.is_some() {
                        let pointer = if axis.is_horizontal() {
                            e.position.x
                        } else {
                            e.position.y
                        };
                        let size = state
                            .drag_origin
                            .map(|(at, size)| size + pointer - at)
                            .unwrap_or_else(|| {
                                pointer
                                    - if axis.is_horizontal() {
                                        panel.bounds.left()
                                    } else {
                                        panel.bounds.top()
                                    }
                            });
                        state.resize_bounded(ix, size, cx);
                        return;
                    }
                    match axis {
                        Axis::Horizontal => state.resize_panel_at_handle(
                            ix,
                            e.position.x - panel.bounds.left(),
                            window,
                            cx,
                        ),
                        Axis::Vertical => state.resize_panel_at_handle(
                            ix,
                            e.position.y - panel.bounds.top(),
                            window,
                            cx,
                        ),
                    }
                    cx.notify();
                })
            }
        });

        // When any mouse up, stop dragging
        window.on_mouse_event({
            let state = self.state.clone();
            let current_ix = state.read(cx).resizing_panel_ix;
            let interaction = self.interaction;
            let on_resize = self.on_resize.clone();
            let axis = self.axis;
            move |event: &MouseUpEvent, phase, window, cx| {
                let current_ix = if interaction.is_some() {
                    state.read(cx).resizing_panel_ix
                } else {
                    current_ix
                };
                if current_ix.is_none() {
                    return;
                }
                if phase.bubble() {
                    if interaction.is_some() {
                        if event.button != gpui::MouseButton::Left {
                            return;
                        }
                        let changed = state.update(cx, |s, cx| {
                            if let Some((at, size)) = s.drag_origin {
                                let pointer = if axis.is_horizontal() {
                                    event.position.x
                                } else {
                                    event.position.y
                                };
                                s.resize_bounded(current_ix.unwrap(), size + pointer - at, cx);
                            }
                            let changed = s.drag_changed;
                            s.resizing_panel_ix = None;
                            s.drag_origin = None;
                            s.drag_changed = false;
                            changed
                        });
                        if !changed {
                            return;
                        }
                    }
                    state.update(cx, |state, cx| state.done_resizing(cx));
                    on_resize(&state, window, cx);
                }
            }
        })
    }
}
