use super::*;
use gpui_kit::base::{Root, test_support::snapshots};
use gpui_kit::{Bounds, Modifiers, Pixels, TestAppContext, VisualTestContext};

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| {
        w.refresh();
        w.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}
fn open(cx: &mut TestAppContext) -> (Entity<VirtualListPage>, &mut VisualTestContext) {
    cx.update(|cx| {
        yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
        cx.set_reduce_motion(true);
    });
    let mut page = None;
    let (_, cx) = cx.add_window_view(|w, cx| {
        let view = cx.new(|cx| VirtualListPage::new(w, cx));
        page = Some(view.clone());
        Root::new(view, w, cx)
    });
    cx.simulate_resize(gpui_kit::size(px(1_400.), px(1_000.)));
    draw(cx);
    (page.unwrap(), cx)
}
fn bounds(cx: &mut VisualTestContext, label: &str) -> Bounds<Pixels> {
    cx.update(|w, _| {
        let matches = snapshots(w)
            .into_iter()
            .filter(|n| n.label() == Some(label))
            .collect::<Vec<_>>();
        // The tab and its active list share the scene's accessible name.
        // Select the semantic tab for navigation, not a coincident list target.
        if let Some(tab) = matches.iter().find(|n| n.role() == Some(Role::Tab)) {
            return tab.bounds();
        }
        assert_eq!(matches.len(), 1, "unique showcase target: {label}");
        matches[0].bounds()
    })
}
fn click(cx: &mut VisualTestContext, label: &str) {
    let point = bounds(cx, label).center();
    cx.simulate_click(point, Modifiers::default());
    draw(cx);
}
fn select(page: &Entity<VirtualListPage>, cx: &mut VisualTestContext, demo: Demo) {
    click(cx, demo.label());
    assert_eq!(page.read_with(cx, |p, _| p.active), demo);
}
fn state(
    page: &Entity<VirtualListPage>,
    cx: &mut VisualTestContext,
    demo: Demo,
) -> Entity<VirtualizerState> {
    page.read_with(cx, |p, _| p.states[demo.ix()].clone())
}
fn snapshot(
    state: &Entity<VirtualizerState>,
    cx: &mut VisualTestContext,
) -> yue_component::VirtualizerSnapshot {
    state.read_with(cx, |s, _| s.snapshot(s.viewport().unwrap()).unwrap())
}
fn wheel(cx: &mut VisualTestContext, demo: Demo, x: f32, y: f32) {
    let position = viewport(cx, demo).center();
    cx.simulate_event(gpui_kit::ScrollWheelEvent {
        position,
        delta: gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(x), px(y))),
        modifiers: Default::default(),
        touch_phase: gpui_kit::TouchPhase::Moved,
    });
    draw(cx);
}
fn viewport(cx: &mut VisualTestContext, demo: Demo) -> Bounds<Pixels> {
    cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .find(|n| n.path().last() == Some(&ElementId::from(("virtual-viewport", demo.ix()))))
            .expect("demo viewport")
            .bounds()
    })
}
fn start_drag(cx: &mut VisualTestContext, demo: Demo) -> Bounds<Pixels> {
    // A native drag requires an active window; deactivation now cancels it.
    cx.update(|w, _| w.activate_window());
    // The production Scrolling mode reveals its thumb after actual scroll activity.
    wheel(cx, demo, 0., -36.);
    wheel(cx, demo, 0., 36.);
    let area = viewport(cx, demo);
    let start = gpui_kit::point(area.right() - px(9.), area.top() + px(10.));
    cx.simulate_mouse_move(start, None, Modifiers::default());
    draw(cx);
    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    draw(cx);
    area
}
fn drag_step(cx: &mut VisualTestContext, area: Bounds<Pixels>, fraction: f32, held: bool) {
    let at = gpui_kit::point(
        area.right() - px(9.),
        area.top() + px(10.) + (area.size.height - px(20.)) * fraction,
    );
    if held {
        cx.simulate_mouse_move(at, Some(MouseButton::Left), Modifiers::default());
    } else {
        cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::default());
    }
    draw(cx);
}

