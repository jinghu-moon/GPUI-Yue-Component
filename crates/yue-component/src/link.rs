//! Resource links. Base owns activation and focus; Yue owns token presentation.
use std::rc::Rc;

use gpui_kit::base::{Link, Transition, transition};
use gpui_kit::{
    App, ClickEvent, ElementId, FocusHandle, FontWeight, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, SharedString, StatefulInteractiveElement as _, Styled as _,
    UnderlineStyle, Window, div, prelude::FluentBuilder as _, px,
};

use crate::theme::{TokenKey, keys};
use crate::tokens::{duration, easing, mix_rgba, number, pixels, rems, rgba, token};
use crate::{Icon, IconDefinition, YueTheme};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LinkVariant {
    Default,
    #[default]
    Primary,
    Danger,
    Warning,
    Success,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LinkSize {
    Small,
    #[default]
    Medium,
    Large,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LinkHover {
    #[default]
    Underline,
    Color,
    None,
}

type OpenHandler = Rc<dyn Fn(&str, &ClickEvent, &mut Window, &mut App)>;
type ActivationHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// A text link to a URL or email resource, opened by the system by default.
/// Use a Button for application commands. Icons decorate the text; they do not
/// create additional focus targets. Enter and Space follow Base Link activation.
///
/// ```
/// use yue_component::{YueLink, LinkVariant, outline};
/// let link = YueLink::new("docs", "使用指南", "https://example.com/guide")
///     .variant(LinkVariant::Primary)
///     .trailing_icon(outline::ExternalLink);
/// ```
#[derive(IntoElement)]
pub struct YueLink {
    id: ElementId,
    label: SharedString,
    href: SharedString,
    theme: Option<YueTheme>,
    variant: LinkVariant,
    size: LinkSize,
    underline: Option<bool>,
    hover: LinkHover,
    disabled: bool,
    leading: Option<Icon>,
    trailing: Option<Icon>,
    name: Option<SharedString>,
    description: Option<SharedString>,
    focus: Option<FocusHandle>,
    tab_index: isize,
    tab_stop: bool,
    key_context: Option<SharedString>,
    open_with: Option<OpenHandler>,
    on_activate: Option<ActivationHandler>,
    layout: crate::control_layout::ControlLayout,
}

crate::control_layout::control_layout!(YueLink);

impl YueLink {
    /// Creates a resource link with the application's current Yue theme.
    ///
    /// # Panics
    /// Panics if the visible label or href is empty or whitespace-only.
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        href: impl Into<SharedString>,
    ) -> Self {
        let label = label.into();
        let href = href.into();
        assert!(!label.trim().is_empty(), "YueLink requires visible text");
        assert!(
            !href.trim().is_empty(),
            "YueLink requires a resource target"
        );
        Self {
            id: id.into(),
            label,
            href,
            theme: None,
            variant: LinkVariant::default(),
            size: LinkSize::default(),
            underline: None,
            hover: LinkHover::default(),
            disabled: false,
            leading: None,
            trailing: None,
            name: None,
            description: None,
            focus: None,
            tab_index: 0,
            tab_stop: true,
            key_context: None,
            open_with: None,
            on_activate: None,
            layout: Default::default(),
        }
    }

    pub fn theme(mut self, theme: YueTheme) -> Self {
        self.theme = Some(theme);
        self
    }
    pub fn use_default_theme(mut self) -> Self {
        self.theme = None;
        self
    }
    pub fn variant(mut self, variant: LinkVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn size(mut self, size: LinkSize) -> Self {
        self.size = size;
        self
    }
    /// Overrides the theme's default decoration independently of the hover policy.
    pub fn underline(mut self, underline: bool) -> Self {
        self.underline = Some(underline);
        self
    }
    pub fn hover(mut self, hover: LinkHover) -> Self {
        self.hover = hover;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn leading_icon(mut self, icon: impl IconDefinition) -> Self {
        self.leading = Some(Icon::new(icon));
        self
    }
    pub fn trailing_icon(mut self, icon: impl IconDefinition) -> Self {
        self.trailing = Some(Icon::new(icon));
        self
    }

    /// # Panics
    /// Panics if the accessible name is empty or whitespace-only.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        let label = label.into();
        assert!(
            !label.trim().is_empty(),
            "YueLink requires an accessible name"
        );
        self.name = Some(label);
        self
    }
    pub fn accessibility_description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }
    /// Caller-owned focus; tab_index and tab_stop also apply to this handle.
    pub fn track_focus(mut self, focus: &FocusHandle) -> Self {
        self.focus = Some(focus.clone());
        self
    }
    pub fn tab_index(mut self, index: isize) -> Self {
        self.tab_index = index;
        self
    }
    pub fn tab_stop(mut self, stop: bool) -> Self {
        self.tab_stop = stop;
        self
    }
    pub fn key_context(mut self, context: impl Into<SharedString>) -> Self {
        self.key_context = Some(context.into());
        self
    }
    /// Replaces system opening. The href is passed through without normalization.
    pub fn open_with(
        mut self,
        open: impl Fn(&str, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.open_with = Some(Rc::new(open));
        self
    }
    /// Observes activation after the opening strategy; does not suppress opening.
    pub fn on_activate(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_activate = Some(Rc::new(handler));
        self
    }
}

impl LinkVariant {
    fn colors(self) -> (TokenKey, TokenKey) {
        match self {
            Self::Default => (keys::LINK_DEFAULT_COLOR, keys::LINK_DEFAULT_COLOR_HOVER),
            Self::Primary => (keys::LINK_COLOR, keys::LINK_COLOR_HOVER),
            Self::Danger => (keys::LINK_DANGER_COLOR, keys::LINK_DANGER_COLOR_HOVER),
            Self::Warning => (keys::LINK_WARNING_COLOR, keys::LINK_WARNING_COLOR_HOVER),
            Self::Success => (keys::LINK_SUCCESS_COLOR, keys::LINK_SUCCESS_COLOR_HOVER),
        }
    }
}
impl LinkSize {
    fn metrics(self) -> (TokenKey, TokenKey, TokenKey) {
        match self {
            Self::Small => (
                keys::LINK_FONT_SIZE_SM,
                keys::LINK_LINE_HEIGHT_SM,
                keys::LINK_ICON_SIZE_SM,
            ),
            Self::Medium => (
                keys::LINK_FONT_SIZE_MD,
                keys::LINK_LINE_HEIGHT_MD,
                keys::LINK_ICON_SIZE_MD,
            ),
            Self::Large => (
                keys::LINK_FONT_SIZE_LG,
                keys::LINK_LINE_HEIGHT_LG,
                keys::LINK_ICON_SIZE_LG,
            ),
        }
    }
}

#[derive(Default)]
struct LinkMotion {
    hovered: bool,
}

impl RenderOnce for YueLink {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = self.theme.unwrap_or_else(|| crate::current_theme(cx));
        let focus = self.focus.unwrap_or_else(|| {
            window
                .use_keyed_state((self.id.clone(), "yue-link-focus"), cx, |_, cx| {
                    cx.focus_handle()
                })
                .read(cx)
                .clone()
        });
        let motion = window.use_keyed_state((self.id.clone(), "yue-link-motion"), cx, |_, _| {
            LinkMotion::default()
        });
        if self.disabled && motion.read(cx).hovered {
            motion.update(cx, |state, _| state.hovered = false);
        }
        let hovered = !self.disabled && motion.read(cx).hovered;
        let progress = transition(
            (self.id.clone(), "yue-link-hover"),
            if hovered { 1.0 } else { 0.0 },
            Transition::new(duration(theme, keys::LINK_DURATION))
                .ease(easing(theme, keys::LINK_EASE)),
            window,
            cx,
        );
        let (rest, hover) = self.variant.colors();
        let foreground = if self.disabled {
            let mut color = rgba(theme, rest);
            color.a *= number(theme, keys::LINK_OPACITY_DISABLED);
            color
        } else if self.hover == LinkHover::Color {
            mix_rgba(rgba(theme, rest), rgba(theme, hover), progress)
        } else {
            rgba(theme, rest)
        };
        let always_underline = self
            .underline
            .unwrap_or_else(|| token(theme, keys::LINK_DECORATION).value() == "underline");
        let underline_progress = if always_underline {
            1.0
        } else if !self.disabled && self.hover == LinkHover::Underline {
            progress
        } else {
            0.0
        };
        let (font, line, icon) = self.size.metrics();
        let mut text = div().child(self.label.clone());
        // Keep the same text layout at every frame; only underline alpha changes.
        let mut underline_color = foreground;
        underline_color.a *= underline_progress;
        text.text_style().underline = Some(UnderlineStyle {
            thickness: px(pixels(theme, keys::LINK_UNDERLINE_WIDTH)),
            color: Some(underline_color.into()),
            wavy: false,
        });
        let disabled = self.disabled;
        let mut link = Link::new(self.id.clone())
            .href(self.href)
            .track_focus(&focus)
            .tab_index(self.tab_index)
            .tab_stop(self.tab_stop)
            .accessibility_label(self.name.unwrap_or(self.label))
            .disabled(disabled)
            .relative()
            .flex()
            .self_start()
            .items_center()
            .flex_shrink_0()
            .gap(rems(theme, keys::LINK_GAP))
            .text_size(rems(theme, font))
            .line_height(rems(theme, line))
            .font_weight(FontWeight(number(theme, keys::LINK_FONT_WEIGHT)))
            .text_color(foreground)
            .cursor_default()
            .when(!disabled, |link| link.cursor_pointer())
            .on_hover(
                window.listener_for(&motion, move |state, hovered, window, cx| {
                    let hovered = !disabled && *hovered;
                    if state.hovered != hovered {
                        state.hovered = hovered;
                        cx.notify();
                        window.refresh();
                    }
                }),
            );
        if let Some(icon_value) = self.leading {
            link = link.child(icon_value.size(rems(theme, icon)).color_value(foreground));
        }
        link = link.child(text);
        if let Some(icon_value) = self.trailing {
            link = link.child(icon_value.size(rems(theme, icon)).color_value(foreground));
        }
        if !disabled && focus.is_focused(window) && window.last_input_was_keyboard() {
            let outside = rems(theme, keys::LINK_FOCUS_RING_OFFSET)
                + rems(theme, keys::LINK_FOCUS_RING_WIDTH);
            link = link.child(
                div()
                    .absolute()
                    .top(-outside)
                    .right(-outside)
                    .bottom(-outside)
                    .left(-outside)
                    .rounded(rems(theme, keys::LINK_FOCUS_RING_RADIUS))
                    .border(rems(theme, keys::LINK_FOCUS_RING_WIDTH))
                    .border_color(rgba(theme, keys::LINK_FOCUS_RING_COLOR)),
            );
        }
        if let Some(description) = self.description {
            link = link.aria_description(description);
        }
        if let Some(context) = self.key_context {
            link = link.key_context(context.as_ref());
        }
        link = match self.open_with {
            Some(open) => {
                link.open_with(move |href, event, window, cx| open(href, event, window, cx))
            }
            None => link.open_with(|href, _, _, cx| cx.open_url(href)),
        };
        if let Some(handler) = self.on_activate {
            link = link.on_activate(move |event, window, cx| handler(event, window, cx));
        }
        self.layout.apply(link)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "visible text")]
    fn whitespace_label_is_rejected() {
        let _ = YueLink::new("bad", " \n", "https://example.com");
    }
    #[test]
    #[should_panic(expected = "resource target")]
    fn whitespace_target_is_rejected() {
        let _ = YueLink::new("bad", "目标", " \n");
    }
    #[test]
    #[should_panic(expected = "accessible name")]
    fn empty_accessible_name_is_rejected() {
        let _ = YueLink::new("bad", "目标", "mailto:help@example.com").accessibility_label(" ");
    }
}
