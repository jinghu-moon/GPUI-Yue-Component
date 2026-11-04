//! Adoption experiments use the same workload against isolated Git baselines.
//! CPU/allocations are thread-local headless evidence, not GPU or physical FPS.
use gpui_kit::base::virtualization::*;
use gpui_kit::{
    AppContext as _, Axis, Context, ElementId, Entity, IntoElement, ParentElement as _, Render,
    Styled, TestAppContext, Window, div, point, px,
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

struct CacheHarness {
    state: Entity<VirtualMasonryState>,
    builds: Rc<Cell<usize>>,
    mounted: bool,
}
impl Render for CacheHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        if !self.mounted {
            return div().into_any_element();
        }
        let builds = self.builds.clone();
        VirtualMasonry::new("cache-bench", &self.state, move |_, _, _| {
            builds.set(builds.get() + 1);
            div().h(px(20.)).child("cached card")
        })
        .w(px(240.))
        .h(px(100.))
        .into_any_element()
    }
}
#[gpui_kit::test]
fn masonry_cache_workload(cx: &mut TestAppContext) {
    for estimate in [2., 20.] {
        for n in [300, 10_000] {
            for sample in 0..3 {
                let calls = Rc::new(Cell::new(0));
                let builds = Rc::new(Cell::new(0));
                let (view, cx) = cx.add_window_view(|_, cx| CacheHarness {
                    state: cx.new(|_| {
                        VirtualMasonryState::new(
                            source(n, &calls),
                            VirtualizerSizing::measured(px(estimate)),
                            3,
                        )
                        .unwrap()
                    }),
                    builds: builds.clone(),
                    mounted: true,
                });
                cx.update(|w, cx| w.draw(cx).clear(cx));
                #[cfg(feature = "masonry-cache")]
                let cache =
                    view.read_with(cx, |v, cx| v.state.read(cx).measurement_cache().unwrap());
                view.update(cx, |v, cx| {
                    v.mounted = false;
                    cx.notify();
                });
                cx.update(|w, cx| w.draw(cx).clear(cx));
                let mut times = Vec::new();
                let (mut allocations, mut peak, mut reads, mut roots) = (0, 0, 0, 0);
                for step in 0..30 {
                    builds.set(0);
                    calls.set(0);
                    let (_, ns, stats) = measured(|| {
                        view.update(cx, |v, cx| {
                            v.state = cx.new(|_| {
                                let state = VirtualMasonryState::new(
                                    source(n, &calls),
                                    VirtualizerSizing::measured(px(estimate)),
                                    3,
                                )
                                .unwrap();
                                #[cfg(feature = "masonry-cache")]
                                let state = {
                                    let mut state = state;
                                    state.restore_measurement_cache(&cache).unwrap();
                                    state
                                };
                                state
                            });
                            v.mounted = true;
                            cx.notify();
                        });
                        cx.update(|w, cx| w.draw(cx).clear(cx));
                    });
                    if step >= 10 {
                        times.push(ns);
                        allocations += stats.count;
                        peak = peak.max(stats.peak);
                        reads += calls.get();
                        roots += builds.get();
                    }
                    view.update(cx, |v, cx| {
                        v.mounted = false;
                        cx.notify();
                    });
                    cx.update(|w, cx| w.draw(cx).clear(cx));
                }
                let total: u128 = times.iter().sum();
                times.sort_unstable();
                println!(
                    "SERIES,case=masonry-cache-{n}-hint-{estimate},sample={sample},ns={total},p50={},p95={},p99={},allocations={allocations},peak={peak},reads={reads},builds={roots}",
                    times[10], times[19], times[19]
                );
            }
        }
    }
}

#[test]
fn masonry_cache_workload_sizes() {
    println!(
        "CONTROLLER_SIZES,linear={},masonry={},layout={},viewport={}",
        std::mem::size_of::<VirtualizerState>(),
        std::mem::size_of::<VirtualMasonryState>(),
        std::mem::size_of::<VirtualizerLayout>(),
        std::mem::size_of::<VirtualizerViewport>()
    );
}
