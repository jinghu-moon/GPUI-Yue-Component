use super::*;
use gpui_kit::base::virtualization::{VirtualMasonry, VirtualMasonryState};
use std::cell::RefCell;

type PreparedRows = Rc<RefCell<Vec<(usize, gpui_kit::WeakEntity<MasonryRow>)>>>;
struct MasonryRow;
impl Render for MasonryRow {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().h(px(20.))
    }
}
struct MasonryPreparation {
    state: Entity<VirtualMasonryState>,
    rows: PreparedRows,
    revision: u64,
    mounted: bool,
    cached: bool,
    width: f32,
}
impl Render for MasonryPreparation {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        if !self.mounted {
            return div().into_any_element();
        }
        let rows = self.rows.clone();
        let prepare = move |identity: yue_component::VirtualizerItemIdentity,
                            _: &mut Window,
                            cx: &mut App| {
            let row = cx.new(|_| MasonryRow);
            rows.borrow_mut().push((identity.index(), row.downgrade()));
            row
        };
        let element = if self.cached {
            VirtualMasonry::new_with_item_views("adoption-masonry", &self.state, prepare)
                .content_revision(self.revision)
        } else {
            VirtualMasonry::new("adoption-masonry", &self.state, move |item, w, cx| {
                prepare(item.item().identity(), w, cx)
            })
        };
        element.w(px(self.width)).h(px(100.)).into_any_element()
    }
}
fn sizing(mode: usize, n: usize) -> VirtualizerSizing {
    match mode {
        0 => VirtualizerSizing::fixed(px(20.)),
        1 => VirtualizerSizing::known(vec![px(20.); n]),
        _ => VirtualizerSizing::measured(px(20.)),
    }
}
struct RevisionSource(u64);
impl VirtualizerSource for RevisionSource {
    fn len(&self) -> usize {
        1000
    }
    fn key_at(&self, ix: usize) -> ElementId {
        ix.into()
    }
    fn index_of(&self, key: &ElementId) -> Option<usize> {
        if let ElementId::Integer(ix) = key {
            (*ix < 1000).then_some(*ix as usize)
        } else {
            None
        }
    }
    fn revision_at(&self, ix: usize) -> u64 {
        if ix == 0 { self.0 } else { 0 }
    }
}

#[gpui_kit::test]
fn masonry_preparation_reuses_geometry_and_refreshes_only_explicit_content(
    cx: &mut TestAppContext,
) {
    for mode in 0..3 {
        let (view, cx) = cx.add_window_view(|_, cx| MasonryPreparation {
            state: cx.new(|_| {
                VirtualMasonryState::new(Rc::new(RevisionSource(0)), sizing(mode, 1000), 3).unwrap()
            }),
            rows: Rc::default(),
            revision: 0,
            mounted: true,
            cached: true,
            width: 300.,
        });
        cx.update(|w, cx| w.draw(cx).clear(cx));
        let (state, rows) = view.read_with(cx, |v, _| (v.state.clone(), v.rows.clone()));
        let initial = rows.borrow().len();
        for _ in 0..4 {
            cx.update(|w, cx| {
                w.refresh();
                w.draw(cx).clear(cx);
            });
        }
        assert_eq!(rows.borrow().len(), initial);
        view.update(cx, |v, cx| {
            v.width = 330.;
            cx.notify();
        });
        cx.update(|w, cx| w.draw(cx).clear(cx));
        assert_eq!(
            rows.borrow().len(),
            initial,
            "width changes must not refresh content"
        );
        state.update(cx, |s, cx| {
            assert_eq!(
                s.invalidate_items(&[0usize.into(), 9999usize.into()]),
                Err(VirtualizerError::MissingKey)
            );
            s.invalidate_items(&[0usize.into(), 0usize.into()]).unwrap();
            cx.notify();
        });
        cx.update(|w, cx| w.draw(cx).clear(cx));
        assert_eq!(rows.borrow().len(), initial + 1);
        assert_eq!(rows.borrow().last().unwrap().0, 0);
        let before = rows.borrow().len();
        state.update(cx, |s, cx| {
            s.append_source(source(0, 1001), sizing(mode, 1001))
                .unwrap();
            cx.notify();
        });
        cx.update(|w, cx| w.draw(cx).clear(cx));
        assert_eq!(
            rows.borrow().len(),
            before,
            "append preserves prefix content epochs"
        );
        // Dropping a local index domain establishes a new barrier, once only.
        state.update(cx, |s, cx| {
            s.replace_source(Rc::new(RevisionSource(0)), sizing(mode, 1000))
                .unwrap();
            cx.notify();
        });
        cx.update(|w, cx| w.draw(cx).clear(cx));
        let before = rows.borrow().len();
        state.update(cx, |s, cx| {
            s.replace_source(Rc::new(RevisionSource(1)), sizing(mode, 1000))
                .unwrap();
            cx.notify();
        });
        cx.update(|w, cx| w.draw(cx).clear(cx));
        assert_eq!(
            rows.borrow().len(),
            before + 1,
            "source revision refresh is item-local"
        );
        let before = rows.borrow().len();
        view.update(cx, |v, cx| {
            v.revision += 1;
            cx.notify();
        });
        cx.update(|w, cx| w.draw(cx).clear(cx));
        assert_eq!(rows.borrow().len() - before, initial);
    }
}

