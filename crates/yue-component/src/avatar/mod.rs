//! Value-style avatars and compact avatar groups.

mod element;

use crate::{YueTheme, outline};
use gpui_kit::{ElementId, ImageSource, IntoElement, SharedString};
use yue_component_icons::{Icon, IconDefinition};

/// The fixed avatar dimensions defined by Yue Avatar tokens.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AvatarSize {
    Small,
    #[default]
    Medium,
    Large,
    XLarge,
}

impl AvatarSize {
    pub(crate) fn token(self) -> crate::theme::TokenKey {
        use crate::theme::keys;
        match self {
            Self::Small => keys::AVATAR_SIZE_SM,
            Self::Medium => keys::AVATAR_SIZE_MD,
            Self::Large => keys::AVATAR_SIZE_LG,
            Self::XLarge => keys::AVATAR_SIZE_XL,
        }
    }
}

/// The clipping shape used for the avatar image and fallback.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AvatarShape {
    #[default]
    Circle,
    Rounded,
}

/// A static identity marker rendered as an image or a name fallback.
#[derive(IntoElement)]
pub struct YueAvatar {
    pub(crate) id: ElementId,
    pub(crate) theme: YueTheme,
    pub(crate) use_default_theme: bool,
    pub(crate) source: Option<ImageSource>,
    pub(crate) name: Option<SharedString>,
    pub(crate) fallback_text: Option<SharedString>,
    pub(crate) placeholder: Icon,
    pub(crate) size: AvatarSize,
    pub(crate) shape: AvatarShape,
    pub(crate) auto_fit_font_size: bool,
    pub(crate) accessibility_label: Option<SharedString>,
    pub(crate) semantics: bool,
}

impl YueAvatar {
    pub fn new(id: impl Into<ElementId>, theme: YueTheme) -> Self {
        Self {
            id: id.into(),
            theme,
            use_default_theme: false,
            source: None,
            name: None,
            fallback_text: None,
            placeholder: Icon::new(outline::User),
            size: AvatarSize::Medium,
            shape: AvatarShape::Circle,
            auto_fit_font_size: true,
            accessibility_label: None,
            semantics: true,
        }
    }

    pub fn use_default_theme(mut self) -> Self {
        self.use_default_theme = true;
        self
    }

    pub fn theme(mut self, theme: YueTheme) -> Self {
        self.theme = theme;
        self.use_default_theme = false;
        self
    }

    pub fn src(mut self, source: impl Into<ImageSource>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn name(mut self, name: impl Into<SharedString>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn fallback_text(mut self, text: impl Into<SharedString>) -> Self {
        self.fallback_text = Some(text.into());
        self
    }

    pub fn placeholder(mut self, icon: impl IconDefinition) -> Self {
        self.placeholder = Icon::new(icon);
        self
    }

    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    pub fn shape(mut self, shape: AvatarShape) -> Self {
        self.shape = shape;
        self
    }

    pub fn auto_fit_font_size(mut self, enabled: bool) -> Self {
        self.auto_fit_font_size = enabled;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self.semantics = true;
        self
    }

    pub fn semantics(mut self, enabled: bool) -> Self {
        self.semantics = enabled;
        self
    }
}

/// Overlapping static avatars with an optional count for hidden entries.
#[derive(IntoElement)]
pub struct YueAvatarGroup {
    id: ElementId,
    theme: YueTheme,
    avatars: Vec<YueAvatar>,
    size: Option<AvatarSize>,
    shape: Option<AvatarShape>,
    max: usize,
    overlap: f32,
    accessibility_label: Option<SharedString>,
}

impl YueAvatarGroup {
    pub fn new(id: impl Into<ElementId>, theme: YueTheme) -> Self {
        Self {
            id: id.into(),
            theme,
            avatars: Vec::new(),
            size: None,
            shape: None,
            max: 4,
            overlap: 0.28,
            accessibility_label: None,
        }
    }

    pub fn child(mut self, avatar: YueAvatar) -> Self {
        self.avatars.push(avatar);
        self
    }

    pub fn children(mut self, avatars: impl IntoIterator<Item = YueAvatar>) -> Self {
        self.avatars.extend(avatars);
        self
    }

    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = Some(size);
        self
    }

    pub fn shape(mut self, shape: AvatarShape) -> Self {
        self.shape = Some(shape);
        self
    }

    /// Maximum number of avatars shown before the `+N` marker. Zero shows all.
    pub fn max(mut self, visible: usize) -> Self {
        self.max = visible;
        self
    }

    /// Fraction of each avatar overlapped by its next sibling, clamped to 0.2..=0.4.
    pub fn overlap(mut self, ratio: f32) -> Self {
        self.overlap = if ratio.is_finite() {
            ratio.clamp(0.2, 0.4)
        } else {
            0.28
        };
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

pub(crate) fn derive_fallback(name: &str) -> Option<SharedString> {
    let words: Vec<&str> = name.split_whitespace().collect();
    if words.is_empty() {
        return None;
    }
    let first = words[0].chars().next()?;
    let cjk = matches!(first as u32, 0x3400..=0x9fff | 0xf900..=0xfaff);
    let fallback = if cjk {
        first.to_string()
    } else if words.len() > 1 {
        words
            .iter()
            .take(2)
            .filter_map(|word| word.chars().next())
            .collect::<String>()
    } else {
        words[0].chars().take(2).collect::<String>()
    };
    Some(fallback.to_uppercase().into())
}

#[cfg(test)]
mod tests;
