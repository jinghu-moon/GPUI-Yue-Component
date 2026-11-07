use crate::ShortcutSpec;
use gpui_kit::{
    App, Context, EventEmitter, FocusHandle, Focusable, Keystroke, Modifiers, SharedString,
    Subscription, Window,
};
use std::rc::Rc;

/// The built-in reservation/modifier checks run before this host validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShortcutRecorderValidation {
    Accept,
    Reject(SharedString),
    /// The user must explicitly choose 覆盖. The host resolves other bindings
    /// when it receives Change with overwrite=true; no global registry is used.
    Conflict(SharedString),
}

/// Emitted once after a changed user value is committed. Programmatic setters
/// and cancellation are silent. The state is already coherent when delivered.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShortcutRecorderEvent {
    Change {
        value: Option<ShortcutSpec>,
        overwrite: bool,
    },
}

type Validator = Rc<dyn Fn(&ShortcutSpec, &App) -> ShortcutRecorderValidation>;
const WINDOWS_RESERVED: [&str; 4] = ["Alt+F4", "Win+L", "Win+D", "Ctrl+Alt+Delete"];

/// The sole value and draft owner. Keep one state per mounted recorder, like
/// InputState. Disabling, external changes, blur and deactivation cancel drafts.
pub struct YueShortcutRecorderState {
    value: Option<ShortcutSpec>,
    pub(super) recording: bool,
    pub(super) pending: Option<ShortcutSpec>,
    pub(super) modifiers: Modifiers,
    pub(super) message: SharedString,
    pub(super) invalid: bool,
    pub(super) disabled: bool,
    pub(super) focus: FocusHandle,
    pub(super) region: FocusHandle,
    require_modifier: bool,
    reserved: Vec<ShortcutSpec>,
    validator: Option<Validator>,
    // Finishing on key-down must not let auto-repeat execute a real command.
    blocked_key: Option<String>,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<ShortcutRecorderEvent> for YueShortcutRecorderState {}
impl Focusable for YueShortcutRecorderState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl YueShortcutRecorderState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle().tab_stop(true);
        let region = cx.focus_handle();
        let blur = cx.on_focus_out(&region, window, |s, _, _, cx| {
            s.cancel(cx);
            s.blocked_key = None;
        });
        let activation = cx.observe_window_activation(window, |s, w, cx| {
            if !w.is_window_active() {
                s.cancel(cx);
                s.blocked_key = None;
            }
        });
        let owner = cx.weak_entity();
        let field = focus.clone();
        let group = region.clone();
        let intercept = cx.intercept_keystrokes(move |e, w, cx| {
            // Focus alone can survive removal of a retained page for a frame.
            // The rendered dispatch context must also contain our recorder.
            if group.contains_focused(w, cx)
                && e.context_stack
                    .iter()
                    .any(|c| c.contains("YueShortcutRecorder"))
            {
                let _ = owner.update(cx, |s, cx| {
                    if s.disabled {
                        return;
                    }
                    if s.pending.is_some() && bare(&e.keystroke, "escape") {
                        s.cancel(cx);
                        s.blocked_key = Some(e.keystroke.key.clone());
                        s.focus.focus(w, cx);
                        cx.stop_propagation();
                    } else if field.is_focused(w) {
                        s.capture(&e.keystroke, cx);
                    }
                });
            }
        });
        Self {
            value: None,
            recording: false,
            pending: None,
            modifiers: Modifiers::default(),
            message: "".into(),
            invalid: false,
            disabled: false,
            focus,
            region,
            require_modifier: true,
            reserved: WINDOWS_RESERVED
                .map(|s| ShortcutSpec::parse(s).unwrap())
                .to_vec(),
            validator: None,
            blocked_key: None,
            _subscriptions: vec![blur, activation, intercept],
        }
    }

    pub fn value(&self) -> Option<&ShortcutSpec> {
        self.value.as_ref()
    }
    pub fn is_recording(&self) -> bool {
        self.recording
    }
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
    pub fn pending_value(&self) -> Option<&ShortcutSpec> {
        self.pending.as_ref()
    }
    pub fn message(&self) -> &SharedString {
        &self.message
    }

    /// Supply one chord with exactly one primary key. Sequences, alternatives
    /// and multi-primary chords are rejected without changing the current edit.
    /// This silent setter bypasses user validation for restoring existing values.
    pub fn set_value(
        &mut self,
        value: Option<ShortcutSpec>,
        cx: &mut Context<Self>,
    ) -> Result<(), &'static str> {
        if value.as_ref().is_some_and(|s| !s.is_recordable_chord()) {
            return Err("ShortcutRecorder requires one chord with exactly one primary key");
        }
        self.cancel(cx);
        self.value = value;
        cx.notify();
        Ok(())
    }

    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        if self.disabled != disabled {
            self.disabled = disabled;
            self.cancel(cx);
            cx.notify();
        }
    }
    /// By default Ctrl, Alt or Win is required; F1–F12 may stand alone (or Shift).
    pub fn set_require_modifier(&mut self, required: bool, cx: &mut Context<Self>) {
        self.require_modifier = required;
        self.cancel(cx);
    }
    /// Additional reservations. The four reference Windows reservations always
    /// apply. This is application validation, not a guarantee of OS delivery.
    pub fn set_reserved(
        &mut self,
        values: Vec<ShortcutSpec>,
        cx: &mut Context<Self>,
    ) -> Result<(), &'static str> {
        if values.iter().any(|s| !s.is_recordable_chord()) {
            return Err("reserved shortcuts must be single chords");
        }
        self.reserved.truncate(WINDOWS_RESERVED.len());
        self.reserved.extend(values);
        self.cancel(cx);
        Ok(())
    }
    /// Read the host's current command registry on every attempt, and again on
    /// overwrite. Capture a WeakEntity when consulting the owning view.
    pub fn set_validator(
        &mut self,
        validator: impl Fn(&ShortcutSpec, &App) -> ShortcutRecorderValidation + 'static,
        cx: &mut Context<Self>,
    ) {
        self.validator = Some(Rc::new(validator));
        self.cancel(cx);
    }
    /// Start recording from a user command or the field's activation.
    pub fn start(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        self.cancel(cx);
        self.recording = true;
        self.message = "按下快捷键 · Esc 取消 · Backspace 清除".into();
        self.focus.focus(window, cx);
        cx.notify();
    }
    pub fn cancel(&mut self, cx: &mut Context<Self>) {
        let changed = self.recording || self.pending.is_some() || !self.message.is_empty();
        self.recording = false;
        self.pending = None;
        self.modifiers = Modifiers::default();
        self.message = "".into();
        self.invalid = false;
        if changed {
            cx.notify();
        }
    }

    pub(super) fn release(&mut self, key: &str, modifiers: Modifiers, cx: &mut Context<Self>) {
        if self.blocked_key.as_deref() == Some(key_identity(key)) {
            self.blocked_key = None;
        }
        self.modifiers_changed(modifiers, cx);
    }
    pub(super) fn modifiers_changed(&mut self, modifiers: Modifiers, cx: &mut Context<Self>) {
        if self.recording && self.modifiers != modifiers {
            self.modifiers = modifiers;
            cx.notify();
        }
    }
    fn capture(&mut self, stroke: &Keystroke, cx: &mut Context<Self>) {
        if self.blocked_key.as_deref() == Some(key_identity(&stroke.key)) {
            cx.stop_propagation();
            return;
        }
        if !self.recording {
            return;
        }
        // Tab and Shift+Tab retain traversal. Modified Ctrl/Alt/Win+Tab is a chord.
        if stroke.key == "tab"
            && !stroke.modifiers.control
            && !stroke.modifiers.alt
            && !stroke.modifiers.platform
        {
            return;
        }
        cx.stop_propagation();
        self.modifiers = stroke.modifiers;
        if bare(stroke, "escape") {
            self.cancel(cx);
        } else if bare(stroke, "backspace") {
            self.commit(None, false, cx);
        } else {
            let mut keys = modifier_names(stroke.modifiers);
            let key = match stroke.key.as_str() {
                "+" => "Plus",
                "|" => "Pipe",
                "menu" => "ContextMenu",
                key => key,
            };
            // Native Windows modifier-only input normally uses ModifiersChanged.
            if ["control", "ctrl", "shift", "alt", "win"].contains(&key) {
                cx.notify();
                return;
            }
            keys.push(key);
            match ShortcutSpec::parse(&keys.join("+")) {
                Ok(candidate) => self.attempt(candidate, false, cx),
                Err(_) => {
                    self.invalid = true;
                    self.message = "暂不支持此按键，请使用英文键位或功能键".into();
                    cx.notify();
                }
            }
        }
        self.blocked_key = Some(key_identity(&stroke.key).to_owned());
    }
    fn validate(&self, candidate: &ShortcutSpec, cx: &App) -> ShortcutRecorderValidation {
        use ShortcutRecorderValidation::*;
        if self.reserved.contains(candidate) {
            return Reject("系统保留，无法使用".into());
        }
        if self.require_modifier && !candidate.has_recorder_modifier() {
            return Reject("需要包含 Ctrl / Alt / Win（F1–F12 除外）".into());
        }
        self.validator.as_ref().map_or(Accept, |v| v(candidate, cx))
    }
    fn attempt(&mut self, candidate: ShortcutSpec, overwrite: bool, cx: &mut Context<Self>) {
        match self.validate(&candidate, cx) {
            ShortcutRecorderValidation::Accept => self.commit(Some(candidate), false, cx),
            ShortcutRecorderValidation::Conflict(_) if overwrite => {
                self.commit(Some(candidate), true, cx)
            }
            ShortcutRecorderValidation::Conflict(message) => {
                self.pending = Some(candidate);
                self.recording = false;
                self.invalid = false;
                self.message = message;
                self.modifiers = Modifiers::default();
                cx.notify();
            }
            ShortcutRecorderValidation::Reject(message) => {
                self.pending = None;
                self.recording = true;
                self.invalid = true;
                self.message = message;
                cx.notify();
            }
        }
    }
    pub(super) fn overwrite(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        if let Some(candidate) = self.pending.take() {
            self.attempt(candidate, true, cx);
        }
        self.focus.focus(window, cx);
    }
    fn commit(&mut self, value: Option<ShortcutSpec>, overwrite: bool, cx: &mut Context<Self>) {
        self.cancel(cx);
        if self.value != value || overwrite {
            self.value = value.clone();
            cx.emit(ShortcutRecorderEvent::Change { value, overwrite });
        }
        cx.notify();
    }
}

fn bare(stroke: &Keystroke, key: &str) -> bool {
    stroke.key == key && !stroke.modifiers.modified()
}
// Only for release/repeat matching, not for the recorded logical value.
// Windows folds Shift into ASCII punctuation; release after Shift can use the
// unshifted spelling. No physical scan-code/layout translation is claimed.
fn key_identity(key: &str) -> &str {
    match key {
        "!" => "1",
        "@" => "2",
        "#" => "3",
        "$" => "4",
        "%" => "5",
        "^" => "6",
        "&" => "7",
        "*" => "8",
        "(" => "9",
        ")" => "0",
        "_" => "-",
        "+" => "=",
        "{" => "[",
        "}" => "]",
        "|" => "\\",
        ":" => ";",
        "\"" => "'",
        "<" => ",",
        ">" => ".",
        "?" => "/",
        "~" => "`",
        key => key,
    }
}
pub(super) fn modifier_names(m: Modifiers) -> Vec<&'static str> {
    [
        ("Ctrl", m.control),
        ("Alt", m.alt),
        ("Shift", m.shift),
        ("Win", m.platform),
    ]
    .into_iter()
    .filter_map(|(key, down)| down.then_some(key))
    .collect()
}
