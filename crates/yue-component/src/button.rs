use std::rc::Rc;

use gpui_kit::{
    AnyElement, App, Axis, ClickEvent, ElementId, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement as _, RenderOnce, SharedString, StatefulInteractiveElement as _,
    Styled as _, Window, div, prelude::FluentBuilder as _,
};

use crate::tokens::{
    duration as token_duration, easing as token_ease, mix_rgba, number as token_number,
    rems as token_rems, rgba as token_rgba, token,
};

fn token_color(theme: YueTheme, name: impl crate::theme::TokenSource) -> u32 {
    token(theme, name)
        .color()
        .unwrap_or_else(|| panic!("Yue token {name} is not an opaque color"))
}
use crate::YueTheme;
use yue_component_icons::{Icon, IconDefinition};

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

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
    use_default_theme: bool,
    variant: ButtonVariant,
    size: ButtonSize,
    shape: ButtonShape,
    icon_position: IconPosition,
    icon_size_override: Option<gpui_kit::Rems>,
    content: ButtonContent,
    disabled: bool,
    loading: bool,
    block: bool,
    expanded: Option<bool>,
    tooltip: Option<SharedString>,
    tooltip_options: crate::TooltipOptions,
    on_click: Option<ClickHandler>,
    layout: crate::control_layout::ControlLayout,
    focus_handle: Option<gpui_kit::FocusHandle>,
    key_context: Option<gpui_kit::SharedString>,
    description: Option<gpui_kit::SharedString>,
    accessibility_label: Option<SharedString>,
    tab_index: isize,
    tab_stop: bool,
}

crate::control_layout::control_layout!(YueButton);

