//! Noninteractive counts, notification dots and compact status labels.

mod element;

use crate::YueTheme;
use gpui_kit::{AnyElement, ElementId, IntoElement, Point, Rems, Rgba, SharedString, point, rems};

/// Token-defined badge density. Small preserves the readable text size.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BadgeSize {
    Small,
    #[default]
    Medium,
}

/// Shape of a count or text badge. Notification dots are always circular.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BadgeShape {
    #[default]
    Pill,
    Rounded,
}

/// Semantic color of the mark, independently of its content.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BadgeTone {
    Default,
    Primary,
    Success,
    Warning,
    #[default]
    Danger,
}

#[derive(Clone, Debug, PartialEq)]
enum Content {
    Count(u64),
    Dot,
    Text(SharedString),
}

/// A supplied count, dot or short label, optionally attached to one host.
///
/// The application owns counts and read state. This marker never takes focus
/// or handles clicks. A host control should supply its own accessible description
/// when its focused name needs to include the count.
///
/// ```
/// use yue_component::{BadgeTone, YueBadge};
/// let count = YueBadge::new("unread").count(128); // displays 99+
/// let status = YueBadge::new("sync").dot().tone(BadgeTone::Success).label("已同步");
/// ```
#[derive(IntoElement)]
pub struct YueBadge {
    id: ElementId,
    theme: Option<YueTheme>,
    content: Content,
    maximum: u64,
    show_zero: bool,
    visible: bool,
    child: Option<AnyElement>,
    label: Option<SharedString>,
    size: BadgeSize,
    shape: BadgeShape,
    tone: BadgeTone,
    colors: Option<(Rgba, Rgba)>,
    separator_color: Option<Rgba>,
    offset: Point<Rems>,
    animated: bool,
    accessibility_label: Option<SharedString>,
    semantics: bool,
}

impl YueBadge {
    /// Creates a hidden zero count using the current application theme.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            theme: None,
            content: Content::Count(0),
            maximum: 99,
            show_zero: false,
            visible: true,
            child: None,
            label: None,
            size: BadgeSize::Medium,
            shape: BadgeShape::Pill,
            tone: BadgeTone::Danger,
            colors: None,
            separator_color: None,
            offset: point(rems(0.), rems(0.)),
            animated: false,
            accessibility_label: None,
            semantics: true,
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
    /// Replaces any prior dot or text mode with a nonnegative count.
    pub fn count(mut self, count: u64) -> Self {
        self.content = Content::Count(count);
        self
    }
    /// Replaces any prior content with a dot, visible independently of count.
    pub fn dot(mut self) -> Self {
        self.content = Content::Dot;
        self
    }
    /// Replaces prior content with normalized single-line text. Panics if blank.
    pub fn text(mut self, text: impl Into<SharedString>) -> Self {
        self.content = Content::Text(single_line(text.into()));
        self
    }
    /// Maximum precise count; larger values display `maximum+`. Must be positive.
    pub fn max_count(mut self, maximum: u64) -> Self {
        assert!(maximum > 0, "YueBadge max_count must be positive");
        self.maximum = maximum;
        self
    }
    pub fn show_zero(mut self, enabled: bool) -> Self {
        self.show_zero = enabled;
        self
    }
    /// Hides only the badge, preserving its host and the supplied content.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
    /// Attaches to one host. A second call replaces the first host.
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }
    /// Visible standalone description. Cannot be combined with a host child.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(single_line(label.into()));
        self
    }
    pub fn size(mut self, size: BadgeSize) -> Self {
        self.size = size;
        self
    }
    pub fn shape(mut self, shape: BadgeShape) -> Self {
        self.shape = shape;
        self
    }
    /// Restores semantic colors, clearing a preceding custom color pair.
    pub fn tone(mut self, tone: BadgeTone) -> Self {
        self.tone = tone;
        self.colors = None;
        self
    }
    /// Explicit data colors. The caller owns contrast on the actual surface.
    pub fn colors(mut self, background: Rgba, foreground: Rgba) -> Self {
        validate_color(background);
        validate_color(foreground);
        self.colors = Some((background, foreground));
        self
    }
    /// Sets the outer separator color to match a nondefault host surface.
    pub fn separator_color(mut self, color: Rgba) -> Self {
        validate_color(color);
        self.separator_color = Some(color);
        self
    }
    /// Rem-based offset from the host's top-right corner; positive y is down.
    pub fn offset(mut self, offset: Point<Rems>) -> Self {
        assert!(
            offset.x.0.is_finite() && offset.y.0.is_finite(),
            "YueBadge offset must be finite"
        );
        self.offset = offset;
        self
    }
    /// Enables short visibility transitions. Static badges request no frames.
    pub fn animated(mut self, enabled: bool) -> Self {
        self.animated = enabled;
        self
    }
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(single_line(label.into()));
        self.semantics = true;
        self
    }
    /// Controls only the mark's semantics, preserving the host's accessible tree.
    pub fn semantics(mut self, enabled: bool) -> Self {
        self.semantics = enabled;
        self
    }

    fn is_visible(&self) -> bool {
        self.visible && !matches!(self.content, Content::Count(0) if !self.show_zero)
    }

    fn display_text(&self) -> Option<SharedString> {
        match &self.content {
            Content::Count(count) if *count > self.maximum => {
                Some(format!("{}+", self.maximum).into())
            }
            Content::Count(count) => Some(count.to_string().into()),
            Content::Dot => None,
            Content::Text(text) => Some(text.clone()),
        }
    }

    fn accessible_name(&self) -> SharedString {
        if let Some(name) = &self.accessibility_label {
            return name.clone();
        }
        let content = match &self.content {
            Content::Count(count) => count.to_string().into(),
            Content::Dot => return self.label.clone().unwrap_or_else(|| "有新内容".into()),
            Content::Text(text) => text.clone(),
        };
        self.label.as_ref().map_or(content.clone(), |label| {
            format!("{label}，{content}").into()
        })
    }
}

fn single_line(text: SharedString) -> SharedString {
    let normalized = text
        .chars()
        .map(|c| {
            if c.is_control() || c.is_whitespace() {
                ' '
            } else {
                c
            }
        })
        .collect::<String>();
    let normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(!normalized.is_empty(), "YueBadge requires nonblank text");
    normalized.into()
}

fn validate_color(color: Rgba) {
    assert!(
        [color.r, color.g, color.b, color.a]
            .into_iter()
            .all(|c| c.is_finite() && (0.0..=1.0).contains(&c)),
        "YueBadge colors require finite channels in 0..=1"
    );
}

#[cfg(test)]
mod tests;
