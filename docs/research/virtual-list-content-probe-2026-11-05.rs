//! Identical Release A/B workloads, including observations of the old defect.
//! Headless CPU and thread-local allocations, never GPU FPS or physical input latency.
use gpui_kit::InteractiveElement as _;
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

use gpui_kit::{
    AnyElement, App, AvailableSpace, Bounds, Element, GlobalElementId, InspectorElementId,
    LayoutId, ParentElement as _, Pixels, size,
};
struct Probe {
    inner: AnyElement,
    enabled: bool,
    largest: Rc<Cell<f32>>,
    probes: Rc<Cell<usize>>,
}
impl Element for Probe {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        w: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let id = self.inner.request_layout(w, cx);
        if self.enabled {
            let measured = self.inner.layout_as_root(
                size(AvailableSpace::MaxContent, AvailableSpace::MaxContent),
                w,
                cx,
            );
            self.largest
                .set(self.largest.get().max(f32::from(measured.width)));
            self.probes.set(self.probes.get() + 1);
        }
        (id, ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        w: &mut Window,
        cx: &mut App,
    ) {
        self.inner.prepaint_at(bounds.origin, w, cx);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        w: &mut Window,
        cx: &mut App,
    ) {
        self.inner.paint(w, cx);
    }
}
impl IntoElement for Probe {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
struct Harness {
    axes: [Entity<VirtualizerState>; 2],
    scroll: ScrollHandle,
    enabled: bool,
    largest: Rc<Cell<f32>>,
    probes: Rc<Cell<usize>>,
    builds: Rc<Cell<usize>>,
}
impl Render for Harness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let enabled = self.enabled;
        let largest = self.largest.clone();
        let probes = self.probes.clone();
        let builds = self.builds.clone();
        VirtualGrid::new(
            "content-probe",
            &self.axes[0],
            &self.axes[1],
            &self.scroll,
            move |cell, _, _| {
                builds.set(builds.get() + 1);
                Probe {
                    inner: div()
                        .id("probe")
                        .overflow_hidden()
                        .child(if cell.row().index() == 900 {
                            "Remote wide content ".repeat(64)
                        } else {
                            "Short content".to_owned()
                        })
                        .into_any_element(),
                    enabled,
                    largest: largest.clone(),
                    probes: probes.clone(),
                }
            },
        )
        .w(px(240.))
        .h(px(100.))
    }
}
#[gpui_kit::test]
fn content_probe_workload(cx: &mut TestAppContext) {
    for enabled in [false, true] {
        for sample in 0..3 {
            let calls = Rc::new(Cell::new(0));
            let builds = Rc::new(Cell::new(0));
            let probes = Rc::new(Cell::new(0));
            let largest = Rc::new(Cell::new(0.));
            let (view, cx) = cx.add_window_view(|_, cx| {
                let binding = VirtualizerBinding::new();
                Harness {
                    axes: [Axis::Vertical, Axis::Horizontal].map(|axis| {
                        cx.new(|_| {
                            VirtualizerState::new_attached(
                                axis,
                                Rc::new(Source {
                                    count: 1000,
                                    calls: calls.clone(),
                                }),
                                VirtualizerSizing::fixed(px(if axis == Axis::Vertical {
                                    20.
                                } else {
                                    50.
                                })),
                                &binding,
                            )
                            .unwrap()
                        })
                    }),
                    scroll: ScrollHandle::new(),
                    enabled: enabled && cfg!(feature = "content-probe"),
                    largest: largest.clone(),
                    probes: probes.clone(),
                    builds: builds.clone(),
                }
            });
            let mut times = Vec::new();
            let (mut allocations, mut peak, mut reads, mut roots, mut measures) = (0, 0, 0, 0, 0);
            for step in 0..180 {
                calls.set(0);
                builds.set(0);
                probes.set(0);
                let (ns, stats) = measured(|| {
                    view.update(cx, |_, cx| cx.notify());
                    cx.update(|w, cx| w.draw(cx).clear(cx));
                });
                if step >= 20 {
                    times.push(ns);
                    allocations += stats.count;
                    peak = peak.max(stats.peak);
                    reads += calls.get();
                    roots += builds.get();
                    measures += probes.get();
                }
            }
            let first = largest.get();
            view.update(cx, |v, cx| {
                v.scroll.set_precise_offset(point(0., -18000.));
                cx.notify();
            });
            cx.update(|w, cx| w.draw(cx).clear(cx));
            if enabled && cfg!(feature = "content-probe") {
                assert!(
                    largest.get() > first * 2.,
                    "unseen content cannot be inferred from observed cells: before={first}, after={}",
                    largest.get()
                );
            }
            let total: u128 = times.iter().sum();
            times.sort_unstable();
            println!(
                "SERIES,case=intrinsic-{enabled},sample={sample},ns={total},p50={},p95={},p99={},allocations={allocations},peak={peak},reads={reads},builds={roots},intrinsic_layouts={measures},unseen_width_delta={}",
                times[80],
                times[152],
                times[158],
                largest.get() - first
            );
        }
    }
}
