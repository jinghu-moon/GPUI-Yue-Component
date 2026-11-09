use super::*;
use gpui_kit::AppContext as _;
use gpui_kit::base::test_support::{ElementSnapshot, snapshots};
use gpui_kit::{
    Bounds, Context, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels,
    Render, TestAppContext, VisualTestContext, point, px, rems,
};

struct Fixture {
    state: Entity<YueResizableState>,
    width: Pixels,
    vertical: bool,
    disabled: bool,
    commits: usize,
    reverse: bool,
    theme: YueTheme,
    mounted: bool,
    count: usize,
}
impl Render for Fixture {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let panel = |key: &'static str,
                     label: &'static str,
                     initial: Option<f32>,
                     min: f32,
                     max: f32,
                     fixed: bool| {
            let p = YueResizablePanel::new(key)
                .label(label)
                .min_size(px(min))
                .max_size(px(max))
                .fixed(fixed)
                .child(div().debug_selector(move || label.into()).size_full());
            if let Some(initial) = initial {
                p.initial_size(px(initial))
            } else {
                p
            }
        };
        let mut panels = vec![
            panel("nav", "Navigation", Some(240.), 120., 400., true),
            panel("main", "Document", None, 180., 600., false),
            panel("aside", "Properties", Some(160.), 100., 300., false),
        ];
        if self.reverse {
            panels.reverse();
        }
        panels.truncate(self.count);
        let group = if self.vertical {
            YueResizable::vertical("layout", &self.state)
        } else {
            YueResizable::horizontal("layout", &self.state)
        };
        div()
            .w(if self.vertical { px(240.) } else { self.width })
            .h(if self.vertical { self.width } else { px(160.) })
            .when(self.mounted, |d| {
                d.child(
                    group
                        .theme(self.theme)
                        .disabled(self.disabled)
                        .children(panels)
                        .on_resize(cx.listener(|s, _, _, cx| {
                            s.commits += 1;
                            cx.notify();
                        })),
                )
            })
    }
}
fn open(cx: &mut TestAppContext, vertical: bool) -> (Entity<Fixture>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init(
            cx,
            YueTheme::new(crate::ThemeMode::Light, crate::Accent::Azure),
        )
        .unwrap();
        cx.set_reduce_motion(true);
    });
    let mut saved = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        window.activate_window();
        let view = cx.new(|cx| Fixture {
            state: cx.new(|_| YueResizableState::default()),
            width: px(840.),
            vertical,
            disabled: false,
            commits: 0,
            reverse: false,
            theme: crate::current_theme(cx),
            mounted: true,
            count: 3,
        });
        saved = Some(view.clone());
        gpui_kit::base::Root::new(view, window, cx)
    });
    settle(cx);
    (saved.unwrap(), cx)
}
fn settle(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.update(|w, cx| {
            w.draw(cx).clear(cx);
        });
        cx.run_until_parked();
    }
}
fn handles(cx: &mut VisualTestContext) -> Vec<ElementSnapshot> {
    let mut result = cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .filter(|s| s.role() == Some(gpui_kit::Role::Splitter))
            .collect::<Vec<_>>()
    });
    result.sort_by(|a, b| {
        (a.bounds().origin.x + a.bounds().origin.y)
            .partial_cmp(&(b.bounds().origin.x + b.bounds().origin.y))
            .unwrap()
    });
    result
}
fn bounds(cx: &mut VisualTestContext) -> [Bounds<Pixels>; 3] {
    ["Navigation", "Document", "Properties"].map(|label| cx.debug_bounds(label).expect(label))
}
fn widths(cx: &mut VisualTestContext, vertical: bool) -> [Pixels; 3] {
    bounds(cx).map(|b| {
        if vertical {
            b.size.height
        } else {
            b.size.width
        }
    })
}
fn state(view: &Entity<Fixture>, cx: &VisualTestContext) -> Entity<YueResizableState> {
    view.read_with(cx, |v, _| v.state.clone())
}
fn key(cx: &mut VisualTestContext, stroke: &str) {
    cx.simulate_keystrokes(stroke);
    settle(cx);
}
fn enter_tab_order(cx: &mut VisualTestContext) {
    // Initial window focus is the host's responsibility. Enter the rendered
    // Tab order, then exercise real key events between the splitters.
    cx.update(|window, cx| window.focus_next(cx));
    settle(cx);
}
fn press(cx: &mut VisualTestContext, at: gpui_kit::Point<Pixels>, count: usize) {
    cx.simulate_event(MouseDownEvent {
        position: at,
        button: MouseButton::Left,
        modifiers: Modifiers::default(),
        click_count: count,
        first_mouse: false,
    });
    settle(cx);
}
fn release(cx: &mut VisualTestContext, at: gpui_kit::Point<Pixels>, count: usize) {
    cx.simulate_event(MouseUpEvent {
        position: at,
        button: MouseButton::Left,
        modifiers: Modifiers::default(),
        click_count: count,
    });
    settle(cx);
}
fn move_drag(cx: &mut VisualTestContext, at: gpui_kit::Point<Pixels>) {
    cx.simulate_event(MouseMoveEvent {
        position: at,
        pressed_button: Some(MouseButton::Left),
        modifiers: Modifiers::default(),
    });
    settle(cx);
}