#[gpui_kit::test]
fn original_modes_prepend_expand_and_million_item_end_still_work(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    let fixed = state(&page, cx, Demo::Fixed);
    assert!(page.read_with(cx, |p, _| p.builds[0].get()) < 100);
    click(cx, "读取统计");
    assert!(page.read_with(cx, |p, _| p.report.contains("renderer")));
    click(cx, "定位末尾");
    assert!(snapshot(&fixed, cx).visible_range().contains(&999_999));
    let old = snapshot(&fixed, cx);
    let anchor = old.item(old.visible_range().start).unwrap().key().clone();
    click(cx, "头部插入一项");
    let new = snapshot(&fixed, cx);
    assert_eq!(page.read_with(cx, |p, _| p.count), 1_000_001);
    assert!(
        new.visible_range()
            .contains(&new.item_for_key(&anchor).unwrap().index())
    );
    select(&page, cx, Demo::Known);
    click(cx, "定位末尾");
    assert!(
        snapshot(&state(&page, cx, Demo::Known), cx)
            .visible_range()
            .contains(&9_999)
    );
    select(&page, cx, Demo::Measured);
    let measured = state(&page, cx, Demo::Measured);
    let height = snapshot(&measured, cx).item(0).unwrap().extent();
    click(cx, "展开／收起首张卡片");
    assert!(page.read_with(cx, |p, _| p.expanded.get()));
    assert!(snapshot(&measured, cx).item(0).unwrap().extent() > height);
    click(cx, "读取统计");
    assert!(page.read_with(cx, |p, _| p.report.contains("估算")));
}

#[gpui_kit::test]
fn eight_scenarios_mount_only_the_active_list_and_preserve_offsets(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    assert!(page.read_with(cx, |p, _| p.builds.iter().skip(1).all(|b| b.get() == 0)));
    click(cx, "定位末尾");
    let fixed = state(&page, cx, Demo::Fixed);
    let end = snapshot(&fixed, cx).viewport().offset();
    for demo in Demo::ALL.into_iter().skip(1) {
        select(&page, cx, demo);
        assert!(state(&page, cx, demo).read_with(cx, |s, _| s.viewport().is_some()));
        // Pointer-down can redraw the old panel before the tab's click commits.
        // Once the switch has settled, subsequent frames must not build it.
        let before = page.read_with(cx, |p, _| p.builds.each_ref().map(|b| b.get()));
        draw(cx);
        let after = page.read_with(cx, |p, _| p.builds.each_ref().map(|b| b.get()));
        for other in Demo::ALL {
            if other != demo {
                assert_eq!(before[other.ix()], after[other.ix()], "inactive {other:?}");
            }
        }
        assert!(snapshot(&state(&page, cx, demo), cx).rendered_range().len() < 100);
    }
    select(&page, cx, Demo::Fixed);
    assert_eq!(snapshot(&fixed, cx).viewport().offset(), end);
    assert!(snapshot(&fixed, cx).visible_range().contains(&999_999));
}

#[gpui_kit::test]
fn horizontal_timeline_uses_horizontal_input_and_reaches_both_ends(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    select(&page, cx, Demo::Horizontal);
    let horizontal = state(&page, cx, Demo::Horizontal);
    assert_eq!(horizontal.read_with(cx, |s, _| s.axis()), Axis::Horizontal);
    wheel(cx, Demo::Horizontal, -300., 0.);
    assert!(snapshot(&horizontal, cx).viewport().offset() > 0.);
    click(cx, "定位末尾");
    assert!(snapshot(&horizontal, cx).visible_range().contains(&499));
    click(cx, "回到开头");
    assert_eq!(snapshot(&horizontal, cx).visible_range().start, 0);
    assert!(page.read_with(cx, |p, _| p.builds[Demo::Horizontal.ix()].get()) < 500);
}

#[gpui_kit::test]
fn card_selection_survives_virtualization_and_scenario_changes(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    select(&page, cx, Demo::Cards);
    click(cx, "选择项目 2");
    assert_eq!(page.read_with(cx, |p, _| p.selected_card.get()), Some(2));
    click(cx, "定位末尾");
    assert!(
        snapshot(&state(&page, cx, Demo::Cards), cx)
            .visible_range()
            .contains(&2_999)
    );
    assert!(bounds(cx, "选择项目 9000").size.width > px(0.));
    select(&page, cx, Demo::Known);
    select(&page, cx, Demo::Cards);
    click(cx, "回到开头");
    assert_eq!(page.read_with(cx, |p, _| p.selected_card.get()), Some(2));
    assert!(bounds(cx, "选择项目 2").size.width > px(0.));
}

