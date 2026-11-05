//! Identical production Grid workload for isolated before/after ABBA.
//! Validation is timed inside render; remaining draw work is reported separately.
use gpui_kit::base::virtualization::*;
use gpui_kit::{
    AppContext as _, Axis, Context, ElementId, Entity, IntoElement, Render, ScrollHandle, Styled,
    TestAppContext, Window, div, point, px,
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    rc::Rc,
    time::Instant,
};
type CellOutput = (usize, usize, usize, usize);

struct Keys;
impl VirtualizerSource for Keys {
    fn len(&self) -> usize {
        100_000
    }
    fn key_at(&self, index: usize) -> ElementId {
        index.into()
    }
    fn index_of(&self, key: &ElementId) -> Option<usize> {
        match key {
            ElementId::Integer(i) if *i < 100_000 => Some(*i as usize),
            _ => None,
        }
    }
}
struct Harness {
    axes: [Entity<VirtualizerState>; 2],
    scroll: ScrollHandle,
    #[cfg(not(feature = "span-cache"))]
    spans: Vec<VirtualGridSpan>,
    #[cfg(feature = "span-cache")]
    span_set: VirtualGridSpanSet,
    config_ns: Rc<Cell<u128>>,
    builds: Rc<Cell<usize>>,
    observed: Rc<RefCell<Vec<CellOutput>>>,
}
impl Render for Harness {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let builds = self.builds.clone();
        let observed = self.observed.clone();
        let grid = VirtualGrid::new(
            "span-cost",
            &self.axes[0],
            &self.axes[1],
            &self.scroll,
            move |cell, _, _| {
                builds.set(builds.get() + 1);
                assert_eq!(cell.extent().width, px(50. * cell.column_span() as f32));
                assert_eq!(cell.extent().height, px(20. * cell.row_span() as f32));
                observed.borrow_mut().push((
                    cell.row().index(),
                    cell.column().index(),
                    cell.row_span(),
                    cell.column_span(),
                ));
                div()
            },
        );
        let start = Instant::now();
        #[cfg(not(feature = "span-cache"))]
        let grid = grid.spans(&self.spans, cx).unwrap();
        #[cfg(feature = "span-cache")]
        let grid = grid.span_set(&self.span_set, cx).unwrap();
        self.config_ns
            .set(self.config_ns.get() + start.elapsed().as_nanos());
        grid.w(px(240.)).h(px(100.))
    }
}
fn oracle(count: usize, same_row: bool, x: f64, y: f64) -> BTreeSet<CellOutput> {
    let rows = (y / 20.).floor() as usize..((y + 100.) / 20.).ceil() as usize;
    let columns = (x / 50.).floor() as usize..((x + 240.) / 50.).ceil() as usize;
    let mut output = BTreeSet::new();
    for row in rows.clone() {
        for column in columns.clone() {
            let i = column / 3;
            let origin_row = if same_row { 0 } else { i * 3 };
            if i < count && column % 3 < 2 && (origin_row..origin_row + 2).contains(&row) {
                output.insert((origin_row, i * 3, 2, 2));
            } else {
                output.insert((row, column, 1, 1));
            }
        }
    }
    output
}
#[gpui_kit::test]
fn series_span_frames(cx: &mut TestAppContext) {
    for same_row in [false, true] {
        for count in [0, 100, 1000, 10_000] {
            let config = Rc::new(Cell::new(0));
            let builds = Rc::new(Cell::new(0));
            let observed = Rc::new(RefCell::new(Vec::new()));
            let (view, cx) = cx.add_window_view(|_, cx| {
                let binding = VirtualizerBinding::new();
                Harness {
                    axes: [Axis::Vertical, Axis::Horizontal].map(|axis| {
                        cx.new(|_| {
                            VirtualizerState::new_attached(
                                axis,
                                Rc::new(Keys),
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
                    #[cfg(feature = "span-cache")]
                    span_set: VirtualGridSpanSet::new(descriptors(count, same_row)),
                    #[cfg(not(feature = "span-cache"))]
                    spans: (0..count)
                        .map(|i| {
                            VirtualGridSpan::new(
                                VirtualGridCellKey::new(if same_row { 0 } else { i * 3 }, i * 3),
                                2,
                                2,
                            )
                        })
                        .collect(),
                    config_ns: config.clone(),
                    builds: builds.clone(),
                    observed: observed.clone(),
                }
            });
            cx.update(|w, cx| w.draw(cx).clear(cx));
            for sample in 0..5 {
                let mut total = Vec::new();
                let mut config_total = 0;
                let mut render_total = 0;
                builds.set(0);
                for step in 0..64 {
                    config.set(0);
                    observed.borrow_mut().clear();
                    let offset = ((step * 397) % 20_000) as f64;
                    let y = if same_row { 0. } else { offset * 0.4 };
                    let started = Instant::now();
                    view.update(cx, |v, cx| {
                        v.scroll.set_precise_offset(point(
                            -offset,
                            if same_row { 0. } else { -offset * 0.4 },
                        ));
                        cx.notify();
                    });
                    cx.update(|w, cx| w.draw(cx).clear(cx));
                    let elapsed = started.elapsed().as_nanos();
                    assert!(config.get() <= elapsed);
                    total.push(elapsed);
                    config_total += config.get();
                    render_total += elapsed - config.get();
                    let actual: BTreeSet<_> = observed.borrow().iter().copied().collect();
                    assert_eq!(
                        actual,
                        oracle(count, same_row, offset, y),
                        "count={count} same_row={same_row} step={step}"
                    );
                }
                total.sort_unstable();
                println!(
                    "SERIES,case=span-{same_row}-{count},sample={sample},ns={},p50={},p95={},p99={},allocations=unmeasured,peak=unmeasured,reads=unmeasured,builds={},config_ns={config_total},render_ns={render_total}",
                    total.iter().sum::<u128>(),
                    total[32],
                    total[60],
                    total[63],
                    builds.get()
                );
            }
        }
    }
}

fn descriptors(count: usize, same_row: bool) -> Vec<VirtualGridSpan> {
    (0..count)
        .map(|i| {
            VirtualGridSpan::new(
                VirtualGridCellKey::new(if same_row { 0 } else { i * 3 }, i * 3),
                2,
                2,
            )
        })
        .collect()
}
#[gpui_kit::test]
fn series_span_cold(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let binding=VirtualizerBinding::new();
        let axes=[Axis::Vertical,Axis::Horizontal].map(|axis|cx.new(|_|VirtualizerState::new_attached(axis,Rc::new(Keys),VirtualizerSizing::fixed(px(20.)),&binding).unwrap()));
        let scroll=ScrollHandle::new();
        for same_row in [false,true] {for count in [0,100,1000,10_000] {
            let template=descriptors(count,same_row);
            for sample in 0..5 {
                let mut total=Vec::new();
                for _ in 0..16 {
                    let start=Instant::now();
                    // Both versions pay for equivalent descriptor ownership.
                    let owned=template.clone();
                    let grid=VirtualGrid::new("cold",&axes[0],&axes[1],&scroll,|_,_,_|div());
                    #[cfg(not(feature="span-cache"))]
                    let grid=grid.spans(&owned,cx).unwrap();
                    #[cfg(feature="span-cache")]
                    let grid=grid.span_set(&VirtualGridSpanSet::new(owned),cx).unwrap();
                    std::hint::black_box(grid);
                    total.push(start.elapsed().as_nanos());
                }
                total.sort_unstable();
                println!("SERIES,case=cold-{same_row}-{count},sample={sample},ns={},p50={},p95={},p99={},allocations=unmeasured,peak=unmeasured,reads=unmeasured,builds=0,config_ns={},render_ns=0",total.iter().sum::<u128>(),total[8],total[15],total[15],total.iter().sum::<u128>());
            }
        }}
    });
}
