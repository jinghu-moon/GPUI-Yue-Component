//! Check resolved layout and paint output, not just the appearance inputs.
use super::*;
use crate::{Accent, ThemeMode};
use gpui_kit::base::Root;
use gpui_kit::{AppContext as _, Context, Primitive, Render, TestAppContext, px, rgb};

struct Probe {
    theme: YueTheme,
    size: KbdSize,
    variant: KbdVariant,
    state: KbdState,
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let child = YueKbd::new("K")
            .id("animated-key")
            .theme(self.theme)
            .size(self.size)
            .variant(self.variant)
            .state(self.state);
        div()
            .flex()
            .items_start()
            .p_4()
            .text_color(rgb(0xffffff))
            .child(child)
    }
}

struct ShortcutProbe {
    separator: ShortcutSeparator,
    disabled: bool,
}
impl Render for ShortcutProbe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().flex().flex_col().items_start().p_4().child(
            YueShortcut::parse("shortcut-probe", "Ctrl+K C | Ctrl+/")
                .unwrap()
                .separator(self.separator)
                .state(if self.disabled {
                    KbdState::Disabled
                } else {
                    KbdState::Default
                })
                .key_state("control", KbdState::Pressed)
                .unwrap(),
        )
    }
}

#[gpui_kit::test]
fn individual_key_states_and_separator_preserve_sequence_semantics(cx: &mut TestAppContext) {
    use gpui_kit::base::test_support::snapshots;
    cx.update(|cx| {
        crate::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
        cx.set_reduce_motion(true);
    });
    let mut saved = None;
    let (_, cx) = cx.add_window_view(|w, cx| {
        let view = cx.new(|_| ShortcutProbe {
            separator: ShortcutSeparator::Plus,
            disabled: false,
        });
        saved = Some(view.clone());
        Root::new(view, w, cx)
    });
    let view = saved.unwrap();
    let mut widths = Vec::new();
    for separator in [ShortcutSeparator::Plus, ShortcutSeparator::None] {
        view.update(cx, |p, cx| {
            p.separator = separator;
            cx.notify();
        });
        cx.run_until_parked();
        widths.push(cx.update(|w, cx| {
            w.refresh();
            w.draw(cx).clear(cx);
            let groups: Vec<_> = snapshots(w)
                .into_iter()
                .filter(|n| n.role() == Some(Role::Image))
                .collect();
            assert_eq!(groups.len(), 1);
            assert_eq!(groups[0].label(), Some("Ctrl + K 然后 C 或 Ctrl + Slash"));
            let blue: Vec<_> = w
                .painted_quads()
                .into_iter()
                .filter(|q| q.background.as_solid() == Some(rgb(0xe6f0ff).into()))
                .collect();
            assert_eq!(
                blue.len(),
                2,
                "both Ctrl occurrences should be pressed, other keys should remain normal"
            );
            groups[0].bounds().size.width
        }));
    }
    assert!(
        widths[1] < widths[0],
        "removing + must reduce width without altering the label"
    );
    view.update(cx, |p, cx| {
        p.disabled = true;
        cx.notify();
    });
    cx.run_until_parked();
    cx.update(|w, cx| {
        w.refresh();
        w.draw(cx).clear(cx);
        assert!(
            !w.painted_quads()
                .iter()
                .any(|q| q.background.as_solid() == Some(rgb(0xe6f0ff).into()))
        );
    });
}

