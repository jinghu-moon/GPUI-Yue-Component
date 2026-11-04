//! Identical Release A/B workloads, including observations of the old defect.
//! Headless CPU and thread-local allocations, never GPU FPS or physical input latency.
use gpui_kit::base::virtualization::*;
use gpui_kit::{
    AppContext as _, Axis, Context, ElementId, Entity, IntoElement, Render, Styled, TestAppContext,
    Window, div, point, px,
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
    state: Entity<VirtualizerState>,
    builds: Rc<Cell<usize>>,
}
impl Render for Harness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let builds = self.builds.clone();
        VirtualList::new("motion-workload", &self.state, move |_, _, _| {
            builds.set(builds.get() + 1);
            div().h(px(20.))
        })
        .w(px(240.))
        .h(px(100.))
    }
}
#[gpui_kit::test]
fn measured_motion_workload(cx: &mut TestAppContext) {
    for trace in ["idle", "short", "tail", "reduced"] {
        for sample in 0..3 {
            let calls = Rc::new(Cell::new(0));
            let builds = Rc::new(Cell::new(0));
            let (view, cx) = cx.add_window_view(|_, cx| Harness {
                state: cx.new(|_| {
                    let mut s = VirtualizerState::new(
                        Axis::Vertical,
                        Rc::new(Source {
                            count: 1000,
                            calls: calls.clone(),
                        }),
                        VirtualizerSizing::measured(px(20.)),
                    )
                    .unwrap();
                    s.set_layout(VirtualizerLayout::default().overscan(px(100.)))
                        .unwrap();
                    s
                }),
                builds: builds.clone(),
            });
            cx.update(|w, _| w.activate_window());
            cx.run_until_parked();
            cx.update(|w, cx| w.draw(cx).clear(cx));
            let mut times = Vec::new();
            let (mut allocations, mut peak, mut reads, mut roots) = (0, 0, 0, 0);
            let mut final_error = 0f64;
            for step in 0..180 {
                calls.set(0);
                builds.set(0);
                let key = if step / 60 % 2 == 0 {
                    if trace == "tail" { 900usize } else { 2usize }
                } else {
                    0usize
                };
                let (ns, stats) = measured(|| {
                    view.update(cx, |v, cx| {
                        if step % 60 == 0 && trace != "idle" {
                            let motion = VirtualizerMotion::Smooth {
                                duration: std::time::Duration::from_millis(200),
                                max_distance: px(100.),
                            };
                            #[cfg(feature = "measured-motion")]
                            let motion = if trace == "tail" {
                                VirtualizerMotion::JumpThenSmooth {
                                    duration: std::time::Duration::from_millis(200),
                                    max_distance: px(100.),
                                }
                            } else {
                                motion
                            };
                            v.state.update(cx, |s, _| {
                                s.scroll_to_key(
                                    key.into(),
                                    VirtualizerScrollOptions::default()
                                        .align(VirtualizerAlign::Start)
                                        .motion(motion)
                                        .reduced_motion(trace == "reduced"),
                                )
                                .unwrap();
                            });
                        }
                        cx.notify();
                    });
                    cx.executor()
                        .advance_clock(std::time::Duration::from_millis(10));
                    cx.update(|w, cx| {
                        w.simulate_next_frame(cx);
                        w.draw(cx).clear(cx);
                    });
                });
                if step >= 20 {
                    times.push(ns);
                    allocations += stats.count;
                    peak = peak.max(stats.peak);
                    reads += calls.get();
                    roots += builds.get();
                }
                if step % 60 == 59 && trace != "idle" {
                    final_error = final_error.max(view.read_with(cx, |v, cx| {
                        (v.state.read(cx).viewport().unwrap().offset() - key as f64 * 20.).abs()
                    }));
                }
            }
            let total: u128 = times.iter().sum();
            times.sort_unstable();
            assert!(final_error < 0.01);
            println!(
                "SERIES,case=measured-{trace},sample={sample},ns={total},p50={},p95={},p99={},allocations={allocations},peak={peak},reads={reads},builds={roots},final_error={final_error}",
                times[80], times[152], times[158]
            );
        }
    }
}
