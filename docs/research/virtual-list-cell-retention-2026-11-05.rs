//! Identical Release A/B workloads, including observations of the old defect.
//! Headless CPU and thread-local allocations, never GPU FPS or physical input latency.
use gpui_kit::base::virtualization::*;
use gpui_kit::{
    AppContext as _, Axis, Context, ElementId, Entity, IntoElement, Render, ScrollHandle, Styled,
    TestAppContext, Window, div, point, px,
};
use std::{cell::Cell, rc::Rc, time::Instant};

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
    unsafe fn alloc(&self, l: std::alloc::Layout) -> *mut u8 {
        let p = unsafe { std::alloc::System.alloc(l) };
        if !p.is_null() {
            account(l.size() as isize, true);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: std::alloc::Layout) {
        account(-(l.size() as isize), false);
        unsafe {
            std::alloc::System.dealloc(p, l);
        }
    }
    unsafe fn realloc(&self, p: *mut u8, l: std::alloc::Layout, n: usize) -> *mut u8 {
        let next = unsafe { std::alloc::System.realloc(p, l, n) };
        if !next.is_null() {
            account(n as isize - l.size() as isize, true);
        }
        next
    }
}
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
fn measured(f: impl FnOnce()) -> (u128, Allocations) {
    STATS.with(|s| {
        assert!(s.get().is_none());
        s.set(Some(Allocations::default()));
    });
    let now = Instant::now();
    f();
    (now.elapsed().as_nanos(), STATS.with(|s| s.take().unwrap()))
}
struct Source {
    count: usize,
    calls: Rc<Cell<usize>>,
}
impl VirtualizerSource for Source {
    fn len(&self) -> usize {
        self.count
    }
    fn key_at(&self, ix: usize) -> ElementId {
        self.calls.set(self.calls.get() + 1);
        ix.into()
    }
    fn index_of(&self, key: &ElementId) -> Option<usize> {
        self.calls.set(self.calls.get() + 1);
        match key {
            ElementId::Integer(ix) if (*ix as usize) < self.count => Some(*ix as usize),
            _ => None,
        }
    }
}

struct Harness {
    axes: [Entity<VirtualizerState>; 2],
    scroll: ScrollHandle,
    retained: usize,
    frozen: usize,
    builds: Rc<Cell<usize>>,
}
impl Render for Harness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let builds = self.builds.clone();
        let grid = VirtualGrid::new(
            "sparse-grid",
            &self.axes[0],
            &self.axes[1],
            &self.scroll,
            move |_, _, _| {
                builds.set(builds.get() + 1);
                div()
            },
        )
        .frozen_rows(self.frozen)
        .frozen_columns(self.frozen)
        .w(px(240.))
        .h(px(100.));
        #[cfg(feature = "grid-cells")]
        let grid = grid.retained_cells(
            (0..self.retained)
                .map(|i| VirtualGridCellKey::new(500usize + i, 600usize + i))
                .collect(),
        );
        #[cfg(not(feature = "grid-cells"))]
        let _ = self.retained;
        grid
    }
}
#[gpui_kit::test]
fn sparse_cells_workload(cx: &mut TestAppContext) {
    for retained in [0, 2] {
        for frozen in [0, 1] {
            for sample in 0..3 {
                let calls = Rc::new(Cell::new(0));
                let builds = Rc::new(Cell::new(0));
                let (view, cx) = cx.add_window_view(|_, cx| {
                    let binding = VirtualizerBinding::new();
                    let mut axis = |vertical| {
                        cx.new(|_| {
                            let mut s = VirtualizerState::new_attached(
                                if vertical {
                                    Axis::Vertical
                                } else {
                                    Axis::Horizontal
                                },
                                Rc::new(Source {
                                    count: 100_000,
                                    calls: calls.clone(),
                                }),
                                VirtualizerSizing::fixed(px(if vertical { 20. } else { 50. })),
                                &binding,
                            )
                            .unwrap();
                            #[cfg(not(feature = "grid-cells"))]
                            s.set_retained_keys(
                                (0..retained)
                                    .map(|i| {
                                        ElementId::from(if vertical { 500 + i } else { 600 + i })
                                    })
                                    .collect(),
                            )
                            .unwrap();
                            #[cfg(feature = "grid-cells")]
                            let _ = &mut s;
                            s
                        })
                    };
                    Harness {
                        axes: [axis(true), axis(false)],
                        scroll: ScrollHandle::new(),
                        retained,
                        frozen,
                        builds: builds.clone(),
                    }
                });
                let mut times = Vec::new();
                let (mut allocations, mut peak, mut reads, mut roots) = (0, 0, 0, 0);
                for step in 0..180 {
                    calls.set(0);
                    builds.set(0);
                    let (ns, stats) = measured(|| {
                        view.update(cx, |v, cx| {
                            v.scroll.set_precise_offset(point(-1000., -2000.));
                            cx.notify();
                        });
                        cx.update(|w, cx| w.draw(cx).clear(cx));
                    });
                    if step >= 20 {
                        times.push(ns);
                        allocations += stats.count;
                        peak = peak.max(stats.peak);
                        reads += calls.get();
                        roots += builds.get();
                    }
                }
                let total: u128 = times.iter().sum();
                times.sort_unstable();
                println!(
                    "SERIES,case=sparse-{retained}-frozen-{frozen},sample={sample},ns={total},p50={},p95={},p99={},allocations={allocations},peak={peak},reads={reads},builds={roots}",
                    times[80], times[152], times[158]
                );
            }
        }
    }
}
