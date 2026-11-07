//! Windows shortcut labels. These elements display caller-owned bindings and
//! never register or listen for keyboard commands.

mod appearance;
mod layout;

#[cfg(all(test, feature = "headless-tests"))]
mod visual_tests;

use std::fmt;

use appearance::Appearance;
use gpui_kit::base::TestSupportExt as _;

use gpui_kit::{
    App, ElementId, InteractiveElement as _, IntoElement, ParentElement, Rems, RenderOnce, Role,
    SharedString, StatefulInteractiveElement as _, Styled, Window, div, rems,
};

use crate::YueTheme;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum KbdSize {
    Small,
    #[default]
    Medium,
    Large,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum KbdVariant {
    #[default]
    Raised,
    Flat,
    Ghost,
    Inverse,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum KbdState {
    #[default]
    Default,
    Pressed,
    Active,
    Disabled,
}

/// A single non-interactive Windows keycap.
#[derive(IntoElement)]
pub struct YueKbd {
    id: Option<ElementId>,
    label: SharedString,
    theme: Option<YueTheme>,
    size: KbdSize,
    variant: KbdVariant,
    state: KbdState,
    width: Option<Rems>,
}

impl YueKbd {
    pub fn new(label: impl Into<SharedString>) -> Self {
        let label = label.into();
        assert!(!label.trim().is_empty(), "YueKbd requires a visible label");
        Self {
            id: None,
            label,
            theme: None,
            size: KbdSize::default(),
            variant: KbdVariant::default(),
            state: KbdState::default(),
            width: None,
        }
    }

    /// Stable identity enables 100ms visual transitions; no input is registered.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn theme(mut self, theme: YueTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    pub fn size(mut self, size: KbdSize) -> Self {
        self.size = size;
        self
    }

    pub fn variant(mut self, variant: KbdVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn state(mut self, state: KbdState) -> Self {
        self.state = state;
        self
    }

    pub fn width(mut self, width: Rems) -> Self {
        self.width = Some(width);
        self
    }
}

impl RenderOnce for YueKbd {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = self.theme.unwrap_or_else(|| crate::current_theme(cx));
        let metrics = self.size.metrics();
        let mut appearance = Appearance::new(
            theme,
            self.variant,
            self.state,
            window.text_style().color.into(),
        );
        if let Some(id) = self.id {
            appearance.animate(id, window, cx);
        }
        let ghost = self.variant == KbdVariant::Ghost;
        let font = crate::typography::ui_font(theme);
        let content = if self.label.eq_ignore_ascii_case("win")
            || self.label.eq_ignore_ascii_case("windows")
        {
            "Win".into_any_element()
        } else {
            self.label.into_any_element()
        };
        let mut element = div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .min_w(rems(if ghost { 0. } else { metrics.height / 16. }))
            .h(rems(metrics.height / 16.))
            .px(rems(if ghost { 1. } else { metrics.padding } / 16.))
            .border(rems(1. / 16.))
            .border_b(rems(appearance.bottom / 16.))
            .border_color(appearance.border)
            .rounded(rems(4. / 16.))
            .bg(appearance.background)
            .text_color(appearance.foreground)
            .font_family(font.family)
            .text_size(rems(metrics.font / 16.))
            .line_height(gpui_kit::relative(1.))
            .font_weight(gpui_kit::FontWeight::MEDIUM)
            .opacity(appearance.opacity)
            .child(content);
        if let Some(width) = self.width {
            element = element.w(width);
        }
        if appearance.translation != 0. {
            element = element.relative().top(rems(appearance.translation / 16.));
        }
        element
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShortcutParseError {
    Empty,
    EmptyAlternative,
    EmptySequenceStep,
    EmptyKey,
    UnknownKey(String),
    DuplicateModifier(String),
    ModifierWithoutKey,
}

impl fmt::Display for ShortcutParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("shortcut cannot be empty"),
            Self::EmptyAlternative => f.write_str("shortcut alternative cannot be empty"),
            Self::EmptySequenceStep => f.write_str("shortcut sequence step cannot be empty"),
            Self::EmptyKey => f.write_str("shortcut chord contains an empty key"),
            Self::UnknownKey(key) => write!(f, "unknown Windows key label: {key}"),
            Self::DuplicateModifier(key) => write!(f, "duplicate modifier: {key}"),
            Self::ModifierWithoutKey => {
                f.write_str("shortcut chord must include a non-modifier key")
            }
        }
    }
}