#[gpui_kit::test]
fn initial_sizes_fill_the_flexible_panel_without_moving_specified_sizes(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, false);
    assert_eq!(widths(cx, false), [px(240.), px(440.), px(160.)]);
    assert_eq!(view.read_with(cx, |v, _| v.commits), 0);
    let h = handles(cx);
    assert_eq!(h.len(), 2);
    assert_eq!(h[0].orientation(), Some(gpui_kit::Orientation::Vertical));
    assert_eq!(h[0].bounds().size.width, px(9.));
    assert_eq!(h[0].numeric_value(), Some(240.));
    assert_eq!(h[0].min_numeric_value(), Some(120.));
    assert_eq!(h[0].max_numeric_value(), Some(400.));
}

#[gpui_kit::test]
fn keyboard_respects_both_panels_ranges_and_commits_once(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, false);
    enter_tab_order(cx);
    assert_eq!(handles(cx)[0].focused(), Some(true));
    key(cx, "right");
    assert_eq!(widths(cx, false), [px(248.), px(432.), px(160.)]);
    key(cx, "shift-right");
    assert_eq!(widths(cx, false), [px(280.), px(400.), px(160.)]);
    key(cx, "end");
    assert_eq!(widths(cx, false), [px(400.), px(280.), px(160.)]);
    let count = view.read_with(cx, |v, _| v.commits);
    key(cx, "right");
    assert_eq!(
        view.read_with(cx, |v, _| v.commits),
        count,
        "at limit is not a new commit"
    );
    key(cx, "home");
    assert_eq!(widths(cx, false), [px(120.), px(560.), px(160.)]);
    key(cx, "enter");
    assert_eq!(widths(cx, false), [px(240.), px(440.), px(160.)]);
    key(cx, "tab");
    key(cx, "home");
    assert_eq!(
        widths(cx, false),
        [px(240.), px(300.), px(300.)],
        "right maximum bounds middle minimum"
    );
    assert_eq!(handles(cx)[1].focused(), Some(true));
    key(cx, "up");
    assert_eq!(
        widths(cx, false),
        [px(240.), px(300.), px(300.)],
        "wrong-axis arrows propagate"
    );
}

#[gpui_kit::test]
fn pointer_drag_and_double_click_share_constraints_and_reset(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, false);
    let at = handles(cx)[0].bounds().center();
    cx.simulate_mouse_move(at, None, Modifiers::default());
    settle(cx);
    press(cx, at, 1);
    move_drag(cx, at + point(px(15.), px(0.)));
    move_drag(cx, at + point(px(80.), px(0.)));
    assert_eq!(
        widths(cx, false)[0],
        px(320.),
        "retain the pointer's original grab offset"
    );
    assert_eq!(widths(cx, false)[2], px(160.));
    release(cx, at + point(px(85.), px(0.)), 1);
    assert_eq!(
        widths(cx, false)[0],
        px(325.),
        "mouseup flushes its latest coordinate"
    );
    assert_eq!(view.read_with(cx, |v, _| v.commits), 1);
    let reset_at = handles(cx)[0].bounds().center();
    cx.simulate_mouse_move(reset_at, None, Modifiers::default());
    settle(cx);
    press(cx, reset_at, 2);
    release(cx, reset_at, 2);
    assert_eq!(widths(cx, false), [px(240.), px(440.), px(160.)]);
    assert_eq!(view.read_with(cx, |v, _| v.commits), 2);
    assert!(!state(&view, cx).read_with(cx, |s, _| s.is_resizing()));
}

