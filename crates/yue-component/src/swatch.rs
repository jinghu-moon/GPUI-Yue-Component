//! Controlled background options; checkerboards and gradients are not RGBA values.
use crate::color_picker::paint;
use crate::{
    ColorPickerTrigger, YueColorPicker, YueColorPickerState, YueTheme,
    control_layout::ControlLayout,
    tokens::{pixels, rems, rgba},
};
use gpui_kit::base::{Radio, TestSupportExt as _};
use gpui_kit::{
    App, Background, ElementId, Entity, FocusHandle, InteractiveElement as _, IntoElement,
    KeyDownEvent, MouseButton, ParentElement as _, RenderOnce, Rgba, Role, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, canvas, div, prelude::FluentBuilder as _,
    px,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Clone)]
enum SwatchInk {
    Solid(Rgba),
    Checker(Rgba, Rgba),
    Gradient(Background),
}
#[derive(Clone)]
pub struct YueSwatchOption {
    value: SharedString,
    label: SharedString,
    ink: SwatchInk,
    disabled: bool,
}
impl YueSwatchOption {
    /// # Panics
    /// If a color channel is nonfinite or outside 0..=1.
    pub fn solid(
        value: impl Into<SharedString>,
        label: impl Into<SharedString>,
        color: Rgba,
    ) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            ink: SwatchInk::Solid(valid_color(color)),
            disabled: false,
        }
    }
    /// # Panics
    /// If a color channel is nonfinite or outside 0..=1.
    pub fn checkerboard(
        value: impl Into<SharedString>,
        label: impl Into<SharedString>,
        light: Rgba,
        dark: Rgba,
    ) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            ink: SwatchInk::Checker(valid_color(light), valid_color(dark)),
            disabled: false,
        }
    }
    /// # Panics
    /// If the angle or an RGBA channel is not finite, or a channel is out of range.
    pub fn gradient(
        value: impl Into<SharedString>,
        label: impl Into<SharedString>,
        angle: f32,
        from: Rgba,
        to: Rgba,
    ) -> Self {
        assert!(angle.is_finite(), "Gradient angle must be finite");
        for c in [from, to] {
            assert!(
                [c.r, c.g, c.b, c.a]
                    .iter()
                    .all(|v| v.is_finite() && (0.0..=1.0).contains(v)),
                "Gradient channels must be finite in 0..=1"
            );
        }
        let background = gpui_kit::linear_gradient(
            angle.rem_euclid(360.),
            gpui_kit::linear_color_stop(from, 0.),
            gpui_kit::linear_color_stop(to, 1.),
        )
        .color_space(gpui_kit::ColorSpace::Srgb);
        Self {
            value: value.into(),
            label: label.into(),
            ink: SwatchInk::Gradient(background),
            disabled: false,
        }
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}
fn valid_color(color: Rgba) -> Rgba {
    assert!(
        [color.r, color.g, color.b, color.a]
            .iter()
            .all(|v| v.is_finite() && (0.0..=1.0).contains(v)),
        "Swatch requires finite RGBA in 0..=1"
    );
    color
}
type Handler = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
#[derive(IntoElement)]
pub struct YueSwatchGroup {
    id: ElementId,
    theme: YueTheme,
    inherited: bool,
    value: SharedString,
    label: SharedString,
    options: Vec<YueSwatchOption>,
    disabled: bool,
    handler: Option<Handler>,
    layout: ControlLayout,
    custom: Option<(SharedString, SharedString, Entity<YueColorPickerState>)>,
    pub(crate) paint_only: bool,
    pub(crate) on_activate: Option<Handler>,
    pub(crate) entry: Option<Rc<RefCell<Vec<FocusHandle>>>>,
}
crate::control_layout::control_layout!(YueSwatchGroup);
impl YueSwatchGroup {
    pub fn new(
        id: impl Into<ElementId>,
        theme: YueTheme,
        selected_value: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            theme,
            inherited: false,
            value: selected_value.into(),
            label: SharedString::default(),
            options: vec![],
            disabled: false,
            handler: None,
            layout: Default::default(),
            custom: None,
            entry: None,
            on_activate: None,
            paint_only: false,
        }
    }
    pub fn use_default_theme(mut self) -> Self {
        self.inherited = true;
        self
    }
    pub fn theme(mut self, theme: YueTheme) -> Self {
        self.theme = theme;
        self.inherited = false;
        self
    }
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }
    pub fn accessibility_label(self, label: impl Into<SharedString>) -> Self {
        self.label(label)
    }
    /// # Panics
    /// If values repeat, are empty, or a label is empty.
    pub fn options(mut self, options: impl IntoIterator<Item = YueSwatchOption>) -> Self {
        self.options = options.into_iter().collect();
        for (ix, o) in self.options.iter().enumerate() {
            assert!(
                !o.value.trim().is_empty() && !o.label.trim().is_empty(),
                "Swatch needs value and label"
            );
            assert!(
                !self.options[..ix].iter().any(|p| p.value == o.value),
                "Duplicate swatch value"
            );
        }
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn on_change(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.handler = Some(Rc::new(handler));
        self
    }
    pub fn custom_color(
        mut self,
        value: impl Into<SharedString>,
        label: impl Into<SharedString>,
        state: &Entity<YueColorPickerState>,
    ) -> Self {
        self.custom = Some((value.into(), label.into(), state.clone()));
        self
    }
}
impl RenderOnce for YueSwatchGroup {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.inherited {
            self.theme = crate::current_theme(cx);
        }
        let theme = self.theme;
        assert!(!self.label.trim().is_empty(), "SwatchGroup needs a label");
        if let Some((v, l, _)) = &self.custom {
            assert!(
                !v.trim().is_empty()
                    && !l.trim().is_empty()
                    && !self.options.iter().any(|o| &o.value == v),
                "Custom swatch needs a unique value and label"
            );
        }
        if self.paint_only {
            let mut row = div().flex().flex_wrap().gap(rems(theme, "--swatch-gap"));
            for option in self.options {
                let ink = option.ink;
                row = row.child(
                    div()
                        .size(rems(theme, "--swatch-hit-size"))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .size(rems(theme, "--swatch-size"))
                                .rounded(rems(theme, "--swatch-radius"))
                                .overflow_hidden()
                                .border_1()
                                .border_color(rgba(theme, "--swatch-border-color"))
                                .child(
                                    canvas(
                                        |b, _, _| b,
                                        move |_, b, window, _| match ink {
                                            SwatchInk::Solid(c) => {
                                                window.paint_quad(gpui_kit::fill(b, c))
                                            }
                                            SwatchInk::Checker(a, c) => paint::checker(
                                                b,
                                                a,
                                                c,
                                                px(pixels(theme, "--swatch-checker-size")
                                                    * f32::from(window.rem_size())
                                                    / 16.),
                                                window,
                                            ),
                                            SwatchInk::Gradient(bg) => {
                                                window.paint_quad(gpui_kit::fill(b, bg))
                                            }
                                        },
                                    )
                                    .size_full(),
                                ),
                        ),
                );
            }
            return self.layout.apply(row).into_any_element();
        }
        let custom_value = self.custom.as_ref().map(|(v, _, _)| v.clone());
        let count = self.options.len() + usize::from(self.custom.is_some());
        let focuses = self
            .options
            .iter()
            .map(|o| o.value.clone())
            .chain(custom_value.iter().cloned())
            .map(|value| {
                window
                    .use_keyed_state(
                        ((self.id.clone(), value).into(), "swatch-focus"),
                        cx,
                        |_, cx| cx.focus_handle(),
                    )
                    .read(cx)
                    .clone()
            })
            .collect::<Vec<_>>();
        let focus_list = Rc::new(RefCell::new(
            Vec::<(SharedString, FocusHandle, usize)>::new(),
        ));
        for (ix, option) in self.options.iter().enumerate() {
            if !self.disabled && !option.disabled {
                focus_list
                    .borrow_mut()
                    .push((option.value.clone(), focuses[ix].clone(), ix));
            }
        }
        if !self.disabled
            && let Some(value) = &custom_value
        {
            focus_list
                .borrow_mut()
                .push((value.clone(), focuses[count - 1].clone(), count - 1));
        }
        let cols = Rc::new(Cell::new(1usize));
        let columns = cols.clone();
        let unit = pixels(theme, "--swatch-hit-size") + pixels(theme, "--swatch-gap");
        let scale = f32::from(window.rem_size()) / 16.;
        // The focused item must remain a native Tab stop after arrow navigation;
        // otherwise GPUI cannot locate it in the tab-stop list to leave the group.
        let selected_stop = {
            let list = focus_list.borrow();
            list.iter()
                .find(|(_, f, _)| f.is_focused(window))
                .or_else(|| list.iter().find(|(v, _, _)| v == &self.value))
                .or(list.first())
                .map(|(_, _, ix)| *ix)
        };
        let mut root = div()
            .id(self.id.clone())
            .test_support()
            .role(Role::RadioGroup)
            .aria_label(self.label)
            .relative()
            .flex()
            .flex_wrap()
            .gap(rems(theme, "--swatch-gap"))
            .items_center()
            .child(
                canvas(
                    move |b, _, _| {
                        columns.set(
                            ((f32::from(b.size.width) + pixels(theme, "--swatch-gap") * scale)
                                / (unit * scale))
                                .floor()
                                .max(1.) as usize,
                        )
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            );
        for (ix, option) in self.options.into_iter().enumerate() {
            let id = ElementId::from((self.id.clone(), option.value.clone()));
            let focus = focuses[ix].clone();
            let disabled = self.disabled || option.disabled;
            let checked = option.value == self.value;
            let handler = self.handler.clone();
            let activate = self.on_activate.clone();
            let value = option.value.clone();
            let on_focus = focus.clone();
            let press_focus = focus.clone();
            let ink = option.ink.clone();
            let ring = rems(theme, "--swatch-selected-width");
            let ring_visible =
                !disabled && focus.is_focused(window) && window.last_input_was_keyboard();
            let selection = div()
                .absolute()
                .inset(gpui_kit::rems(
                    ((pixels(theme, "--swatch-hit-size") - pixels(theme, "--swatch-size")) / 2.
                        - pixels(theme, "--swatch-selected-offset")
                        - pixels(theme, "--swatch-selected-width"))
                        / 16.,
                ))
                .rounded(rems(theme, "--swatch-radius"))
                .border(ring)
                .border_color(if checked {
                    rgba(theme, "--swatch-selected-color")
                } else {
                    gpui_kit::transparent_black().into()
                });
            let focus_ring = div()
                .absolute()
                .inset(-gpui_kit::rems(
                    (pixels(theme, "--swatch-focus-ring-offset")
                        + pixels(theme, "--swatch-focus-ring-width"))
                        / 16.,
                ))
                .rounded(rems(theme, "--swatch-radius"))
                .border(rems(theme, "--swatch-focus-ring-width"))
                .border_color(if ring_visible {
                    rgba(theme, "--swatch-focus-ring-color")
                } else {
                    gpui_kit::transparent_black().into()
                });
            let item = Radio::new(id)
                .checked(checked)
                .disabled(disabled)
                .accessibility_label(option.label)
                .track_focus(&focus)
                .tab_stop(!disabled && selected_stop == Some(ix))
                .set_position(ix + 1, count)
                .size(rems(theme, "--swatch-hit-size"))
                .flex()
                .items_center()
                .justify_center()
                .rounded(rems(theme, "--swatch-radius"))
                .relative()
                .when(!disabled, |b| b.cursor_pointer())
                .when(!disabled, |b| {
                    b.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        press_focus.focus(window, cx)
                    })
                })
                .when(!disabled, |b| {
                    b.on_click(move |_, window, cx| {
                        on_focus.focus(window, cx);
                        cx.stop_propagation();
                        if !checked && let Some(h) = &handler {
                            h(&value, window, cx);
                        }
                        if let Some(h) = &activate {
                            h(&value, window, cx);
                        }
                    })
                })
                .child(selection)
                .child(focus_ring)
                .child(
                    div()
                        .size(rems(theme, "--swatch-size"))
                        .rounded(rems(theme, "--swatch-radius"))
                        .overflow_hidden()
                        .border_1()
                        .border_color(rgba(theme, "--swatch-border-color"))
                        .child(
                            canvas(
                                |b, _, _| b,
                                move |_, b, window, _| match ink {
                                    SwatchInk::Solid(c) => {
                                        paint::checker(
                                            b,
                                            rgba(theme, "--color-picker-checker-light-color"),
                                            rgba(theme, "--color-picker-checker-dark-color"),
                                            px(pixels(theme, "--swatch-checker-size")
                                                * f32::from(window.rem_size())
                                                / 16.),
                                            window,
                                        );
                                        window.paint_quad(gpui_kit::fill(b, c));
                                    }
                                    SwatchInk::Checker(a, c) => paint::checker(
                                        b,
                                        a,
                                        c,
                                        px(pixels(theme, "--swatch-checker-size")
                                            * f32::from(window.rem_size())
                                            / 16.),
                                        window,
                                    ),
                                    SwatchInk::Gradient(bg) => {
                                        window.paint_quad(gpui_kit::fill(b, bg))
                                    }
                                },
                            )
                            .size_full(),
                        ),
                );
            root = root.child(item);
        }
        if let Some((value, label, state)) = self.custom {
            let id = ElementId::from((self.id.clone(), value.clone()));
            let focus = focuses[count - 1].clone();
            let handler = self.handler.clone();
            let selected = self.value == value;
            let mut picker = YueColorPicker::new(id, theme, &state)
                .accessibility_label(label)
                .trigger(ColorPickerTrigger::Swatch)
                .size(crate::ColorPickerSize::Small)
                .width(rems(theme, "--swatch-hit-size"))
                .disabled(self.disabled)
                .group_focus(focus, selected_stop == Some(count - 1), selected);
            picker.on_confirm = Some(Rc::new(move |window, cx| {
                if !selected && let Some(h) = &handler {
                    h(&value, window, cx);
                }
            }));
            root = root.child(picker);
        }
        if let Some(entry) = self.entry {
            let list = focus_list.borrow();
            let chosen = list
                .iter()
                .find(|(_, f, _)| f.is_focused(window))
                .or_else(|| list.iter().find(|(v, _, _)| v == &self.value))
                .or(list.first());
            *entry.borrow_mut() = chosen.map(|(_, f, _)| vec![f.clone()]).unwrap_or_default();
        }
        let handler = self.handler;
        let activate = self.on_activate;
        let selected = self.value;
        root = root.on_key_down(move |event: &KeyDownEvent, window, cx| {
            if self.disabled
                || event.keystroke.modifiers.control
                || event.keystroke.modifiers.alt
                || event.keystroke.modifiers.platform
            {
                return;
            }
            let list = focus_list.borrow();
            let Some(ix) = list.iter().position(|(_, f, _)| f.is_focused(window)) else {
                return;
            };
            let n = list.len();
            let key = event.keystroke.key.as_str();
            let next = match key {
                "right" => Some((ix + 1) % n),
                "left" => Some((ix + n - 1) % n),
                "down" => list
                    .iter()
                    .position(|(_, _, pos)| *pos >= list[ix].2 + cols.get()),
                "up" => list[ix]
                    .2
                    .checked_sub(cols.get())
                    .and_then(|target| list.iter().rposition(|(_, _, pos)| *pos <= target)),
                "home" => Some(0),
                "end" => Some(n - 1),
                _ => None,
            };
            if let Some(next) = next {
                cx.stop_propagation();
                list[next].1.focus(window, cx);
            } else if matches!(key, "enter" | "space") && custom_value.as_ref() != Some(&list[ix].0)
            {
                cx.stop_propagation();
                if list[ix].0 != selected
                    && let Some(handler) = &handler
                {
                    handler(&list[ix].0, window, cx);
                }
                if let Some(h) = &activate {
                    h(&list[ix].0, window, cx);
                }
            }
        });
        self.layout.apply(root).into_any_element()
    }
}