impl std::error::Error for ShortcutParseError {}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Chord(Vec<SharedString>);

#[derive(Clone, Debug, Eq, PartialEq)]
struct Sequence(Vec<Chord>);

/// A validated presentation value: alternatives contain sequences of chords.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShortcutSpec(Vec<Sequence>);

impl ShortcutSpec {
    pub(crate) fn is_recordable_chord(&self) -> bool {
        self.0.len() == 1
            && self.0[0].0.len() == 1
            && self.0[0].0[0]
                .0
                .iter()
                .filter(|k| modifier_index(k).is_none())
                .count()
                == 1
    }
    pub(crate) fn has_recorder_modifier(&self) -> bool {
        self.0[0].0[0]
            .0
            .iter()
            .any(|k| matches!(k.as_ref(), "Ctrl" | "Alt" | "Win"))
            || self.0[0].0[0].0.last().is_some_and(|k| {
                k.strip_prefix('F')
                    .and_then(|n| n.parse::<u8>().ok())
                    .is_some_and(|n| (1..=12).contains(&n))
            })
    }
    pub fn parse(value: &str) -> Result<Self, ShortcutParseError> {
        if value.trim().is_empty() {
            return Err(ShortcutParseError::Empty);
        }
        let mut alternatives = Vec::new();
        for alternative in value.split('|') {
            let alternative = alternative.trim();
            if alternative.is_empty() {
                return Err(ShortcutParseError::EmptyAlternative);
            }
            let mut steps = Vec::new();
            for step in alternative.split_whitespace() {
                steps.push(parse_chord(step)?);
            }
            if steps.is_empty() {
                return Err(ShortcutParseError::EmptySequenceStep);
            }
            alternatives.push(Sequence(steps));
        }
        Ok(Self(alternatives))
    }

    pub fn display_label(&self) -> String {
        self.0
            .iter()
            .map(sequence_label)
            .collect::<Vec<_>>()
            .join(" 或 ")
    }

    /// Returns ARIA's chord syntax when every alternative is one chord.
    /// Sequences are intentionally omitted because they are not a single chord.
    pub fn aria_keyshortcuts_value(&self) -> Option<String> {
        if self.0.iter().any(|sequence| sequence.0.len() != 1) {
            return None;
        }
        self.0
            .iter()
            .map(|sequence| {
                sequence.0[0]
                    .0
                    .iter()
                    .map(|key| match key.as_ref() {
                        "Ctrl" => "Control".to_owned(),
                        "Win" => "Meta".to_owned(),
                        "Esc" => "Escape".to_owned(),
                        "Space" => "Space".to_owned(),
                        "Left" => "ArrowLeft".to_owned(),
                        "Right" => "ArrowRight".to_owned(),
                        "Up" => "ArrowUp".to_owned(),
                        "Down" => "ArrowDown".to_owned(),
                        "Plus" => "Plus".to_owned(),
                        key @ ("Minus" | "Equal" | "Comma" | "Period" | "Slash" | "Backslash"
                        | "Semicolon" | "Quote" | "Backquote" | "BracketLeft"
                        | "BracketRight" | "Pipe") => key_label(key).to_string(),
                        key => key.to_owned(),
                    })
                    .collect::<Vec<_>>()
                    .join("+")
            })
            .reduce(|mut all, next| {
                all.push(' ');
                all.push_str(&next);
                all
            })
    }
}

fn sequence_label(sequence: &Sequence) -> String {
    sequence
        .0
        .iter()
        .map(|chord| {
            chord
                .0
                .iter()
                .map(|key| key.as_ref())
                .collect::<Vec<_>>()
                .join(" + ")
        })
        .collect::<Vec<_>>()
        .join(" 然后 ")
}