#[gpui_kit::test]
fn vertical_split_uses_up_down_and_horizontal_separator_semantics(cx: &mut TestAppContext) {
    let (_, cx) = open(cx, true);
    assert_eq!(widths(cx, true), [px(240.), px(440.), px(160.)]);
    assert_eq!(
        handles(cx)[0].orientation(),
        Some(gpui_kit::Orientation::Horizontal)
    );
    enter_tab_order(cx);
    key(cx, "down");
    assert_eq!(widths(cx, true), [px(248.), px(432.), px(160.)]);
    key(cx, "shift-up");
    assert_eq!(widths(cx, true), [px(216.), px(464.), px(160.)]);
}

#[gpui_kit::test]
fn fixed_panel_and_bounded_flexible_sizes_survive_container_changes(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, false);
    for width in [1140., 500., 300., 840.] {
        view.update(cx, |v, cx| {
            v.width = px(width);
            cx.notify();
        });
        settle(cx);
        let sizes = widths(cx, false);
        assert_eq!(sizes[0], px(240.));
        assert!((px(180.)..=px(600.)).contains(&sizes[1]));
        assert!((px(100.)..=px(300.)).contains(&sizes[2]));
        if width >= 520. {
            assert!((sizes.iter().map(|v| v.as_f32()).sum::<f32>() - width).abs() < 0.1);
        }
    }
    assert_eq!(view.read_with(cx, |v, _| v.commits), 0);
}

#[gpui_kit::test]
fn snapshot_restore_reset_and_reorder_preserve_one_state_owner(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, false);
    let s = state(&view, cx);
    s.update(cx, |s, cx| {
        assert!(s.set_sizes(&[px(300.), px(300.), px(240.)], cx))
    });
    settle(cx);
    assert_eq!(widths(cx, false), [px(300.), px(300.), px(240.)]);
    s.update(cx, |s, cx| {
        assert!(!s.set_sizes(&[px(f32::NAN), px(1.), px(2.)], cx));
        assert!(!s.set_sizes(&[px(3.)], cx));
    });
    view.update(cx, |v, cx| {
        v.reverse = true;
        cx.notify();
    });
    settle(cx);
    assert_eq!(widths(cx, false), [px(300.), px(300.), px(240.)]);
    s.update(cx, |s, cx| s.reset_layout(cx));
    settle(cx);
    assert_eq!(widths(cx, false), [px(240.), px(440.), px(160.)]);
    assert_eq!(
        view.read_with(cx, |v, _| v.commits),
        0,
        "programmatic edits do not echo user callback"
    );
}

#[gpui_kit::test]
fn disabled_splitters_ignore_pointer_and_leave_tab_order(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, false);
    view.update(cx, |v, cx| {
        v.disabled = true;
        cx.notify();
    });
    settle(cx);
    let h = handles(cx);
    assert!(h.iter().all(|h| h.disabled() == Some(true)));
    key(cx, "tab");
    key(cx, "right");
    assert!(handles(cx).iter().all(|h| h.focused() != Some(true)));
    let at = h[0].bounds().center();
    cx.simulate_mouse_move(at, None, Modifiers::default());
    settle(cx);
    press(cx, at, 1);
    move_drag(cx, at + point(px(60.), px(0.)));
    release(cx, at, 1);
    assert_eq!(widths(cx, false), [px(240.), px(440.), px(160.)]);
    assert_eq!(view.read_with(cx, |v, _| v.commits), 0);
}

#[gpui_kit::test]
fn rem_resolves_hit_area_and_keyboard_steps_in_the_current_window(cx: &mut TestAppContext) {
    let (_, cx) = open(cx, false);
    cx.update(|w, _| {
        w.set_rem_size(px(24.));
        w.refresh();
    });
    settle(cx);
    assert_eq!(handles(cx)[0].bounds().size.width, px(13.5));
    enter_tab_order(cx);
    key(cx, "right");
    assert_eq!(widths(cx, false)[0], px(252.));
    assert_eq!(rems(1.).to_pixels(px(24.)), px(24.));
}

