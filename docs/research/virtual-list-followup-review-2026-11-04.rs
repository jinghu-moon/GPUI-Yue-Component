//! Independent observations, not regression expectations for desired behavior.
//! See the accompanying follow-up review for setup, raw output and limitations.
use gpui_kit::base::{TestSupportExt as _, test_support::snapshots, virtualization::VirtualList};
use gpui_kit::{
    AppContext as _, Axis, Context, ElementId, Entity, InteractiveElement as _, IntoElement,
    Render, Role, StatefulInteractiveElement as _, Styled, TestAppContext, Window, div, px, size,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};
use yue_component::{
    VirtualGridSnapshot, VirtualMasonry, VirtualMasonryState, VirtualizerAlign, VirtualizerChange,
    VirtualizerEvent, VirtualizerLayout, VirtualizerMotion, VirtualizerScrollOptions,
    VirtualizerSizeUpdate, VirtualizerSizing, VirtualizerSource, VirtualizerState,
    VirtualizerViewport,
};

struct Source {
    count: usize,
    reads: Rc<Cell<usize>>,
}
impl VirtualizerSource for Source {
    fn len(&self) -> usize {
        self.count
    }
    fn key_at(&self, ix: usize) -> ElementId {
        self.reads.set(self.reads.get() + 1);
        ix.into()
    }
    fn index_of(&self, key: &ElementId) -> Option<usize> {
        self.reads.set(self.reads.get() + 1);
        match key {
            ElementId::Integer(ix) => ((*ix as usize) < self.count).then_some(*ix as usize),
            _ => None,
        }
    }
}
fn source(count: usize) -> Rc<Source> {
    Rc::new(Source {
        count,
        reads: Rc::default(),
    })
}

struct PreparedRow(u64);
impl Render for PreparedRow {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().h(px(20.))
    }
}
struct ContentHarness {
    state: Entity<VirtualizerState>,
    version: Rc<Cell<u64>>,
    row: Rc<RefCell<Option<Entity<PreparedRow>>>>,
    factories: Rc<Cell<usize>>,
}
impl Render for ContentHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let version = self.version.clone();
        let row = self.row.clone();
        let factories = self.factories.clone();
        VirtualList::new_with_item_views("followup-content", &self.state, move |item, _, cx| {
            factories.set(factories.get() + 1);
            let view = cx.new(|_| PreparedRow(version.get()));
            if item.index() == 0 {
                *row.borrow_mut() = Some(view.clone());
            }
            view
        })
        .w(px(240.))
        .h(px(100.))
    }
}
#[gpui_kit::test]
fn content_invalidation_before_append_observation(cx: &mut TestAppContext) {
    for mode in 0..3 {
        for order in [
            "invalidate_only",
            "append_then_invalidate",
            "invalidate_then_append",
        ] {
            let sizing = |count| match mode {
                0 => VirtualizerSizing::fixed(px(20.)),
                1 => VirtualizerSizing::known(vec![px(20.); count]),
                _ => VirtualizerSizing::measured(px(20.)),
            };
            let (view, cx) = cx.add_window_view(|_, cx| ContentHarness {
                state: cx.new(|_| {
                    VirtualizerState::new(Axis::Vertical, source(100), sizing(100)).unwrap()
                }),
                version: Rc::new(Cell::new(0)),
                row: Rc::default(),
                factories: Rc::default(),
            });
            cx.update(|w, cx| w.draw(cx).clear(cx));
            let state = view.read_with(cx, |v, _| {
                v.version.set(1);
                v.factories.set(0);
                v.state.clone()
            });
            state.update(cx, |s, cx| {
                if order != "append_then_invalidate" {
                    s.invalidate_items(0..1).unwrap();
                }
                if order != "invalidate_only" {
                    s.replace_source(
                        source(101),
                        sizing(101),
                        VirtualizerChange::Splice {
                            old: 100..100,
                            inserted: 1,
                        },
                    )
                    .unwrap();
                }
                if order == "append_then_invalidate" {
                    s.invalidate_items(0..1).unwrap();
                }
                cx.notify();
            });
            cx.update(|w, cx| w.draw(cx).clear(cx));
            let (value, factories) = view.read_with(cx, |v, cx| {
                (
                    v.row.borrow().as_ref().unwrap().read(cx).0,
                    v.factories.get(),
                )
            });
            println!(
                "FOLLOWUP_CONTENT_BATCH,mode={mode},order={order},requested=1,prepared={value},factories={factories}"
            );
        }
    }
}
fn viewport(state: &VirtualizerState, revision: u64, cross: f32) -> VirtualizerViewport {
    VirtualizerViewport::new(state.binding().unwrap(), revision, 0., px(100.), px(cross))
}