fn parse_chord(value: &str) -> Result<Chord, ShortcutParseError> {
    let mut modifiers: [Option<String>; 4] = [None, None, None, None];
    let mut keys = Vec::new();
    for raw in value.split('+') {
        let raw = raw.trim();
        if raw.is_empty() {
            return Err(ShortcutParseError::EmptyKey);
        }
        let key = normalize_key(raw)?;
        if let Some(index) = modifier_index(&key) {
            if modifiers[index].is_some() {
                return Err(ShortcutParseError::DuplicateModifier(key));
            }
            modifiers[index] = Some(key);
        } else {
            keys.push(SharedString::from(key));
        }
    }
    if keys.is_empty() {
        return Err(ShortcutParseError::ModifierWithoutKey);
    }
    let mut ordered = modifiers
        .into_iter()
        .flatten()
        .map(SharedString::from)
        .collect::<Vec<_>>();
    ordered.extend(keys);
    Ok(Chord(ordered))
}

fn modifier_index(key: &str) -> Option<usize> {
    match key {
        "Ctrl" => Some(0),
        "Alt" => Some(1),
        "Shift" => Some(2),
        "Win" => Some(3),
        _ => None,
    }
}

fn normalize_key(raw: &str) -> Result<String, ShortcutParseError> {
    let lower = raw.to_ascii_lowercase();
    let normalized = match lower.as_str() {
        "ctrl" | "control" => "Ctrl",
        "alt" => "Alt",
        "shift" => "Shift",
        "win" | "windows" => "Win",
        "esc" | "escape" => "Esc",
        "enter" | "return" => "Enter",
        "backspace" => "Backspace",
        "delete" | "del" => "Delete",
        "tab" => "Tab",
        "space" | "spacebar" => "Space",
        "capslock" => "CapsLock",
        "numlock" => "NumLock",
        "scrolllock" => "ScrollLock",
        "printscreen" | "prtsc" => "PrintScreen",
        "pause" => "Pause",
        "insert" | "ins" => "Insert",
        "home" => "Home",
        "end" => "End",
        "pageup" | "pgup" => "PageUp",
        "pagedown" | "pgdn" => "PageDown",
        "left" | "arrowleft" => "Left",
        "right" | "arrowright" => "Right",
        "up" | "arrowup" => "Up",
        "down" | "arrowdown" => "Down",
        "contextmenu" | "apps" => "ContextMenu",
        "plus" => "Plus",
        "minus" | "-" => "Minus",
        "equal" | "=" => "Equal",
        "comma" | "," => "Comma",
        "period" | "." => "Period",
        "slash" | "/" => "Slash",
        "backslash" | "\\" => "Backslash",
        "semicolon" | ";" => "Semicolon",
        "quote" | "'" => "Quote",
        "backquote" | "`" => "Backquote",
        "bracketleft" | "[" => "BracketLeft",
        "bracketright" | "]" => "BracketRight",
        "pipe" => "Pipe",
        // Windows GPUI reports shifted punctuation as logical symbols, with
        // Shift folded into the symbol. Preserve that identity for recording.
        "!" | "@" | "#" | "$" | "%" | "^" | "&" | "*" | "(" | ")" | "_" | "{" | "}" | ":"
        | "\"" | "<" | ">" | "?" | "~" => return Ok(raw.to_owned()),
        _ if raw.len() == 1 && raw.as_bytes()[0].is_ascii_alphanumeric() => {
            if raw.as_bytes()[0].is_ascii_alphabetic() {
                return Ok(raw.to_ascii_uppercase());
            }
            return Ok(raw.to_owned());
        }
        _ if lower
            .strip_prefix('f')
            .is_some_and(|n| n.parse::<u8>().is_ok_and(|n| (1..=24).contains(&n))) =>
        {
            return Ok(lower.to_ascii_uppercase());
        }
        _ => return Err(ShortcutParseError::UnknownKey(raw.to_owned())),
    };
    Ok(normalized.to_owned())
}

/// Separator between simultaneously held keys. Sequence/alternative labels remain visible.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ShortcutSeparator {
    #[default]
    Plus,
    None,
}