#[gpui_kit::test]
fn real_editor_preserves_value_and_focus_without_creating_one_editor_per_record(
    cx: &mut TestAppContext,
) {
    let (page, cx) = open(cx);
    select(&page, cx, Demo::Editor);
    click(cx, "首条记录备注");
    cx.simulate_input("保留的备注");
    draw(cx);
    let input = page.read_with(cx, |p, _| p.editor.clone());
    wheel(cx, Demo::Editor, 0., -2_000.);
    assert!(bounds(cx, "首条记录备注").bottom() < px(0.));
    assert!(cx.update(|w, cx| input.read(cx).focus_handle(cx).is_focused(w)));
    click(cx, "回到开头");
    select(&page, cx, Demo::Horizontal);
    select(&page, cx, Demo::Editor);
    assert_eq!(
        input.read_with(cx, |i, _| i.value().to_string()),
        "保留的备注"
    );
    assert!(bounds(cx, "首条记录备注").size.width > px(0.));
    assert!(
        snapshot(&state(&page, cx, Demo::Editor), cx)
            .rendered_range()
            .len()
            < 100
    );
}

#[gpui_kit::test]
fn logs_follow_append_and_preserve_history_when_paused(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    select(&page, cx, Demo::Logs);
    let logs = state(&page, cx, Demo::Logs);
    click(cx, "跟随最新");
    click(cx, "追加二十条");
    assert_eq!(page.read_with(cx, |p, _| p.log_count), 220);
    assert!(snapshot(&logs, cx).visible_range().contains(&219));
    assert!(logs.read_with(cx, |s, _| s.is_following_tail()));
    wheel(cx, Demo::Logs, 0., 500.);
    assert!(!logs.read_with(cx, |s, _| s.is_following_tail()));
    click(cx, "暂停跟随");
    click(cx, "回到开头");
    let before = snapshot(&logs, cx).viewport().offset();
    click(cx, "追加二十条");
    assert_eq!(snapshot(&logs, cx).viewport().offset(), before);
    assert!(!snapshot(&logs, cx).visible_range().contains(&239));
    click(cx, "跟随最新");
    assert!(snapshot(&logs, cx).visible_range().contains(&239));
}

#[gpui_kit::test]
fn empty_list_builds_nothing_and_recovers_after_clear_and_refill(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    select(&page, cx, Demo::Empty);
    let empty = state(&page, cx, Demo::Empty);
    assert_eq!(
        page.read_with(cx, |p, _| p.builds[Demo::Empty.ix()].get()),
        0
    );
    assert!(snapshot(&empty, cx).is_empty());
    click(cx, "加载示例数据");
    click(cx, "定位末尾");
    assert!(snapshot(&empty, cx).visible_range().contains(&999));
    click(cx, "清空示例");
    assert!(snapshot(&empty, cx).is_empty());
    let before = page.read_with(cx, |p, _| p.builds[Demo::Empty.ix()].get());
    draw(cx);
    assert_eq!(
        page.read_with(cx, |p, _| p.builds[Demo::Empty.ix()].get()),
        before
    );
    click(cx, "加载示例数据");
    assert_eq!(snapshot(&empty, cx).visible_range().start, 0);
}

#[gpui_kit::test]
fn scenario_tabs_keep_keyboard_navigation_in_a_narrow_dark_window(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    cx.update(|_, cx| yue_component::set_theme(cx, YueTheme::new(ThemeMode::Dark, Accent::Azure)));
    cx.simulate_resize(gpui_kit::size(px(740.), px(1_000.)));
    cx.update(|w, _| w.activate_window());
    draw(cx);
    click(cx, Demo::Fixed.label());
    cx.simulate_keystrokes("end");
    draw(cx);
    assert_eq!(page.read_with(cx, |p, _| p.active), Demo::Empty);
    click(cx, "加载示例数据");
    assert_eq!(snapshot(&state(&page, cx, Demo::Empty), cx).len(), 1_000);
    click(cx, Demo::Empty.label());
    cx.simulate_keystrokes("home");
    draw(cx);
    assert_eq!(page.read_with(cx, |p, _| p.active), Demo::Fixed);
}

