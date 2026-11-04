//! Identical production API workloads for each committed optimization experiment.
//! CPU/allocations are thread-local headless evidence, not GPU or physical FPS.
use gpui_kit::base::virtualization::*;
use gpui_kit::{
    AppContext as _, Axis, Context, ElementId, Entity, InputEvent, IntoElement, ParentElement as _,
    Render, ScrollDelta, ScrollWheelEvent, Styled, TestAppContext, TouchPhase, Window, div, point,
    px, size,
};
use std::{cell::Cell, hint::black_box, rc::Rc, time::Instant};

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
fn measured<T>(f: impl FnOnce() -> T) -> (T, u128, Allocations) {
    STATS.with(|s| {
        assert!(s.get().is_none());
        s.set(Some(Allocations::default()));
    });
    let t = Instant::now();
    let result = f();
    let elapsed = t.elapsed().as_nanos();
    let stats = STATS.with(|s| s.take().unwrap());
    (result, elapsed, stats)
}
struct Source {
    count: usize,
    calls: Rc<Cell<usize>>,
}
impl VirtualizerSource for Source {
    fn len(&self) -> usize {
        self.count
    }
    fn key_at(&self, index: usize) -> ElementId {
        self.calls.set(self.calls.get() + 1);
        index.into()
    }
    fn index_of(&self, key: &ElementId) -> Option<usize> {
        self.calls.set(self.calls.get() + 1);
        match key {
            ElementId::Integer(i) if (*i as usize) < self.count => Some(*i as usize),
            _ => None,
        }
    }
}
fn source(n: usize, calls: &Rc<Cell<usize>>) -> Rc<dyn VirtualizerSource> {
    Rc::new(Source {
        count: n,
        calls: calls.clone(),
    })
}
#[test]
fn series_working_items() {
    for mode in 0..2 {
        for retained in [false, true] {
            let calls = Rc::new(Cell::new(0));
            let mut state = VirtualizerState::new(
                Axis::Vertical,
                source(100_000, &calls),
                if mode == 0 {
                    VirtualizerSizing::fixed(px(20.))
                } else {
                    VirtualizerSizing::known(vec![px(20.); 100_000])
                },
            )
            .unwrap();
            if retained {
                state
                    .set_retained_keys(vec![1usize.into(), 99_999usize.into()])
                    .unwrap();
            }
            let v =
                VirtualizerViewport::new(state.binding().unwrap(), 1, 1000., px(600.), px(300.));
            let snapshot = state.snapshot(&v).unwrap();
            let expected = snapshot.working_indices();
            assert_eq!(
                snapshot
                    .working_items()
                    .map(|i| i.unwrap().index())
                    .collect::<Vec<_>>(),
                expected
            );
            for sample in 0..5 {
                calls.set(0);
                let (_, ns, stats) = measured(|| {
                    for _ in 0..10_000 {
                        let sum = snapshot
                            .working_items()
                            .map(|i| i.unwrap().index())
                            .sum::<usize>();
                        black_box(sum);
                    }
                });
                println!(
                    "SERIES,case=work-{mode}-{retained},sample={sample},ns={ns},p50=0,p95=0,p99=0,allocations={},peak={},reads={},builds=0",
                    stats.count,
                    stats.peak,
                    calls.get()
                );
            }
        }
    }
}
#[test]
fn series_masonry_noop() {
    let calls = Rc::new(Cell::new(0));
    let mut state = VirtualMasonryState::new(
        source(100_000, &calls),
        VirtualizerSizing::measured(px(40.)),
        3,
    )
    .unwrap();
    let updates: Vec<_> = (0usize..30)
        .map(|i| VirtualizerSizeUpdate::new(i, px(40.)))
        .collect();
    state.update_sizes(&updates).unwrap();
    let revision = state
        .snapshot(0., size(px(300.), px(600.)))
        .unwrap()
        .revision();
    for sample in 0..5 {
        calls.set(0);
        let (_, ns, stats) = measured(|| {
            for _ in 0..10_000 {
                state.update_sizes(&updates).unwrap();
            }
        });
        assert_eq!(
            state
                .snapshot(0., size(px(300.), px(600.)))
                .unwrap()
                .revision(),
            revision
        );
        println!(
            "SERIES,case=masonry-noop,sample={sample},ns={ns},p50=0,p95=0,p99=0,allocations={},peak={},reads={},builds=0",
            stats.count,
            stats.peak,
            calls.get()
        );
    }
}
enum State {
    List(Entity<VirtualizerState>),
    Masonry(Entity<VirtualMasonryState>),
}
struct Harness {
    state: State,
    builds: Rc<Cell<usize>>,
    width: f32,
    complex: bool,
    height: f32,
}
impl Render for Harness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let builds = self.builds.clone();
        let complex = self.complex;
        let height = self.height;
        let row = move |index: usize| {
            builds.set(builds.get() + 1);
            div().h(px(height)).children(
                (0..if complex { 12 } else { 0 })
                    .map(|i| div().child(format!("Row {index} field {i}"))),
            )
        };
        match &self.state {
            State::List(s) => VirtualList::new("series", s, move |i, _, _| row(i.index()))
                .w(px(self.width))
                .h(px(160.))
                .into_any_element(),
            State::Masonry(s) => {
                VirtualMasonry::new("series", s, move |i, _, _| row(i.item().index()))
                    .w(px(self.width))
                    .h(px(160.))
                    .into_any_element()
            }
        }
    }
}
#[gpui_kit::test]
fn series_trajectories(cx: &mut TestAppContext) {
    for mode in 0..3 {
        for trace in ["steady", "rapid", "complex", "resize", "size"] {
            if trace == "size" && mode < 2 {
                continue;
            }
            for sample in 0..3 {
                let calls = Rc::new(Cell::new(0));
                let builds = Rc::new(Cell::new(0));
                let (view, cx) = cx.add_window_view(|_, cx| Harness {
                    state: if mode < 2 {
                        State::List(cx.new(|_| {
                            VirtualizerState::new(
                                Axis::Vertical,
                                source(100_000, &calls),
                                if mode == 0 {
                                    VirtualizerSizing::fixed(px(40.))
                                } else {
                                    VirtualizerSizing::known(vec![px(40.); 100_000])
                                },
                            )
                            .unwrap()
                        }))
                    } else {
                        State::Masonry(cx.new(|_| {
                            VirtualMasonryState::new(
                                source(10_000, &calls),
                                VirtualizerSizing::measured(px(40.)),
                                3,
                            )
                            .unwrap()
                        }))
                    },
                    builds: builds.clone(),
                    width: 360.,
                    complex: trace == "complex",
                    height: 40.,
                });
                cx.update(|w, cx| w.draw(cx).clear(cx));
                let mut times = Vec::with_capacity(120);
                let mut allocations = 0;
                let mut peak = 0;
                let mut reads = 0;
                let mut roots = 0;
                for step in 0..140 {
                    calls.set(0);
                    builds.set(0);
                    let (_, ns, stats) = measured(|| {
                        view.update(cx, |h, cx| {
                            if trace == "resize" {
                                h.width = if step % 2 == 0 { 360. } else { 420. };
                            }
                            if trace == "size" && step % 10 == 0 {
                                h.height = if h.height == 40. { 60. } else { 40. };
                                if let State::Masonry(s) = &h.state {
                                    s.update(cx, |s, _| {
                                        s.set_layout(
                                            VirtualizerLayout::default().revision(step),
                                            px(0.),
                                        )
                                        .unwrap()
                                    });
                                }
                            }
                            if trace == "rapid" {
                                let offset = if step % 2 == 0 { 120_000. } else { 10_000. };
                                match &h.state {
                                    State::List(s) => s
                                        .read(cx)
                                        .native_scroll_handle()
                                        .unwrap()
                                        .set_precise_offset(point(0., -offset)),
                                    State::Masonry(s) => s
                                        .read(cx)
                                        .scroll_handle()
                                        .set_precise_offset(point(0., -offset)),
                                }
                            }
                            cx.notify();
                        });
                        cx.update(|w, cx| {
                            if trace != "rapid" {
                                w.dispatch_event(
                                    ScrollWheelEvent {
                                        position: point(px(80.), px(80.)),
                                        delta: ScrollDelta::Pixels(point(
                                            px(0.),
                                            px(if step % 2 == 0 { -1. } else { 1. }),
                                        )),
                                        modifiers: Default::default(),
                                        touch_phase: TouchPhase::Moved,
                                    }
                                    .to_platform_input(),
                                    cx,
                                );
                            }
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
                }
                let total: u128 = times.iter().sum();
                times.sort_unstable();
                println!(
                    "SERIES,case=frames-{mode}-{trace},sample={sample},ns={total},p50={},p95={},p99={},allocations={allocations},peak={peak},reads={reads},builds={roots}",
                    times[60], times[114], times[118]
                );
            }
        }
    }
}
#[test]
fn series_append_snapshot() {
    for count in [10_000, 100_000, 1_000_000] {
        for sample in 0..5 {
            let calls = Rc::new(Cell::new(0));
            let mut state = VirtualMasonryState::new(
                source(count, &calls),
                VirtualizerSizing::fixed(px(20.)),
                3,
            )
            .unwrap();
            let snapshot = state.snapshot(0., size(px(300.), px(160.))).unwrap();
            calls.set(0);
            let (_, ns, stats) = measured(|| {
                state
                    .append_source(
                        source(count + 10, &calls),
                        VirtualizerSizing::fixed(px(20.)),
                    )
                    .unwrap()
            });
            assert!(snapshot.item(count).is_err());
            assert_eq!(snapshot.item(count - 1).unwrap().item().index(), count - 1);
            assert_eq!(
                state
                    .snapshot(0., size(px(300.), px(160.)))
                    .unwrap()
                    .item(count + 9)
                    .unwrap()
                    .item()
                    .index(),
                count + 9
            );
            println!(
                "SERIES,case=append-{count},sample={sample},ns={ns},p50=0,p95=0,p99=0,allocations={},peak={},reads={},builds=0",
                stats.count,
                stats.peak,
                calls.get()
            );
        }
    }
}
