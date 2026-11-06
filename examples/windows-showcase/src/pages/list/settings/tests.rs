use super::*;
use gpui_kit::base::{
    Root,
    test_support::{ElementSnapshot, snapshots},
};
use gpui_kit::{ElementId, Entity, Modifiers, TestAppContext, VisualTestContext, px, size};
use yue_component::{Accent, ThemeMode};

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| {
        w.refresh();
        w.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}
fn open(cx: &mut TestAppContext) -> (Entity<SettingsExamples>, &mut VisualTestContext) {
    cx.update(|cx| {
        yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
        cx.set_reduce_motion(true);
    });
    let mut view = None;
    let (_, cx) = cx.add_window_view(|w, cx| {
        let settings = cx.new(|cx| SettingsExamples::new(w, cx));
        view = Some(settings.clone());
        Root::new(settings, w, cx)
    });
    cx.simulate_resize(size(px(1000.), px(1800.)));
    draw(cx);
    (view.unwrap(), cx)
}
fn node(cx: &mut VisualTestContext, id: &str) -> ElementSnapshot {
    cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .find(|n| n.path().last() == Some(&ElementId::from(id.to_owned())))
            .unwrap_or_else(|| panic!("missing {id}"))
    })
}
fn click(cx: &mut VisualTestContext, id: &str) {
    let center = node(cx, id).bounds().center();
    cx.simulate_click(center, Modifiers::default());
    draw(cx);
}
fn press(cx: &mut VisualTestContext, key: &str) {
    let keystroke = gpui_kit::Keystroke::parse(key).unwrap();
    cx.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui_kit::KeyUpEvent { keystroke });
}
fn click_label(cx: &mut VisualTestContext, label: &str) {
    let center = cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .find(|n| n.label() == Some(label) && n.visible())
            .unwrap_or_else(|| panic!("missing {label}"))
            .bounds()
            .center()
    });
    cx.simulate_click(center, Modifiers::default());
    draw(cx);
}

#[gpui_kit::test]
fn settings_choices_change_real_state_density_and_restore_defaults(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    let ordinary_height = node(cx, "settings-general").bounds().size.height;
    click(cx, "settings-autosave");
    assert!(!view.read_with(cx, |v, _| v.autosave));
    click_label(cx, "紧凑");
    assert!(view.read_with(cx, |v, _| v.compact));
    assert!(node(cx, "settings-general").bounds().size.height < ordinary_height);
    click(cx, "settings-language");
    click_label(cx, "English");
    assert!(view.read_with(cx, |v, cx| v.language.read(cx).value().contains("en")));
    assert_eq!(
        view.read_with(cx, |v, _| v.feedback.clone()),
        "示例语言已设为English"
    );
    // Pointer focus followed by actual arrow input exercises the slider owner.
    click_label(cx, "提示音量");
    press(cx, "right");
    draw(cx);
    assert_eq!(
        view.read_with(cx, |v, cx| v.volume.read(cx).value(cx).end()),
        65.
    );
    click(cx, "settings-clear-cache");
    assert_eq!(view.read_with(cx, |v, _| v.cache_mb), 0);
    assert_eq!(node(cx, "settings-clear-cache").disabled(), Some(true));
    click(cx, "settings-reset");
    assert!(view.read_with(cx, |v, cx| v.autosave
        && !v.compact
        && v.language.read(cx).value().contains("system")
        && v.volume.read(cx).value(cx).end() == 60.));
    assert_eq!(view.read_with(cx, |v, _| v.cache_mb), 0);
}

#[gpui_kit::test]
fn notification_dependencies_preserve_preferences_and_block_disabled_input(
    cx: &mut TestAppContext,
) {
    let (view, cx) = open(cx);
    click(cx, "settings-notifications-toggle");
    assert!(!view.read_with(cx, |v, _| v.notifications));
    assert_eq!(node(cx, "settings-sound").disabled(), Some(true));
    click(cx, "settings-sound");
    assert!(view.read_with(cx, |v, _| v.sound));
    let before = view.read_with(cx, |v, cx| v.volume.read(cx).value(cx));
    click_label(cx, "提示音量");
    press(cx, "end");
    draw(cx);
    assert_eq!(
        view.read_with(cx, |v, cx| v.volume.read(cx).value(cx)),
        before
    );
    click(cx, "settings-policy");
    assert_eq!(node(cx, "settings-policy").checked(), Some(true));
    assert_eq!(node(cx, "settings-policy").disabled(), Some(true));
    click(cx, "settings-notifications-toggle");
    assert!(view.read_with(cx, |v, _| v.sound));
    click(cx, "settings-sound");
    assert!(!view.read_with(cx, |v, _| v.sound));
    click_label(cx, "提示音量");
    press(cx, "end");
    draw(cx);
    assert_eq!(
        view.read_with(cx, |v, cx| v.volume.read(cx).value(cx)),
        before
    );
    click(cx, "settings-reset");
    assert!(view.read_with(cx, |v, _| v.sound && v.notifications));
}

#[gpui_kit::test]
fn inline_and_stacked_settings_keep_slot_bounds_at_narrow_width_and_rem(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    let rows = [
        "settings-autosave-row",
        "settings-language-row",
        "settings-density-row",
        "settings-notifications-row",
        "settings-sound-row",
        "settings-volume-row",
        "settings-policy-row",
        "settings-cache-row",
        "settings-version-row",
        "settings-reset-row",
    ];
    for id in rows {
        let row = node(cx, id).bounds();
        cx.update(|w, _| {
            let nodes = snapshots(w);
            let extra = nodes
                .iter()
                .find(|n| {
                    n.path().contains(&ElementId::from(id))
                        && n.path().last() == Some(&ElementId::from("list-extra"))
                })
                .unwrap()
                .bounds();
            let main = nodes
                .iter()
                .find(|n| {
                    n.path().contains(&ElementId::from(id))
                        && n.path().last() == Some(&ElementId::from("list-main"))
                })
                .unwrap()
                .bounds();
            assert!(main.right() <= extra.left());
            assert!(extra.right() <= row.right());
        });
    }
    click_label(cx, "下方控件");
    cx.simulate_resize(size(px(480.), px(3000.)));
    for (mode, accent, rem) in [
        (ThemeMode::Light, Accent::Azure, 16.),
        (ThemeMode::Dark, Accent::Azure, 20.),
    ] {
        cx.update(|w, cx| {
            yue_component::set_theme(cx, YueTheme::new(mode, accent));
            w.set_rem_size(px(rem));
        });
        draw(cx);
        for id in rows {
            let row = node(cx, id).bounds();
            assert!(row.right() <= px(480.));
            cx.update(|w, _| {
                let nodes = snapshots(w);
                let actions = nodes
                    .iter()
                    .find(|n| {
                        n.path().contains(&ElementId::from(id))
                            && n.path().last() == Some(&ElementId::from("list-actions"))
                    })
                    .unwrap()
                    .bounds();
                let meta = nodes
                    .iter()
                    .find(|n| {
                        n.path().contains(&ElementId::from(id))
                            && n.path().last() == Some(&ElementId::from("list-meta-row"))
                    })
                    .unwrap()
                    .bounds();
                assert!(
                    actions.top() >= meta.bottom(),
                    "{id}: {actions:?}, {meta:?}"
                );
                assert!(
                    actions.left() >= row.left() && actions.right() <= row.right(),
                    "{id}"
                );
            });
        }
    }
    click(cx, "settings-autosave");
    assert!(!view.read_with(cx, |v, _| v.autosave));
}