impl YueButton {
    /// Override the accessible command name without changing visible text.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        let label = label.into();
        assert!(!label.is_empty(), "YueButton requires an accessible name");
        self.accessibility_label = Some(label);
        self
    }
    /// Use caller-owned focus without replacing the activation handler.
    pub fn track_focus(mut self, focus: &gpui_kit::FocusHandle) -> Self {
        self.focus_handle = Some(focus.clone());
        self
    }
    pub fn key_context(mut self, context: impl Into<gpui_kit::SharedString>) -> Self {
        self.key_context = Some(context.into());
        self
    }
    pub fn accessibility_description(
        mut self,
        description: impl Into<gpui_kit::SharedString>,
    ) -> Self {
        self.description = Some(description.into());
        self
    }
    /// Focus traversal order. It is applied to whichever handle the control
    /// tracks, including one passed to `track_focus`, so set the order here
    /// rather than on that handle.
    pub fn tab_index(mut self, index: isize) -> Self {
        self.tab_index = index;
        self
    }
    /// Whether Tab traversal stops on the control; like `tab_index`, it is
    /// applied to a caller-owned handle too. The default is `true`.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    /// A text command using the application theme. The owner supplies its handler.
    /// ```
    /// use yue_component::YueButton;
    /// let save = YueButton::new("save", "保存").on_click(|_, _, _| {});
    /// ```
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self::text(
            id,
            YueTheme::new(crate::ThemeMode::Light, crate::Accent::Azure),
            label,
        )
        .use_default_theme()
    }

    /// Resolve the application default at render time, including later theme changes.
    pub fn use_default_theme(mut self) -> Self {
        self.use_default_theme = true;
        self
    }
    /// Override the application default for this component instance.
    pub fn theme(mut self, theme: YueTheme) -> Self {
        self.theme = theme;
        self.use_default_theme = false;
        self
    }

    pub fn text(id: impl Into<ElementId>, theme: YueTheme, label: impl Into<SharedString>) -> Self {
        Self::with_content(id, theme, ButtonContent::Text(label.into()))
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
        Self::with_content(
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
        Self::with_content(
            id,
            theme,
            ButtonContent::IconText {
                icon: Icon::new(icon),
                label: label.into(),
            },
        )
    }

    fn with_content(id: impl Into<ElementId>, theme: YueTheme, content: ButtonContent) -> Self {
        Self {
            id: id.into(),
            theme,
            use_default_theme: false,
            layout: Default::default(),
            focus_handle: None,
            key_context: None,
            description: None,
            accessibility_label: None,
            tab_index: 0,
            tab_stop: true,
            variant: ButtonVariant::Default,
            size: ButtonSize::Medium,
            shape: ButtonShape::RoundedRectangle,
            icon_position: IconPosition::Leading,
            icon_size_override: None,
            content,
            disabled: false,
            loading: false,
            block: false,
            expanded: None,
            tooltip: None,
            tooltip_options: crate::TooltipOptions::default(),
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
    /// Internal component glyph sizing; preserves the normal Button hit target.
    pub(crate) fn icon_size_override(mut self, size: gpui_kit::Rems) -> Self {
        self.icon_size_override = Some(size);
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
    /// The expanded state of a disclosure control: announces itself, and turns
    /// the button's own chevron half a circle towards the panel it opened.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = Some(expanded);
        self
    }
    pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.tooltip = Some(text.into());
        self
    }
    /// Configures the shared Tooltip layer; advanced triggers use YueTooltip.
    pub fn tooltip_options(mut self, options: crate::TooltipOptions) -> Self {
        self.tooltip_options = options;
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

/// The share of `color-mix(in srgb, currentColor calc(N*100%), transparent)`.
fn current_color_wash(value: &str) -> Option<f32> {
    let amount = value
        .strip_prefix("color-mix(in srgb, currentColor calc(")?
        .strip_suffix("*100%), transparent)")?
        .parse::<f32>()
        .ok()?;
    (0.0..=1.0).contains(&amount).then_some(amount)
}

impl RenderOnce for YueButton {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.use_default_theme {
            self.theme = crate::current_theme(cx);
        }
        let accessibility_label =
            self.accessibility_label
                .clone()
                .unwrap_or_else(|| match &self.content {
                    ButtonContent::Text(label) | ButtonContent::IconText { label, .. } => {
                        label.clone()
                    }
                    ButtonContent::Icon {
                        accessibility_label,
                        ..
                    } => accessibility_label.clone(),
                });
        let icon_only = matches!(&self.content, ButtonContent::Icon { .. });
        let (background_token, hover_token, foreground_token, border_token) = match self.variant {
            ButtonVariant::Primary => (
                crate::theme::keys::BUTTON_PRIMARY_BACKGROUND,
                crate::theme::keys::BUTTON_PRIMARY_BACKGROUND_HOVER,
                crate::theme::keys::BUTTON_PRIMARY_COLOR,
                crate::theme::keys::BUTTON_PRIMARY_BORDER_COLOR,
            ),
            ButtonVariant::Default => (
                crate::theme::keys::BUTTON_DEFAULT_BACKGROUND,
                crate::theme::keys::BUTTON_DEFAULT_BACKGROUND_HOVER,
                crate::theme::keys::BUTTON_DEFAULT_COLOR,
                crate::theme::keys::BUTTON_DEFAULT_BORDER_COLOR,
            ),
            ButtonVariant::Outline => (
                crate::theme::keys::BUTTON_OUTLINE_BACKGROUND,
                crate::theme::keys::BUTTON_SECONDARY_BACKGROUND_HOVER,
                crate::theme::keys::BUTTON_SECONDARY_COLOR,
                crate::theme::keys::BUTTON_SECONDARY_BORDER_COLOR,
            ),
            ButtonVariant::Ghost => (
                crate::theme::keys::BUTTON_GHOST_BACKGROUND,
                crate::theme::keys::BUTTON_GHOST_BACKGROUND_HOVER,
                crate::theme::keys::BUTTON_GHOST_COLOR,
                crate::theme::keys::BUTTON_GHOST_BORDER_COLOR,
            ),
            ButtonVariant::Text => (
                crate::theme::keys::BUTTON_SUBTLE_BACKGROUND,
                crate::theme::keys::BUTTON_SUBTLE_BACKGROUND_HOVER,
                crate::theme::keys::BUTTON_LINK_COLOR,
                crate::theme::keys::BUTTON_GHOST_BORDER_COLOR,
            ),
            ButtonVariant::Danger => (
                crate::theme::keys::BUTTON_DANGER_BACKGROUND,
                crate::theme::keys::BUTTON_DANGER_BACKGROUND_HOVER,
                crate::theme::keys::BUTTON_DANGER_COLOR,
                crate::theme::keys::BUTTON_DANGER_BORDER_COLOR,
            ),
        };
        let foreground = token_color(self.theme, foreground_token);
        let background = token_rgba(self.theme, background_token);
        let hover = token(self.theme, hover_token);
        // Only a `currentColor` wash is left for the button to resolve, since
        // the label colour is its own; any other unreadable value is a broken
        // token and must not quietly become a tint.
        let hover_background = hover.rgba().unwrap_or_else(|| {
            let amount = current_color_wash(hover.value())
                .unwrap_or_else(|| panic!("Yue token {} is not a supported color", hover.name()));
            mix_rgba(gpui_kit::rgba(0), gpui_kit::rgb(foreground), amount)
        });
        let border = if self.disabled || self.loading {
            token_rgba(self.theme, crate::theme::keys::BUTTON_DISABLED_BORDER_COLOR)
        } else {
            token_rgba(self.theme, border_token)
        };
        let button_duration = token_duration(self.theme, crate::theme::keys::BUTTON_DURATION);
        let content_color = if self.disabled || self.loading {
            token_rgba(self.theme, crate::theme::keys::BUTTON_DISABLED_COLOR)
        } else {
            gpui_kit::rgb(foreground)
        };
        let disabled_background =
            token_rgba(self.theme, crate::theme::keys::BUTTON_DISABLED_BACKGROUND);
        let disabled = self.disabled || self.loading;
        let focus_handle = self.focus_handle.clone().unwrap_or_else(|| {
            window
                .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
                .read(cx)
                .clone()
        });
        let ring_visible =
            !disabled && focus_handle.is_focused(window) && window.last_input_was_keyboard();
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
                .ease(token_ease(self.theme, crate::theme::keys::BUTTON_EASE)),
            window,
            cx,
        );
        let press_progress = gpui_kit::base::transition(
            (self.id.clone(), "yue-button-press"),
            if pressed { 1.0 } else { 0.0 },
            gpui_kit::base::Transition::new(button_duration)
                .ease(token_ease(self.theme, crate::theme::keys::BUTTON_EASE)),
            window,
            cx,
        );
        let animated_background = mix_rgba(background, hover_background, hover_progress);
        let press_inset =
            token_rems(self.theme, crate::theme::keys::BUTTON_BORDER_WIDTH) * press_progress;
        // A disclosure's chevron turns half a circle towards the panel it opened,
        // on the same clock the button's own hover and press ramps use. Only the
        // icon that faces the disclosure turns: a leading icon belongs to the
        // action, not to the panel.
        let expand_turn = self
            .expanded
            .filter(|_| icon_only || self.icon_position == IconPosition::Trailing)
            .map(|open| {
                gpui_kit::base::transition(
                    (self.id.clone(), "yue-button-expand"),
                    if open { 0.5 } else { 0.0 },
                    gpui_kit::base::Transition::new(button_duration)
                        .ease(token_ease(self.theme, crate::theme::keys::BUTTON_EASE)),
                    window,
                    cx,
                )
            });
        let expand = |icon: Icon| -> Icon {
            match expand_turn {
                Some(turn) => icon.rotation(turn),
                None => icon,
            }
        };

        let mut button = gpui_kit::base::Button::new(self.id.clone())
            .track_focus(&focus_handle)
            .disabled(disabled)
            .accessibility_label(accessibility_label)
            .flex()
            .items_center()
            .justify_center()
            .text_size(token_rems(
                self.theme,
                match self.size {
                    ButtonSize::Small => crate::theme::keys::BUTTON_FONT_SIZE_SM,
                    ButtonSize::Medium => crate::theme::keys::BUTTON_FONT_SIZE_MD,
                    ButtonSize::Large => crate::theme::keys::BUTTON_FONT_SIZE_LG,
                },
            ))
            .line_height(token_rems(
                self.theme,
                crate::theme::keys::BUTTON_LINE_HEIGHT,
            ))
            .font_weight(gpui_kit::FontWeight(token_number(
                self.theme,
                crate::theme::keys::BUTTON_FONT_WEIGHT,
            )))
            .bg(gpui_kit::transparent_black())
            .border(token_rems(
                self.theme,
                crate::theme::keys::BUTTON_BORDER_WIDTH,
            ))
            .border_color(gpui_kit::transparent_black())
            .text_color(content_color)
            .px(token_rems(
                self.theme,
                crate::theme::keys::BUTTON_PADDING_INLINE_FLUSH,
            ))
            .when(!disabled, |button| button.cursor_pointer());

        if let Some(expanded) = self.expanded {
            button = button.aria_expanded(expanded);
        }

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
                crate::theme::keys::BUTTON_HEIGHT_SM,
                crate::theme::keys::BUTTON_PADDING_INLINE_SM,
                crate::theme::keys::BUTTON_ICON_SIZE_SM,
            ),
            ButtonSize::Medium => (
                crate::theme::keys::BUTTON_HEIGHT_MD,
                crate::theme::keys::BUTTON_PADDING_INLINE_MD,
                crate::theme::keys::BUTTON_ICON_SIZE_MD,
            ),
            ButtonSize::Large => (
                crate::theme::keys::BUTTON_HEIGHT_LG,
                crate::theme::keys::BUTTON_PADDING_INLINE_LG,
                crate::theme::keys::BUTTON_ICON_SIZE_LG,
            ),
        };
        let height = token_rems(self.theme, height_token);
        let horizontal_padding = token_rems(self.theme, padding_token);
        let icon_size = self
            .icon_size_override
            .unwrap_or_else(|| token_rems(self.theme, icon_token));
        let button_gap = token_rems(self.theme, crate::theme::keys::BUTTON_GAP);
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
            .border(token_rems(
                self.theme,
                crate::theme::keys::BUTTON_BORDER_WIDTH,
            ))
            .border_color(border);
        let surface = match self.shape {
            ButtonShape::RoundedRectangle => surface.rounded(token_rems(
                self.theme,
                crate::theme::keys::BUTTON_BORDER_RADIUS,
            )),
            ButtonShape::RoundedSquare => {
                surface.rounded(token_rems(self.theme, crate::theme::keys::RADIUS_MD))
            }
            ButtonShape::Pill => surface.rounded(token_rems(
                self.theme,
                crate::theme::keys::BUTTON_BORDER_RADIUS_FULL,
            )),
            ButtonShape::Circle => surface.rounded(token_rems(
                self.theme,
                crate::theme::keys::BUTTON_BORDER_RADIUS_FULL,
            )),
        };
        let render_content = || -> AnyElement {
            match &self.content {
                ButtonContent::Text(label) => div()
                    .text_color(content_color)
                    .child(label.clone())
                    .into_any_element(),
                ButtonContent::Icon { icon, .. } => div()
                    .child(expand(icon.size(icon_size).color_value(content_color)))
                    .into_any_element(),
                ButtonContent::IconText { icon, label } => {
                    let icon = expand(icon.size(icon_size).color_value(content_color));
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
            crate::loading::spinner_glyph(
                self.theme,
                icon_size,
                token_rems(self.theme, crate::theme::keys::SPINNER_BORDER_WIDTH),
                content_color,
                token_duration(self.theme, crate::theme::keys::BUTTON_SPINNER_DURATION),
                true,
            )
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
                    ButtonSize::Small => crate::theme::keys::BUTTON_FONT_SIZE_SM,
                    ButtonSize::Medium => crate::theme::keys::BUTTON_FONT_SIZE_MD,
                    ButtonSize::Large => crate::theme::keys::BUTTON_FONT_SIZE_LG,
                },
            ))
            .text_color(content_color)
            .child(visible_content);
        button = button.child(sized_content).child(surface).child(content);
        // Focus is paint and never changes intrinsic width or sibling positions.
        if ring_visible {
            let radius = match self.shape {
                ButtonShape::RoundedRectangle => crate::theme::keys::BUTTON_BORDER_RADIUS,
                ButtonShape::RoundedSquare => crate::theme::keys::RADIUS_MD,
                ButtonShape::Pill | ButtonShape::Circle => {
                    crate::theme::keys::BUTTON_BORDER_RADIUS_FULL
                }
            };
            let outer_border = token_rems(self.theme, crate::theme::keys::BUTTON_BORDER_WIDTH);
            button = button.child(
                div()
                    .absolute()
                    .top(-outer_border)
                    .right(-outer_border)
                    .bottom(-outer_border)
                    .left(-outer_border)
                    .border(token_rems(
                        self.theme,
                        crate::theme::keys::BUTTON_FOCUS_RING_WIDTH,
                    ))
                    .border_color(token_rgba(
                        self.theme,
                        crate::theme::keys::BUTTON_FOCUS_RING_COLOR,
                    ))
                    .rounded(token_rems(self.theme, radius)),
            );
        }
        if let Some(handler) = self.on_click {
            button = button.on_click(move |event, window, cx| handler(event, window, cx));
        }
        if let Some(text) = self.tooltip {
            if self.description.is_none() && !text.trim().is_empty() {
                button = button.aria_description(text.clone());
            }
            let attachment = crate::tooltip::attach(
                self.id.clone(),
                crate::tooltip::Config::new(text, self.theme, self.tooltip_options)
                    .disabled(disabled),
                Some(focus_handle.clone()),
                window,
                cx,
            );
            button = button.relative().child(attachment);
        }
        button = self
            .layout
            .apply(button)
            .tab_index(self.tab_index)
            .tab_stop(self.tab_stop);
        if let Some(context) = self.key_context {
            button = button.key_context(context.as_ref());
        }
        if let Some(description) = self.description {
            button = button.aria_description(description);
        }
        button
    }
}

