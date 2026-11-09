use super::*;
use crate::{Accent, ShortcutSpec, ThemeMode, YueButton};
use gpui_kit::base::{Root, test_support::snapshots};
use gpui_kit::{
    AppContext as _, Context, Entity, InteractiveElement as _, KeyDownEvent, KeyUpEvent, Keystroke,
    Modifiers, ModifiersChangedEvent, ParentElement as _, Render, Styled as _, Subscription,
    TestAppContext, VisualTestContext, Window, div, px,
};

gpui_kit::actions!(recorder_test, [Save]);
struct Probe {
    state: Entity<YueShortcutRecorderState>,
    second: Entity<YueShortcutRecorderState>,
    events: Vec<ShortcutRecorderEvent>,
    commands: usize,
    visible: bool,
    _subscription: Subscription,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context("RecorderTest")
            .flex()
            .flex_col()
            .items_start()
            .gap_4()
            .p_4()
            .on_action(cx.listener(|s, _: &Save, _, _| s.commands += 1))
            .children(self.visible.then(|| {
                YueShortcutRecorder::new("first", &self.state).accessibility_label("录入")
            }))
            .child(YueShortcutRecorder::new("second", &self.second).accessibility_label("第二项"))
            .child(YueButton::new("outside", "其他操作"))
    }
}
fn spec(value: &str) -> ShortcutSpec {
    ShortcutSpec::parse(value).unwrap()
}
fn open(cx: &mut TestAppContext) -> (Entity<Probe>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
        cx.set_reduce_motion(true);
        cx.bind_keys([gpui_kit::KeyBinding::new(
            "ctrl-s",
            Save,
            Some("RecorderTest"),
        )]);
    });
    let mut saved = None;
    let (_, cx) = cx.add_window_view(|w, cx| {
        let page = cx.new(|cx| {
            let state = cx.new(|cx| {
                let mut s = YueShortcutRecorderState::new(w, cx);
                s.set_value(Some(spec("Ctrl+K")), cx).unwrap();
                s
            });
            let second = cx.new(|cx| YueShortcutRecorderState::new(w, cx));
            let subscription = cx.subscribe(
                &state,
                |s: &mut Probe, _, event: &ShortcutRecorderEvent, _| s.events.push(event.clone()),
            );
            Probe {
                state,
                second,
                events: vec![],
                commands: 0,
                visible: true,
                _subscription: subscription,
            }
        });
        saved = Some(page.clone());
        Root::new(page, w, cx)
    });
    cx.update(|w, _| w.activate_window());
    cx.simulate_resize(gpui_kit::size(px(900.), px(700.)));
    draw(cx);
    (saved.unwrap(), cx)
}
fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| {
        w.refresh();
        w.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}
fn click(cx: &mut VisualTestContext, label: &str) {
    let center = cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .find(|n| n.label().is_some_and(|s| s.starts_with(label)))
            .unwrap_or_else(|| panic!("missing {label}"))
            .bounds()
            .center()
    });
    cx.simulate_click(center, Modifiers::default());
    draw(cx);
}
fn down(cx: &mut VisualTestContext, key: &str) {
    cx.simulate_event(KeyDownEvent {
        keystroke: Keystroke::parse(key).unwrap(),
        is_held: false,
        prefer_character_input: false,
    });
    draw(cx);
}
fn up(cx: &mut VisualTestContext, key: &str) {
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse(key).unwrap(),
    });
    draw(cx);
}
fn key(cx: &mut VisualTestContext, key: &str) {
    down(cx, key);
    up(cx, key);
}
fn value(state: &Entity<YueShortcutRecorderState>, cx: &VisualTestContext) -> Option<String> {
    state.read_with(cx, |s, _| s.value().map(ShortcutSpec::display_label))
}

#[gpui_kit::test]
fn record_previews_modifiers_suppresses_actions_and_repeats(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    let state = page.read_with(cx, |s, _| s.state.clone());
    click(cx, "录入，");
    assert!(state.read_with(cx, |s, _| s.is_recording()));
    cx.simulate_event(ModifiersChangedEvent {
        modifiers: Modifiers {
            control: true,
            shift: true,
            ..Default::default()
        },
        ..Default::default()
    });
    draw(cx);
    assert!(state.read_with(cx, |s, _| s.modifiers.control && s.modifiers.shift));
    down(cx, "ctrl-s");
    assert_eq!(value(&state, cx).as_deref(), Some("Ctrl + S"));
    for _ in 0..3 {
        down(cx, "ctrl-s");
    }
    page.read_with(cx, |s, _| {
        assert_eq!(s.commands, 0);
        assert_eq!(s.events.len(), 1);
    });
    up(cx, "ctrl-s");
    key(cx, "ctrl-s");
    assert_eq!(
        page.read_with(cx, |s, _| s.commands),
        1,
        "idle recorder must not hijack commands"
    );
}

