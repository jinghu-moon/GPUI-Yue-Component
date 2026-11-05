//! Real Windows acceptance. Physical input/UIA evidence is collected separately
//! through the interactive surface; synthetic owner writes are not input latency.
use super::*;
use std::{cell::RefCell, time::Duration};

#[test]
#[ignore = "opens an active native window, resizes and minimizes/restores it"]
fn native_resize_visibility_resume() {
    let result = Rc::new(RefCell::new(None));
    let observed = result.clone();
    gpui_kit::application().run(move |cx| {
        yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
        cx.set_reduce_motion(false);
        println!("NATIVE_ENV,displays={}", cx.displays().len());
        for display in cx.displays() { println!("NATIVE_DISPLAY,id={:?},bounds={:?},refresh={:?}", display.id(), display.bounds(), display.refresh_interval()); }
        let (window, page) = gpui_kit::open_window(WindowOptions {
            focus: true,
            window_bounds: Some(gpui_kit::WindowBounds::Windowed(gpui_kit::Bounds::new(gpui_kit::point(px(40.), px(40.)), gpui_kit::size(px(1200.), px(800.))))),
            ..Default::default()
        }, cx, |w, cx| { w.set_window_title("Yue Virtualization Native Acceptance"); cx.new(|cx| VirtualLayoutsPage::new_with_focus(w, cx)) }).unwrap();
        window.update(cx, |_, w, cx| page.update(cx, |p, cx| {
            p.grid_cursor = (1, 1);
            p.grid_focus.focus(w, cx);
            cx.notify();
        })).unwrap();
        cx.spawn(async move |cx| {
            let checked: Result<(), String> = async {
                for step in 0..24 {
                    let ready = Rc::new(Cell::new(false)); let done = ready.clone();
                    window.update(cx, |_, w, cx| {
                        w.resize(gpui_kit::size(px(if step % 2 == 0 { 1100. } else { 900. }), px(if step % 2 == 0 { 760. } else { 680. })));
                        w.set_rem_size(px(if step % 3 == 0 { 20. } else { 16. }));
                        page.update(cx, |p, cx| {
                            p.rtl = step % 2 == 1; p.spanning = true;
                            for (axis, offset) in [(&p.rows, (step * 397) as f64), (&p.columns, (step * 113) as f64)] {
                                axis.update(cx, |s, cx| { s.scroll_to_offset(offset, VirtualizerScrollOptions::default()).unwrap(); cx.notify(); });
                            }
                            cx.notify();
                        });
                        w.on_next_frame(move |_, _| done.set(true)); w.refresh();
                    }).unwrap();
                    for _ in 0..400 {
                        cx.background_executor().timer(Duration::from_millis(5)).await;
                        if ready.get() && cx.update(|cx| {
                            let p = page.read(cx);
                            p.rows.read(cx).viewport().is_some() && p.columns.read(cx).viewport().is_some() && !p.rows.read(cx).has_scroll_command() && !p.columns.read(cx).has_scroll_command()
                        }) { break; }
                    }
                    if !ready.get() { return Err(format!("native resize frame timeout step={step}")); }
                    window.update(cx, |_, w, cx| {
                        let p = page.read(cx); let rows = p.rows.read(cx); let columns = p.columns.read(cx);
                        let rv = rows.viewport().unwrap(); let cv = columns.viewport().unwrap();
                        assert!((rv.offset() - (step * 397) as f64).abs() < 0.01);
                        assert!((cv.offset() - (step * 113) as f64).abs() < 0.01);
                        let physical_x = if p.rtl { (columns.total_extent() - f64::from(cv.extent())).max(0.) - cv.offset() } else { cv.offset() };
                        assert!((-p.scroll.precise_offset().x - physical_x).abs() < 0.01);
                        assert!((-p.scroll.precise_offset().y - rv.offset()).abs() < 0.01);
                        for s in [rows, columns] {
                            let snap = s.snapshot(s.viewport().unwrap()).unwrap();
                            assert!(snap.working_indices().contains(&0));
                            assert!(snap.working_indices().len() < 40);
                        }
                        println!("NATIVE_RESIZE,step={step},rtl={},scale={},rem={},bounds={:?},x={},y={}", p.rtl, w.scale_factor(), f32::from(w.rem_size()), w.bounds(), cv.offset(), rv.offset());
                    }).unwrap();
                }
                let before = cx.update(|cx| { let p = page.read(cx); [p.rows.read(cx).viewport().unwrap().offset(), p.columns.read(cx).viewport().unwrap().offset()] });
                window.update(cx, |_, w, _| w.minimize_window()).unwrap();
                let mut hidden = false;
                for _ in 0..200 {
                    cx.background_executor().timer(Duration::from_millis(5)).await;
                    hidden = window.update(cx, |_, w, _| !w.is_visible()).unwrap(); if hidden { break; }
                }
                if !hidden { return Err("native minimization did not reach hidden visibility".into()); }
                // Real native visibility restoration, not TestWindow's flag.
                window.update(cx, |_, w, _| w.activate_window()).unwrap();
                let mut restored = false;
                for _ in 0..400 {
                    cx.background_executor().timer(Duration::from_millis(5)).await;
                    restored = window.update(cx, |_, w, cx| { w.refresh(); w.is_visible() && page.read(cx).rows.read(cx).viewport().is_some() }).unwrap();
                    if restored { break; }
                }
                if !restored { return Err("native restoration did not become visible".into()); }
                cx.background_executor().timer(Duration::from_millis(100)).await;
                cx.update(|cx| {
                    let p = page.read(cx);
                    let after = [p.rows.read(cx).viewport().unwrap().offset(), p.columns.read(cx).viewport().unwrap().offset()];
                    assert_eq!(after, before);
                    println!("NATIVE_VISIBILITY,hidden={hidden},restored={restored},reading_offsets_preserved=true");
                });
                window.update(cx, |_, w, cx| {
                    let p = page.read(cx);
                    assert!(p.grid_focus.is_focused(w), "native Grid focus survives restore");
                    assert_eq!(p.grid_cursor, (1, 1), "native reading scroll never changes the cursor");
                    println!("NATIVE_GRID_FOCUS,restored=true,cursor=1:1,reading_offsets_preserved=true");
                }).unwrap();
                Ok(())
            }.await;
            *observed.borrow_mut() = Some(checked);
            let _ = window.update(cx, |_, w, _| w.remove_window()); cx.update(|cx| cx.quit());
        }).detach();
    });
    result
        .borrow_mut()
        .take()
        .expect("native acceptance executed")
        .unwrap();
}

