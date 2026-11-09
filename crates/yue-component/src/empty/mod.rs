//! Empty content presentation. The host owns data, commands and focus recovery.

mod element;

use gpui_kit::{AnyElement, Axis, ElementId, ImageSource, IntoElement, SharedString};

use crate::{Icon, IconDefinition, YueTheme, icon::YueIcon};

/// Density of an empty state; media and spacing scale with its text.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EmptySize {
    Small,
    #[default]
    Medium,
}

enum Media {
    Icon(Icon),
    Image(ImageSource),
    Custom(AnyElement),
    Hidden,
}

enum Description {
    Text(SharedString),
    Custom(AnyElement),
}

/// Media, explanation and optional commands for an empty content region.
///
/// Defaults to an Inbox icon and “暂无数据”. This non-focusable element does not
/// load application data, scroll, announce live results or move focus. Supply
/// existing buttons in `actions`; the host owns their operations and state.
#[derive(IntoElement)]
pub struct YueEmpty {
    id: ElementId,
    theme: Option<YueTheme>,
    size: EmptySize,
    axis: Axis,
    inset: bool,
    media: Media,
    title: Option<SharedString>,
    description: Option<Description>,
    actions: Option<AnyElement>,
}

impl YueEmpty {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            theme: None,
            size: EmptySize::Medium,
            axis: Axis::Vertical,
            inset: true,
            media: Media::Icon(Icon::new(YueIcon::Empty)),
            title: None,
            description: Some(Description::Text("暂无数据".into())),
            actions: None,
        }
    }

    /// Override the current Yue theme for this element.
    pub fn theme(mut self, theme: YueTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    pub fn size(mut self, size: EmptySize) -> Self {
        self.size = size;
        self
    }

    /// Vertical centers content; horizontal places media beside a leading text column.
    pub fn layout(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Disable the outer inset when the containing List or panel already supplies it.
    pub fn inset(mut self, inset: bool) -> Self {
        self.inset = inset;
        self
    }

    /// Replace the media slot. Media setters use last-call-wins ordering.
    pub fn icon(mut self, icon: impl IconDefinition) -> Self {
        self.media = Media::Icon(Icon::new(icon));
        self
    }

    /// Display a contained image in a stable frame, falling back to Inbox on failure.
    pub fn image(mut self, source: impl Into<ImageSource>) -> Self {
        self.media = Media::Image(source.into());
        self
    }

    /// Supply intrinsic media. Its owner handles scaling, fallbacks and semantics.
    pub fn media(mut self, content: impl IntoElement) -> Self {
        self.media = Media::Custom(content.into_any_element());
        self
    }

    pub fn hide_media(mut self) -> Self {
        self.media = Media::Hidden;
        self
    }

    /// Set an optional title. Whitespace-only strings remove the title slot.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        let title = title.into();
        self.title = (!title.trim().is_empty()).then_some(title);
        self
    }

    /// Replace the explanation with accessible text. Empty text removes the slot.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        let text = description.into();
        self.description = (!text.trim().is_empty()).then_some(Description::Text(text));
        self
    }

    /// Replace the explanation with rich content, preserving descendant semantics.
    /// Use semantic text or explicitly labelled elements inside this slot.
    pub fn description_content(mut self, content: impl IntoElement) -> Self {
        self.description = Some(Description::Custom(content.into_any_element()));
        self
    }

    pub fn hide_description(mut self) -> Self {
        self.description = None;
        self
    }

    /// Supply a command or a wrapping group of existing buttons/links.
    /// Replaces the previous action slot; does not append or assign a default action.
    pub fn actions(mut self, content: impl IntoElement) -> Self {
        self.actions = Some(content.into_any_element());
        self
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests;