#[gpui_kit::test]
fn cancel_clear_function_keys_and_validation(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    let state = page.read_with(cx, |s, _| s.state.clone());
    click(cx, "录入，");
    key(cx, "a");
    assert!(state.read_with(cx, |s, _| s.is_recording() && s.invalid));
    assert_eq!(value(&state, cx).as_deref(), Some("Ctrl + K"));
    key(cx, "alt-f4");
    assert_eq!(
        state.read_with(cx, |s, _| s.message().to_string()),
        "系统保留，无法使用"
    );
    key(cx, "escape");
    assert_eq!(value(&state, cx).as_deref(), Some("Ctrl + K"));
    assert!(page.read_with(cx, |s, _| s.events.is_empty()));
    click(cx, "录入，");
    key(cx, "f5");
    assert_eq!(value(&state, cx).as_deref(), Some("F5"));
    click(cx, "录入，");
    key(cx, "backspace");
    assert_eq!(value(&state, cx), None);
    assert_eq!(page.read_with(cx, |s, _| s.events.len()), 2);
    click(cx, "录入，");
    key(cx, "backspace");
    assert_eq!(
        page.read_with(cx, |s, _| s.events.len()),
        2,
        "clearing empty is not a change"
    );
    state.update(cx, |s, cx| s.set_require_modifier(false, cx));
    click(cx, "录入，");
    key(cx, "shift-a");
    assert_eq!(value(&state, cx).as_deref(), Some("Shift + A"));
}

#[gpui_kit::test]
fn validation_and_success_feedback_follow_reference_timing(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    let state = page.read_with(cx, |s, _| s.state.clone());
    click(cx, "录入，");
    key(cx, "a");
    assert!(state.read_with(cx, |s, _| s.error_flash && s.invalid));

    cx.executor()
        .advance_clock(std::time::Duration::from_millis(200));
    draw(cx);
    key(cx, "alt-f4");
    assert!(state.read_with(cx, |s, _| s.error_flash));
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(299));
    draw(cx);
    assert!(state.read_with(cx, |s, _| s.error_flash));
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(1));
    draw(cx);
    assert!(state.read_with(cx, |s, _| !s.error_flash && s.invalid));
    assert_eq!(
        state.read_with(cx, |s, _| s.message().to_string()),
        "系统保留，无法使用",
        "the error message remains after the red feedback ends"
    );

    key(cx, "escape");
    click(cx, "录入，");
    key(cx, "ctrl-p");
    assert!(state.read_with(cx, |s, _| s.success));
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(699));
    draw(cx);
    assert!(state.read_with(cx, |s, _| s.success));
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(1));
    draw(cx);
    assert!(!state.read_with(cx, |s, _| s.success));
    assert_eq!(value(&state, cx).as_deref(), Some("Ctrl + P"));
}

#[gpui_kit::test]
fn conflict_requires_explicit_overwrite_and_revalidates(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    let state = page.read_with(cx, |s, _| s.state.clone());
    let reject = std::rc::Rc::new(std::cell::Cell::new(false));
    let policy = reject.clone();
    state.update(cx, |s, cx| {
        s.set_validator(
            move |_, _| {
                if policy.get() {
                    ShortcutRecorderValidation::Reject("命令已禁用".into())
                } else {
                    ShortcutRecorderValidation::Conflict("与「保存」冲突".into())
                }
            },
            cx,
        )
    });
    click(cx, "录入，");
    key(cx, "ctrl-s");
    assert!(state.read_with(cx, |s, _| s.pending_value() == Some(&spec("Ctrl+S"))));
    assert_eq!(value(&state, cx).as_deref(), Some("Ctrl + K"));
    assert!(page.read_with(cx, |s, _| s.events.is_empty()));
    click(cx, "取消");
    assert!(state.read_with(cx, |s, _| s.pending_value().is_none()));
    assert_eq!(value(&state, cx).as_deref(), Some("Ctrl + K"));
    click(cx, "录入，");
    key(cx, "ctrl-s");
    key(cx, "tab");
    key(cx, "escape");
    assert!(state.read_with(cx, |s, _| s.pending_value().is_none()));
    assert!(page.read_with(cx, |s, _| s.events.is_empty()));
    click(cx, "录入，");
    key(cx, "ctrl-s");
    reject.set(true);
    click(cx, "覆盖");
    assert_eq!(value(&state, cx).as_deref(), Some("Ctrl + K"));
    assert_eq!(
        state.read_with(cx, |s, _| s.message().to_string()),
        "命令已禁用"
    );
    reject.set(false);
    key(cx, "ctrl-s");
    // Tab into the overwrite button stays inside the recorder's focus region.
    key(cx, "tab");
    assert!(state.read_with(cx, |s, _| s.pending_value().is_some()));
    key(cx, "enter");
    assert_eq!(value(&state, cx).as_deref(), Some("Ctrl + S"));
    assert_eq!(
        page.read_with(cx, |s, _| s.events.clone()),
        vec![ShortcutRecorderEvent::Change {
            value: Some(spec("Ctrl+S")),
            overwrite: true
        }]
    );
}

