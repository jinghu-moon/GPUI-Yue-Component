//! Opt-in native rendering contract. This uses the real Windows platform,
//! dispatcher, font shaping and renderer, not TestAppContext or a fake window.
use super::*;
use std::{cell::RefCell, time::Duration};

#[test]
#[ignore = "requires an unlocked Windows desktop and GPU; opens a temporary frozen-grid/cache window"]
fn native_frozen_grid_and_masonry_cache_restore() {
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
                    gpui_kit::size(px(1200.), px(800.)),
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| cx.new(|cx| VirtualLayoutsPage::new_with_focus(window, cx)),
        )
        .unwrap();
        cx.spawn(async move |cx| {
            let checked: Result<(), String> = async {
                let mut old = None;
                let mut expected = None;
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
                                    !p.rows.read(cx).has_scroll_command()
                                        && !p.columns.read(cx).has_scroll_command()
                                        && (-p.scroll.precise_offset().y - 35964.).abs() < 0.1
                                        && (-p.scroll.precise_offset().x - 14256.).abs() < 0.1
                                }
                                2 => {
                                    (-p.scroll.precise_offset().y - 36213.75).abs() < 0.01
                                        && (-p.scroll.precise_offset().x - 14280.75).abs() < 0.01
                                }
                                3 => p.masonry.read(cx).measurement_cache().is_ok(),
                                4 => -p.masonry.read(cx).scroll_handle().precise_offset().y >= 499.,
                                _ => {
                                    p.masonry.read(cx).measurement_cache().is_ok()
                                        && old.as_ref().is_some_and(
                                            |w: &gpui_kit::WeakEntity<VirtualMasonryState>| {
                                                w.upgrade().is_none()
                                            },
                                        )
                                }
                            }
                        });
                        if ready {
                            break;
                        }
                    }
                    if !ready {
                        return Err(format!("frozen/cache native phase {phase} did not settle"));
                    }
                    cx.update(|cx| {
                        page.update(cx, |p, cx| {
                            match phase {
                                0 => {
                                    for (s, key) in [(&p.rows, 1000usize), (&p.columns, 100usize)] {
                                        s.update(cx, |s, cx| {
                                            s.scroll_to_key(
                                                key.into(),
                                                VirtualizerScrollOptions::default()
                                                    .align(VirtualizerAlign::Start),
                                            )
                                            .unwrap();
                                            cx.notify();
                                        });
                                    }
                                }
                                1 => {
                                    for axis in [&p.rows, &p.columns] {
                                        let s = axis.read(cx);
                                        let snapshot = s.snapshot(s.viewport().unwrap()).unwrap();
                                        assert!(
                                            snapshot.working_indices().contains(&0),
                                            "native frozen prefix remains presented"
                                        );
                                    }
                                    assert!(
                                        p.builds[0].get() < 5000,
                                        "native grid remains bounded"
                                    );
                                    // The first visible body keys are row 1000
                                    // and column 100. Leading padding and the
                                    // frozen gap move the strip too; compensate
                                    // only the scrollable difference.
                                    for axis in [&p.rows, &p.columns] {
                                        axis.update(cx, |s, cx| {
                                            s.set_layout(
                                                VirtualizerLayout::default()
                                                    .gap(px(0.25))
                                                    .padding(px(2.), px(3.)),
                                            )
                                            .unwrap();
                                            cx.notify();
                                        });
                                    }
                                }
                                2 => {
                                    for (axis, key) in [(&p.rows, 1000), (&p.columns, 100)] {
                                        let s = axis.read(cx);
                                        let snapshot = s.snapshot(s.viewport().unwrap()).unwrap();
                                        assert_eq!(snapshot.visible_range().start, key);
                                    }
                                    p.active = 1;
                                }
                                3 => p.masonry.update(cx, |s, cx| {
                                    s.scroll_handle()
                                        .set_precise_offset(gpui_kit::point(0., -500.));
                                    cx.notify();
                                }),
                                4 => {
                                    let cache = p.masonry.read(cx).measurement_cache().unwrap();
                                    expected =
                                        cache.scroll_anchor().map(|(k, pos)| (k.clone(), pos));
                                    old = Some(p.masonry.downgrade());
                                    let new =
                                        VirtualLayoutsPage::new_masonry(cache.lane_count(), cx);
                                    new.update(cx, |s, _| {
                                        s.restore_measurement_cache(&cache).unwrap();
                                    });
                                    p.masonry = new;
                                }
                                _ => {
                                    let cache = p.masonry.read(cx).measurement_cache().unwrap();
                                    assert_eq!(
                                        cache.scroll_anchor().map(|(k, pos)| (k.clone(), pos)),
                                        expected
                                    );
                                    assert!(
                                        !p.masonry.read(cx).is_scrolling(),
                                        "cold restore is not native user input"
                                    );
                                }
                            }
                            cx.notify();
                        })
                    });
                }
                Ok(())
            }
            .await;
            *observed.borrow_mut() = Some(checked);
            let _ = window.update(cx, |_, w, _| w.remove_window());
            cx.update(|cx| cx.quit());
        })
        .detach();
    });
    result
        .borrow_mut()
        .take()
        .expect("native frozen/cache runner completed")
        .unwrap();
}

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
            |window, cx| cx.new(|cx| VirtualLayoutsPage::new_with_focus(window, cx)),
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
    measured: Entity<VirtualizerState>,
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
            measured: cx.new(|_| {
                let mut state = VirtualizerState::new(
                    Axis::Vertical,
                    Rc::new(Records(1000)),
                    VirtualizerSizing::measured(px(20.)),
                )
                .unwrap();
                state
                    .set_layout(VirtualizerLayout::default().overscan(px(100.)))
                    .unwrap();
                state
            }),
        }
    }
}
impl Render for MotionProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                .right_to_left(true)
                .frozen_rows(1)
                .frozen_columns(1)
                .spans(
                    &[VirtualGridSpan::new(
                        VirtualGridCellKey::new(1usize, 1usize),
                        2,
                        3,
                    )],
                    cx,
                )
                .unwrap()
                .w(px(300.))
                .h(px(100.)),
            )
            .child(
                VirtualList::new("native-measured-motion", &self.measured, |_, _, _| {
                    div().h(px(20.))
                })
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
                    p.measured.update(cx, |s,cx| {s.scroll_to_key(2usize.into(),options).unwrap();cx.notify();});
                }));
                let mut grid_middle = false;
                let mut masonry_middle = false;
                let mut settled = false;
                let mut measured_middle=false;
                for _ in 0..100 {
                    cx.background_executor().timer(Duration::from_millis(20)).await;
                    let (position, offset, measured, pending) = cx.update(|cx| {
                        let p = page.read(cx);
                        let column=p.columns.read(cx);let q=column.viewport().unwrap().offset();
                        assert!((-p.scroll.precise_offset().x-(column.total_extent()-300.-q)).abs()<0.01);
                        (gpui_kit::point(-q,p.scroll.precise_offset().y), -p.masonry.read(cx).scroll_handle().precise_offset().y,
                         p.measured.read(cx).viewport().unwrap().offset(),
                         p.rows.read(cx).has_scroll_command() || column.has_scroll_command() || p.masonry.read(cx).has_scroll_command()||p.measured.read(cx).has_scroll_command())
                    });
                    grid_middle |= position.x < -0.01 && position.x > -49.99 && position.y < -0.01 && position.y > -59.99;
                    masonry_middle |= offset > 0.01 && offset < 79.99;
                    measured_middle |= measured>0.01&&measured<39.99;
                    if !pending {
                        if position != gpui_kit::point(-50., -60.) || (offset - 80.).abs() > 0.01 || (measured-40.).abs()>0.01 {
                            return Err(format!("incorrect native targets: grid={position:?}, masonry={offset}"));
                        }
                        settled = true; break;
                    }
                }
                if !settled || !grid_middle || !masonry_middle ||!measured_middle {
                    return Err(format!("native smooth contract: settled={settled}, grid middle={grid_middle}, masonry middle={masonry_middle}, measured middle={measured_middle}"));
                }
                cx.update(|cx|page.update(cx,|p,cx|p.measured.update(cx,|s,cx|{
                    s.scroll_to_key(900usize.into(),VirtualizerScrollOptions::default().align(VirtualizerAlign::Start)
                        .motion(VirtualizerMotion::JumpThenSmooth{duration:Duration::from_millis(360),max_distance:px(100.)})).unwrap();cx.notify();
                })));
                let mut tail_middle=false;let mut tail_settled=false;
                for _ in 0..100 {
                    cx.background_executor().timer(Duration::from_millis(20)).await;
                    let (q,pending)=cx.update(|cx|{let s=page.read(cx).measured.read(cx);(s.viewport().unwrap().offset(),s.has_scroll_command())});
                    tail_middle|=q>17950.01&&q<17999.99;
                    if !pending {tail_settled=(q-18000.).abs()<0.01;break;}
                }
                if !tail_middle||!tail_settled{return Err(format!("native measured tail: middle={tail_middle}, settled={tail_settled}"));}
                println!("native smooth: RTL frozen/spanned Grid x/y, Masonry and Measured sampled intermediate positions; measured jump tail settled; no GPU timing claim");
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
