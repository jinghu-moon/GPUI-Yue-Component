//! Opt-in native rendering contract. This uses the real Windows platform,
//! dispatcher, font shaping and renderer, not TestAppContext or a fake window.
use super::*;
use std::{cell::RefCell, time::Duration};

#[test]
#[ignore = "requires an unlocked Windows desktop and GPU; opens a temporary showcase window"]
fn native_layouts_render_and_settle() {
    let result = Rc::new(RefCell::new(None));
    let observed = result.clone();
    gpui_kit::application().run(move |cx| {
        yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
        cx.set_reduce_motion(true);
        let (window, page) = gpui_kit::open_window(
            WindowOptions {
                focus: false,
                window_bounds: Some(gpui_kit::WindowBounds::Windowed(gpui_kit::Bounds::new(
                    gpui_kit::point(px(40.), px(40.)),
                    gpui_kit::size(px(1_200.), px(800.)),
                ))),
                ..Default::default()
            },
            cx,
            |_, cx| cx.new(VirtualLayoutsPage::new),
        )
        .unwrap();
        cx.spawn(async move |cx| {
            let checked: Result<(), String> = async {
                let mut masonry_before_reflow = 0;
                // Wait for actual native frames, with a bounded deadline. A
                // layout that never progresses fails rather than hanging CI.
                for phase in 0..6 {
                    let mut ready = false;
                    for _ in 0..100 {
                        cx.background_executor()
                            .timer(Duration::from_millis(20))
                            .await;
                        ready = cx.update(|cx| {
                            let p = page.read(cx);
                            match phase {
                                0 => p.rows.read(cx).viewport().is_some(),
                                1 => {
                                    let rows = p.rows.read(cx);
                                    let columns = p.columns.read(cx);
                                    rows.viewport().is_some_and(|v| {
                                        rows.snapshot(v).unwrap().visible_range().contains(&999_999)
                                    }) && columns.viewport().is_some_and(|v| {
                                        columns.snapshot(v).unwrap().visible_range().contains(&999)
                                    }) && !rows.has_scroll_command()
                                        && !columns.has_scroll_command()
                                }
                                2 => p.builds[1].get() > 0,
                                3 => {
                                    p.masonry.read(cx).lane_count() == 5
                                        && p.builds[1].get() > masonry_before_reflow
                                }
                                4 => p.horizontal.read(cx).viewport().is_some(),
                                _ => {
                                    let s = p.horizontal.read(cx);
                                    s.viewport().is_some_and(|v| {
                                        s.snapshot(v).unwrap().visible_range().contains(&1_999)
                                    }) && !s.has_scroll_command()
                                }
                            }
                        });
                        if ready {
                            break;
                        }
                    }
                    if !ready {
                        return Err(format!(
                            "native phase {phase} did not settle within 2 seconds"
                        ));
                    }
                    cx.update(|cx| {
                        page.update(cx, |p, cx| {
                            match phase {
                                0 => {
                                    for (s, key) in
                                        [(&p.rows, 999_999usize), (&p.columns, 999usize)]
                                    {
                                        s.update(cx, |s, cx| {
                                            s.scroll_to_key(
                                                key.into(),
                                                VirtualizerScrollOptions::default()
                                                    .align(VirtualizerAlign::End),
                                            )
                                            .unwrap();
                                            cx.notify();
                                        });
                                    }
                                }
                                1 => p.active = 1,
                                2 => {
                                    masonry_before_reflow = p.builds[1].get();
                                    p.masonry.update(cx, |s, cx| {
                                        s.reflow(5).unwrap();
                                        cx.notify();
                                    });
                                }
                                3 => p.active = 2,
                                4 => p.horizontal.update(cx, |s, cx| {
                                    s.scroll_to_key(
                                        1_999usize.into(),
                                        VirtualizerScrollOptions::default()
                                            .align(VirtualizerAlign::End),
                                    )
                                    .unwrap();
                                    cx.notify();
                                }),
                                _ => {}
                            }
                            cx.notify();
                        });
                    });
                }
                cx.update(|cx| {
                    let p = page.read(cx);
                    // A whole million-row model or all 2,000 measured nodes
                    // must not be built to reach a far target.
                    if p.builds[0].get() >= 10_000 || p.builds[2].get() >= 2_000 {
                        return Err(format!(
                            "unbounded rendering: grid={}, horizontal={}",
                            p.builds[0].get(),
                            p.builds[2].get()
                        ));
                    }
                    println!(
                        "native layouts: grid={}, masonry={}, horizontal={} renderer calls",
                        p.builds[0].get(),
                        p.builds[1].get(),
                        p.builds[2].get()
                    );
                    Ok(())
                })
            }
            .await;
            *observed.borrow_mut() = Some(checked);
            window
                .update(cx, |_, window, _| window.remove_window())
                .unwrap();
            cx.update(|cx| cx.quit());
        })
        .detach();
    });
    result
        .borrow_mut()
        .take()
        .expect("native application returned without running the probe")
        .unwrap();
}