#[test]
fn grid_axis_metadata_reads_observation() {
    let rows_source = source(100_000);
    let columns_source = source(100_000);
    let binding = yue_component::VirtualizerBinding::new();
    let rows = VirtualizerState::new_attached(
        Axis::Vertical,
        rows_source.clone(),
        VirtualizerSizing::fixed(px(20.)),
        &binding,
    )
    .unwrap();
    let columns = VirtualizerState::new_attached(
        Axis::Horizontal,
        columns_source.clone(),
        VirtualizerSizing::fixed(px(20.)),
        &binding,
    )
    .unwrap();
    let grid = VirtualGridSnapshot::new(
        rows.snapshot(&VirtualizerViewport::new(
            &binding,
            1,
            0.,
            px(400.),
            px(400.),
        ))
        .unwrap(),
        columns
            .snapshot(&VirtualizerViewport::new(
                &binding,
                1,
                0.,
                px(400.),
                px(400.),
            ))
            .unwrap(),
    )
    .unwrap();
    rows_source.reads.set(0);
    columns_source.reads.set(0);
    let cells = grid.cells().count();
    println!(
        "FOLLOWUP_GRID,cells={cells},row_source_calls={},column_source_calls={}",
        rows_source.reads.get(),
        columns_source.reads.get()
    );
    rows_source.reads.set(0);
    columns_source.reads.set(0);
    let row_items: Vec<_> = grid.rows().working_items().map(Result::unwrap).collect();
    let col_items: Vec<_> = grid.columns().working_items().map(Result::unwrap).collect();
    let pairs = row_items
        .iter()
        .flat_map(|row| col_items.iter().map(move |col| (row, col)))
        .count();
    println!(
        "FOLLOWUP_GRID_ONCE,cells={pairs},row_source_calls={},column_source_calls={}",
        rows_source.reads.get(),
        columns_source.reads.get()
    );
}

#[test]
fn masonry_unmeasured_invalidation_observation() {
    for measured in [false, true] {
        let mut state =
            VirtualMasonryState::new(source(100), VirtualizerSizing::measured(px(20.)), 3).unwrap();
        if measured {
            state
                .update_sizes(&[VirtualizerSizeUpdate::new(0usize, px(20.))])
                .unwrap();
        }
        let old = state.snapshot(0., size(px(300.), px(100.))).unwrap();
        state.invalidate_items(&[0usize.into()]).unwrap();
        let current = state.snapshot(0., size(px(300.), px(100.))).unwrap();
        let accepted = state
            .update_sizes_from_snapshot(&old, &[VirtualizerSizeUpdate::new(0usize, px(99.))])
            .is_ok();
        println!(
            "FOLLOWUP_MASONRY_INVALIDATION,measured={measured},old_revision={},new_revision={},late_accepted={accepted}",
            old.revision(),
            current.revision()
        );
    }
}

