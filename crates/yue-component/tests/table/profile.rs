use super::*;
use gpui_kit::perf_stats::{Capture, Snapshot, scope};

pub(super) fn start() -> Option<Capture> {
    (std::env::var("TABLE_PROFILE").as_deref() == Ok("1")).then(|| Capture::start().unwrap())
}
pub(super) fn finish(capture: Option<Capture>) {
    if let Some(capture) = capture {
        let snapshot = capture.finish().unwrap();
        assert_eq!(snapshot.dropped(), 0, "diagnostic budget exhausted");
        print_snapshot(&snapshot);
    }
}
fn print_snapshot(s: &Snapshot) {
    println!(
        "TABLE_CAPTURE,elapsed_ns={},unattributed_ns={},dropped={}",
        s.elapsed_ns(),
        s.unattributed_ns(),
        s.dropped()
    );
    for row in s.stages() {
        println!(
            "TABLE_STAGE,name={},calls={},inclusive_ns={},self_ns={},max_ns={}",
            row.name(),
            row.calls(),
            row.inclusive_ns(),
            row.self_ns(),
            row.max_ns()
        );
    }
}

#[test]
fn nested_stats_reconcile_and_reject_unfinished_or_overlapping_captures() {
    let c = Capture::start().unwrap();
    assert!(Capture::start().is_err());
    {
        let _outer = scope("outer");
        for _ in 0..3 {
            let _inner = scope("inner");
            std::hint::black_box((0..1000).sum::<usize>());
        }
    }
    let s = c.finish().unwrap();
    let outer = s.stages().iter().find(|s| s.name() == "outer").unwrap();
    let inner = s.stages().iter().find(|s| s.name() == "inner").unwrap();
    assert_eq!(inner.calls(), 3);
    assert_eq!(outer.inclusive_ns(), outer.self_ns() + inner.inclusive_ns());
    assert_eq!(
        s.elapsed_ns(),
        s.unattributed_ns() + s.stages().iter().map(|s| s.self_ns()).sum::<u128>()
    );
    let c = Capture::start().unwrap();
    let old_scope = scope("unfinished");
    assert!(c.finish().is_err());
    let c = Capture::start().unwrap();
    drop(old_scope); // Cancellation cannot contaminate the next capture.
    assert!(c.finish().unwrap().stages().is_empty());
}

#[gpui_kit::test]
fn real_pipeline_reports_source_input_layout_text_and_release(cx: &mut TestAppContext) {
    let c = Capture::start().unwrap();
    let (p, cx) = open(cx, 1000, true);
    let s = state(&p, cx);
    cx.update(|w, cx| s.focus_handle(cx).focus(w, cx));
    cx.simulate_keystrokes("down");
    draw(cx);
    let snapshot = c.finish().unwrap();
    for required in [
        "table.source.index",
        "table.source.validate",
        "table.commit_view",
        "table.sync_layout",
        "table.element.row",
        "table.column.render",
        "base.grid.prepaint",
        "gpui.input.dispatch",
        "gpui.layout.compute",
        "gpui.draw",
        "gpui.paint",
        "gpui.text.shape_line",
        "gpui.arena.clear",
    ] {
        assert!(
            snapshot
                .stages()
                .iter()
                .any(|s| s.name() == required && s.calls() > 0),
            "missing {required}"
        );
    }
    assert_eq!(snapshot.dropped(), 0);
}

#[test]
fn invalid_order_unwinding_and_bounded_budgets_are_reported() {
    let c = Capture::start().unwrap();
    let outer = scope("outer");
    let inner = scope("inner");
    drop(outer);
    drop(inner);
    assert!(c.finish().is_err());
    let c = Capture::start().unwrap();
    let result = std::panic::catch_unwind(|| {
        let _a = scope("unwind");
        panic!("expected probe unwind");
    });
    assert!(result.is_err());
    assert_eq!(c.finish().unwrap().stages()[0].calls(), 1);
    let c = Capture::start().unwrap();
    let mut scopes = (0..257).map(|_| scope("depth")).collect::<Vec<_>>();
    while scopes.pop().is_some() {}
    let s = c.finish().unwrap();
    assert_eq!(s.dropped(), 1);
    assert_eq!(s.stages()[0].calls(), 256);
    let c = Capture::start().unwrap();
    for i in 0..129 {
        let name = Box::leak(format!("budget-{i}").into_boxed_str());
        let _s = scope(name);
    }
    let s = c.finish().unwrap();
    assert_eq!(s.stages().len(), 128);
    assert_eq!(s.dropped(), 1);
}
