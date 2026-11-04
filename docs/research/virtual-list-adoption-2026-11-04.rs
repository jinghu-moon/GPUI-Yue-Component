//! Adoption experiments use the same workload against isolated Git baselines.
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
fn adopt_cache_export() {
    for (n, measured_count) in [(1000, 0), (1_000_000, 64), (10_000, 10_000)] {
        let list = gpui_kit::ListState::new_with_uniform_height(
            n,
            gpui_kit::ListAlignment::Top,
            px(0.),
            px(20.),
        );
        let updates: Vec<_> = (0..measured_count)
            .map(|i| {
                (
                    (i * n / measured_count),
                    size(px(300.), px(20. + (i % 3) as f32)),
                )
            })
            .collect();
        list.set_item_sizes(&updates, true);
        let geometry = list.geometry();
        for sample in 0..5 {
            let (_, ns, stats) = measured(|| {
                for _ in 0..100 {
                    let values = geometry.measured_items();
                    assert_eq!(values.len(), measured_count);
                    for ((actual_ix, actual_size), expected) in values.iter().zip(&updates) {
                        assert_eq!((*actual_ix, *actual_size), *expected);
                    }
                    black_box(values);
                }
            });
            println!(
                "SERIES,case=cache-{n}-{measured_count},sample={sample},ns={ns},p50=0,p95=0,p99=0,allocations={},peak={},reads=0,builds=0",
                stats.count, stats.peak
            );
        }
    }
}

struct Prepared {
    payload: Vec<String>,
}
impl Render for Prepared {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .h(px(40.))
            .child(format!("{} fields", self.payload.len()))
    }
}
struct PreparationHarness {
    state: Entity<VirtualMasonryState>,
    calls: Rc<Cell<usize>>,
}
impl Render for PreparationHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let calls = self.calls.clone();
        let prepare =
            move |identity: VirtualizerItemIdentity, _: &mut Window, cx: &mut gpui_kit::App| {
                calls.set(calls.get() + 1);
                cx.new(|_| Prepared {
                    payload: (0..256)
                        .map(|i| {
                            format!(
                                "record {} field {i}: immutable application data",
                                identity.index()
                            )
                        })
                        .collect(),
                })
            };
        #[cfg(feature = "masonry-views")]
        let list = VirtualMasonry::new_with_item_views("prepared", &self.state, prepare);
        #[cfg(not(feature = "masonry-views"))]
        let list = VirtualMasonry::new("prepared", &self.state, move |item, w, cx| {
            prepare(item.item().identity(), w, cx)
        });
        list.w(px(360.)).h(px(160.))
    }
}
#[gpui_kit::test]
fn adopt_masonry_preparation(cx: &mut TestAppContext) {
    for rapid in [false, true] {
        for sample in 0..5 {
            let factories = Rc::new(Cell::new(0));
            let reads = Rc::new(Cell::new(0));
            let (view, cx) = cx.add_window_view(|_, cx| PreparationHarness {
                state: cx.new(|_| {
                    VirtualMasonryState::new(
                        source(100_000, &reads),
                        VirtualizerSizing::measured(px(40.)),
                        3,
                    )
                    .unwrap()
                }),
                calls: factories.clone(),
            });
            cx.update(|w, cx| w.draw(cx).clear(cx));
            let state = view.read_with(cx, |v, _| v.state.clone());
            factories.set(0);
            reads.set(0);
            let (_, ns, stats) = measured(|| {
                for frame in 0..120 {
                    if rapid {
                        state.read_with(cx, |s, _| {
                            s.scroll_handle().set_precise_offset(point(
                                0.,
                                -if frame % 2 == 0 { 10000. } else { 120000. },
                            ))
                        });
                    }
                    cx.update(|w, cx| {
                        w.refresh();
                        w.draw(cx).clear(cx);
                    });
                }
            });
            println!(
                "SERIES,case=prepare-{rapid},sample={sample},ns={ns},p50=0,p95=0,p99=0,allocations={},peak={},reads={},builds={}",
                stats.count,
                stats.peak,
                reads.get(),
                factories.get()
            );
        }
    }
}

