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