#[gpui_kit::test]
fn unmount_disable_and_deactivation_cancel_without_synthetic_commits(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, false);
    let s = state(&view, cx);
    for cancellation in 0..3 {
        cx.update(|w, _| w.activate_window());
        view.update(cx, |v, cx| {
            v.mounted = true;
            v.disabled = false;
            cx.notify();
        });
        settle(cx);
        let at = handles(cx)[0].bounds().center();
        cx.simulate_mouse_move(at, None, Modifiers::default());
        settle(cx);
        press(cx, at, 1);
        move_drag(cx, at + point(px(15.), px(0.)));
        move_drag(cx, at + point(px(25.), px(0.)));
        assert!(s.read_with(cx, |s, _| s.is_resizing()));
        let before = s.read_with(cx, |s, _| s.sizes().clone());
        match cancellation {
            0 => view.update(cx, |v, cx| {
                v.mounted = false;
                cx.notify();
            }),
            1 => view.update(cx, |v, cx| {
                v.disabled = true;
                cx.notify();
            }),
            _ => cx.deactivate_window(),
        }
        settle(cx);
        assert!(
            !s.read_with(cx, |s, _| s.is_resizing()),
            "cancellation {cancellation}"
        );
        release(cx, at + point(px(90.), px(0.)), 1);
        assert_eq!(s.read_with(cx, |s, _| s.sizes().clone()), before);
        assert_eq!(view.read_with(cx, |v, _| v.commits), 0);
    }
}

#[gpui_kit::test]
fn empty_and_single_panel_have_no_splitters_and_no_idle_frames(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, false);
    for count in [0, 1, 3, 0] {
        view.update(cx, |v, cx| {
            v.count = count;
            cx.notify();
        });
        settle(cx);
        assert_eq!(handles(cx).len(), count.saturating_sub(1));
        let callbacks = cx.update(|w, cx| w.simulate_next_frame(cx));
        assert_eq!(callbacks, 0, "settled layout does not run a frame loop");
    }
}

#[gpui_kit::test]
fn settled_minimum_size_does_not_notify_on_unrelated_redraws(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, false);
    let s = state(&view, cx);
    s.update(cx, |s, cx| {
        assert!(s.set_sizes(&[px(240.), px(500.), px(100.)], cx));
    });
    settle(cx);
    assert_eq!(widths(cx, false), [px(240.), px(500.), px(100.)]);
    let notifications = Rc::new(std::cell::Cell::new(0));
    let observed = notifications.clone();
    let _subscription =
        cx.update(|_, cx| cx.observe(&s, move |_, _| observed.set(observed.get() + 1)));
    for _ in 0..3 {
        cx.update(|w, cx| {
            w.refresh();
            w.draw(cx).clear(cx);
        });
        cx.run_until_parked();
    }
    assert_eq!(notifications.get(), 0, "100px is a valid retained size");
    assert_eq!(widths(cx, false), [px(240.), px(500.), px(100.)]);
}

#[gpui_kit::test]
fn local_theme_repaints_lines_and_keeps_sizes_and_focus(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, false);
    enter_tab_order(cx);
    let dimensions = widths(cx, false);
    for mode in [crate::ThemeMode::Light, crate::ThemeMode::Dark] {
        for accent in [crate::Accent::Azure, crate::Accent::Neutral] {
            let theme = YueTheme::new(mode, accent);
            view.update(cx, |v, cx| {
                v.theme = theme;
                cx.notify();
            });
            settle(cx);
            assert_eq!(widths(cx, false), dimensions);
            assert_eq!(handles(cx)[0].focused(), Some(true));
            let expected: gpui_kit::Hsla = rgba(theme, "--resizable-outline-color").into();
            let centre = handles(cx)[0].bounds().center().x.as_f32();
            assert!(
                cx.update(|w, _| {
                    let scale = w.scale_factor();
                    w.painted_quads().iter().any(|q| {
                        q.background.as_solid() == Some(expected)
                            && (q.bounds.size.width.0 / scale - 2.).abs() < 0.01
                            && ((q.bounds.origin.x.0 + q.bounds.size.width.0 / 2.) / scale - centre)
                                .abs()
                                < 0.01
                    })
                }),
                "focused line uses the local theme and centres on its hit band"
            );
        }
    }
}