struct ResizeHarness {
    state: Entity<VirtualizerState>,
    width: f32,
}
impl Render for ResizeHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        VirtualList::new("resize", &self.state, |_, _, _| div().h(px(20.)))
            .w(px(self.width))
            .h(px(100.))
    }
}
#[gpui_kit::test]
fn adopt_width_resize(cx: &mut TestAppContext) {
    for n in [1000, 100_000, 1_000_000] {
        for sample in 0..3 {
            let reads = Rc::new(Cell::new(0));
            let (view, cx) = cx.add_window_view(|_, cx| ResizeHarness {
                state: cx.new(|_| {
                    VirtualizerState::new(
                        Axis::Vertical,
                        source(n, &reads),
                        VirtualizerSizing::measured(px(20.)),
                    )
                    .unwrap()
                }),
                width: 300.,
            });
            cx.update(|w, cx| w.draw(cx).clear(cx));
            let (_, ns, stats) = measured(|| {
                for frame in 0..40 {
                    view.update(cx, |v, cx| {
                        v.width = if frame % 2 == 0 { 360. } else { 300. };
                        cx.notify();
                    });
                    cx.update(|w, cx| w.draw(cx).clear(cx));
                }
            });
            println!(
                "SERIES,case=resize-{n},sample={sample},ns={ns},p50=0,p95=0,p99=0,allocations={},peak={},reads={},builds=0",
                stats.count,
                stats.peak,
                reads.get()
            );
        }
    }
}

#[test]
fn adopt_metadata_memory() {
    for n in [1000, 100_000, 1_000_000] {
        for sample in 0..3 {
            let calls = Rc::new(Cell::new(0));
            let (state, ns, stats) = measured(|| {
                VirtualizerState::new(
                    Axis::Vertical,
                    source(n, &calls),
                    VirtualizerSizing::measured(px(20.)),
                )
                .unwrap()
            });
            assert_eq!(state.total_extent(), n as f64 * 20.);
            println!(
                "SERIES,case=memory-{n},sample={sample},ns={ns},p50=0,p95=0,p99=0,allocations={},peak={},reads={},builds={}",
                stats.count,
                stats.peak,
                calls.get(),
                stats.live
            );
        }
    }
}

#[test]
fn adopt_masonry_index_memory() {
    for n in [10_000, 100_000, 1_000_000] {
        for sample in 0..5 {
            let reads = Rc::new(Cell::new(0));
            let (state, ns, stats) = measured(|| {
                VirtualMasonryState::new(source(n, &reads), VirtualizerSizing::fixed(px(20.)), 3)
                    .unwrap()
            });
            assert_eq!(state.total_extent(), n.div_ceil(3) as f64 * 20.);
            println!(
                "SERIES,case=masonry-index-memory-{n},sample={sample},ns={ns},p50=0,p95=0,p99=0,allocations={},peak={},reads={},builds={}",
                stats.count,
                stats.peak,
                reads.get(),
                stats.live
            );
        }
    }
}

#[gpui_kit::test]
fn adopt_focus_memory(cx: &mut TestAppContext) {
    // Include Rc allocation overhead when focus metadata is unusually dense.
    for focused in [64, 10_000] {
        for sample in 0..5 {
            let handles =
                cx.update(|cx| (0..focused).map(|_| cx.focus_handle()).collect::<Vec<_>>());
            let (state, ns, stats) = measured(|| {
                let state = gpui_kit::ListState::new(0, gpui_kit::ListAlignment::Top, px(0.));
                state.splice_focusable(0..0, (0..10_000).map(|ix| handles.get(ix).cloned()));
                state
            });
            assert_eq!(state.geometry().len(), 10_000);
            println!(
                "SERIES,case=focus-memory-{focused},sample={sample},ns={ns},p50=0,p95=0,p99=0,allocations={},peak={},reads=0,builds={}",
                stats.count, stats.peak, stats.live
            );
        }
    }
}

