//! Actual geometry churn, measured identically against isolated Git sources.
use gpui_kit::{px, size};
use std::{cell::Cell, time::Instant};

// Match the adoption workload's thread-local heap payload accounting.
#[derive(Clone, Copy, Default)]
struct Allocations {
    count: usize,
    live: isize,
    peak: isize,
}
thread_local! { static STATS: Cell<Option<Allocations>> = const { Cell::new(None) }; }
struct Allocator;
fn account(delta: isize, allocation: bool) {
    let _ = STATS.try_with(|cell| {
        if let Some(mut s) = cell.get() {
            s.count += usize::from(allocation);
            s.live += delta;
            s.peak = s.peak.max(s.live);
            cell.set(Some(s));
        }
    });
}
unsafe impl std::alloc::GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let p = unsafe { std::alloc::System.alloc(layout) };
        if !p.is_null() {
            account(layout.size() as isize, true);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: std::alloc::Layout) {
        account(-(layout.size() as isize), false);
        unsafe {
            std::alloc::System.dealloc(p, layout);
        }
    }
    unsafe fn realloc(&self, p: *mut u8, layout: std::alloc::Layout, n: usize) -> *mut u8 {
        let next = unsafe { std::alloc::System.realloc(p, layout, n) };
        if !next.is_null() {
            account(n as isize - layout.size() as isize, true);
        }
        next
    }
}
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
fn measured<T>(f: impl FnOnce() -> T) -> (T, u128, Allocations) {
    STATS.with(|s| {
        assert!(s.get().is_none());
        s.set(Some(Allocations::default()));
    });
    let time = Instant::now();
    let result = f();
    let elapsed = time.elapsed().as_nanos();
    let stats = STATS.with(|s| s.take().unwrap());
    (result, elapsed, stats)
}

#[test]
fn next_native_churn() {
    for (count, sparse, order) in [
        (1, false, 0),
        (32, false, 0),
        (32, true, 0),
        (4096, false, 0),
        (4096, true, 0),
        (32, true, 1),
        (4096, true, 1),
        (4096, true, 2),
    ] {
        for sample in 0..5 {
            let state = gpui_kit::ListState::new_with_uniform_height(
                1_000_000,
                gpui_kit::ListAlignment::Top,
                px(0.),
                px(20.),
            );
            let mut updates: Vec<_> = (0..count)
                .map(|i| {
                    (
                        if sparse { i * 1_000_000 / count } else { i },
                        size(px(300.), px(30.)),
                    )
                })
                .collect();
            if order == 1 {
                updates.reverse();
            } else if order == 2 {
                updates.extend(
                    (0..count / 2)
                        .rev()
                        .map(|i| (i * 1_000_000 / count, size(px(320.), px(40.)))),
                );
            }
            let frozen = state.geometry();
            let (_, ns, stats) = measured(|| {
                for pass in 0..20 {
                    for (at, (_, size)) in updates.iter_mut().enumerate() {
                        // Every call changes actual geometry. Count input
                        // preparation identically on both sides of the test.
                        let base = if order == 2 && at >= count { 40. } else { 30. };
                        size.height = px(base + (pass % 2) as f32);
                    }
                    state.set_item_sizes(&updates, true);
                }
            });
            assert_eq!(state.geometry().measured_count(), count);
            assert_eq!(frozen.measured_count(), 0);
            assert_eq!(frozen.item(0).unwrap().extent(), px(20.));
            assert_eq!(
                state.geometry().item(0).unwrap().extent(),
                px(if order == 2 { 41. } else { 31. })
            );
            println!(
                "SERIES,case=native-churn-{count}-{sparse}-{order},sample={sample},ns={ns},p50=0,p95=0,p99=0,allocations={},peak={},reads=0,builds=0",
                stats.count, stats.peak
            );
        }
    }
}
