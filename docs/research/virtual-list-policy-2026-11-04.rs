//! Consumer-policy probes; allocations/peak=-1 means not instrumented here.
use gpui_kit::base::ScrollbarHandle as _;
use gpui_kit::base::virtualization::*;
use gpui_kit::{
    AppContext as _, Axis, Context, ElementId, Entity, IntoElement, Render, Styled, TestAppContext,
    Window, div, point, px,
};
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
    time::Instant,
};

struct Source {
    n: usize,
    reads: Rc<Cell<usize>>,
}
impl VirtualizerSource for Source {
    fn len(&self) -> usize {
        self.n
    }
    fn key_at(&self, ix: usize) -> ElementId {
        self.reads.set(self.reads.get() + 1);
        ix.into()
    }
    fn index_of(&self, key: &ElementId) -> Option<usize> {
        self.reads.set(self.reads.get() + 1);
        match key {
            ElementId::Integer(ix) if (*ix as usize) < self.n => Some(*ix as usize),
            _ => None,
        }
    }
}
struct Harness {
    state: Entity<VirtualizerState>,
    seen: Rc<RefCell<HashSet<usize>>>,
    builds: Rc<Cell<usize>>,
}
impl Render for Harness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let seen = self.seen.clone();
        let builds = self.builds.clone();
        VirtualList::new("policy", &self.state, move |item, _, _| {
            seen.borrow_mut().insert(item.index());
            builds.set(builds.get() + 1);
            div().h(px(20.))
        })
        .w(px(300.))
        .h(px(100.))
    }
}

#[gpui_kit::test]
fn policy_directional_buffer(cx: &mut TestAppContext) {
    for mode in 0..3 {
        for trace in ["forward", "reversal", "jump"] {
            for directional in [false, true] {
                for sample in 0..5 {
                    let direction = Rc::new(Cell::new(1i8));
                    let reads = Rc::new(Cell::new(0));
                    let builds = Rc::new(Cell::new(0));
                    let seen = Rc::new(RefCell::new(HashSet::new()));
                    let (view, cx) = cx.add_window_view(|_, cx| Harness {
                        state: cx.new(|_| {
                            let mut state = VirtualizerState::new(
                                Axis::Vertical,
                                Rc::new(Source {
                                    n: 100_000,
                                    reads: reads.clone(),
                                }),
                                match mode {
                                    0 => VirtualizerSizing::fixed(px(20.)),
                                    1 => VirtualizerSizing::known(vec![px(20.); 100_000]),
                                    _ => VirtualizerSizing::measured(px(20.)),
                                },
                            )
                            .unwrap();
                            state
                                .set_layout(
                                    VirtualizerLayout::default().overscan(px(if directional {
                                        0.
                                    } else {
                                        80.
                                    })),
                                )
                                .unwrap();
                            if directional {
                                let direction = direction.clone();
                                state.set_range_extractor(Some(Rc::new(move |range| {
                                    let visible = range.visible_range();
                                    if direction.get() >= 0 {
                                        (visible.end..(visible.end + 4).min(range.count()))
                                            .collect()
                                    } else {
                                        (visible.start.saturating_sub(4)..visible.start).collect()
                                    }
                                })));
                            }
                            state
                        }),
                        seen: seen.clone(),
                        builds: builds.clone(),
                    });
                    cx.update(|w, cx| w.draw(cx).clear(cx));
                    let state = view.read_with(cx, |v, _| v.state.clone());
                    state.read_with(cx, |s, _| {
                        s.scrollbar_handle()
                            .unwrap()
                            .set_offset(point(px(0.), px(-2000.)))
                    });
                    cx.update(|w, cx| {
                        w.refresh();
                        w.draw(cx).clear(cx);
                    });
                    let mut previous = state.read_with(cx, |s, _| {
                        s.snapshot(s.viewport().unwrap()).unwrap().working_indices()
                    });
                    let mut offset = 2000.;
                    let mut duration = 0;
                    let mut blank = 0;
                    let mut prepared_misses = 0;
                    reads.set(0);
                    builds.set(0);
                    for frame in 0..120 {
                        let next = match trace {
                            "forward" => offset + 16.,
                            "reversal" => offset + if (frame / 10) % 2 == 0 { 16. } else { -16. },
                            _ => {
                                if frame % 2 == 0 {
                                    20_000.
                                } else {
                                    100_000.
                                }
                            }
                        };
                        direction.set(if next >= offset { 1 } else { -1 });
                        offset = next;
                        seen.borrow_mut().clear();
                        let started = Instant::now();
                        state.read_with(cx, |s, _| {
                            s.scrollbar_handle()
                                .unwrap()
                                .set_offset(point(px(0.), px(-offset as f32)))
                        });
                        cx.update(|w, cx| {
                            w.refresh();
                            w.draw(cx).clear(cx);
                        });
                        duration += started.elapsed().as_nanos();
                        let snapshot =
                            state.read_with(cx, |s, _| s.snapshot(s.viewport().unwrap()).unwrap());
                        for ix in snapshot.visible_range() {
                            blank += usize::from(!seen.borrow().contains(&ix));
                            prepared_misses += usize::from(!previous.contains(&ix));
                        }
                        previous = snapshot.working_indices();
                    }
                    assert_eq!(blank, 0, "no uncovered visible row after drawing");
                    println!(
                        "SERIES,case=policy-{mode}-{trace}-{directional},sample={sample},ns={duration},p50=0,p95=0,p99=0,allocations=-1,peak=-1,reads={},builds={},blank={blank},prepared_misses={prepared_misses},extent_error=0",
                        reads.get(),
                        builds.get()
                    );
                }
            }
        }
    }
}

#[test]
fn policy_bounded_estimate() {
    for distribution in ["uniform", "mixed", "biased-prefix"] {
        let height = |ix: usize| match distribution {
            "uniform" => 40.,
            "mixed" => {
                if ix % 2 == 0 {
                    20.
                } else {
                    60.
                }
            }
            _ => {
                if ix < 32 {
                    10.
                } else {
                    100.
                }
            }
        };
        let actual: f64 = (0..100_000).map(height).sum();
        for calibrated in [false, true] {
            for sample in 0..5 {
                let started = Instant::now();
                let estimate = if calibrated {
                    let mut values: Vec<f64> = (0..32).map(height).collect();
                    values.sort_by(f64::total_cmp);
                    (values[15] + values[16]) / 2.
                } else {
                    20.
                };
                let reads = Rc::new(Cell::new(0));
                let state = VirtualizerState::new(
                    Axis::Vertical,
                    Rc::new(Source { n: 100_000, reads }),
                    VirtualizerSizing::measured(px(estimate as f32)),
                )
                .unwrap();
                let error = (state.total_extent() - actual).abs();
                let duration = started.elapsed().as_nanos();
                println!(
                    "SERIES,case=estimate-{distribution}-{calibrated},sample={sample},ns={duration},p50=0,p95=0,p99=0,allocations=-1,peak=-1,reads={},builds=0,blank=0,prepared_misses=0,extent_error={error}",
                    if calibrated { 32 } else { 0 }
                );
            }
        }
    }
}