struct NativeResizeHarness {
    list: gpui_kit::ListState,
    width: f32,
}
impl Render for NativeResizeHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        gpui_kit::list(self.list.clone(), |_, _, _| {
            div().h(px(20.)).into_any_element()
        })
        .w(px(self.width))
        .h(px(100.))
    }
}
#[gpui_kit::test]
fn adopt_width_dense(cx: &mut TestAppContext) {
    for n in [10_000, 100_000] {
        for sample in 0..5 {
            let state = gpui_kit::ListState::new_with_uniform_height(
                n,
                gpui_kit::ListAlignment::Top,
                px(0.),
                px(20.),
            );
            let (view, cx) = cx.add_window_view(|_, _| NativeResizeHarness {
                list: state.clone(),
                width: 300.,
            });
            cx.update(|w, cx| w.draw(cx).clear(cx));
            state.set_item_sizes(
                &(0..n)
                    .map(|ix| (ix, size(px(300.), px(20.))))
                    .collect::<Vec<_>>(),
                true,
            );
            let frozen = state.geometry();
            let (_, ns, stats) = measured(|| {
                view.update(cx, |v, cx| {
                    v.width = 360.;
                    cx.notify();
                });
                cx.update(|w, cx| w.draw(cx).clear(cx));
            });
            assert_eq!(frozen.measured_count(), n);
            assert!(state.geometry().measured_count() < 20);
            println!(
                "SERIES,case=resize-dense-{n},sample={sample},ns={ns},p50=0,p95=0,p99=0,allocations={},peak={},reads=0,builds=0",
                stats.count, stats.peak
            );
        }
    }
}

struct MasonryResizeHarness {
    state: Entity<VirtualMasonryState>,
    width: f32,
    builds: Rc<Cell<usize>>,
}
impl Render for MasonryResizeHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let builds = self.builds.clone();
        VirtualMasonry::new("next-resize", &self.state, move |_, _, _| {
            builds.set(builds.get() + 1);
            div().h(px(20.))
        })
        .w(px(self.width))
        .h(px(100.))
    }
}
#[gpui_kit::test]
fn next_masonry_width(cx: &mut TestAppContext) {
    for (n, measured_count) in [
        (1000, 1),
        (100_000, 64),
        (1_000_000, 64),
        (100_000, 100_000),
    ] {
        for sample in 0..3 {
            let reads = Rc::new(Cell::new(0));
            let builds = Rc::new(Cell::new(0));
            let (view, cx) = cx.add_window_view(|_, cx| MasonryResizeHarness {
                state: cx.new(|_| {
                    VirtualMasonryState::new(
                        source(n, &reads),
                        VirtualizerSizing::measured(px(20.)),
                        3,
                    )
                    .unwrap()
                }),
                width: 300.,
                builds: builds.clone(),
            });
            cx.update(|w, cx| w.draw(cx).clear(cx));
            let state = view.read_with(cx, |v, _| v.state.clone());
            state.update(cx, |s, _| {
                s.update_sizes(
                    &(0..measured_count)
                        .map(|i| VirtualizerSizeUpdate::new(i * n / measured_count, px(37.)))
                        .collect::<Vec<_>>(),
                )
                .unwrap()
            });
            let frozen =
                state.read_with(cx, |s, _| s.snapshot(0., size(px(300.), px(100.))).unwrap());
            reads.set(0);
            builds.set(0);
            let (_, ns, stats) = measured(|| {
                for frame in 0..40 {
                    view.update(cx, |v, cx| {
                        v.width = if frame % 2 == 0 { 360. } else { 300. };
                        cx.notify();
                    });
                    cx.update(|w, cx| w.draw(cx).clear(cx));
                }
            });
            assert!(frozen.item(0).unwrap().item().is_measured());
            assert_eq!(frozen.item(0).unwrap().item().extent(), px(37.));
            println!(
                "SERIES,case=masonry-width-{n}-{measured_count},sample={sample},ns={ns},p50=0,p95=0,p99=0,allocations={},peak={},reads={},builds={}",
                stats.count,
                stats.peak,
                reads.get(),
                builds.get()
            );
        }
    }
}

#[test]
fn next_native_batch() {
    // order 0 is ascending, 1 descending, 2 ascending then duplicate overrides.
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
                for _ in 0..20 {
                    state.set_item_sizes(&updates, true);
                }
            });
            assert_eq!(state.geometry().measured_count(), count);
            assert_eq!(frozen.measured_count(), 0);
            assert_eq!(
                state.geometry().item(0).unwrap().extent(),
                px(if order == 2 { 40. } else { 30. })
            );
            println!(
                "SERIES,case=native-batch-{count}-{sparse}-{order},sample={sample},ns={ns},p50=0,p95=0,p99=0,allocations={},peak={},reads=0,builds=0",
                stats.count, stats.peak
            );
        }
    }
}