#[gpui_kit::test]
fn traversal_blur_deactivation_external_sync_and_disabled_cancel(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    let state = page.read_with(cx, |s, _| s.state.clone());
    click(cx, "录入，");
    key(cx, "tab");
    assert!(!state.read_with(cx, |s, _| s.is_recording()));
    click(cx, "录入，");
    click(cx, "其他操作");
    assert!(!state.read_with(cx, |s, _| s.is_recording()));
    click(cx, "录入，");
    cx.deactivate_window();
    draw(cx);
    assert!(!state.read_with(cx, |s, _| s.is_recording()));
    cx.update(|w, _| w.activate_window());
    draw(cx);
    click(cx, "录入，");
    state.update(cx, |s, cx| {
        assert!(s.set_value(Some(spec("G I")), cx).is_err());
        assert!(s.set_value(Some(spec("Ctrl+K+C")), cx).is_err());
        assert!(s.set_value(Some(spec("Ctrl+K | Ctrl+S")), cx).is_err());
        assert!(s.is_recording());
        s.set_value(Some(spec("Alt+P")), cx).unwrap();
        assert!(!s.is_recording());
        s.set_disabled(true, cx);
    });
    draw(cx);
    click(cx, "录入，");
    key(cx, "ctrl-s");
    assert!(!state.read_with(cx, |s, _| s.is_recording()));
    assert_eq!(value(&state, cx).as_deref(), Some("Alt + P"));
    assert!(page.read_with(cx, |s, _| s.events.is_empty()));
}

#[gpui_kit::test]
fn native_symbols_custom_reservations_and_multiple_recorders(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    let state = page.read_with(cx, |s, _| s.state.clone());
    state.update(cx, |s, cx| {
        s.set_reserved(vec![spec("Ctrl+Q")], cx).unwrap()
    });
    click(cx, "录入，");
    key(cx, "ctrl-q");
    assert!(state.read_with(cx, |s, _| s.invalid));
    // Simulate the Windows spelling: Shift is folded into '+', key_char is
    // deliberately different so a text/IME result cannot become the binding.
    cx.simulate_event(KeyDownEvent {
        keystroke: Keystroke {
            key: "+".into(),
            key_char: Some("文".into()),
            modifiers: Modifiers {
                control: true,
                ..Default::default()
            },
        },
        is_held: false,
        prefer_character_input: true,
    });
    draw(cx);
    assert_eq!(value(&state, cx).as_deref(), Some("Ctrl + Plus"));
    up(cx, "ctrl-=");
    key(cx, "ctrl-s");
    assert_eq!(page.read_with(cx, |s, _| s.commands), 1);
    click(cx, "录入，");
    key(cx, "ctrl-?");
    assert_eq!(value(&state, cx).as_deref(), Some("Ctrl + ?"));
    click(cx, "第二项，");
    key(cx, "ctrl-p");
    let second = page.read_with(cx, |s, _| s.second.clone());
    assert_eq!(value(&second, cx).as_deref(), Some("Ctrl + P"));
    assert_eq!(value(&state, cx).as_deref(), Some("Ctrl + ?"));
    click(cx, "录入，");
    page.update(cx, |s, cx| {
        s.visible = false;
        cx.notify();
    });
    draw(cx);
    key(cx, "ctrl-s");
    assert_eq!(
        value(&state, cx).as_deref(),
        Some("Ctrl + ?"),
        "unmounted recorders must not capture input"
    );
}

#[gpui_kit::test]
fn keyboard_activation_disabled_draft_and_accessibility(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    let state = page.read_with(cx, |s, _| s.state.clone());
    cx.update(|w, cx| state.read(cx).focus.clone().focus(w, cx));
    draw(cx);
    key(cx, "enter");
    assert!(state.read_with(cx, |s, _| s.is_recording()));
    state.update(cx, |s, cx| s.set_disabled(true, cx));
    draw(cx);
    assert!(!state.read_with(cx, |s, _| s.is_recording()));
    cx.update(|w, _| {
        let nodes = snapshots(w);
        let field = nodes
            .iter()
            .find(|n| n.label() == Some("录入，Ctrl + K"))
            .unwrap();
        assert_eq!(field.role(), Some(gpui_kit::Role::Button));
        assert_eq!(field.disabled(), Some(true));
        assert_eq!(field.bounds().size.height, px(32.));
        assert!(
            !nodes
                .iter()
                .any(|n| n.role() == Some(gpui_kit::Role::Image)),
            "keycaps must not duplicate the field's name"
        );
    });
    assert!(page.read_with(cx, |s, _| s.events.is_empty()));
    state.update(cx, |s, cx| s.set_disabled(false, cx));
    draw(cx);
    click(cx, "录入，");
    key(cx, "f13");
    assert!(state.read_with(cx, |s, _| s.invalid));
    key(cx, "ctrl-tab");
    assert_eq!(
        value(&state, cx).as_deref(),
        Some("Ctrl + Tab"),
        "modified Tab is recordable"
    );
}
