//! Focus-scoped Windows shortcut recording. Retain the state in the host;
//! recording edits a value, it never installs an application key binding.
mod element;
mod state;

#[cfg(all(test, feature = "headless-tests"))]
mod tests;

use crate::{InputSize, YueTheme};
use gpui_kit::{DefiniteLength, ElementId, Entity, IntoElement, SharedString};
pub use state::{ShortcutRecorderEvent, ShortcutRecorderValidation, YueShortcutRecorderState};

/// Records one chord. Sequences and alternatives remain display-only features
/// of `YueShortcut`. The host owns command registration and conflict replacement.
#[derive(IntoElement)]
pub struct YueShortcutRecorder {
    id: ElementId,
    state: Entity<YueShortcutRecorderState>,
    theme: Option<YueTheme>,
    label: SharedString,
    size: InputSize,
    width: Option<DefiniteLength>,
}
impl YueShortcutRecorder {
    /// Use a stable field ID and one retained state per mounted recorder.
    pub fn new(id: impl Into<ElementId>, state: &Entity<YueShortcutRecorderState>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            theme: None,
            label: "快捷键".into(),
            size: InputSize::Medium,
            width: None,
        }
    }
    pub fn theme(mut self, theme: YueTheme) -> Self {
        self.theme = Some(theme);
        self
    }
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }
    pub fn size(mut self, size: InputSize) -> Self {
        self.size = size;
        self
    }
    pub fn width(mut self, width: impl Into<DefiniteLength>) -> Self {
        self.width = Some(width.into());
        self
    }
}
