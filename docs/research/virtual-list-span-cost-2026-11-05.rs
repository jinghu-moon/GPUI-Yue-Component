//! Identical production Grid workload for isolated before/after ABBA.
//! Validation is timed inside render; remaining draw work is reported separately.
use gpui_kit::base::virtualization::*;
use gpui_kit::{
    AppContext as _, Axis, Context, ElementId, Entity, IntoElement, Render, ScrollHandle, Styled,
    TestAppContext, Window, div, point, px,
};
use std::{cell::Cell, rc::Rc, time::Instant};

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
    spans: Vec<VirtualGridSpan>,
    config_ns: Rc<Cell<u128>>,
    builds: Rc<Cell<usize>>,
}
impl Render for Harness {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let builds = self.builds.clone();
        let grid = VirtualGrid::new(
            "span-cost",
            &self.axes[0],
            &self.axes[1],
            &self.scroll,
            move |_, _, _| {
                builds.set(builds.get() + 1);
                div()
            },
        );
        let start = Instant::now();
        let grid = grid.spans(&self.spans, cx).unwrap();
        self.config_ns
            .set(self.config_ns.get() + start.elapsed().as_nanos());
        grid.w(px(240.)).h(px(100.))
    }
}
#[gpui_kit::test]
fn series_span_frames(cx: &mut TestAppContext) {
    for same_row in [false, true] {
        for count in [0, 100, 1000, 10_000] {
            let config = Rc::new(Cell::new(0));
            let builds = Rc::new(Cell::new(0));
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
                    let started = Instant::now();
                    view.update(cx, |v, cx| {
                        let offset = ((step * 397) % 20_000) as f64;
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