/// A shortcut composed from a validated Windows shortcut string.
#[derive(IntoElement)]
pub struct YueShortcut {
    id: ElementId,
    spec: ShortcutSpec,
    separator: ShortcutSeparator,
    key_states: Vec<(SharedString, KbdState)>,
    semantics: bool,
    theme: Option<YueTheme>,
    size: KbdSize,
    variant: KbdVariant,
    state: KbdState,
}

impl YueShortcut {
    pub fn new(id: impl Into<ElementId>, spec: ShortcutSpec) -> Self {
        Self {
            id: id.into(),
            spec,
            separator: ShortcutSeparator::default(),
            key_states: Vec::new(),
            semantics: true,
            theme: None,
            size: KbdSize::default(),
            variant: KbdVariant::default(),
            state: KbdState::default(),
        }
    }

    pub fn parse(id: impl Into<ElementId>, value: &str) -> Result<Self, ShortcutParseError> {
        ShortcutSpec::parse(value).map(|spec| Self::new(id, spec))
    }

    pub fn separator(mut self, separator: ShortcutSeparator) -> Self {
        self.separator = separator;
        self
    }
    /// Override every occurrence of a normalized key; unrelated keys keep `state`.
    pub fn key_state(mut self, key: &str, state: KbdState) -> Result<Self, ShortcutParseError> {
        let key = SharedString::from(normalize_key(key)?);
        if let Some(entry) = self.key_states.iter_mut().find(|(name, _)| *name == key) {
            entry.1 = state;
        } else {
            self.key_states.push((key, state));
        }
        Ok(self)
    }
    /// Disable child semantics when a Button/MenuItem/Tooltip owns the full name.
    pub fn semantics(mut self, enabled: bool) -> Self {
        self.semantics = enabled;
        self
    }
    fn resolved_state(&self, key: &str) -> KbdState {
        if self.state == KbdState::Disabled {
            return KbdState::Disabled;
        }
        self.key_states
            .iter()
            .find(|(name, _)| name.as_ref() == key)
            .map_or(self.state, |(_, state)| *state)
    }
    pub fn theme(mut self, theme: YueTheme) -> Self {
        self.theme = Some(theme);
        self
    }
    pub fn size(mut self, size: KbdSize) -> Self {
        self.size = size;
        self
    }
    pub fn variant(mut self, variant: KbdVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn state(mut self, state: KbdState) -> Self {
        self.state = state;
        self
    }
    pub fn spec(&self) -> &ShortcutSpec {
        &self.spec
    }
}

impl RenderOnce for YueShortcut {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = self.theme.unwrap_or_else(|| crate::current_theme(cx));
        let metrics = self.size.metrics();
        let appearance = Appearance::new(
            theme,
            self.variant,
            self.state,
            window.text_style().color.into(),
        );
        let key_gap = if self.variant == KbdVariant::Ghost {
            1.
        } else {
            metrics.key_gap
        };
        let mut root = div()
            .id(self.id.clone())
            .test_support()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap_2()
            .font_family(crate::typography::ui_font(theme).family)
            .font_weight(gpui_kit::FontWeight::NORMAL);
        if self.semantics {
            root = root.role(Role::Image).aria_label(self.spec.display_label());
        }
        for (alternative_index, alternative) in self.spec.0.iter().enumerate() {
            if alternative_index > 0 {
                root = root.child(shortcut_separator("或", appearance.secondary, 12.));
            }
            let mut alternative_element = div().flex().flex_shrink_0().items_center().gap_2();
            for (step_index, step) in alternative.0.iter().enumerate() {
                if step_index > 0 {
                    alternative_element = alternative_element.child(shortcut_separator(
                        "然后",
                        appearance.secondary,
                        12.,
                    ));
                }
                let mut chord_element = div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap(rems(key_gap / 16.));
                for (key_index, key) in step.0.iter().enumerate() {
                    if key_index > 0 && self.separator == ShortcutSeparator::Plus {
                        chord_element = chord_element.child(shortcut_separator(
                            "+",
                            appearance.secondary,
                            metrics.font,
                        ));
                    }
                    chord_element = chord_element.child(
                        YueKbd::new(key_label(key))
                            .id((
                                self.id.clone(),
                                SharedString::from(format!(
                                    "{alternative_index}/{step_index}/{key_index}/{key}"
                                )),
                            ))
                            .theme(theme)
                            .size(self.size)
                            .variant(self.variant)
                            .state(self.resolved_state(key)),
                    );
                }
                alternative_element = alternative_element.child(chord_element);
            }
            root = root.child(alternative_element);
        }
        root
    }
}

