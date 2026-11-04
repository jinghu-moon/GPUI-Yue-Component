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

struct MotionProbe {
    rows: Entity<VirtualizerState>,
    columns: Entity<VirtualizerState>,
    scroll: ScrollHandle,
    masonry: Entity<VirtualMasonryState>,
}
impl MotionProbe {
    fn new(cx: &mut Context<Self>) -> Self {
        let binding = VirtualizerBinding::new();
        Self {
            rows: cx.new(|_| {
                VirtualizerState::new_attached(
                    Axis::Vertical,
                    Rc::new(Records(1_000_000)),
                    VirtualizerSizing::fixed(px(20.)),
                    &binding,
                )
                .unwrap()
            }),
            columns: cx.new(|_| {
                VirtualizerState::new_attached(
                    Axis::Horizontal,
                    Rc::new(Records(1000)),
                    VirtualizerSizing::known(vec![px(50.); 1000]),
                    &binding,
                )
                .unwrap()
            }),
            scroll: ScrollHandle::new(),
            masonry: cx.new(|_| {
                VirtualMasonryState::new(
                    Rc::new(Records(10_000)),
                    VirtualizerSizing::known(vec![px(20.); 10_000]),
                    3,
                )
                .unwrap()
            }),
        }
    }
}
impl Render for MotionProbe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .child(
                VirtualGrid::new(
                    "native-motion-grid",
                    &self.rows,
                    &self.columns,
                    &self.scroll,
                    |cell, _, _| {
                        div().child(format!("{}:{}", cell.row().index(), cell.column().index()))
                    },
                )
                .w(px(300.))
                .h(px(100.)),
            )
            .child(
                VirtualMasonry::new("native-motion-masonry", &self.masonry, |item, _, _| {
                    div().child(format!("Card {}", item.item().index()))
                })
                .w(px(300.))
                .h(px(100.)),
            )
    }
}

#[test]
#[ignore = "requires an unlocked Windows desktop and GPU; activates a temporary motion probe window"]
fn native_smooth_layouts_sample_and_settle() {
    use yue_component::VirtualizerMotion;
    let result = Rc::new(RefCell::new(None));
    let observed = result.clone();
    gpui_kit::application().run(move |cx| {
        yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
        cx.set_reduce_motion(false);
        let (window, page) = gpui_kit::open_window(WindowOptions {
            focus: true,
            window_bounds: Some(gpui_kit::WindowBounds::Windowed(gpui_kit::Bounds::new(gpui_kit::point(px(40.), px(40.)), gpui_kit::size(px(640.), px(480.))))),
            ..Default::default()
        }, cx, |w, cx| { w.activate_window(); cx.new(MotionProbe::new) }).unwrap();
        cx.spawn(async move |cx| {
            let checked: Result<(), String> = async {
                let mut mounted = false;
                for _ in 0..100 {
                    cx.background_executor().timer(Duration::from_millis(20)).await;
                    mounted = cx.update(|cx| page.read(cx).rows.read(cx).viewport().is_some());
                    if mounted { break; }
                }
                if !mounted { return Err("motion probe did not mount".into()); }
                if !window.update(cx, |_, w, _| w.is_window_active() && w.is_visible()).unwrap() {
                    return Err("motion probe requires an active visible native window".into());
                }
                let options = VirtualizerScrollOptions::default().align(VirtualizerAlign::Start).motion(VirtualizerMotion::Smooth { duration: Duration::from_millis(360), max_distance: px(100.) });
                cx.update(|cx| page.update(cx, |p, cx| {
                    for (state, key) in [(&p.rows, 4usize), (&p.columns, 2usize)] {
                        state.update(cx, |s, cx| { s.scroll_to_key(key.into(), options).unwrap(); cx.notify(); });
                    }
                    p.masonry.update(cx, |s, cx| { s.scroll_to_offset(80., options).unwrap(); cx.notify(); });
                }));
                let mut grid_middle = false;
                let mut masonry_middle = false;
                let mut settled = false;
                for _ in 0..100 {
                    cx.background_executor().timer(Duration::from_millis(20)).await;
                    let (position, offset, pending) = cx.update(|cx| {
                        let p = page.read(cx);
                        (p.scroll.precise_offset(), -p.masonry.read(cx).scroll_handle().precise_offset().y,
                         p.rows.read(cx).has_scroll_command() || p.columns.read(cx).has_scroll_command() || p.masonry.read(cx).has_scroll_command())
                    });
                    grid_middle |= position.x < -0.01 && position.x > -99.99 && position.y < -0.01 && position.y > -79.99;
                    masonry_middle |= offset > 0.01 && offset < 79.99;
                    if !pending {
                        if position != gpui_kit::point(-100., -80.) || (offset - 80.).abs() > 0.01 {
                            return Err(format!("incorrect native targets: grid={position:?}, masonry={offset}"));
                        }
                        settled = true; break;
                    }
                }
                if !settled || !grid_middle || !masonry_middle {
                    return Err(format!("native smooth contract: settled={settled}, grid middle={grid_middle}, masonry middle={masonry_middle}"));
                }
                println!("native smooth: Grid x/y and Masonry sampled intermediate positions and settled; no GPU timing claim");
                Ok(())
            }.await;
            *observed.borrow_mut() = Some(checked);
            window.update(cx, |_, w, _| w.remove_window()).unwrap();
            cx.update(|cx| cx.quit());
        }).detach();
    });
    result
        .borrow_mut()
        .take()
        .expect("native motion probe did not run")
        .unwrap();
}
