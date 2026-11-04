//! Opt-in real-platform draw/present samples, driven by native owner offsets.
//! These are application work durations, not GPU completion or hardware input.
use super::*;
use gpui_kit::profiler::{self, FrameEvent, FrameTimingCollector};
use std::{cell::RefCell, time::Duration};

#[test]
#[ignore = "opens a native profiling window; run release on an unlocked desktop"]
fn native_frame_workload() {
    let result = Rc::new(RefCell::new(None));
    let observed = result.clone();
    profiler::set_trace_enabled(true);
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
        ).unwrap();
        cx.spawn(async move |cx| {
            let checked: Result<(), String> = async {
                let mut collector = FrameTimingCollector::new();
                for rem in [16., 20.] {
                    for scenario in 0..4 {
                        let active = if scenario == 0 { 0 } else if scenario == 3 { 2 } else { 1 };
                        window.update(cx, |_, w, cx| {
                            w.set_rem_size(px(rem));
                            page.update(cx, |p, cx| { p.active = active; cx.notify(); });
                        }).unwrap();
                        let mut measured = 0;
                        for step in 0..108 {
                            collector.collect_unseen();
                            let done = Rc::new(Cell::new(false));
                            let ready = done.clone();
                            window.update(cx, |_, w, cx| {
                                page.update(cx, |p, cx| {
                                    let offset = ((step * 397) % 10_000) as f64;
                                    match scenario {
                                        0 => p.scroll.set_precise_offset(gpui_kit::point(-offset / 3., -offset)),
                                        1 => p.masonry.read(cx).scroll_handle().set_precise_offset(gpui_kit::point(0., -offset)),
                                        2 => {
                                            if step % 12 == 0 {
                                                p.masonry.update(cx, |s, _| s.reflow(if step % 24 == 0 { 5 } else { 3 }).unwrap());
                                            }
                                            p.masonry.read(cx).scroll_handle().set_precise_offset(gpui_kit::point(0., -offset));
                                        }
                                        _ => p.horizontal.update(cx, |s, _| {
                                            s.scroll_to_offset(offset, VirtualizerScrollOptions::default()).unwrap();
                                        }),
                                    }
                                    cx.notify();
                                });
                                w.on_next_frame(move |_, _| ready.set(true));
                                w.refresh();
                            }).unwrap();
                            for _ in 0..1_000 {
                                cx.background_executor().timer(Duration::from_millis(2)).await;
                                if done.get() { break; }
                            }
                            if !done.get() { return Err(format!("native frame timeout scenario={scenario} step={step}")); }
                            let events = collector.collect_unseen();
                            let mut draws = Vec::new();
                            let mut presents = Vec::new();
                            for event in events {
                                match event {
                                    FrameEvent::Draw(frame) if frame.window_id == window.window_id() => draws.push(frame),
                                    FrameEvent::Present(frame) if frame.window_id == window.window_id() => presents.push(frame),
                                    _ => {}
                                }
                            }
                            if draws.is_empty() || presents.is_empty() { return Err(format!("missing native draw/present scenario={scenario} step={step}")); }
                            if step >= 12 {
                                let scale = window.update(cx, |_, w, _| w.scale_factor()).unwrap();
                                for (sample, frame) in draws.iter().enumerate() {
                                    measured += 1;
                                    let present_ns = presents.iter().filter(|p| p.present_start >= frame.draw_end).map(|p| p.present_duration().as_nanos()).next().unwrap_or(0);
                                    println!("FRAME,case=native-{scenario}-rem{rem},sample={step}-{sample},draw_ns={},layout_ns={},prepaint_ns={},paint_ns={},present_ns={present_ns},refresh_ns={},scale={scale}",frame.draw_duration().as_nanos(),frame.phases.request_layout.as_nanos(),frame.phases.prepaint.as_nanos(),frame.phases.paint.as_nanos(),frame.refresh_interval.map_or(0, |d| d.as_nanos()));
                                }
                            }
                        }
                        if measured < 96 { return Err(format!("insufficient native frames {measured}")); }
                    }
                }
                Ok(())
            }.await;
            *observed.borrow_mut() = Some(checked);
            window.update(cx, |_, w, _| w.remove_window()).unwrap();
            cx.update(|cx| cx.quit());
        }).detach();
    });
    profiler::set_trace_enabled(false);
    result
        .borrow_mut()
        .take()
        .expect("native probe did not execute")
        .unwrap();
}
