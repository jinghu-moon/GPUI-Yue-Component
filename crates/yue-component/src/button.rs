use std::{rc::Rc, time::Duration};

use gpui_kit::{
    Animation, AnimationExt as _, AnyElement, App, AppContext as _, Axis, ClickEvent, ElementId,
    InteractiveElement as _, IntoElement, MouseButton, ParentElement as _, RenderOnce,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    prelude::FluentBuilder as _,
};

use crate::{YueTheme, YueToken};
use yue_component_icons::{Icon, IconDefinition, outline};

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

fn token(theme: YueTheme, name: &'static str) -> YueToken {
    theme
        .token(name)
        .unwrap_or_else(|| panic!("missing Yue token {name}"))
}

fn token_color(theme: YueTheme, name: &'static str) -> u32 {
    token(theme, name)
        .color()
        .unwrap_or_else(|| panic!("Yue token {name} is not an opaque color"))
}

fn token_rgba(theme: YueTheme, name: &'static str) -> gpui_kit::Rgba {
    token(theme, name)
        .rgba()
        .unwrap_or_else(|| panic!("Yue token {name} is not a supported color"))
}

fn mix_rgba(from: gpui_kit::Rgba, to: gpui_kit::Rgba, amount: f32) -> gpui_kit::Rgba {
    gpui_kit::rgba(
        (((from.r * (1.0 - amount) + to.r * amount) * 255.0).round() as u32) << 24
            | (((from.g * (1.0 - amount) + to.g * amount) * 255.0).round() as u32) << 16
            | (((from.b * (1.0 - amount) + to.b * amount) * 255.0).round() as u32) << 8
            | ((from.a * (1.0 - amount) + to.a * amount) * 255.0).round() as u32,
    )
}

fn token_pixels(theme: YueTheme, name: &'static str) -> f32 {
    token(theme, name)
        .pixels()
        .unwrap_or_else(|| panic!("Yue token {name} is not a pixel length"))
}

fn token_rems(theme: YueTheme, name: &'static str) -> gpui_kit::Rems {
    gpui_kit::rems(token_pixels(theme, name) / 16.0)
}

fn token_number(theme: YueTheme, name: &'static str) -> f32 {
    token(theme, name)
        .number()
        .unwrap_or_else(|| panic!("Yue token {name} is not a number"))
}

fn token_duration(theme: YueTheme, name: &'static str) -> Duration {
    let value = token(theme, name).value().trim();
    let value = value
        .strip_prefix("calc(")
        .and_then(|value| value.strip_suffix(')'))
        .unwrap_or(value);
    if let Some((left, right)) = value.split_once('*') {
        let left = left
            .trim()
            .strip_suffix("ms")
            .and_then(|value| value.trim().parse::<f64>().ok());
        let right = right.trim().parse::<f64>().ok();
        if let (Some(left), Some(right)) = (left, right) {
            return Duration::from_secs_f64(left * right / 1000.0);
        }
    }
    let millis = value
        .strip_suffix("ms")
        .and_then(|value| value.trim().parse::<f64>().ok())
        .unwrap_or_else(|| panic!("Yue token {name} is not a duration"));
    Duration::from_secs_f64(millis / 1000.0)
}