#[gpui_kit::test]
fn rapid_native_scrollbar_drag_keeps_work_bounded_and_other_scenarios_idle(
    cx: &mut TestAppContext,
) {
    let (page, cx) = open(cx);
    for demo in [Demo::Fixed, Demo::Known, Demo::Measured] {
        select(&page, cx, demo);
        click(cx, "回到开头");
        let current = state(&page, cx, demo);
        let area = start_drag(cx, demo);
        assert!(
            snapshot(&current, cx).viewport().offset() < 1.,
            "press must grab the initial thumb"
        );
        let inactive = page.read_with(cx, |p, _| p.builds.each_ref().map(|b| b.get()));
        let mut furthest = 0;
        for i in 0..24 {
            let before = page.read_with(cx, |p, _| p.builds[demo.ix()].get());
            drag_step(cx, area, (i * 17 % 23) as f32 / 22., true);
            let batch_builds = page.read_with(cx, |p, _| p.builds[demo.ix()].get()) - before;
            let snap = snapshot(&current, cx);
            furthest = furthest.max(snap.visible_range().start);
            assert!(
                batch_builds < 150,
                "{demo:?} batch built {batch_builds} roots"
            );
            assert!(snap.rendered_range().len() < 100);
        }
        drag_step(cx, area, 0.5, false);
        assert!(
            furthest > page.read_with(cx, |p, _| p.len(demo)) / 2,
            "{demo:?} thumb must actually move far, got {furthest}"
        );
        for other in Demo::ALL {
            if other != demo {
                assert_eq!(
                    page.read_with(cx, |p, _| p.builds[other.ix()].get()),
                    inactive[other.ix()]
                );
            }
        }
    }
}

/// CPU-only input/submission batches on the real Showcase page. Not physical frames.
/// Run explicitly in release; timings have no pass/fail threshold.
#[gpui_kit::test]
#[ignore = "manual release CPU probe; emits raw SHOWCASE_DRAG CSV rows"]
fn showcase_scrollbar_drag_cpu_probe(cx: &mut TestAppContext) {
    let (page, cx) = open(cx);
    for (demo, n) in [
        (Demo::Fixed, 1_000),
        (Demo::Fixed, 100_000),
        (Demo::Fixed, 1_000_000),
        (Demo::Known, 10_000),
        (Demo::Measured, 10_000),
    ] {
        select(&page, cx, demo);
        if demo == Demo::Fixed {
            page.update(cx, |p, cx| {
                p.count = n;
                p.states[demo.ix()].update(cx, |s, cx| {
                    s.replace_source(
                        Rc::new(Records {
                            first: 1_000,
                            count: n,
                        }),
                        sizing(demo, n),
                        VirtualizerChange::Replace {
                            preserve_anchor: false,
                        },
                    )
                    .unwrap();
                    cx.notify();
                });
                cx.notify();
            });
            draw(cx);
        }
        click(cx, "回到开头");
        let current = state(&page, cx, demo);
        let area = start_drag(cx, demo);
        assert!(snapshot(&current, cx).viewport().offset() < 1.);
        let mut furthest = 0;
        for sample in 0..180 {
            let before = page.read_with(cx, |p, _| p.builds[demo.ix()].get());
            let started = std::time::Instant::now();
            drag_step(cx, area, (sample * 17 % 101) as f32 / 100., true);
            let elapsed = started.elapsed().as_nanos();
            let built = page.read_with(cx, |p, _| p.builds[demo.ix()].get()) - before;
            let snap = snapshot(&current, cx);
            furthest = furthest.max(snap.visible_range().start);
            assert!(built < 150);
            assert!(snap.rendered_range().len() < 100);
            println!(
                "SHOWCASE_DRAG,{},{},{},{},{},{},{}",
                demo.key(),
                n,
                sample,
                elapsed,
                built,
                snap.rendered_range().len(),
                snap.visible_range().start
            );
        }
        drag_step(cx, area, 0.5, false);
        assert!(furthest > n / 2);
    }
}
