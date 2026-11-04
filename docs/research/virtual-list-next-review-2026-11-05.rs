//! Observations at f0f7583, not a new behavior contract or an A/B speed claim.
//! Run in an isolated fixture with current vendor patches and test-support.
use gpui_kit::base::virtualization::*;
use gpui_kit::{
    AppContext as _, Axis, Context, ElementId, Entity, IntoElement, Render, ScrollHandle, Styled,
    TestAppContext, Window, div, px,
};
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
};

struct Source(usize);
impl VirtualizerSource for Source {
    fn len(&self) -> usize {
        self.0
    }
    fn key_at(&self, index: usize) -> ElementId {
        index.into()
    }
    fn index_of(&self, key: &ElementId) -> Option<usize> {
        match key {
            ElementId::Integer(value) if (*value as usize) < self.0 => Some(*value as usize),
            _ => None,
        }
    }
}
fn source() -> Rc<dyn VirtualizerSource> {
    Rc::new(Source(1_000_000))
}

struct GridHarness {
    rows: Entity<VirtualizerState>,
    columns: Entity<VirtualizerState>,
    scroll: ScrollHandle,
    calls: Rc<Cell<usize>>,
    unique: Rc<RefCell<HashSet<(usize, usize)>>>,
}
impl Render for GridHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let calls = self.calls.clone();
        let unique = self.unique.clone();
        VirtualGrid::new(
            "next-review-grid",
            &self.rows,
            &self.columns,
            &self.scroll,
            move |cell, _, _| {
                calls.set(calls.get() + 1);
                unique
                    .borrow_mut()
                    .insert((cell.row().index(), cell.column().index()));
                div()
            },
        )
        .w(px(240.))
        .h(px(100.))
    }
}

#[gpui_kit::test]
fn review_grid_stable_redraw_renderer_calls(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| {
        let binding = VirtualizerBinding::new();
        GridHarness {
            rows: cx.new(|_| {
                VirtualizerState::new_attached(
                    Axis::Vertical,
                    source(),
                    VirtualizerSizing::fixed(px(20.)),
                    &binding,
                )
                .unwrap()
            }),
            columns: cx.new(|_| {
                VirtualizerState::new_attached(
                    Axis::Horizontal,
                    source(),
                    VirtualizerSizing::fixed(px(50.)),
                    &binding,
                )
                .unwrap()
            }),
            scroll: ScrollHandle::new(),
            calls: Rc::default(),
            unique: Rc::default(),
        }
    });
    cx.update(|w, cx| w.draw(cx).clear(cx));
    let (calls, unique) = view.read_with(cx, |view, _| (view.calls.clone(), view.unique.clone()));
    calls.set(0);
    unique.borrow_mut().clear();
    // Explicitly dirty the owner; no data, dimensions or viewport change.
    let mut calls_per_draw = Vec::new();
    for _ in 0..10 {
        let before = calls.get();
        view.update(cx, |_, cx| cx.notify());
        cx.update(|w, cx| w.draw(cx).clear(cx));
        calls_per_draw.push(calls.get() - before);
    }
    assert_eq!(unique.borrow().len(), 25);
    // A draw cycle may perform multiple internal layout/prepaint passes.
    // Observe that fact instead of equating a draw with a displayed frame.
    assert!(calls_per_draw.iter().all(|calls| *calls >= 25));
    assert!(calls.get() > unique.borrow().len());
    println!(
        "REVIEW,case=grid-stable-redraw,draws=10,renderer_calls={},unique_cells={},calls_per_draw={calls_per_draw:?}",
        calls.get(),
        unique.borrow().len()
    );
}

#[test]
fn review_grid_symmetric_overscan_workset() {
    for (overscan, expected_rows, expected_columns) in [(0., 5, 5), (100., 15, 9), (200., 25, 13)] {
        let binding = VirtualizerBinding::new();
        let mut rows = VirtualizerState::new_attached(
            Axis::Vertical,
            source(),
            VirtualizerSizing::fixed(px(20.)),
            &binding,
        )
        .unwrap();
        let mut columns = VirtualizerState::new_attached(
            Axis::Horizontal,
            source(),
            VirtualizerSizing::fixed(px(50.)),
            &binding,
        )
        .unwrap();
        let layout = VirtualizerLayout::default().overscan(px(overscan));
        rows.set_layout(layout).unwrap();
        columns.set_layout(layout).unwrap();
        let rows = rows
            .snapshot(&VirtualizerViewport::new(
                &binding,
                1,
                1_000.,
                px(100.),
                px(240.),
            ))
            .unwrap();
        let columns = columns
            .snapshot(&VirtualizerViewport::new(
                &binding,
                1,
                1_000.,
                px(240.),
                px(100.),
            ))
            .unwrap();
        assert_eq!(rows.working_items().count(), expected_rows);
        assert_eq!(columns.working_items().count(), expected_columns);
        let cells = VirtualGridSnapshot::new(rows, columns)
            .unwrap()
            .cells()
            .count();
        assert_eq!(cells, expected_rows * expected_columns);
        println!(
            "REVIEW,case=grid-overscan,overscan={overscan},rows={expected_rows},columns={expected_columns},cells={cells}"
        );
    }
}
