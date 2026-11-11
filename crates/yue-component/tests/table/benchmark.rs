use super::*;
use std::time::Instant;

/// CPU draw evidence on the test renderer. Does not measure GPU/presentation,
/// hardware pointer latency or Windows compositor scheduling.
#[gpui_kit::test]
#[ignore = "opt-in TB01 headless CPU sample; run with tools/sample-table.py after building"]
fn tb01_headless_frame_sample(cx: &mut TestAppContext) {
    frame_sample(cx, false);
}

/// Rows layout workload; deliberately not labelled TB02 (Cells lands in M5).
#[gpui_kit::test]
#[ignore = "opt-in M2 layout CPU sample; build before running tools/sample-table.py"]
fn m2_headless_layout_frame_sample(cx: &mut TestAppContext) {
    frame_sample(cx, true);
}

fn frame_sample(cx: &mut TestAppContext, layout: bool) {
    #[cfg(feature = "table-profile")]
    let capture = super::profile::start();
    let count: usize = std::env::var("YUE_TABLE_BENCH_ROWS")
        .expect("runner supplies rows")
        .parse()
        .unwrap();
    assert!(count == 1000 || count == 100_000);
    let start = Instant::now();
    let (p, cx) = open(cx, count, true);
    if layout {
        p.update(cx, |p, cx| {
            p.frozen = (1, 1);
            p.footer_fixed = true;
            cx.notify();
        });
        layout::configure(&p, cx, |mut cols| {
            let last = cols.len() - 1;
            cols[0] = cols[0]
                .clone()
                .pin(yue_component::TablePin::Start)
                .footer("Total");
            cols[last] = cols[last].clone().pin(yue_component::TablePin::End);
            for col in &mut cols[1..last] {
                *col = col
                    .clone()
                    .header_path(vec![yue_component::TableHeaderGroup::new(
                        "values", "Values",
                    )]);
            }
            cols
        });
    }
    let mount_ns = start.elapsed().as_nanos();
    let s = state(&p, cx);
    let calls = p.read_with(cx, |p, _| p.calls.clone());
    cx.update(|w, cx| {
        w.set_rem_size(px(16.));
        yue_component::set_theme(cx, YueTheme::new(ThemeMode::Light, Accent::Neutral));
        cx.set_reduce_motion(true);
    });
    let step = |frame: usize, cx: &mut VisualTestContext| {
        if layout {
            p.update(cx, |p, cx| {
                p.host_width = rems([48., 64., 80.][(frame / 10) % 3]);
                cx.notify();
            });
            // Resolve container feedback before the timed committed frame.
            draw(cx);
        }
        let sample = cx.update(|w, cx| {
            s.read(cx).scroll_handle().set_offset(gpui_kit::point(
                px(if layout {
                    -(((frame * 19) % 400) as f32)
                } else {
                    0.
                }),
                px(-(((frame * 37) % 8000) as f32)),
            ));
            w.refresh();
            calls.set(0);
            let start = Instant::now();
            w.draw(cx).clear(cx);
            (start.elapsed().as_nanos(), calls.get())
        });
        // Match the application loop: drain effects/tasks between frames, outside
        // the draw timing. A tight draw-only loop is not a sustained UI workload.
        {
            #[cfg(feature = "table-profile")]
            let _perf = gpui_kit::perf_stats::scope("fixture.effects");
            cx.run_until_parked();
        }
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
    if layout {
        for key in ["row-0".to_owned(), format!("row-{}", count - 1)] {
            let first = cell(cx, &key, "col-0");
            let last = cell(cx, &key, "col-11");
            close(first.bounds().left(), px(0.));
            assert!(last.bounds().left() > first.bounds().right());
        }
    }
    #[cfg(feature = "table-profile")]
    super::profile::finish(capture);
}