#[gpui_kit::test]
fn masonry_preparation_releases_evicted_swapped_and_unmounted_views(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| MasonryPreparation {
        state: cx.new(|_| VirtualMasonryState::new(source(0, 1000), sizing(2, 1000), 3).unwrap()),
        rows: Rc::default(),
        revision: 0,
        mounted: true,
        cached: true,
        width: 300.,
    });
    cx.update(|w, cx| w.draw(cx).clear(cx));
    let (state, rows) = view.read_with(cx, |v, _| (v.state.clone(), v.rows.clone()));
    let initial = rows.borrow().len();
    state.update(cx, |s, cx| {
        s.set_retained_keys(vec![0usize.into()]).unwrap();
        s.scroll_to_key(900usize.into(), VirtualizerAlign::Start)
            .unwrap();
        cx.notify();
    });
    cx.update(|w, cx| w.draw(cx).clear(cx));
    cx.run_until_parked();
    assert!(
        rows.borrow()[0].1.upgrade().is_some(),
        "remote retained row stays prepared"
    );
    assert!(
        rows.borrow()[1..initial]
            .iter()
            .all(|(_, row)| row.upgrade().is_none())
    );
    let old = rows.borrow().clone();
    view.update(cx, |v, cx| {
        v.state =
            cx.new(|_| VirtualMasonryState::new(source(0, 1000), sizing(2, 1000), 3).unwrap());
        cx.notify();
    });
    cx.update(|w, cx| w.draw(cx).clear(cx));
    cx.run_until_parked();
    assert!(
        old.iter().all(|(_, row)| row.upgrade().is_none()),
        "same mount id with a new owner releases old views"
    );
    view.update(cx, |v, cx| {
        v.cached = false;
        cx.notify();
    });
    let before = rows.borrow().len();
    cx.update(|w, cx| w.draw(cx).clear(cx));
    cx.update(|w, cx| {
        w.refresh();
        w.draw(cx).clear(cx);
    });
    assert!(
        rows.borrow().len() >= before + initial * 2,
        "ordinary renderer still runs every frame"
    );
    view.update(cx, |v, cx| {
        v.mounted = false;
        cx.notify();
    });
    for _ in 0..3 {
        cx.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.draw(cx).clear(cx);
        });
    }
    cx.run_until_parked();
    assert!(rows.borrow().iter().all(|(_, row)| row.upgrade().is_none()));
}

#[test]
fn sparse_measurement_export_preserves_order_constraints_and_frozen_geometry() {
    let list = ListState::new_with_uniform_height(10_000, ListAlignment::Top, px(0.), px(20.));
    assert!(list.geometry().measured_items().is_empty());
    let updates = [
        (9999, gpui_kit::size(px(240.), px(37.))),
        (0, gpui_kit::size(px(240.), px(21.))),
        (501, gpui_kit::size(px(240.), px(23.))),
    ];
    list.set_item_sizes(&updates, true);
    let frozen = list.geometry();
    let expected = vec![updates[1], updates[2], updates[0]];
    assert_eq!(frozen.measured_items(), expected);
    list.remeasure_items(500..502);
    assert_eq!(
        list.geometry().measured_items(),
        vec![updates[1], updates[0]]
    );
    assert_eq!(frozen.measured_items(), expected);
    list.set_item_sizes(&[(700, gpui_kit::size(px(300.), px(24.)))], false);
    assert_eq!(
        list.geometry().measured_items(),
        vec![updates[1], updates[0]]
    );
}