/// Opens the production page for physical wheel/keyboard/UIA checks. This is a
/// bounded manual fixture, not an automated accessibility or latency assertion.
#[test]
#[ignore = "manual Windows wheel, keyboard, screen-reader and visual acceptance surface"]
fn native_interactive_acceptance_surface() {
    gpui_kit::application().run(|cx| {
        yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
        let (window, page) = gpui_kit::open_window(WindowOptions {
            focus: true,
            window_bounds: Some(gpui_kit::WindowBounds::Windowed(gpui_kit::Bounds::new(gpui_kit::point(px(40.), px(40.)), gpui_kit::size(px(1200.), px(800.))))),
            ..Default::default()
        }, cx, |w, cx| {
            w.set_window_title("Yue Virtualization Interactive Acceptance");
            w.on_window_should_close(cx, |_, cx| { cx.quit(); true });
            cx.new(|cx| VirtualLayoutsPage::new_with_focus(w, cx))
        }).unwrap();
        println!("NATIVE_INTERACTIVE,opened=true,displays={},timeout_seconds=300", cx.displays().len());
        cx.spawn(async move |cx| {
            let mut last = String::new();
            for tick in 0..1200 {
                cx.background_executor().timer(Duration::from_millis(250)).await;
                let state = window.update(cx, |_, w, cx| {
                    let p = page.read(cx);
                    format!("tab={},rtl={},span={},frozen={},x={},y={},masonry_y={},horizontal_x={},scale={},bounds={:?},visible={},cursor={}:{},grid_focused={}", p.active,p.rtl,p.spanning,p.frozen,p.columns.read(cx).viewport().map_or(0., |v|v.offset()),p.rows.read(cx).viewport().map_or(0.,|v|v.offset()),-p.masonry.read(cx).scroll_handle().precise_offset().y,p.horizontal.read(cx).viewport().map_or(0.,|v|v.offset()),w.scale_factor(),w.bounds(),w.is_visible(),p.grid_cursor.0,p.grid_cursor.1,p.grid_focus.is_focused(w))
                });
                let Ok(state) = state else { break; };
                if state != last { println!("NATIVE_INTERACTIVE,tick={tick},{state}"); last=state; }
            }
            let _=window.update(cx,|_,w,_|w.remove_window()); cx.update(|cx|cx.quit());
        }).detach();
    });
}
