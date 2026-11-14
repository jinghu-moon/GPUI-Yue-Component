use super::*;
use crate::{Accent, NeutralTone, ThemeMode, YueButton};
use gpui_kit::{
    AppContext as _, Context, Entity, InteractiveElement as _, Modifiers, Render, Role,
    Styled as _, TestAppContext, VisualTestContext,
    base::{
        Root, TestSupportExt as _,
        test_support::{ElementSnapshot, snapshots},
    },
    div, px,
};

struct Probe {
    visible: bool,
    accept: bool,
    requests: Vec<AlertCloseReason>,
    closed: usize,
    width: f32,
    description: bool,
    action: bool,
    closable: bool,
    expanded: bool,
    lines: Option<usize>,
    duration: Option<Duration>,
    focus: FocusHandle,
    mounted: bool,
    offscreen: bool,
    brief: bool,
    kind: AlertKind,
    variant: AlertVariant,
    size: AlertSize,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut alert = YueAlert::new("alert", "保存状态")
            .visible(self.visible)
            .closable(self.closable)
            .kind(self.kind)
            .variant(self.variant)
            .size(self.size)
            .duration(self.duration)
            .expanded(self.expanded)
            .focus_after_close(&self.focus)
            .on_expanded_change({
                let h = cx.listener(|s, value: &bool, _, cx| {
                    s.expanded = *value;
                    cx.notify();
                });
                move |value, w, cx| h(&value, w, cx)
            })
            .on_close({
                let h = cx.listener(|s, reason: &AlertCloseReason, _, cx| {
                    s.requests.push(*reason);
                    if s.accept {
                        s.visible = false;
                    }
                    cx.notify();
                });
                move |r, w, cx| h(&r, w, cx)
            })
            .on_closed({
                let h = cx.listener(|s, _: &(), _, cx| {
                    s.closed += 1;
                    cx.notify();
                });
                move |w, cx| h(&(), w, cx)
            });
        if self.description {
            alert=alert.description(if self.brief {"提示内容在窄面板中需要多行显示，调整宽度后应自动隐藏不再需要的展开按钮。".into()} else {"这是一段足够长的中文提示说明，用于确认窄容器、长段落、连续标识符和多行说明都能完整显示。LongIdentifierWithoutSpacesABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ。".repeat(3)});
        }
        if let Some(n) = self.lines {
            alert = alert.max_lines(n);
        }
        if self.action {
            alert = alert.action(YueButton::new("retry", "重试"));
        }
        div()
            .p_4()
            .w(px(self.width))
            .flex()
            .flex_col()
            .child(YueButton::new("outside", "外部操作").track_focus(&self.focus))
            .when(self.mounted, |d| {
                d.child(
                    div()
                        .mt(if self.offscreen { px(2000.) } else { px(0.) })
                        .child(alert),
                )
            })
            .child(div().id("after").test_support().h(px(20.)))
    }
}
use gpui_kit::prelude::FluentBuilder as _;
fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| w.simulate_next_frame(cx));
    cx.update(|w, cx| {
        w.refresh();
        w.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}
fn open(cx: &mut TestAppContext) -> (Entity<Probe>, &mut VisualTestContext) {
    cx.update(|cx| crate::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap());
    let mut saved = None;
    let (_, cx) = cx.add_window_view(|w, cx| {
        let view = cx.new(|cx| Probe {
            visible: true,
            accept: true,
            requests: vec![],
            closed: 0,
            width: 600.,
            description: false,
            action: false,
            closable: true,
            expanded: false,
            lines: None,
            duration: None,
            focus: cx.focus_handle(),
            mounted: true,
            offscreen: false,
            brief: false,
            kind: AlertKind::Info,
            variant: AlertVariant::Tonal,
            size: AlertSize::Medium,
        });
        saved = Some(view.clone());
        Root::new(view, w, cx)
    });
    cx.update(|w, _| w.activate_window());
    draw(cx);
    draw(cx);
    (saved.unwrap(), cx)
}
fn find(w: &Window, id: ElementId) -> Option<ElementSnapshot> {
    snapshots(w)
        .into_iter()
        .find(|n| n.path().last() == Some(&id))
}
fn part(w: &Window, name: &str) -> ElementSnapshot {
    find(
        w,
        ElementId::from((ElementId::from("alert"), name.to_string())),
    )
    .unwrap_or_else(|| panic!("missing {name}"))
}
fn tick(cx: &mut VisualTestContext, ms: u64) {
    cx.executor().advance_clock(Duration::from_millis(ms));
    draw(cx);
}

#[gpui_kit::test]
fn semantic_variants_and_density_preserve_wrapped_geometry(cx: &mut TestAppContext) {
    let (v, cx) = open(cx);
    for kind in [
        AlertKind::Info,
        AlertKind::Success,
        AlertKind::Warning,
        AlertKind::Error,
    ] {
        for variant in [
            AlertVariant::Tonal,
            AlertVariant::Outlined,
            AlertVariant::Solid,
        ] {
            v.update(cx, |p, cx| {
                p.kind = kind;
                p.variant = variant;
                p.description = true;
                cx.notify();
            });
            draw(cx);
            draw(cx);
            cx.update(|w, _| {
                let a = find(w, "alert".into()).unwrap();
                assert_eq!(
                    a.role(),
                    Some(match kind {
                        AlertKind::Info | AlertKind::Success => Role::Status,
                        _ => Role::Alert,
                    })
                );
                assert!(part(w, "description").bounds().bottom() <= a.bounds().bottom());
                assert!(part(w, "close").bounds().right() <= a.bounds().right());
            });
        }
    }
    v.update(cx, |p, cx| {
        p.description = false;
        p.size = AlertSize::Small;
        cx.notify();
    });
    draw(cx);
    let small = cx.update(|w, _| find(w, "alert".into()).unwrap().bounds().size.height);
    v.update(cx, |p, cx| {
        p.size = AlertSize::Medium;
        cx.notify();
    });
    draw(cx);
    cx.update(|w, _| assert!(find(w, "alert".into()).unwrap().bounds().size.height > small));
}
#[gpui_kit::test]
fn narrow_actions_move_below_body_without_losing_close_lane(cx: &mut TestAppContext) {
    let (v, cx) = open(cx);
    v.update(cx, |p, cx| {
        p.action = true;
        p.description = true;
        cx.notify();
    });
    draw(cx);
    draw(cx);
    cx.update(|w, _| assert!(part(w, "action").bounds().left() > part(w, "text").bounds().left()));
    v.update(cx, |p, cx| {
        p.width = 240.;
        cx.notify();
    });
    draw(cx);
    draw(cx);
    tick(cx, 500);
    cx.update(|w, _| {
        assert!(part(w, "action").bounds().top() >= part(w, "text").bounds().bottom());
        let a = find(w, "alert".into()).unwrap().bounds();
        assert!(part(w, "close").bounds().right() <= a.right());
        assert!(part(w, "description").bounds().right() <= a.right());
    });
}

#[gpui_kit::test]
fn single_line_icon_text_and_commands_share_center_and_even_padding(cx: &mut TestAppContext) {
    let (v, cx) = open(cx);
    for size in [AlertSize::Small, AlertSize::Medium] {
        for action in [false, true] {
            for closable in [false, true] {
                v.update(cx, |p, cx| {
                    p.size = size;
                    p.action = action;
                    p.closable = closable;
                    cx.notify();
                });
                draw(cx);
                draw(cx);
                tick(cx, 500);
                cx.update(|w, _| {
                    let alert = find(w, "alert".into()).unwrap().bounds();
                    let message = part(w, "message").bounds();
                    let icon = part(w, "icon").bounds();
                    let text_offset = message.center().y - alert.center().y;
                    assert!(text_offset.abs() <= px(0.1),
                        "{size:?}, action={action}, closable={closable}: message {message:?}, alert {alert:?}");
                    assert!((icon.center().y - message.center().y).abs() <= px(0.1),
                        "icon {icon:?}, message {message:?}");
                    if action {
                        let button = find(w, "retry".into()).unwrap().bounds();
                        let slot = part(w, "action").bounds();
                        assert!((slot.size.height - button.size.height).abs() <= px(0.1),
                            "anonymous line box must not enlarge action slot: {slot:?} vs {button:?}");
                        let top = button.top() - alert.top();
                        let bottom = alert.bottom() - button.bottom();
                        assert!((top - bottom).abs() <= px(0.1), "action padding {top:?}/{bottom:?}");
                    }
                    if closable {
                        let close = part(w, "close").bounds();
                        assert!((close.center().y - alert.center().y).abs() <= px(0.1));
                    }
                });
            }
        }
    }
}

#[gpui_kit::test]
fn multiline_icon_aligns_with_first_line_not_the_whole_description(cx: &mut TestAppContext) {
    let (v, cx) = open(cx);
    for rem in [16., 20., 24.] {
        cx.update(|w, _| w.set_rem_size(px(rem)));
        for size in [AlertSize::Small, AlertSize::Medium] {
            v.update(cx, |p, cx| {
                p.description = true;
                p.action = true;
                p.width = 1000.;
                p.size = size;
                cx.notify();
            });
            draw(cx);
            draw(cx);
            tick(cx, 500);
            cx.update(|w, _| {
                let message = part(w, "message").bounds();
                let icon = part(w, "icon").bounds();
                assert!(
                    (message.center().y - icon.center().y).abs() <= px(0.1),
                    "{size:?}: {message:?} vs {icon:?}"
                );
                let action = part(w, "action").bounds();
                let button = find(w, "retry".into()).unwrap().bounds();
                assert!((action.size.height - button.size.height).abs() <= px(0.1));
            });
        }
    }
}

#[gpui_kit::test]
fn narrow_action_does_not_pull_icon_down_from_the_title(cx: &mut TestAppContext) {
    let (v, cx) = open(cx);
    v.update(cx, |p, cx| {
        p.action = true;
        p.width = 240.;
        p.description = false;
        cx.notify();
    });
    draw(cx);
    draw(cx);
    tick(cx, 500);
    cx.update(|w, _| {
        let message = part(w, "message").bounds();
        let icon = part(w, "icon").bounds();
        assert!((message.center().y - icon.center().y).abs() <= px(0.1));
        assert!(part(w, "action").bounds().top() >= message.bottom());
    });
}

#[gpui_kit::test]
fn outlined_paints_a_real_semantic_border_while_tonal_does_not(cx: &mut TestAppContext) {
    let (v, cx) = open(cx);
    for kind in [
        AlertKind::Info,
        AlertKind::Success,
        AlertKind::Warning,
        AlertKind::Error,
    ] {
        for variant in [AlertVariant::Tonal, AlertVariant::Outlined] {
            v.update(cx, |p, cx| {
                p.kind = kind;
                p.variant = variant;
                p.closable = false;
                cx.notify();
            });
            draw(cx);
            tick(cx, 500);
            cx.update(|w, cx| {
                let bounds = find(w, "alert".into())
                    .unwrap()
                    .bounds()
                    .scale(w.scale_factor());
                let theme = crate::current_theme(cx);
                let (_, _, border) = super::element::colors(theme, kind, variant);
                // GPUI submits the background and border as separate quads.
                let quads = w.painted_quads();
                if variant == AlertVariant::Outlined {
                    let q = quads
                        .iter()
                        .find(|q| q.bounds == bounds && q.border_widths.top.0 > 0.)
                        .expect("painted alert outline");
                    let width = crate::tokens::rems(theme, crate::theme::keys::STATUS_BORDER_WIDTH)
                        .to_pixels(w.rem_size())
                        .scale(w.scale_factor());
                    assert_eq!(q.border_widths.top, width, "{kind:?} outline width");
                    assert_eq!(q.border_widths.bottom, width);
                    assert_eq!(q.border_widths.left, width);
                    assert_eq!(q.border_widths.right, width);
                    assert_eq!(q.border_color, gpui_kit::Hsla::from(border));
                } else {
                    assert!(
                        !quads
                            .iter()
                            .any(|q| q.bounds == bounds && q.border_widths.top.0 > 0.)
                    );
                }
            });
        }
    }
}

#[gpui_kit::test]
fn countdown_paints_a_nonzero_progress_line_that_shrinks(cx: &mut TestAppContext) {
    let (v, cx) = open(cx);
    v.update(cx, |p, cx| {
        p.duration = Some(Duration::from_secs(2));
        cx.notify();
    });
    draw(cx);
    let progress = |cx: &mut VisualTestContext| {
        cx.update(|w, cx| {
            let bounds = find(w, "alert".into())
                .unwrap()
                .bounds()
                .scale(w.scale_factor());
            let theme = crate::current_theme(cx);
            let (_, fg, _) = super::element::colors(theme, AlertKind::Info, AlertVariant::Tonal);
            let q = w
                .painted_quads()
                .into_iter()
                .find(|q| {
                    q.bounds.bottom() == bounds.bottom()
                        && q.bounds.left() == bounds.left()
                        && q.bounds.size.height.0 > 0.
                        && q.bounds.size.height < bounds.size.height
                        && q.background.as_solid() == Some(gpui_kit::Hsla::from(fg))
                })
                .expect("visible countdown line");
            q.bounds.size.width
        })
    };
    let full = progress(cx);
    tick(cx, 500);
    let left = progress(cx);
    assert!(left.0 > 0. && left < full);
}
#[gpui_kit::test]
fn controlled_keyboard_close_detaches_then_completes_once_and_restores_focus(
    cx: &mut TestAppContext,
) {
    let (v, cx) = open(cx);
    let focus = v.read_with(cx, |p, _| p.focus.clone());
    cx.update(|w, cx| focus.focus(w, cx));
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|w, _| assert_eq!(part(w, "close").focused(), Some(true)));
    let keystroke = gpui_kit::Keystroke::parse("enter").unwrap();
    cx.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui_kit::KeyUpEvent { keystroke });
    draw(cx);
    v.read_with(cx, |p, _| {
        assert_eq!(p.requests, vec![AlertCloseReason::CloseButton])
    });
    cx.update(|w, _| assert!(find(w, "alert".into()).is_none()));
    tick(cx, 1000);
    draw(cx);
    v.read_with(cx, |p, _| assert_eq!(p.closed, 1));
    let focus = v.read_with(cx, |p, _| p.focus.clone());
    cx.update(|w, _| assert!(focus.is_focused(w)));
    for _ in 0..3 {
        draw(cx);
    }
    v.read_with(cx, |p, _| assert_eq!(p.closed, 1));
}
#[gpui_kit::test]
fn ignored_close_reversal_and_programmatic_hide_do_not_duplicate_requests(cx: &mut TestAppContext) {
    let (v, cx) = open(cx);
    v.update(cx, |p, cx| {
        p.accept = false;
        cx.notify();
    });
    draw(cx);
    let close = cx.update(|w, _| part(w, "close").bounds().center());
    cx.simulate_click(close, Modifiers::default());
    draw(cx);
    v.read_with(cx, |p, _| {
        assert!(p.visible);
        assert_eq!(p.requests.len(), 1);
        assert_eq!(p.closed, 0);
    });
    v.update(cx, |p, cx| {
        p.visible = false;
        cx.notify();
    });
    draw(cx);
    tick(cx, 30);
    v.update(cx, |p, cx| {
        p.visible = true;
        cx.notify();
    });
    draw(cx);
    tick(cx, 1000);
    v.read_with(cx, |p, _| assert_eq!(p.closed, 0));
    v.update(cx, |p, cx| {
        p.visible = false;
        cx.notify();
    });
    draw(cx);
    tick(cx, 1000);
    draw(cx);
    v.read_with(cx, |p, _| {
        assert_eq!(p.closed, 1);
        assert_eq!(p.requests.len(), 1);
    });
}
#[test]
fn semantic_text_contrast_across_all_profiles() {
    fn luminance(c: gpui_kit::Rgba) -> f32 {
        let f = |x: f32| {
            if x <= 0.04045 {
                x / 12.92
            } else {
                ((x + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * f(c.r) + 0.7152 * f(c.g) + 0.0722 * f(c.b)
    }
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        for accent in [Accent::Azure, Accent::Neutral] {
            for neutral in NeutralTone::ALL {
                let theme = YueTheme::new(mode, accent).with_neutral(neutral);
                for kind in [
                    AlertKind::Info,
                    AlertKind::Success,
                    AlertKind::Warning,
                    AlertKind::Error,
                ] {
                    for variant in [
                        AlertVariant::Tonal,
                        AlertVariant::Outlined,
                        AlertVariant::Solid,
                    ] {
                        let (bg, fg, _) = super::element::colors(theme, kind, variant);
                        let (a, b) = (luminance(bg), luminance(fg));
                        let contrast = (a.max(b) + 0.05) / (a.min(b) + 0.05);
                        assert!(
                            contrast >= 4.5,
                            "{theme:?}/{kind:?}/{variant:?}: {contrast}"
                        );
                    }
                }
            }
        }
    }
}

#[gpui_kit::test]
fn description_only_offers_expand_when_it_really_overflows(cx: &mut TestAppContext) {
    let (v, cx) = open(cx);
    v.update(cx, |p, cx| {
        p.description = true;
        p.lines = Some(2);
        p.width = 280.;
        cx.notify();
    });
    draw(cx);
    draw(cx);
    tick(cx, 500);
    let collapsed = cx.update(|w, _| part(w, "description").bounds().size.height);
    let button = cx.update(|w, _| part(w, "expand").bounds().center());
    cx.simulate_click(button, Modifiers::default());
    draw(cx);
    tick(cx, 500);
    v.read_with(cx, |p, _| assert!(p.expanded));
    cx.update(|w, _| assert!(part(w, "description").bounds().size.height > collapsed));
    v.update(cx, |p, cx| {
        p.lines = Some(200);
        cx.notify();
    });
    draw(cx);
    draw(cx);
    cx.update(|w, _| {
        assert!(find(w, ElementId::from((ElementId::from("alert"), "expand"))).is_none())
    });
    v.update(cx, |p, cx| {
        p.lines = Some(2);
        p.expanded = false;
        cx.notify();
    });
    draw(cx);
    draw(cx);
    cx.update(|w, _| {
        part(w, "expand");
    });
}
#[gpui_kit::test]
fn timeout_is_one_request_per_generation_and_restart_is_explicit(cx: &mut TestAppContext) {
    let (v, cx) = open(cx);
    v.update(cx, |p, cx| {
        p.duration = Some(Duration::from_secs(1));
        p.accept = false;
        cx.notify();
    });
    draw(cx);
    tick(cx, 1100);
    draw(cx);
    v.read_with(cx, |p, _| {
        assert_eq!(p.requests, vec![AlertCloseReason::Timeout])
    });
    tick(cx, 5000);
    v.read_with(cx, |p, _| assert_eq!(p.requests.len(), 1));
    v.update(cx, |p, cx| {
        p.duration = Some(Duration::from_secs(2));
        cx.notify();
    });
    draw(cx);
    tick(cx, 1000);
    v.read_with(cx, |p, _| assert_eq!(p.requests.len(), 1));
    tick(cx, 1100);
    v.read_with(cx, |p, _| assert_eq!(p.requests.len(), 2));
    v.update(cx, |p, cx| {
        p.visible = false;
        cx.notify();
    });
    draw(cx);
    tick(cx, 1000);
    v.update(cx, |p, cx| {
        p.visible = true;
        cx.notify();
    });
    draw(cx);
    tick(cx, 300);
    tick(cx, 2100);
    v.read_with(cx, |p, _| assert_eq!(p.requests.len(), 3));
}
#[gpui_kit::test]
fn hover_and_keyboard_focus_pause_independently(cx: &mut TestAppContext) {
    let (v, cx) = open(cx);
    v.update(cx, |p, cx| {
        p.duration = Some(Duration::from_secs(2));
        cx.notify();
    });
    draw(cx);
    tick(cx, 500);
    let close = cx.update(|w, _| part(w, "close").bounds().center());
    // Focus the close button without executing it: keyboard navigation from the host.
    let outside = v.read_with(cx, |p, _| p.focus.clone());
    cx.update(|w, cx| outside.focus(w, cx));
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|w, _| assert_eq!(part(w, "close").focused(), Some(true)));
    cx.simulate_mouse_move(close, None, Modifiers::default());
    tick(cx, 3000);
    cx.simulate_mouse_move(gpui_kit::point(px(1.), px(1.)), None, Modifiers::default());
    tick(cx, 3000);
    v.read_with(cx, |p, _| assert!(p.requests.is_empty()));
    // Mouse pause remains after keyboard focus leaves.
    cx.simulate_mouse_move(close, None, Modifiers::default());
    cx.update(|w, cx| outside.focus(w, cx));
    tick(cx, 3000);
    v.read_with(cx, |p, _| assert!(p.requests.is_empty()));
    cx.simulate_mouse_move(gpui_kit::point(px(1.), px(1.)), None, Modifiers::default());
    tick(cx, 1600);
    v.read_with(cx, |p, _| {
        assert_eq!(p.requests, vec![AlertCloseReason::Timeout])
    });
}
#[gpui_kit::test]
fn hidden_window_pauses_time_and_unmount_leaves_no_timeout(cx: &mut TestAppContext) {
    let (v, cx) = open(cx);
    v.update(cx, |p, cx| {
        p.duration = Some(Duration::from_secs(2));
        cx.notify();
    });
    draw(cx);
    tick(cx, 500);
    cx.simulate_visibility_change(gpui_kit::WindowVisibility::Hidden);
    tick(cx, 5000);
    v.read_with(cx, |p, _| assert!(p.requests.is_empty()));
    cx.simulate_visibility_change(gpui_kit::WindowVisibility::Visible);
    draw(cx);
    tick(cx, 500);
    v.read_with(cx, |p, _| assert!(p.requests.is_empty()));
    v.update(cx, |p, cx| {
        p.mounted = false;
        cx.notify();
    });
    draw(cx);
    tick(cx, 5000);
    v.read_with(cx, |p, _| {
        assert!(p.requests.is_empty());
        assert_eq!(p.closed, 0);
    });
}

#[gpui_kit::test]
fn offscreen_and_inactive_window_preserve_remaining_read_time(cx: &mut TestAppContext) {
    let (v, cx) = open(cx);
    v.update(cx, |p, cx| {
        p.duration = Some(Duration::from_secs(2));
        cx.notify();
    });
    draw(cx);
    tick(cx, 500);
    v.update(cx, |p, cx| {
        p.offscreen = true;
        cx.notify();
    });
    draw(cx);
    tick(cx, 5000);
    v.read_with(cx, |p, _| assert!(p.requests.is_empty()));
    v.update(cx, |p, cx| {
        p.offscreen = false;
        cx.notify();
    });
    draw(cx);
    tick(cx, 500);
    cx.deactivate_window();
    draw(cx);
    tick(cx, 5000);
    v.read_with(cx, |p, _| assert!(p.requests.is_empty()));
    cx.update(|w, _| w.activate_window());
    draw(cx);
    tick(cx, 1100);
    v.read_with(cx, |p, _| {
        assert_eq!(p.requests, vec![AlertCloseReason::Timeout])
    });
}
#[gpui_kit::test]
fn static_and_paused_notices_stop_requesting_frames(cx: &mut TestAppContext) {
    let (v, cx) = open(cx);
    for _ in 0..4 {
        draw(cx);
    }
    assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
    v.update(cx, |p, cx| {
        p.description = true;
        p.lines = Some(2);
        cx.notify();
    });
    draw(cx);
    draw(cx);
    tick(cx, 500);
    draw(cx);
    assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
    v.update(cx, |p, cx| {
        p.duration = Some(Duration::from_secs(5));
        cx.notify();
    });
    draw(cx);
    assert!(cx.update(|w, cx| w.simulate_next_frame(cx)) > 0);
    let close = cx.update(|w, _| part(w, "close").bounds().center());
    cx.simulate_mouse_move(close, None, Modifiers::default());
    draw(cx);
    tick(cx, 500);
    draw(cx);
    assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
}
#[gpui_kit::test]
fn width_remeasurement_removes_unnecessary_fold_control(cx: &mut TestAppContext) {
    let (v, cx) = open(cx);
    v.update(cx, |p, cx| {
        p.description = true;
        p.brief = true;
        p.width = 200.;
        p.lines = Some(2);
        cx.notify();
    });
    draw(cx);
    draw(cx);
    tick(cx, 500);
    cx.update(|w, _| {
        part(w, "expand");
    });
    let focus = v.read_with(cx, |p, _| p.focus.clone());
    cx.update(|w, cx| focus.focus(w, cx));
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|w, _| assert_eq!(part(w, "expand").focused(), Some(true)));
    v.update(cx, |p, cx| {
        p.width = 1000.;
        cx.notify();
    });
    draw(cx);
    draw(cx);
    tick(cx, 500);
    cx.update(|w, _| {
        assert!(find(w, ElementId::from((ElementId::from("alert"), "expand"))).is_none());
        assert!(snapshots(w).iter().any(|n| n.focused() == Some(true)));
    });
}
#[gpui_kit::test]
fn initial_hidden_and_direct_unmount_do_not_report_closed(cx: &mut TestAppContext) {
    let (v, cx) = open(cx);
    v.update(cx, |p, cx| {
        p.mounted = false;
        cx.notify();
    });
    draw(cx);
    tick(cx, 500);
    v.update(cx, |p, cx| {
        p.visible = false;
        p.mounted = true;
        cx.notify();
    });
    draw(cx);
    tick(cx, 500);
    v.read_with(cx, |p, _| assert_eq!(p.closed, 0));
}

#[gpui_kit::test]
fn reduced_motion_closes_at_final_geometry_without_an_animation_loop(cx: &mut TestAppContext) {
    let (v, cx) = open(cx);
    cx.update(|_, cx| cx.set_reduce_motion(true));
    v.update(cx, |p, cx| {
        p.visible = false;
        cx.notify();
    });
    draw(cx);
    draw(cx);
    draw(cx);
    v.read_with(cx, |p, _| assert_eq!(p.closed, 1));
    cx.update(|w, _| assert!(find(w, "alert".into()).is_none()));
    assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
}

#[gpui_kit::test]
fn restarting_duration_keeps_an_existing_hover_pause(cx: &mut TestAppContext) {
    let (v, cx) = open(cx);
    v.update(cx, |p, cx| {
        p.duration = Some(Duration::from_secs(2));
        cx.notify();
    });
    draw(cx);
    let close = cx.update(|w, _| part(w, "close").bounds().center());
    cx.simulate_mouse_move(close, None, Modifiers::default());
    tick(cx, 500);
    v.update(cx, |p, cx| {
        p.duration = Some(Duration::from_secs(1));
        cx.notify();
    });
    draw(cx);
    tick(cx, 5000);
    v.read_with(cx, |p, _| assert!(p.requests.is_empty()));
    cx.simulate_mouse_move(gpui_kit::point(px(1.), px(1.)), None, Modifiers::default());
    tick(cx, 1100);
    v.read_with(cx, |p, _| {
        assert_eq!(p.requests, vec![AlertCloseReason::Timeout])
    });
}
