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

struct PreparedGridCell {
    payload: Vec<String>,
}
impl Render for PreparedGridCell {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().child(format!("{} fields", self.payload.len()))
    }
}
struct GridPreparation {
    rows: Entity<VirtualizerState>,
    columns: Entity<VirtualizerState>,
    scroll: gpui_kit::ScrollHandle,
    prepared: Rc<Cell<usize>>,
    payload: usize,
    revision: u64,
    width: f32,
}
impl Render for GridPreparation {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let prepared = self.prepared.clone();
        let payload = self.payload;
        let factory = move |row: VirtualizerItemIdentity,
                            column: VirtualizerItemIdentity,
                            _: &mut Window,
                            cx: &mut gpui_kit::App| {
            prepared.set(prepared.get() + 1);
            cx.new(|_| PreparedGridCell {
                payload: (0..payload)
                    .map(|ix| {
                        format!(
                            "record {} column {} field {ix}: immutable preparation",
                            row.index(),
                            column.index()
                        )
                    })
                    .collect(),
            })
        };
        #[cfg(feature = "grid-views")]
        let grid = VirtualGrid::new_with_cell_views(
            "bench-grid",
            &self.rows,
            &self.columns,
            &self.scroll,
            move |identity, w, cx| {
                factory(identity.row().clone(), identity.column().clone(), w, cx)
            },
        );
        #[cfg(not(feature = "grid-views"))]
        let grid = VirtualGrid::new(
            "bench-grid",
            &self.rows,
            &self.columns,
            &self.scroll,
            move |cell, w, cx| factory(cell.row().identity(), cell.column().identity(), w, cx),
        );
        #[cfg(feature = "grid-views")]
        let grid = grid.content_revision(self.revision);
        grid.w(px(self.width)).h(px(100.))
    }
}
#[gpui_kit::test]
fn directional_workload(cx: &mut TestAppContext) {
    for payload in [0] {
        for trace in ["forward", "backward", "alternating", "idle"] {
            for sample in 0..3 {
                let calls = Rc::new(Cell::new(0));
                let prepared = Rc::new(Cell::new(0));
                let (view, cx) = cx.add_window_view(|_, cx| {
                    let binding = VirtualizerBinding::new();
                    GridPreparation {
                        rows: cx.new(|_| {
                            VirtualizerState::new_attached(
                                Axis::Vertical,
                                source(100_000, &calls),
                                VirtualizerSizing::fixed(px(20.)),
                                &binding,
                            )
                            .map(|mut state| {
                                let layout = VirtualizerLayout::default().overscan(px(200.));
                                #[cfg(feature = "directional-overscan")]
                                let layout = layout.directional_overscan(px(40.));
                                state.set_layout(layout).unwrap();
                                state
                            })
                            .unwrap()
                        }),
                        columns: cx.new(|_| {
                            VirtualizerState::new_attached(
                                Axis::Horizontal,
                                source(1_000, &calls),
                                VirtualizerSizing::fixed(px(50.)),
                                &binding,
                            )
                            .map(|mut state| {
                                let layout = VirtualizerLayout::default().overscan(px(200.));
                                #[cfg(feature = "directional-overscan")]
                                let layout = layout.directional_overscan(px(40.));
                                state.set_layout(layout).unwrap();
                                state
                            })
                            .unwrap()
                        }),
                        scroll: gpui_kit::ScrollHandle::new(),
                        prepared: prepared.clone(),
                        payload,
                        revision: 0,
                        width: 240.,
                    }
                });
                cx.update(|w, cx| w.draw(cx).clear(cx));
                let mut times = Vec::new();
                let (mut allocations, mut peak, mut reads, mut builds) = (0, 0, 0, 0);
                for step in 0..100 {
                    calls.set(0);
                    prepared.set(0);
                    let (_, ns, stats) = measured(|| {
                        view.update(cx, |v, cx| {
                            if trace == "resize" {
                                v.width = if step % 2 == 0 { 240. } else { 280. };
                            }
                            if trace == "content" && step % 8 == 0 {
                                v.revision += 1;
                            }
                            let offset = match trace {
                                "forward" => 1_000. + step as f64 * 20.,
                                "backward" => 5_000. - step as f64 * 20.,
                                "alternating" => {
                                    if step % 2 == 0 {
                                        1_000.
                                    } else {
                                        2_000.
                                    }
                                }
                                _ => 1_000.,
                            };
                            v.scroll.set_precise_offset(point(-offset, -offset));
                            cx.notify();
                        });
                        cx.update(|w, cx| w.draw(cx).clear(cx));
                    });
                    if step >= 20 {
                        times.push(ns);
                        allocations += stats.count;
                        peak = peak.max(stats.peak);
                        reads += calls.get();
                        builds += prepared.get();
                    }
                }
                let total: u128 = times.iter().sum();
                times.sort_unstable();
                println!(
                    "SERIES,case=directional-grid-{trace},sample={sample},ns={total},p50={},p95={},p99={},allocations={allocations},peak={peak},reads={reads},builds={builds}",
                    times[40], times[76], times[79]
                );
            }
        }
    }
}