fn token_ease(theme: YueTheme, name: &'static str) -> Box<dyn Fn(f32) -> f32> {
    let value = token(theme, name).value();
    let values = value
        .strip_prefix("cubic-bezier(")
        .and_then(|value| value.strip_suffix(')'))
        .and_then(|value| {
            let values: Vec<_> = value
                .split(',')
                .filter_map(|part| part.trim().parse::<f32>().ok())
                .collect();
            (values.len() == 4).then_some((values[0], values[1], values[2], values[3]))
        })
        .unwrap_or_else(|| panic!("Yue token {name} is not a cubic-bezier easing"));
    let (x1, y1, x2, y2) = values;
    Box::new(move |time| {
        let sample_x = |t: f32| {
            let u = 1.0 - t;
            3.0 * u * u * t * x1 + 3.0 * u * t * t * x2 + t * t * t
        };
        let sample_y = |t: f32| {
            let u = 1.0 - t;
            3.0 * u * u * t * y1 + 3.0 * u * t * t * y2 + t * t * t
        };
        let derivative = |t: f32| {
            3.0 * (1.0 - t) * (1.0 - t) * x1
                + 6.0 * (1.0 - t) * t * (x2 - x1)
                + 3.0 * t * t * (1.0 - x2)
        };
        let mut t = time;
        for _ in 0..8 {
            let slope = derivative(t);
            if slope.abs() < 1e-6 {
                break;
            }
            t = (t - (sample_x(t) - time) / slope).clamp(0.0, 1.0);
        }
        sample_y(t)
    })
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ButtonVariant {
    Primary,
    #[default]
    Default,
    Outline,
    Ghost,
    Text,
    Danger,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ButtonSize {
    Small,
    #[default]
    Medium,
    Large,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum IconPosition {
    #[default]
    Leading,
    Trailing,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ButtonShape {
    #[default]
    RoundedRectangle,
    RoundedSquare,
    Pill,
    Circle,
}

enum ButtonContent {
    Text(SharedString),
    Icon {
        icon: Icon,
        accessibility_label: SharedString,
    },
    IconText {
        icon: Icon,
        label: SharedString,
    },
}

/// Yue button with text-only, icon-only, and icon-plus-text content.
#[derive(IntoElement)]
pub struct YueButton {
    id: ElementId,
    theme: YueTheme,
    variant: ButtonVariant,
    size: ButtonSize,
    shape: ButtonShape,
    icon_position: IconPosition,
    content: ButtonContent,
    disabled: bool,
    loading: bool,
    block: bool,
    tooltip: Option<SharedString>,
    on_click: Option<ClickHandler>,
}

impl YueButton {
    pub fn text(id: impl Into<ElementId>, theme: YueTheme, label: impl Into<SharedString>) -> Self {
        Self::new(id, theme, ButtonContent::Text(label.into()))
    }

    pub fn icon(
        id: impl Into<ElementId>,
        theme: YueTheme,
        icon: impl IconDefinition,
        accessibility_label: impl Into<SharedString>,
    ) -> Self {
        let accessibility_label = accessibility_label.into();
        assert!(
            !accessibility_label.is_empty(),
            "icon-only YueButton requires an accessible name"
        );
        Self::new(
            id,
            theme,
            ButtonContent::Icon {
                icon: Icon::new(icon),
                accessibility_label,
            },
        )
    }

    pub fn icon_text(
        id: impl Into<ElementId>,
        theme: YueTheme,
        icon: impl IconDefinition,
        label: impl Into<SharedString>,
    ) -> Self {
        Self::new(
            id,
            theme,
            ButtonContent::IconText {
                icon: Icon::new(icon),
                label: label.into(),
            },
        )
    }

    fn new(id: impl Into<ElementId>, theme: YueTheme, content: ButtonContent) -> Self {
        Self {
            id: id.into(),
            theme,
            variant: ButtonVariant::Default,
            size: ButtonSize::Medium,
            shape: ButtonShape::RoundedRectangle,
            icon_position: IconPosition::Leading,
            content,
            disabled: false,
            loading: false,
            block: false,
            tooltip: None,
            on_click: None,
        }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }
    pub fn shape(mut self, shape: ButtonShape) -> Self {
        self.shape = shape;
        self
    }
    pub fn icon_position(mut self, position: IconPosition) -> Self {
        self.icon_position = position;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }
    pub fn block(mut self) -> Self {
        self.block = true;
        self
    }
    pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.tooltip = Some(text.into());
        self
    }
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for YueButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let accessibility_label = match &self.content {
            ButtonContent::Text(label) | ButtonContent::IconText { label, .. } => label.clone(),
            ButtonContent::Icon {
                accessibility_label,
                ..
            } => accessibility_label.clone(),
        };
        let icon_only = matches!(&self.content, ButtonContent::Icon { .. });
        let (background_token, hover_token, foreground_token, border_token) = match self.variant {
            ButtonVariant::Primary => (
                "--button-primary-background",
                "--button-primary-background-hover",
                "--button-primary-color",
                "--button-primary-border-color",
            ),
            ButtonVariant::Default => (
                "--button-default-background",
                "--button-default-background-hover",
                "--button-default-color",
                "--button-default-border-color",
            ),
            ButtonVariant::Outline => (
                "--button-outline-background",
                "--button-secondary-background-hover",
                "--button-secondary-color",
                "--button-secondary-border-color",
            ),
            ButtonVariant::Ghost => (
                "--button-ghost-background",
                "--button-ghost-background-hover",
                "--button-ghost-color",
                "--button-ghost-border-color",
            ),
            ButtonVariant::Text => (
                "--button-subtle-background",
                "--button-subtle-background-hover",
                "--button-link-color",
                "--button-ghost-border-color",
            ),
            ButtonVariant::Danger => (
                "--button-danger-background",
                "--button-danger-background-hover",
                "--button-danger-color",
                "--button-danger-border-color",
            ),
        };
        let foreground = token_color(self.theme, foreground_token);
        let background = token_rgba(self.theme, background_token);
        let hover_background = token(self.theme, hover_token).rgba().unwrap_or_else(|| {
            let opacity = token_number(self.theme, "--opacity-hover");
            mix_rgba(gpui_kit::rgba(0), gpui_kit::rgb(foreground), opacity)
        });
        let border = if self.disabled || self.loading {
            token_rgba(self.theme, "--button-disabled-border-color")
        } else {
            token_rgba(self.theme, border_token)
        };
        let button_duration = token_duration(self.theme, "--button-duration");
        let button_ease = token_ease(self.theme, "--button-ease");
        let content_color = if self.disabled || self.loading {
            token_rgba(self.theme, "--button-disabled-color")
        } else {
            gpui_kit::rgb(foreground)
        };
        let disabled_background = token_rgba(self.theme, "--button-disabled-background");
        let disabled = self.disabled || self.loading;
        let (hovered, pressed, motion_state) = if disabled {
            (false, false, None)
        } else {
            let state =
                window.use_keyed_state((self.id.clone(), "yue-button-motion"), cx, |_, _| {
                    ButtonMotion::default()
                });
            let current = state.read(cx);
            (current.hovered, current.pressed, Some(state))
        };
        let hover_progress = gpui_kit::base::transition(
            (self.id.clone(), "yue-button-hover"),
            if hovered { 1.0 } else { 0.0 },
            gpui_kit::base::Transition::new(button_duration)
                .ease(token_ease(self.theme, "--button-ease")),
            window,
            cx,
        );
        let press_progress = gpui_kit::base::transition(
            (self.id.clone(), "yue-button-press"),
            if pressed { 1.0 } else { 0.0 },
            gpui_kit::base::Transition::new(button_duration).ease(button_ease),
            window,
            cx,
        );
        let animated_background = mix_rgba(background, hover_background, hover_progress);
        let press_inset = token_rems(self.theme, "--button-border-width") * press_progress;

        let mut button = gpui_kit::base::Button::new(self.id)
            .disabled(disabled)
            .accessibility_label(accessibility_label)
            .flex()
            .items_center()
            .justify_center()
            .text_size(token_rems(
                self.theme,
                match self.size {
                    ButtonSize::Small => "--button-font-size-sm",
                    ButtonSize::Medium => "--button-font-size-md",
                    ButtonSize::Large => "--button-font-size-lg",
                },
            ))
            .line_height(token_rems(self.theme, "--button-line-height"))
            .font_weight(gpui_kit::FontWeight(token_number(
                self.theme,
                "--button-font-weight",
            )))
            .bg(gpui_kit::transparent_black())
            .border(token_rems(self.theme, "--button-border-width"))
            .border_color(gpui_kit::transparent_black())
            .text_color(content_color)
            .px(token_rems(self.theme, "--button-padding-inline-flush"))
            .when(!disabled, |button| button.cursor_pointer())
            .focus_visible(|style| {
                style
                    .border(token_rems(self.theme, "--button-focus-ring-width"))
                    .border_color(token_rgba(self.theme, "--button-focus-ring-color"))
            });

        if let Some(state) = motion_state {
            let hover = window.listener_for(&state, |state, hovered, window, cx| {
                if state.hovered != *hovered {
                    state.hovered = *hovered;
                    cx.notify();
                    window.refresh();
                }
            });
            let down = window.listener_for(&state, |state, _, window, cx| {
                if !state.pressed {
                    state.pressed = true;
                    cx.notify();
                    window.refresh();
                }
            });
            let up = window.listener_for(&state, |state, _, window, cx| {
                if state.pressed {
                    state.pressed = false;
                    cx.notify();
                    window.refresh();
                }
            });
            let up_out = window.listener_for(&state, |state, _, window, cx| {
                if state.pressed {
                    state.pressed = false;
                    state.hovered = false;
                    cx.notify();
                    window.refresh();
                }
            });
            button = button
                .on_hover(hover)
                .on_mouse_down(MouseButton::Left, down)
                .on_mouse_up(MouseButton::Left, up)
                .on_mouse_up_out(MouseButton::Left, up_out);
        }

        let (height_token, padding_token, icon_token) = match self.size {
            ButtonSize::Small => (
                "--button-height-sm",
                "--button-padding-inline-sm",
                "--button-icon-size-sm",
            ),
            ButtonSize::Medium => (
                "--button-height-md",
                "--button-padding-inline-md",
                "--button-icon-size-md",
            ),
            ButtonSize::Large => (
                "--button-height-lg",
                "--button-padding-inline-lg",
                "--button-icon-size-lg",
            ),
        };
        let height = token_rems(self.theme, height_token);
        let horizontal_padding = token_rems(self.theme, padding_token);
        let icon_size = token_rems(self.theme, icon_token);
        let button_gap = token_rems(self.theme, "--button-gap");
        button = button.h(height).when(self.block, |button| button.w_full());
        if icon_only {
            button = button.w(height);
        }

        let surface = div()
            .absolute()
            .top(press_inset)
            .right(press_inset)
            .bottom(press_inset)
            .left(press_inset)
            .bg(if disabled {
                disabled_background
            } else {
                animated_background
            })
            .border(token_rems(self.theme, "--button-border-width"))
            .border_color(border);
        let surface = match self.shape {
            ButtonShape::RoundedRectangle => {
                surface.rounded(token_rems(self.theme, "--button-border-radius"))
            }
            ButtonShape::RoundedSquare => surface.rounded(token_rems(self.theme, "--radius-md")),
            ButtonShape::Pill => {
                surface.rounded(token_rems(self.theme, "--button-border-radius-full"))
            }
            ButtonShape::Circle => {
                surface.rounded(token_rems(self.theme, "--button-border-radius-full"))
            }
        };
        let render_content = || -> AnyElement {
            match &self.content {
                ButtonContent::Text(label) => div()
                    .text_color(content_color)
                    .child(label.clone())
                    .into_any_element(),
                ButtonContent::Icon { icon, .. } => div()
                    .child(icon.size(icon_size).color_value(content_color))
                    .into_any_element(),
                ButtonContent::IconText { icon, label } => {
                    let icon = icon.size(icon_size).color_value(content_color);
                    let row = div()
                        .flex()
                        .items_center()
                        .gap(button_gap)
                        .text_color(content_color);
                    match self.icon_position {
                        IconPosition::Leading => {
                            row.child(icon).child(label.clone()).into_any_element()
                        }
                        IconPosition::Trailing => {
                            row.child(label.clone()).child(icon).into_any_element()
                        }
                    }
                }
            }
        };
        let sized_content = div()
            .child(render_content())
            .when(!icon_only, |content| content.px(horizontal_padding))
            .opacity(0.0);
        let visible_content = if self.loading {
            Icon::new(outline::Loader)
                .size(icon_size)
                .color_value(content_color)
                .with_animation(
                    "yue-button-spinner",
                    Animation::new(token_duration(self.theme, "--button-spinner-duration"))
                        .repeat()
                        .with_easing(token_ease(self.theme, "--button-ease")),
                    |icon, delta| icon.rotation(delta),
                )
                .into_any_element()
        } else {
            render_content()
        };
        let content = div()
            .absolute()
            .top_0()
            .right_0()
            .bottom_0()
            .left_0()
            .flex()
            .items_center()
            .justify_center()
            .gap(button_gap)
            .text_size(token_rems(
                self.theme,
                match self.size {
                    ButtonSize::Small => "--button-font-size-sm",
                    ButtonSize::Medium => "--button-font-size-md",
                    ButtonSize::Large => "--button-font-size-lg",
                },
            ))
            .text_color(content_color)
            .child(visible_content);
        button = button.child(sized_content).child(surface).child(content);
        if let Some(handler) = self.on_click {
            button = button.on_click(move |event, window, cx| handler(event, window, cx));
        }
        if let Some(text) = self.tooltip {
            let theme = self.theme;
            button = button.tooltip(move |_, cx| {
                cx.new(|_| TooltipView {
                    text: text.clone(),
                    theme,
                })
                .into()
            });
        }
        button
    }
}

#[derive(Default)]
struct ButtonMotion {
    hovered: bool,
    pressed: bool,
}

struct TooltipView {
    text: SharedString,
    theme: YueTheme,
}
impl gpui_kit::Render for TooltipView {
    fn render(&mut self, _: &mut Window, _: &mut gpui_kit::Context<Self>) -> impl IntoElement {
        div()
            .px(token_rems(self.theme, "--box-padding-inline-tooltip"))
            .py(token_rems(self.theme, "--box-padding-block-tooltip"))
            .rounded(token_rems(self.theme, "--box-border-radius-tooltip"))
            .bg(token_rgba(self.theme, "--box-background-tooltip"))
            .text_color(token_rgba(self.theme, "--box-color-tooltip"))
            .text_size(token_rems(self.theme, "--box-font-size-tooltip"))
            .child(self.text.clone())
    }
}

/// Lays out related buttons on one axis without changing their individual behavior.
#[derive(IntoElement)]
pub struct ButtonGroup {
    axis: Axis,
    gap: gpui_kit::Rems,
    children: Vec<YueButton>,
}
impl ButtonGroup {
    pub fn new(theme: YueTheme) -> Self {
        Self {
            axis: Axis::Horizontal,
            gap: token_rems(theme, "--button-gap"),
            children: Vec::new(),
        }
    }
    pub fn vertical(mut self) -> Self {
        self.axis = Axis::Vertical;
        self
    }
    pub fn horizontal(mut self) -> Self {
        self.axis = Axis::Horizontal;
        self
    }
    pub fn gap(mut self, gap: gpui_kit::Rems) -> Self {
        self.gap = gap;
        self
    }
    pub fn child(mut self, button: YueButton) -> Self {
        self.children.push(button);
        self
    }
    pub fn children(mut self, buttons: impl IntoIterator<Item = YueButton>) -> Self {
        self.children.extend(buttons);
        self
    }
}
impl RenderOnce for ButtonGroup {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .when(self.axis == Axis::Vertical, |group| group.flex_col())
            .gap(self.gap)
            .children(self.children)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ButtonContent, ButtonGroup, ButtonShape, ButtonSize, ButtonVariant, IconPosition,
        YueButton, mix_rgba, token_color, token_duration, token_ease, token_pixels, token_rgba,
    };
    use crate::{Accent, ThemeMode, YueTheme};
    use std::time::Duration;
    use yue_component_icons::{filled, outline};

    #[test]
    fn constructors_expose_three_content_forms_and_icon_position() {
        let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
        assert!(matches!(
            YueButton::text("text", theme, "Save").content,
            ButtonContent::Text(_)
        ));
        assert!(matches!(
            YueButton::icon("search", theme, outline::Search, "Search").content,
            ButtonContent::Icon { .. }
        ));
        let icon_text = YueButton::icon_text("add", theme, filled::Heart, "Favorite")
            .icon_position(IconPosition::Trailing);
        assert!(matches!(icon_text.content, ButtonContent::IconText { .. }));
        assert_eq!(icon_text.icon_position, IconPosition::Trailing);
    }

    #[test]
    #[should_panic(expected = "icon-only YueButton requires an accessible name")]
    fn icon_only_button_rejects_an_empty_accessible_name() {
        YueButton::icon(
            "search",
            YueTheme::new(ThemeMode::Light, Accent::Azure),
            outline::Search,
            "",
        );
    }

    #[test]
    fn builder_options_retain_requested_states() {
        let button = YueButton::text(
            "options",
            YueTheme::new(ThemeMode::Light, Accent::Azure),
            "Run",
        )
        .variant(ButtonVariant::Danger)
        .size(ButtonSize::Large)
        .shape(ButtonShape::Circle)
        .loading(true)
        .block()
        .tooltip("Runs task");
        assert_eq!(button.variant, ButtonVariant::Danger);
        assert_eq!(button.size, ButtonSize::Large);
        assert_eq!(button.shape, ButtonShape::Circle);
        assert!(button.loading && button.block && button.tooltip.is_some());

        let pill = YueButton::text(
            "pill",
            YueTheme::new(ThemeMode::Light, Accent::Azure),
            "Pill",
        )
        .shape(ButtonShape::Pill);
        assert_eq!(pill.shape, ButtonShape::Pill);
    }

    #[test]
    fn button_presentation_reads_component_tokens() {
        let light = YueTheme::new(ThemeMode::Light, Accent::Azure);
        let dark = YueTheme::new(ThemeMode::Dark, Accent::Azure);

        assert_eq!(token_pixels(light, "--button-height-sm"), 28.0);
        assert_eq!(token_pixels(light, "--button-height-md"), 32.0);
        assert_eq!(token_pixels(light, "--button-height-lg"), 40.0);
        assert_eq!(token_pixels(light, "--button-border-radius"), 4.0);
        assert_eq!(token_pixels(light, "--button-gap"), 8.0);
        assert_eq!(
            token_duration(light, "--button-duration"),
            Duration::from_millis(100)
        );
        assert_eq!(
            token_duration(light, "--button-spinner-duration"),
            Duration::from_millis(900)
        );
        assert_eq!(token_color(light, "--button-danger-background"), 0xa14138);
        assert_eq!(token_color(dark, "--button-danger-background"), 0xf5aaa3);
        assert!(token_rgba(light, "--button-danger-background-hover").a > 0.99);
        assert!(token_rgba(light, "--button-disabled-background").a < 1.0);
        assert!(token_rgba(light, "--button-disabled-color").a < 1.0);
        let _ = token_ease(light, "--button-ease");
        let text_foreground = token_color(light, "--button-link-color");
        let text_hover = mix_rgba(gpui_kit::rgba(0), gpui_kit::rgb(text_foreground), 0.08);
        assert!((text_hover.a - 0.08).abs() < 0.01);
    }

    #[test]
    fn button_group_defaults_to_the_button_gap_token() {
        let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
        let group = ButtonGroup::new(theme);
        assert_eq!(group.gap.0, 0.5);
    }
}