#[derive(Default)]
struct ButtonMotion {
    hovered: bool,
    pressed: bool,
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
            gap: token_rems(theme, crate::theme::keys::BUTTON_GAP),
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
        YueButton, mix_rgba, token_color, token_duration, token_ease, token_rgba,
    };
    use crate::tokens::pixels as token_pixels;
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

        assert_eq!(
            token_pixels(light, crate::theme::keys::BUTTON_HEIGHT_SM),
            28.0
        );
        assert_eq!(
            token_pixels(light, crate::theme::keys::BUTTON_HEIGHT_MD),
            32.0
        );
        assert_eq!(
            token_pixels(light, crate::theme::keys::BUTTON_HEIGHT_LG),
            40.0
        );
        assert_eq!(
            token_pixels(light, crate::theme::keys::BUTTON_BORDER_RADIUS),
            4.0
        );
        assert_eq!(token_pixels(light, crate::theme::keys::BUTTON_GAP), 8.0);
        assert_eq!(
            token_duration(light, crate::theme::keys::BUTTON_DURATION),
            Duration::from_millis(100)
        );
        assert_eq!(
            token_duration(light, crate::theme::keys::BUTTON_SPINNER_DURATION),
            Duration::from_millis(900)
        );
        assert_eq!(token_color(light, "--button-danger-background"), 0xa14138);
        assert_eq!(token_color(dark, "--button-danger-background"), 0xea7d73);
        assert!(token_rgba(light, crate::theme::keys::BUTTON_DANGER_BACKGROUND_HOVER).a > 0.99);
        assert!(token_rgba(light, crate::theme::keys::BUTTON_DISABLED_BACKGROUND).a < 1.0);
        assert!(token_rgba(light, crate::theme::keys::BUTTON_DISABLED_COLOR).a < 1.0);
        let _ = token_ease(light, crate::theme::keys::BUTTON_EASE);
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
