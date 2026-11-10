use super::*;
use std::time::Instant;

/// CPU draw evidence on the test renderer. Does not measure GPU/presentation,
/// hardware pointer latency or Windows compositor scheduling.
#[gpui_kit::test]
#[ignore = "opt-in TB01 headless CPU sample; run with tools/sample-table.py after building"]
fn tb01_headless_frame_sample(cx: &mut TestAppContext) {
    let count: usize = std::env::var("YUE_TABLE_BENCH_ROWS")
        .expect("runner supplies rows")
        .parse()
        .unwrap();
    assert!(count == 1000 || count == 100_000);
    let start = Instant::now();
    let (p, cx) = open(cx, count, true);
    let mount_ns = start.elapsed().as_nanos();
    let s = state(&p, cx);
    let calls = p.read_with(cx, |p, _| p.calls.clone());
    cx.update(|w, cx| {
        w.set_rem_size(px(16.));
        yue_component::set_theme(cx, YueTheme::new(ThemeMode::Light, Accent::Neutral));
        cx.set_reduce_motion(true);
    });
    let step = |frame: usize, cx: &mut VisualTestContext| {
        let sample = cx.update(|w, cx| {
            s.read(cx)
                .scroll_handle()
                .set_offset(gpui_kit::point(px(0.), px(-(((frame * 37) % 8000) as f32))));
            w.refresh();
            calls.set(0);
            let start = Instant::now();
            w.draw(cx).clear(cx);
            (start.elapsed().as_nanos(), calls.get())
        });
        // Match the application loop: drain effects/tasks between frames, outside
        // the draw timing. A tight draw-only loop is not a sustained UI workload.
        cx.run_until_parked();
        sample
    };
    for frame in 0..120 {
        step(frame, cx);
    }
    println!("TABLE_MOUNT,rows={count},ns={mount_ns}");
    for frame in 0..1000 {
        let (ns, calls) = step(frame + 120, cx);
        assert!(calls > 0 && calls < 600);
        println!("TABLE_FRAME,rows={count},frame={frame},ns={ns},renders={calls}");
    }
    let parts = nodes(cx);
    assert!(
        parts
            .iter()
            .any(|n| n.path().contains(&ElementId::from("table-header")))
    );
    assert!(
        parts
            .iter()
            .filter(|n| n.role() == Some(Role::Cell))
            .count()
            < 600
    );
}