fn key_label(key: &str) -> SharedString {
    match key {
        "Left" => "←",
        "Right" => "→",
        "Up" => "↑",
        "Down" => "↓",
        "Delete" => "Del",
        "PageUp" => "PgUp",
        "PageDown" => "PgDn",
        "Plus" => "+",
        "Minus" => "-",
        "Equal" => "=",
        "Comma" => ",",
        "Period" => ".",
        "Slash" => "/",
        "Backslash" => "\\",
        "Semicolon" => ";",
        "Quote" => "'",
        "Backquote" => "`",
        "BracketLeft" => "[",
        "BracketRight" => "]",
        "Pipe" => "|",
        _ => return SharedString::from(key.to_owned()),
    }
    .into()
}

fn shortcut_separator(label: &'static str, color: gpui_kit::Rgba, font: f32) -> impl IntoElement {
    div()
        .flex_shrink_0()
        .text_size(rems(font / 16.))
        .line_height(rems(20. / 16.))
        .text_color(color)
        .child(label)
}

#[cfg(test)]
mod tests {
    use super::{ShortcutParseError, ShortcutSpec};

    #[test]
    fn parser_orders_modifiers_and_normalizes_key_names() {
        let spec = ShortcutSpec::parse("shift+ctrl+arrowleft | alt+F4").unwrap();
        assert_eq!(spec.display_label(), "Ctrl + Shift + Left 或 Alt + F4");
        assert_eq!(
            spec.aria_keyshortcuts_value().as_deref(),
            Some("Control+Shift+ArrowLeft Alt+F4")
        );
    }

    #[test]
    fn parser_preserves_sequence_and_alternative_boundaries() {
        let spec = ShortcutSpec::parse("g i | ctrl+z").unwrap();
        assert_eq!(spec.display_label(), "G 然后 I 或 Ctrl + Z");
        assert_eq!(spec.aria_keyshortcuts_value(), None);
    }

    #[test]
    fn reference_punctuation_keeps_logical_names_and_uses_visual_glyphs() {
        let spec = ShortcutSpec::parse("Ctrl+K C | Ctrl+/").unwrap();
        assert_eq!(spec.display_label(), "Ctrl + K 然后 C 或 Ctrl + Slash");
        assert_eq!(super::key_label("Slash").as_ref(), "/");
        assert_eq!(super::key_label("Left").as_ref(), "←");
        assert_eq!(
            ShortcutSpec::parse("Ctrl+/")
                .unwrap()
                .aria_keyshortcuts_value()
                .as_deref(),
            Some("Control+/")
        );
        assert_eq!(
            ShortcutSpec::parse("Ctrl+Delete | Alt+PageUp | Ctrl+Plus")
                .unwrap()
                .aria_keyshortcuts_value()
                .as_deref(),
            Some("Control+Delete Alt+PageUp Control+Plus")
        );
    }

    #[test]
    fn parser_rejects_malformed_and_unsupported_shortcuts() {
        assert_eq!(ShortcutSpec::parse(" "), Err(ShortcutParseError::Empty));
        assert_eq!(
            ShortcutSpec::parse("ctrl++a"),
            Err(ShortcutParseError::EmptyKey)
        );
        assert_eq!(
            ShortcutSpec::parse("ctrl+ctrl+a"),
            Err(ShortcutParseError::DuplicateModifier("Ctrl".into()))
        );
        assert_eq!(
            ShortcutSpec::parse("shift+alt"),
            Err(ShortcutParseError::ModifierWithoutKey)
        );
        assert_eq!(
            ShortcutSpec::parse("cmd+a"),
            Err(ShortcutParseError::UnknownKey("cmd".into()))
        );
        assert_eq!(
            ShortcutSpec::parse("a | "),
            Err(ShortcutParseError::EmptyAlternative)
        );
    }
}
