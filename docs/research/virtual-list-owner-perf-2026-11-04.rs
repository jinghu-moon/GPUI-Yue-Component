//! Shared Release A/B workload. Run in isolated before/after dependency fixtures.
use gpui_kit::base::virtualization::{
    VirtualList, VirtualMasonry, VirtualMasonryState, VirtualizerSizing, VirtualizerSource,
    VirtualizerState,
};
use gpui_kit::{
    AppContext as _, Axis, Context, ElementId, Entity, InputEvent, IntoElement, Render,
    ScrollDelta, ScrollWheelEvent, Styled, TestAppContext, TouchPhase, Window, div, point, px,
};
use std::{cell::Cell, rc::Rc, time::Instant};

struct Source {
    count: usize,
    reads: Rc<Cell<usize>>,
}
impl VirtualizerSource for Source {
    fn len(&self) -> usize {
        self.count
    }
    fn key_at(&self, index: usize) -> ElementId {
        self.reads.set(self.reads.get() + 1);
        index.into()
    }
    fn index_of(&self, key: &ElementId) -> Option<usize> {
        self.reads.set(self.reads.get() + 1);
        match key {
            ElementId::Integer(ix) if (*ix as usize) < self.count => Some(*ix as usize),
            _ => None,
        }
    }
}
enum State {
    List(Entity<VirtualizerState>),
    Masonry(Entity<VirtualMasonryState>),
}
struct Harness {
    state: State,
    builds: Rc<Cell<usize>>,
}
impl Render for Harness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let builds = self.builds.clone();
        match &self.state {
            State::List(state) => VirtualList::new("perf-list", state, move |_, _, _| {
                builds.set(builds.get() + 1);
                div().h(px(20.))
            })
            .w(px(240.))
            .h(px(100.))
            .into_any_element(),
            State::Masonry(state) => VirtualMasonry::new("perf-masonry", state, move |_, _, _| {
                builds.set(builds.get() + 1);
                div().h(px(40.))
            })
            .w(px(300.))
            .h(px(100.))
            .into_any_element(),
        }
    }
}
#[gpui_kit::test]
fn steady_input_frames(cx: &mut TestAppContext) {
    for mode in 0..3 {
        for sample in 0..5 {
            let reads = Rc::new(Cell::new(0));
            let builds = Rc::new(Cell::new(0));
            let (view, cx) = cx.add_window_view(|_, cx| {
                let source = Rc::new(Source {
                    count: if mode < 2 { 1_000_000 } else { 10_000 },
                    reads: reads.clone(),
                });
                let state = if mode < 2 {
                    State::List(cx.new(|_| {
                        VirtualizerState::new(
                            Axis::Vertical,
                            source,
                            if mode == 0 {
                                VirtualizerSizing::fixed(px(20.))
                            } else {
                                VirtualizerSizing::known(vec![px(20.); 1_000_000])
                            },
                        )
                        .unwrap()
                    }))
                } else {
                    State::Masonry(cx.new(|_| {
                        VirtualMasonryState::new(source, VirtualizerSizing::measured(px(40.)), 3)
                            .unwrap()
                    }))
                };
                Harness {
                    state,
                    builds: builds.clone(),
                }
            });
            cx.update(|w, cx| w.draw(cx).clear(cx));
            let frame = |step: usize, cx: &mut gpui_kit::VisualTestContext| {
                cx.update(|w, cx| {
                    w.dispatch_event(
                        ScrollWheelEvent {
                            position: point(px(100.), px(50.)),
                            delta: ScrollDelta::Pixels(point(
                                px(0.),
                                px(if step.is_multiple_of(2) { -1. } else { 1. }),
                            )),
                            modifiers: Default::default(),
                            touch_phase: TouchPhase::Moved,
                        }
                        .to_platform_input(),
                        cx,
                    );
                    w.draw(cx).clear(cx);
                });
            };
            for step in 0..40 {
                frame(step, cx);
            }
            reads.set(0);
            builds.set(0);
            let started = Instant::now();
            for step in 0..3000 {
                frame(step, cx);
            }
            let micros = started.elapsed().as_micros();
            let (offset, count) = view.read_with(cx, |v, cx| match &v.state {
                State::List(s) => (s.read(cx).viewport().unwrap().offset(), 1_000_000),
                State::Masonry(s) => (-f64::from(s.read(cx).scroll_handle().offset().y), 10_000),
            });
            assert_eq!(offset, 0.);
            println!(
                "OWNER_PERF,mode={mode},sample={sample},items={count},frames=3000,micros={micros},reads={},builds={}",
                reads.get(),
                builds.get()
            );
        }
    }
}