#[gpui_kit::test]
fn identified_keycaps_transition_and_honor_reduced_motion(cx: &mut TestAppContext) {
    use std::time::Duration;
    let light = YueTheme::new(ThemeMode::Light, Accent::Azure);
    cx.update(|cx| {
        crate::init(cx, light).unwrap();
        cx.set_reduce_motion(false);
    });
    let mut saved = None;
    let (_, cx) = cx.add_window_view(|w, cx| {
        let view = cx.new(|_| Probe {
            theme: light,
            size: KbdSize::Medium,
            variant: KbdVariant::Raised,
            state: KbdState::Default,
        });
        saved = Some(view.clone());
        Root::new(view, w, cx)
    });
    let view = saved.unwrap();
    let draw = |cx: &mut gpui_kit::VisualTestContext| {
        cx.update(|w, cx| {
            w.refresh();
            w.draw(cx).clear(cx);
        });
    };
    cx.run_until_parked();
    draw(cx);
    view.update(cx, |p, cx| {
        p.state = KbdState::Pressed;
        cx.notify();
    });
    draw(cx);
    let surface = |cx: &mut gpui_kit::VisualTestContext| {
        cx.update(|w, _| {
            w.painted_quads()
                .into_iter()
                .find(|q| {
                    q.bounds.size.height.0 == 48.
                        && q.background.as_solid().is_some_and(|c| c.a > 0.)
                })
                .unwrap()
                .background
                .as_solid()
                .unwrap()
        })
    };
    let start = surface(cx);
    cx.executor().advance_clock(Duration::from_millis(50));
    draw(cx);
    let middle = surface(cx);
    assert_ne!(
        middle, start,
        "transition must advance without an input event"
    );
    assert_ne!(middle, rgb(0xe6f0ff).into());
    cx.executor().advance_clock(Duration::from_millis(60));
    draw(cx);
    assert_eq!(surface(cx), rgb(0xe6f0ff).into());
    cx.update(|_, cx| cx.set_reduce_motion(true));
    view.update(cx, |p, cx| {
        p.state = KbdState::Default;
        cx.notify();
    });
    draw(cx);
    assert_eq!(surface(cx), rgb(0xffffff).into());
}

struct Hosts {
    commands: usize,
}
impl Render for Hosts {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let spec = ShortcutSpec::parse("Ctrl+S").unwrap();
        div()
            .flex()
            .flex_col()
            .items_start()
            .gap_4()
            .p_4()
            .child(
                crate::YueButton::new("host-save", "Save")
                    .shortcut(spec.clone())
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.commands += 1;
                        cx.notify();
                    })),
            )
            .child(
                crate::YueButton::new("host-disabled", "Disabled")
                    .shortcut(spec.clone())
                    .disabled(true)
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.commands += 100;
                        cx.notify();
                    })),
            )
            .child(
                crate::YueDropdownMenu::new(
                    "host-menu",
                    crate::current_theme(cx),
                    crate::YueButton::new("host-menu-trigger", "Commands"),
                )
                .items([
                    crate::YueMenuItem::new("Save from menu")
                        .shortcut(spec.clone())
                        .on_click(cx.listener(|s, _, _, cx| {
                            s.commands += 1;
                            cx.notify();
                        })),
                    crate::YueMenuItem::new("Disabled export")
                        .shortcut(spec)
                        .disabled(true),
                ]),
            )
    }
}
#[gpui_kit::test]
fn button_and_menu_shortcuts_preserve_command_and_disabled_behavior(cx: &mut TestAppContext) {
    use gpui_kit::base::test_support::snapshots;
    cx.update(|cx| {
        crate::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
        cx.set_reduce_motion(true);
    });
    let mut saved = None;
    let (_, cx) = cx.add_window_view(|w, cx| {
        let view = cx.new(|_| Hosts { commands: 0 });
        saved = Some(view.clone());
        Root::new(view, w, cx)
    });
    let view = saved.unwrap();
    let draw = |cx: &mut gpui_kit::VisualTestContext| {
        cx.run_until_parked();
        cx.update(|w, cx| {
            w.refresh();
            w.draw(cx).clear(cx);
        });
        cx.run_until_parked();
    };
    draw(cx);
    let click = |cx: &mut gpui_kit::VisualTestContext, label: &str| {
        let center = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.label() == Some(label))
                .unwrap()
                .bounds()
                .center()
        });
        cx.simulate_click(center, Default::default());
    };
    click(cx, "Save");
    draw(cx);
    assert_eq!(view.read_with(cx, |v, _| v.commands), 1);
    click(cx, "Disabled");
    draw(cx);
    assert_eq!(view.read_with(cx, |v, _| v.commands), 1);
    click(cx, "Commands");
    draw(cx);
    cx.update(|w, _| {
        let nodes = snapshots(w);
        assert!(
            !nodes.iter().any(|n| n.role() == Some(Role::Image)),
            "hosts own the shortcut semantics"
        );
        assert_eq!(
            nodes
                .iter()
                .find(|n| n.label() == Some("Disabled export"))
                .unwrap()
                .disabled(),
            Some(true)
        );
    });
    click(cx, "Save from menu");
    draw(cx);
    assert_eq!(view.read_with(cx, |v, _| v.commands), 2);
}

