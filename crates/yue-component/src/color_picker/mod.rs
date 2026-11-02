//! Retained HSV color editing with independent HEX/RGB/HSL fields and alpha.
mod color;
pub(crate) mod paint;
mod panel;
mod state;
use crate::popup::{
    PopupAnchor, PopupDismissRoute, PopupHost, PopupId, PopupMotion, PopupPlacement,
    PopupPointerMode, PopupSpec,
};
use crate::tokens::{duration, easing, pixels, rems, rgba};
use crate::{YueTheme, control_layout::ControlLayout};
use color::Color;
pub use color::ColorFormat;
use gpui_kit::base::{Align, Placement, PopoverState, TestSupportExt as _, Transition, transition};
use gpui_kit::{
    App, Bounds, ElementId, Entity, FocusHandle, Focusable as _, InteractiveElement as _,
    IntoElement, ParentElement as _, Pixels, RenderOnce, Rgba, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, div, prelude::FluentBuilder as _, px,
};
pub use state::{ColorPickerEvent, YueColorPickerState};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ColorPickerSize {
    Small,
    #[default]
    Medium,
    Large,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ColorPickerTrigger {
    #[default]
    Field,
    Swatch,
}
#[derive(Clone)]
pub(super) struct Options {
    theme: YueTheme,
    inherited: bool,
    name: SharedString,
    label: Option<SharedString>,
    presets: Vec<Color>,
    recent: bool,
    disabled: bool,
}
impl Options {
    fn new(theme: YueTheme) -> Self {
        Self {
            theme,
            inherited: false,
            name: SharedString::default(),
            label: None,
            presets: vec![],
            recent: true,
            disabled: false,
        }
    }
    fn resolved(mut self, cx: &App) -> Self {
        if self.inherited {
            self.theme = crate::current_theme(cx);
        }
        if let Some(label) = &self.label
            && self.name.trim().is_empty()
        {
            self.name = label.clone();
        }
        assert!(
            !self.name.trim().is_empty(),
            "ColorPicker requires a visible label or accessibility_label"
        );
        self
    }
}
type ConfirmHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// A color field whose nonmodal editor shares the same state as YueColorPickerPanel.
///
/// ```
/// use yue_component::{YueColorPicker, YueColorPickerState, current_theme, gpui_kit};
/// use gpui_kit::{AppContext as _, Context, Entity, IntoElement, Render, Window};
/// struct Properties { fill: Entity<YueColorPickerState> }
/// impl Properties {
///     fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
///         Self { fill: cx.new(|cx| YueColorPickerState::new(window, cx)
///             .default_value(gpui_kit::rgba(0x3366CC80))) }
///     }
/// }
/// impl Render for Properties {
///     fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
///         YueColorPicker::new("fill", current_theme(cx), &self.fill)
///             .label("Fill").use_default_theme()
///     }
/// }
/// ```
#[derive(IntoElement)]
pub struct YueColorPicker {
    id: ElementId,
    state: Entity<YueColorPickerState>,
    options: Options,
    size: ColorPickerSize,
    trigger: ColorPickerTrigger,
    placement: Placement,
    align: Align,
    layout: ControlLayout,
    focus: Option<FocusHandle>,
    tab_stop: bool,
    group_selected: Option<bool>,
    pub(crate) on_confirm: Option<ConfirmHandler>,
}
/// The same editor in a persistent properties pane, without a trigger or popup.
///
/// ```
/// use yue_component::{YueColorPickerPanel, YueColorPickerState, YueTheme, gpui_kit};
/// fn panel(state: &gpui_kit::Entity<YueColorPickerState>, theme: YueTheme) -> YueColorPickerPanel {
///     YueColorPickerPanel::new("fill-panel", theme, state).label("Fill")
///         .width(gpui_kit::px(300.))
/// }
/// ```
#[derive(IntoElement)]
pub struct YueColorPickerPanel {
    id: ElementId,
    state: Entity<YueColorPickerState>,
    options: Options,
    layout: ControlLayout,
}
crate::control_layout::control_layout!(YueColorPicker);
crate::control_layout::control_layout!(YueColorPickerPanel);
macro_rules! builders {
    ($ty:ty) => {
        impl $ty {
            pub fn use_default_theme(mut self) -> Self {
                self.options.inherited = true;
                self
            }
            pub fn theme(mut self, theme: YueTheme) -> Self {
                self.options.theme = theme;
                self.options.inherited = false;
                self
            }
            pub fn label(mut self, label: impl Into<SharedString>) -> Self {
                self.options.label = Some(label.into());
                self
            }
            pub fn accessibility_label(mut self, name: impl Into<SharedString>) -> Self {
                self.options.name = name.into();
                self
            }
            /// # Panics
            /// If a preset has a nonfinite or out-of-range RGBA channel.
            pub fn presets(mut self, colors: impl IntoIterator<Item = Rgba>) -> Self {
                for c in colors {
                    let c = Color::from_rgba(c);
                    if !self.options.presets.contains(&c) {
                        self.options.presets.push(c);
                    }
                }
                self
            }
            pub fn show_recent(mut self, show: bool) -> Self {
                self.options.recent = show;
                self
            }
            pub fn disabled(mut self, disabled: bool) -> Self {
                self.options.disabled = disabled;
                self
            }
        }
    };
}
builders!(YueColorPicker);
builders!(YueColorPickerPanel);
impl YueColorPicker {
    pub(crate) fn group_focus(
        mut self,
        focus: FocusHandle,
        tab_stop: bool,
        selected: bool,
    ) -> Self {
        self.focus = Some(focus);
        self.tab_stop = tab_stop;
        self.group_selected = Some(selected);
        self
    }
    pub fn new(
        id: impl Into<ElementId>,
        theme: YueTheme,
        state: &Entity<YueColorPickerState>,
    ) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            options: Options::new(theme),
            size: Default::default(),
            trigger: Default::default(),
            placement: Placement::Bottom,
            align: Align::Start,
            layout: Default::default(),
            on_confirm: None,
            focus: None,
            tab_stop: true,
            group_selected: None,
        }
    }
    pub fn size(mut self, size: ColorPickerSize) -> Self {
        self.size = size;
        self
    }
    pub fn trigger(mut self, trigger: ColorPickerTrigger) -> Self {
        self.trigger = trigger;
        self
    }
    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }
}
impl YueColorPickerPanel {
    pub fn new(
        id: impl Into<ElementId>,
        theme: YueTheme,
        state: &Entity<YueColorPickerState>,
    ) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            options: Options::new(theme),
            layout: Default::default(),
        }
    }
}
#[derive(Clone)]
pub(super) struct Popup {
    state: Entity<PopoverState>,
    trigger: FocusHandle,
    anchor: PopupAnchor,
    format_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
}
impl Popup {
    fn dismiss(
        &self,
        editor: &Entity<YueColorPickerState>,
        window: &mut Window,
        cx: &mut App,
        restore: bool,
    ) {
        if restore {
            self.trigger.focus(window, cx);
        }
        self.state.update(cx, |s, cx| s.dismiss(window, cx));
        editor.update(cx, |s, cx| s.close(window, cx));
        window.refresh();
    }
}
impl RenderOnce for YueColorPickerPanel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let options = self.options.resolved(cx);
        self.state
            .update(cx, |s, cx| s.prepare(!options.disabled, window, cx));
        let body = panel::body(
            self.id.clone(),
            &self.state,
            &options,
            None,
            true,
            window,
            cx,
        );
        let surface = div()
            .id(self.id)
            .w(rems(options.theme, "--color-picker-panel-width"))
            .child(body);
        self.layout.apply(surface)
    }
}
impl RenderOnce for YueColorPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let options = self.options.resolved(cx);
        let theme = options.theme;
        let id = self.id;
        self.state
            .update(cx, |s, cx| s.prepare(!options.disabled, window, cx));
        let popup = window.use_keyed_state((id.clone(), "color-popup"), cx, |_, cx| {
            PopoverState::new(false, cx)
        });
        let meta = window.use_keyed_state((id.clone(), "color-geometry"), cx, |_, cx| {
            (
                cx.focus_handle(),
                PopupAnchor::default(),
                Rc::new(Cell::new(None)),
            )
        });
        let (default_trigger, anchor, format_bounds) = meta.read(cx).clone();
        let trigger = self.focus.unwrap_or(default_trigger);
        let host = Popup {
            state: popup.clone(),
            trigger: trigger.clone(),
            anchor: anchor.clone(),
            format_bounds,
        };
        if options.disabled && popup.read(cx).is_open() {
            host.dismiss(&self.state, window, cx, false);
        }
        // The subscription is retained, its callback is refreshed each render.
        // A controlled SwatchGroup may change selection or handler meanwhile.
        let editor = self.state.clone();
        let completion = window.use_keyed_state(
            (
                id.clone(),
                format!("color-completion-{}", editor.entity_id().as_u64()),
            ),
            cx,
            move |window, cx| {
                let latest: Rc<RefCell<Option<ConfirmHandler>>> = Rc::default();
                let callback = latest.clone();
                let subscription = cx.subscribe_in(
                    &editor,
                    window,
                    move |_, _, _: &state::ColorConfirmed, window, cx| {
                        let handler = callback.borrow().clone();
                        if let Some(handler) = handler {
                            handler(window, cx);
                        }
                    },
                );
                (latest, subscription)
            },
        );
        *completion.read(cx).0.borrow_mut() = self.on_confirm.clone();
        let open = popup.read(cx).is_open();
        let color = self.state.read(cx).color;
        let formatted = color.text(self.state.read(cx).format, self.state.read(cx).hsv[0]);
        let (height, font, padding, swatch) = match self.size {
            ColorPickerSize::Small => (
                "--color-picker-height-sm",
                "--color-picker-font-size-sm",
                "--color-picker-padding-inline-sm",
                "--color-picker-swatch-size-sm",
            ),
            ColorPickerSize::Medium => (
                "--color-picker-height-md",
                "--color-picker-font-size-md",
                "--color-picker-padding-inline-md",
                "--color-picker-swatch-size-md",
            ),
            ColorPickerSize::Large => (
                "--color-picker-height-lg",
                "--color-picker-font-size-lg",
                "--color-picker-padding-inline-lg",
                "--color-picker-swatch-size-lg",
            ),
        };
        let toggle = host.clone();
        let state = self.state.clone();
        let disabled = options.disabled;
        let hovered = window.use_keyed_state((id.clone(), "color-hover"), cx, |_, _| false);
        let border_target = rgba(
            theme,
            if disabled {
                "--color-picker-disabled-border-color"
            } else if open {
                "--color-picker-border-color-open"
            } else if *hovered.read(cx) {
                "--color-picker-border-color-hover"
            } else {
                "--color-picker-border-color"
            },
        );
        let border = transition(
            (id.clone(), "color-border"),
            gpui_kit::Hsla::from(border_target),
            Transition::new(duration(theme, "--color-picker-state-duration"))
                .ease(easing(theme, "--color-picker-state-ease")),
            window,
            cx,
        );
        let preview = if self.group_selected.is_some() {
            div()
                .relative()
                .size(rems(theme, "--swatch-size"))
                .child(paint::preview(color.rgba(), theme, "--swatch-size"))
                .child(
                    div()
                        .absolute()
                        .inset(gpui_kit::rems(
                            (pixels(theme, "--swatch-size")
                                - pixels(theme, "--color-picker-swatch-size-sm"))
                                / 32.,
                        ))
                        .rounded(rems(theme, "--swatch-radius"))
                        .bg(rgba(theme, "--color-picker-background"))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            crate::Icon::new(crate::outline::Palette)
                                .size(rems(theme, "--color-picker-swatch-size-sm"))
                                .color(theme.text_primary()),
                        ),
                )
                .into_any_element()
        } else {
            paint::preview(color.rgba(), theme, swatch)
        };
        let description = match self.group_selected {
            Some(true) => format!("已选中，{formatted}"),
            Some(false) => format!("未选中，{formatted}"),
            None => formatted.clone(),
        };
        let button = gpui_kit::base::Button::new((id.clone(), "trigger"))
            .accessibility_label(options.name.clone())
            .aria_description(description)
            .aria_expanded(open)
            .disabled(disabled)
            .track_focus(&trigger)
            .tab_stop(self.tab_stop)
            .flex()
            .items_center()
            .justify_center()
            .gap(rems(theme, "--color-picker-trigger-gap"))
            .h(rems(theme, height))
            .text_size(rems(theme, font))
            .px(if self.trigger == ColorPickerTrigger::Swatch {
                gpui_kit::rems(0.)
            } else {
                rems(theme, padding)
            })
            .rounded(rems(theme, "--color-picker-radius"))
            .border_1()
            .border_color(border)
            .bg(rgba(
                theme,
                if disabled {
                    "--color-picker-disabled-background"
                } else {
                    "--color-picker-background"
                },
            ))
            .text_color(rgba(
                theme,
                if disabled {
                    "--color-picker-disabled-color"
                } else {
                    "--color-picker-color"
                },
            ))
            .when(!disabled, |b| b.cursor_pointer())
            .when(!disabled, |b| {
                b.on_hover(move |hover, window, cx| {
                    hovered.update(cx, |v, _| *v = *hover);
                    window.refresh();
                })
                .focus_visible(|b| b.border_color(rgba(theme, "--color-picker-focus-ring-color")))
            })
            .on_click(move |_, window, cx| {
                cx.stop_propagation();
                if disabled {
                    return;
                }
                if toggle.state.read(cx).is_open() {
                    toggle.dismiss(&state, window, cx, true);
                } else {
                    toggle.trigger.focus(window, cx);
                    toggle.state.update(cx, |s, cx| s.show(window, cx));
                    let focus = state.read(cx).focus.clone();
                    focus.focus(window, cx);
                    window.refresh();
                }
            })
            .relative()
            .child(preview)
            .when(self.group_selected == Some(true), |b| {
                b.child(
                    div()
                        .absolute()
                        .inset(gpui_kit::rems(
                            ((pixels(theme, "--swatch-hit-size") - pixels(theme, "--swatch-size"))
                                / 2.
                                - pixels(theme, "--swatch-selected-offset")
                                - pixels(theme, "--swatch-selected-width"))
                                / 16.,
                        ))
                        .rounded(rems(theme, "--swatch-radius"))
                        .border(rems(theme, "--swatch-selected-width"))
                        .border_color(rgba(theme, "--swatch-selected-color")),
                )
            })
            .when(self.trigger == ColorPickerTrigger::Field, |b| {
                b.child(div().flex_1().min_w_0().truncate().child(formatted))
                    .child(
                        crate::Icon::new(crate::outline::ChevronDown)
                            .size(rems(theme, swatch))
                            .color(theme.text_secondary()),
                    )
            })
            .w_full();
        let trigger_row = button.relative().child(anchor.capture());
        let enter = Transition::new(duration(theme, "--color-picker-enter-duration"))
            .ease(easing(theme, "--color-picker-enter-ease"));
        let exit = Transition::new(duration(theme, "--color-picker-exit-duration"))
            .ease(easing(theme, "--color-picker-exit-ease"));
        let mut root = div()
            .id((id.clone(), "color-field"))
            .flex()
            .flex_col()
            .items_start()
            .w(if self.trigger == ColorPickerTrigger::Swatch {
                rems(theme, height)
            } else {
                rems(theme, "--color-picker-trigger-width")
            })
            .gap_1();
        root = self.layout.apply(root);
        if let Some(label) = options.label.clone() {
            root = root.child(div().text_sm().child(label));
        }
        root = root.child(trigger_row);
        let scale = f32::from(window.rem_size()) / 16.;
        let margin = px(pixels(theme, "--color-picker-window-margin") * scale);
        let shadows = crate::tokens::shadow(theme, "--color-picker-panel-shadow");
        let mut outsets: gpui_kit::Edges<Pixels> = Default::default();
        for shadow in &shadows {
            let radius = (shadow.blur_radius + shadow.spread_radius).max(px(0.));
            outsets.left = outsets.left.max(radius - shadow.offset.x);
            outsets.right = outsets.right.max(radius + shadow.offset.x);
            outsets.top = outsets.top.max(radius - shadow.offset.y);
            outsets.bottom = outsets.bottom.max(radius + shadow.offset.y);
        }
        let reserve = [outsets.left, outsets.right, outsets.top, outsets.bottom]
            .into_iter()
            .fold(px(0.), Pixels::max);
        let inset = 2. * (margin + reserve);
        let width = px(pixels(theme, "--color-picker-panel-width") * scale)
            .min(window.viewport_size().width - inset)
            .max(px(1.));
        let maximum_height = (window.viewport_size().height - inset).max(px(1.));
        let close_host = host.clone();
        let editor = self.state.clone();
        let spec = PopupSpec::new(
            PopupId::new(window.current_view(), (id.clone(), "color-popup-surface")),
            anchor.handle(),
            open,
            PopupPlacement::Side(self.placement, self.align),
        )
        .surface_gap(px(pixels(theme, "--color-picker-panel-offset") * scale))
        .margin(margin)
        .pointer_mode(PopupPointerMode::Block)
        .dismiss_route(PopupDismissRoute::Owner)
        .motion(PopupMotion::new(enter, exit))
        .visual_outsets(outsets)
        .on_dismiss(move |_, window, cx| close_host.dismiss(&editor, window, cx, false));
        if let Some(surface) = PopupHost::new(spec).mount(
            move |presentation, _, window, cx| {
                let interactive = presentation.is_interactive();
                let content = panel::body(
                    id.clone(),
                    &self.state,
                    &options,
                    Some(host),
                    interactive,
                    window,
                    cx,
                );
                div()
                    .id((id, "popup-surface"))
                    .test_support()
                    .when(interactive, |s| {
                        s.role(gpui_kit::Role::Dialog)
                            .aria_label(format!("{}颜色编辑器", options.name))
                            .track_focus(&popup.read(cx).focus_handle(cx))
                            .overflow_y_scroll()
                    })
                    .w(width)
                    .max_h(maximum_height)
                    .bg(rgba(theme, "--color-picker-panel-background"))
                    .border_1()
                    .border_color(rgba(theme, "--color-picker-panel-border-color"))
                    .rounded(rems(theme, "--color-picker-panel-radius"))
                    .shadow(shadows)
                    .child(content)
                    .into_any_element()
            },
            window,
            cx,
        ) {
            root = root.child(surface);
        }
        root
    }
}