struct MasonryHarness {
    state: Entity<VirtualMasonryState>,
    rendered: Rc<RefCell<Vec<usize>>>,
}
impl Render for MasonryHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let rendered = self.rendered.clone();
        VirtualMasonry::new("followup-masonry", &self.state, move |item, _, _| {
            rendered.borrow_mut().push(item.item().index());
            div()
                .id("card")
                .h(px(20.))
                .role(Role::ListItem)
                .test_support()
        })
        .w(px(300.))
        .h(px(100.))
    }
}
#[gpui_kit::test]
fn masonry_budget_rendering_observation(cx: &mut TestAppContext) {
    for measured in [false, true] {
        for budget in [2, 1024] {
            let (view, cx) = cx.add_window_view(|_, cx| {
                let mut state = VirtualMasonryState::new(
                    source(100),
                    if measured {
                        VirtualizerSizing::measured(px(20.))
                    } else {
                        VirtualizerSizing::fixed(px(20.))
                    },
                    3,
                )
                .unwrap();
                state
                    .set_layout(
                        VirtualizerLayout::default().measurement_budget(budget),
                        px(0.),
                    )
                    .unwrap();
                MasonryHarness {
                    state: cx.new(|_| state),
                    rendered: Rc::default(),
                }
            });
            let state = view.read_with(cx, |v, _| v.state.clone());
            for frame in 0..3 {
                view.read_with(cx, |v, _| v.rendered.borrow_mut().clear());
                let painted = cx.update(|w, cx| {
                    w.refresh();
                    w.draw(cx).clear(cx);
                    snapshots(w)
                        .iter()
                        .filter(|n| n.role() == Some(Role::ListItem))
                        .count()
                });
                let working = state.read_with(cx, |s, _| {
                    s.snapshot(0., size(px(300.), px(100.)))
                        .unwrap()
                        .working_indices()
                        .len()
                });
                let built = view.read_with(cx, |v, _| v.rendered.borrow().clone());
                println!(
                    "FOLLOWUP_MASONRY_BUDGET,measured={measured},budget={budget},frame={frame},working={working},painted={painted},built={built:?}"
                );
            }
        }
    }
}

#[test]
fn measurement_cache_cross_identity_observation() {
    let mut state = VirtualizerState::new(
        Axis::Vertical,
        source(100),
        VirtualizerSizing::measured(px(20.)),
    )
    .unwrap();
    state.commit(&viewport(&state, 1, 240.)).unwrap();
    state
        .update_sizes(&[VirtualizerSizeUpdate::new(0usize, px(60.))])
        .unwrap();
    let old = state.measurement_cache().unwrap();
    state
        .prepare(&viewport(&state, 2, 480.), Instant::now())
        .unwrap();
    let new = state.measurement_cache().unwrap();
    let mut restored = VirtualizerState::new(
        Axis::Vertical,
        source(100),
        VirtualizerSizing::measured(px(20.)),
    )
    .unwrap();
    let accepted = restored.restore_measurement_cache(&new).unwrap();
    println!(
        "FOLLOWUP_CACHE,measured_cross={},exported_cross={},extent={},restored={accepted}",
        f64::from(old.cross_extent()),
        f64::from(new.cross_extent()),
        f64::from(new.items()[0].extent())
    );
}

#[test]
fn smooth_noop_terminal_observation() {
    for smooth in [false, true] {
        let mut state = VirtualizerState::new(
            Axis::Vertical,
            source(100),
            VirtualizerSizing::fixed(px(20.)),
        )
        .unwrap();
        let v = viewport(&state, 1, 240.);
        state.commit(&v).unwrap();
        state
            .scroll_to_key(
                0usize.into(),
                VirtualizerScrollOptions::default()
                    .align(VirtualizerAlign::Start)
                    .motion(if smooth {
                        VirtualizerMotion::Smooth {
                            duration: Duration::from_millis(200),
                            max_distance: px(100.),
                        }
                    } else {
                        VirtualizerMotion::Instant
                    }),
            )
            .unwrap();
        state.take_events();
        let start = Instant::now();
        let mut confirmations = 0;
        for frame in 0..15 {
            let (_, request) = state
                .prepare(&v, start + Duration::from_millis(frame * 10))
                .unwrap();
            if let Some(request) = request {
                confirmations += 1;
                state.confirm(&request, request.offset(), &v).unwrap();
            }
            if !state.has_scroll_command() {
                break;
            }
        }
        let outcome: Vec<_> = state
            .take_events()
            .into_iter()
            .filter(|event| matches!(event, VirtualizerEvent::CommandFinished { .. }))
            .collect();
        println!("FOLLOWUP_NOOP,smooth={smooth},confirmations={confirmations},events={outcome:?}");
    }
}