#[gpui_kit::test]
fn reference_keycap_geometry_and_state_paint(cx: &mut TestAppContext) {
    let light = YueTheme::new(ThemeMode::Light, Accent::Azure);
    cx.update(|cx| {
        crate::init(cx, light).unwrap();
        cx.set_reduce_motion(true);
    });
    let mut probe = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| Probe {
            theme: light,
            size: KbdSize::Medium,
            variant: KbdVariant::Raised,
            state: KbdState::Default,
        });
        probe = Some(view.clone());
        Root::new(view, window, cx)
    });
    let probe = probe.unwrap();
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        for rem in [16., 20.] {
            for (size, height) in [
                (KbdSize::Small, 20.),
                (KbdSize::Medium, 24.),
                (KbdSize::Large, 28.),
            ] {
                for state in [
                    KbdState::Default,
                    KbdState::Pressed,
                    KbdState::Active,
                    KbdState::Disabled,
                ] {
                    probe.update(cx, |p, cx| {
                        p.theme = YueTheme::new(mode, Accent::Azure);
                        p.size = size;
                        p.state = state;
                        cx.notify();
                    });
                    cx.run_until_parked();
                    cx.update(|window, cx| {
                        window.set_rem_size(px(rem));
                        window.refresh();
                        window.draw(cx).clear(cx);
                        let unit = rem / 16. * window.scale_factor();
                        let quads: Vec<_> = window
                            .painted_quads()
                            .into_iter()
                            .filter(|q| {
                                q.border_widths.top.0 > 0. && q.bounds.size.height.0 <= 35. * unit
                            })
                            .collect();
                        // GPUI splits unequal borders into clipped edge quads.
                        assert!(!quads.is_empty());
                        assert!(quads.iter().all(|q| q.bounds == quads[0].bounds));
                        let key = &quads[0];
                        assert_eq!(
                            key.bounds.size.width.0,
                            height * unit,
                            "single-character keys must be square"
                        );
                        assert_eq!(key.bounds.size.height.0, height * unit);
                        let snap = |n: f32| (n - 0.5).ceil();
                        assert_eq!(key.border_widths.top.0, snap(unit));
                        assert_eq!(key.border_widths.left.0, snap(unit));
                        assert_eq!(key.border_widths.right.0, snap(unit));
                        assert_eq!(
                            key.border_widths.bottom.0,
                            if state == KbdState::Pressed {
                                snap(unit)
                            } else {
                                snap(2. * unit)
                            }
                        );
                        assert_eq!(key.corner_radii.top_left.0, 4. * unit);
                        assert_eq!(
                            key.bounds.top().0,
                            snap((16. + if state == KbdState::Pressed { 1. } else { 0. }) * unit)
                        );
                        let dark = mode == ThemeMode::Dark;
                        let (background, border) = match state {
                            KbdState::Pressed => (
                                if dark { 0x0f2644 } else { 0xe6f0ff },
                                if dark { 0x3581e2 } else { 0x1f75db },
                            ),
                            KbdState::Active => (
                                if dark { 0xf5f5f5 } else { 0x1f1f1f },
                                if dark { 0xf5f5f5 } else { 0x1f1f1f },
                            ),
                            _ => (
                                if dark { 0x1f1f1f } else { 0xffffff },
                                if dark { 0x737373 } else { 0xa3a3a3 },
                            ),
                        };
                        let mut expected_bg = rgb(background);
                        let mut expected_border = rgb(border);
                        if state == KbdState::Disabled {
                            expected_bg.a = 0.4;
                            expected_border.a = 0.4;
                        }
                        assert!(
                            window.painted_quads().iter().any(|q| q.bounds == key.bounds
                                && q.background.as_solid() == Some(expected_bg.into())),
                            "missing key surface: {mode:?} {state:?}"
                        );
                        assert_eq!(key.border_color, expected_border.into());
                        assert!(
                            !window
                                .painted_primitives()
                                .iter()
                                .any(|p| matches!(p, Primitive::Shadow(_))),
                            "reference keycaps have no shadow"
                        );
                    });
                }
            }
        }
    }
}