#[test]
fn four_masonry_feedback() {
    for count in [0, 1, 30, 32, 33, 256] {
        for changes in [false, true] {
            for sample in 0..5 {
                let calls = Rc::new(Cell::new(0));
                let mut state = VirtualMasonryState::new(
                    source(100_000, &calls),
                    VirtualizerSizing::measured(px(40.)),
                    3,
                )
                .unwrap();
                let mut updates = (0..count)
                    .rev()
                    .map(|ix| VirtualizerSizeUpdate::new(ix, px(40.)))
                    .collect::<Vec<_>>();
                state.update_sizes(&updates).unwrap();
                let frozen = state.snapshot(0., size(px(300.), px(600.))).unwrap();
                calls.set(0);
                let (_, ns, stats) = measured(|| {
                    for pass in 0..2_000 {
                        if changes {
                            for (ix, update) in updates.iter_mut().enumerate() {
                                *update = VirtualizerSizeUpdate::new(
                                    count - ix - 1,
                                    px(40. + (pass % 2) as f32),
                                );
                            }
                        }
                        state.update_sizes(&updates).unwrap();
                    }
                });
                let snapshot = state.snapshot(0., size(px(300.), px(600.))).unwrap();
                if count > 0 {
                    assert_eq!(
                        snapshot.item(0).unwrap().item().extent(),
                        px(if changes { 41. } else { 40. })
                    );
                    assert_eq!(frozen.item(0).unwrap().item().extent(), px(40.));
                }
                println!(
                    "SERIES,case=masonry-feedback-{count}-{changes},sample={sample},ns={ns},p50=0,p95=0,p99=0,allocations={},peak={},reads={},builds=0",
                    stats.count,
                    stats.peak,
                    calls.get()
                );
            }
        }
    }
}

#[test]
fn next_linear_feedback() {
    // Known, native actual measurements, and native hints share the public seam.
    for mode in 0..3 {
        for count in [0, 1, 32, 256] {
            for changes in [false, true] {
                for sample in 0..5 {
                    let calls = Rc::new(Cell::new(0));
                    let sizing = if mode == 0 {
                        VirtualizerSizing::known(vec![px(40.); 100_000])
                    } else {
                        VirtualizerSizing::measured(px(40.))
                    };
                    let mut state =
                        VirtualizerState::new(Axis::Vertical, source(100_000, &calls), sizing)
                            .unwrap();
                    state
                        .set_layout(VirtualizerLayout::default().gap(px(0.1)))
                        .unwrap();
                    let viewport = VirtualizerViewport::new(
                        state.binding().unwrap(),
                        1,
                        0.,
                        px(600.),
                        px(300.),
                    );
                    state.commit(&viewport).unwrap();
                    let mut updates = (0..count)
                        .rev()
                        .map(|ix| VirtualizerSizeUpdate::new(ix, px(40.)))
                        .collect::<Vec<_>>();
                    if mode == 2 {
                        state.update_size_hints(&updates).unwrap();
                    } else {
                        state.update_sizes(&updates).unwrap();
                    }
                    let frozen = state.snapshot(&viewport).unwrap();
                    calls.set(0);
                    let (_, ns, stats) = measured(|| {
                        for pass in 0..2_000 {
                            if changes {
                                for (ix, update) in updates.iter_mut().enumerate() {
                                    *update = VirtualizerSizeUpdate::new(
                                        count - ix - 1,
                                        px(40. + (pass % 2) as f32),
                                    );
                                }
                            }
                            if mode == 2 {
                                state.update_size_hints(&updates).unwrap();
                            } else {
                                state.update_sizes(&updates).unwrap();
                            }
                        }
                    });
                    let reads = calls.get();
                    if count > 0 {
                        let next = state.snapshot(&viewport).unwrap();
                        let expected = px(if changes { 41. } else { 40. });
                        // Native slots include a rounded f32 gap; the content
                        // projection is not more precise than that native slot.
                        assert!(
                            (f32::from(next.item(0).unwrap().extent()) - f32::from(expected)).abs()
                                < 0.00001
                        );
                        assert!(
                            (f32::from(frozen.item(0).unwrap().extent()) - 40.).abs() < 0.00001
                        );
                        assert_eq!(next.item(0).unwrap().is_measured(), mode != 2);
                    }
                    println!(
                        "SERIES,case=linear-feedback-{mode}-{count}-{changes},sample={sample},ns={ns},p50=0,p95=0,p99=0,allocations={},peak={},reads={reads},builds=0",
                        stats.count, stats.peak
                    );
                }
            }
        }
    }
}

