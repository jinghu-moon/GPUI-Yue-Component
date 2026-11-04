//! Review observations, not regression expectations for the desired behavior.
use gpui_kit::base::virtualization::VirtualList;
use gpui_kit::{
    AppContext as _, Axis, Context, ElementId, Entity, IntoElement, ParentElement as _, Render,
    Styled, TestAppContext, Window, div, px,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Instant,
};
use yue_component::*;

struct Source(usize);
impl VirtualizerSource for Source {
    fn len(&self) -> usize {
        self.0
    }
    fn key_at(&self, ix: usize) -> ElementId {
        ix.into()
    }
    fn index_of(&self, key: &ElementId) -> Option<usize> {
        match key {
            ElementId::Integer(ix) => ((*ix as usize) < self.0).then_some(*ix as usize),
            _ => None,
        }
    }
}
fn source(n: usize) -> Rc<dyn VirtualizerSource> {
    Rc::new(Source(n))
}
fn viewport(s: &VirtualizerState) -> VirtualizerViewport {
    VirtualizerViewport::new(s.binding().unwrap(), 1, 0., px(100.), px(240.)).input_revision(0)
}

#[test]
fn alignment_policy_revision_probe() {
    let mut s = VirtualizerState::new(
        Axis::Vertical,
        source(100),
        VirtualizerSizing::fixed(px(20.)),
    )
    .unwrap();
    let v = viewport(&s);
    s.scroll_to_key(
        10usize.into(),
        VirtualizerScrollOptions::default().align(VirtualizerAlign::Start),
    )
    .unwrap();
    let (_, old) = s.prepare(&v, Instant::now()).unwrap();
    let old = old.unwrap();
    s.set_layout(VirtualizerLayout::default().scroll_padding(px(15.), px(10.)))
        .unwrap();
    let accepted = s.validate_request(&old, &v).is_ok();
    let (_, new) = s.prepare(&v, Instant::now()).unwrap();
    println!(
        "REVIEW_POLICY,old_request_valid={accepted},old_offset={},new_offset={}",
        old.offset(),
        new.unwrap().offset()
    );
}

struct Row(u64);
impl Render for Row {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().h(px(20.)).child(format!("{}", self.0))
    }
}
struct Harness {
    state: Entity<VirtualizerState>,
    version: Rc<Cell<u64>>,
    factories: Rc<Cell<usize>>,
    rows: Rc<RefCell<std::collections::HashMap<usize, gpui_kit::WeakEntity<Row>>>>,
}
impl Render for Harness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let version = self.version.clone();
        let factories = self.factories.clone();
        let rows = self.rows.clone();
        VirtualList::new_with_item_views("review", &self.state, move |item, _, cx| {
            factories.set(factories.get() + 1);
            let row = cx.new(|_| Row(version.get()));
            rows.borrow_mut().insert(item.index(), row.downgrade());
            row
        })
        .w(px(240.))
        .h(px(100.))
    }
}
#[gpui_kit::test]
fn content_preparation_identity_probe(cx: &mut TestAppContext) {
    for known in [false, true] {
        let (view, cx) = cx.add_window_view(|_, cx| Harness {
            state: cx.new(|_| {
                VirtualizerState::new(
                    Axis::Vertical,
                    source(1000),
                    if known {
                        VirtualizerSizing::known(vec![px(20.); 1000])
                    } else {
                        VirtualizerSizing::fixed(px(20.))
                    },
                )
                .unwrap()
            }),
            version: Rc::new(Cell::new(0)),
            factories: Rc::new(Cell::new(0)),
            rows: Rc::default(),
        });
        cx.update(|w, cx| w.draw(cx).clear(cx));
        let (s, factories, rows) = view.read_with(cx, |v, _| {
            v.version.set(1);
            (v.state.clone(), v.factories.clone(), v.rows.clone())
        });
        factories.set(0);
        s.update(cx, |s, cx| {
            s.invalidate_items(0..1).unwrap();
            cx.notify();
        });
        cx.update(|w, cx| w.draw(cx).clear(cx));
        let value = cx.update(|_, cx| rows.borrow()[&0].upgrade().unwrap().read(cx).0);
        println!(
            "REVIEW_CONTENT,known={known},requested=1,prepared={value},factories={}",
            factories.get()
        );
        if known {
            s.update(cx, |s, cx| {
                s.scroll_to_key(
                    50usize.into(),
                    VirtualizerScrollOptions::default().align(VirtualizerAlign::Start),
                )
                .unwrap();
                cx.notify();
            });
            cx.update(|w, cx| w.draw(cx).clear(cx));
            let old = s.read_with(cx, |s, _| {
                s.snapshot(s.viewport().unwrap()).unwrap().visible_range()
            });
            factories.set(0);
            s.update(cx, |s, cx| {
                s.update_sizes(&[VirtualizerSizeUpdate::new(0usize, px(40.))])
                    .unwrap();
                cx.notify();
            });
            cx.update(|w, cx| w.draw(cx).clear(cx));
            let new = s.read_with(cx, |s, _| {
                s.snapshot(s.viewport().unwrap()).unwrap().visible_range()
            });
            println!(
                "REVIEW_MOVE,same_range={},range={new:?},factories={}",
                old == new,
                factories.get()
            );
        }
    }
}

#[test]
fn append_and_cold_masonry_probe() {
    let estimates = Rc::new(Cell::new(0));
    let sizing = || {
        let count = estimates.clone();
        VirtualizerSizing::measured_with_estimates(px(20.), move |_| {
            count.set(count.get() + 1);
            px(20.)
        })
    };
    let mut s = VirtualizerState::new(Axis::Vertical, source(10000), sizing()).unwrap();
    estimates.set(0);
    s.replace_source(
        source(10001),
        sizing(),
        VirtualizerChange::Splice {
            old: 10000..10000,
            inserted: 1,
        },
    )
    .unwrap();
    println!(
        "REVIEW_ESTIMATES,old_n=10000,inserted=1,calls={}",
        estimates.get()
    );
    println!("REVIEW_MASONRY,n,sample,construct_ns,append_ns");
    for n in [1000, 10000, 100000] {
        for sample in 0..3 {
            let t = Instant::now();
            let mut s =
                VirtualMasonryState::new(source(n), VirtualizerSizing::measured(px(40.)), 3)
                    .unwrap();
            let construct = t.elapsed().as_nanos();
            let t = Instant::now();
            s.replace_source(source(n + 1), VirtualizerSizing::measured(px(40.)))
                .unwrap();
            println!(
                "REVIEW_MASONRY,{n},{sample},{construct},{}",
                t.elapsed().as_nanos()
            );
        }
    }
}