#[test]
fn priority_linear_batches() {
    for mode in 0..3 {
        for count in [2, 32, 256, 4_096] {
            for order in 0..3 {
                for changes in [false, true] {
                    for sample in 0..5 {
                        let calls = Rc::new(Cell::new(0));
                        let sizing = if mode == 0 {
                            VirtualizerSizing::known(vec![px(40.); 100_000])
                        } else {
                            VirtualizerSizing::measured(px(40.))
                        };
                        let mut state =
                            VirtualizerState::new(Axis::Vertical, source(100_000, &calls), sizing)
                                .unwrap();
                        let viewport = VirtualizerViewport::new(
                            state.binding().unwrap(),
                            1,
                            0.,
                            px(600.),
                            px(300.),
                        );
                        state.commit(&viewport).unwrap();
                        let mut keys: Vec<_> = (0..count).map(|ix| ix * 7 + 5).collect();
                        if order == 1 {
                            keys.reverse();
                        }
                        if order == 2 {
                            for ix in (1..count).rev() {
                                keys.swap(ix, (ix * 177 + 31) % (ix + 1));
                            }
                        }
                        let mut updates: Vec<_> = keys
                            .iter()
                            .map(|&ix| VirtualizerSizeUpdate::new(ix, px(40.)))
                            .collect();
                        if mode == 2 {
                            state.update_size_hints(&updates).unwrap();
                        } else {
                            state.update_sizes(&updates).unwrap();
                        }
                        let frozen = state.snapshot(&viewport).unwrap();
                        let passes = if count == 4_096 { 64 } else { 2_000 };
                        calls.set(0);
                        let (_, ns, stats) = measured(|| {
                            for pass in 0..passes {
                                if changes {
                                    for (update, &ix) in updates.iter_mut().zip(&keys) {
                                        *update = VirtualizerSizeUpdate::new(
                                            ix,
                                            px(40. + (pass % 2) as f32),
                                        );
                                    }
                                }
                                if mode == 2 {
                                    state.update_size_hints(&updates).unwrap();
                                } else {
                                    state.update_sizes(&updates).unwrap();
                                }
                            }
                        });
                        let reads = calls.get();
                        assert_eq!(frozen.item(5).unwrap().extent(), px(40.));
                        assert_eq!(
                            state.snapshot(&viewport).unwrap().item(5).unwrap().extent(),
                            px(if changes { 41. } else { 40. })
                        );
                        println!(
                            "SERIES,case=priority-batch-{mode}-{count}-{order}-{changes}-{passes},sample={sample},ns={ns},p50=0,p95=0,p99=0,allocations={},peak={},reads={reads},builds=0",
                            stats.count, stats.peak
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn priority_masonry_scale() {
    for count in [10_000, 100_000, 1_000_000] {
        for append in [false, true] {
            for sample in 0..5 {
                let calls = Rc::new(Cell::new(0));
                let mut state = VirtualMasonryState::new(
                    source(count, &calls),
                    VirtualizerSizing::fixed(px(20.)),
                    3,
                )
                .unwrap();
                let mut frozen = Vec::with_capacity(20);
                calls.set(0);
                let (_, ns, stats) = measured(|| {
                    if append {
                        for pass in 0..20 {
                            frozen.push(state.snapshot(0., size(px(300.), px(100.))).unwrap());
                            state
                                .append_source(
                                    source(count + (pass + 1) * 10, &calls),
                                    VirtualizerSizing::fixed(px(20.)),
                                )
                                .unwrap();
                        }
                    } else {
                        let next = VirtualMasonryState::new(
                            source(count, &calls),
                            VirtualizerSizing::fixed(px(20.)),
                            3,
                        )
                        .unwrap();
                        black_box(next);
                    }
                });
                for (pass, old) in frozen.iter().enumerate() {
                    assert!(old.item(count + pass * 10).is_err());
                }
                println!(
                    "SERIES,case=priority-masonry-scale-{count}-{append},sample={sample},ns={ns},p50=0,p95=0,p99=0,allocations={},peak={},reads={},builds=0",
                    stats.count,
                    stats.peak,
                    calls.get()
                );
            }
        }
    }
}
