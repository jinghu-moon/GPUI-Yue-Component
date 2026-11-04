use super::*;
use std::{cell::RefCell, collections::HashSet};
use yue_component::VirtualGrid;

type ObservedCell = (usize, usize, bool, bool);

struct Harness {
    rows: Entity<VirtualizerState>,
    columns: Entity<VirtualizerState>,
    scroll: ScrollHandle,
    focus: gpui_kit::FocusHandle,
    frozen: (usize, usize),
    clicks: Rc<RefCell<Vec<(usize, usize)>>>,
    seen: Rc<RefCell<Vec<ObservedCell>>>,
}
impl Render for Harness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let clicks = self.clicks.clone();
        let seen = self.seen.clone();
        let focus = self.focus.clone();
        VirtualGrid::new(
            "frozen-grid",
            &self.rows,
            &self.columns,
            &self.scroll,
            move |cell, _, _| {
                let key = (cell.row().index(), cell.column().index());
                seen.borrow_mut().push((
                    key.0,
                    key.1,
                    cell.is_frozen_row(),
                    cell.is_frozen_column(),
                ));
                let clicks = clicks.clone();
                let cell = div().id("cell").size_full();
                let cell = if key == (0, 0) {
                    cell.track_focus(&focus)
                } else {
                    cell
                };
                cell.role(Role::Cell)
                    .aria_label(format!("{}:{}", key.0, key.1))
                    .on_click(move |_, _, _| clicks.borrow_mut().push(key))
                    .test_support()
            },
        )
        .frozen_rows(self.frozen.0)
        .frozen_columns(self.frozen.1)
        .w(px(240.))
        .h(px(100.))
        .role(Role::Grid)
        .test_support()
    }
}
fn harness(cx: &mut Context<Harness>, count: usize, known: bool) -> Harness {
    let binding = VirtualizerBinding::new();
    let focus = cx.focus_handle();
    let mut axis = |vertical: bool| {
        cx.new(|_| {
            let extent = if vertical { 20.5 } else { 50.25 };
            let mut s = VirtualizerState::new_attached(
                if vertical {
                    Axis::Vertical
                } else {
                    Axis::Horizontal
                },
                source(0, count),
                if known {
                    VirtualizerSizing::known(vec![px(extent); count])
                } else {
                    VirtualizerSizing::fixed(px(extent))
                },
                &binding,
            )
            .unwrap();
            s.set_layout(
                VirtualizerLayout::default()
                    .gap(px(1.5))
                    .padding(px(2.), px(3.)),
            )
            .unwrap();
            s.set_focus_handle(0usize.into(), Some(focus.clone()))
                .unwrap();
            s
        })
    };
    Harness {
        rows: axis(true),
        columns: axis(false),
        scroll: ScrollHandle::new(),
        frozen: (1, 1),
        clicks: Rc::default(),
        seen: Rc::default(),
        focus,
    }
}
#[gpui_kit::test]
fn frozen_grid_four_regions_are_unique_clipped_and_route_native_input(cx: &mut TestAppContext) {
    for known in [false, true] {
        let (view, cx) = cx.add_window_view(|_, cx| harness(cx, 100, known));
        cx.update(|w, cx| w.draw(cx).clear(cx));
        view.update(cx, |v, cx| {
            v.scroll.set_precise_offset(gpui_kit::point(-75., -30.));
            v.seen.borrow_mut().clear();
            cx.notify();
        });
        cx.update(|w, cx| w.draw(cx).clear(cx));
        let clicks = view.read_with(cx, |v, _| v.clicks.clone());
        for (point, key) in [
            ((10., 10.), (0, 0)),
            ((70., 10.), (0, 2)),
            ((10., 40.), (3, 0)),
            ((70., 40.), (3, 2)),
        ] {
            clicks.borrow_mut().clear();
            cx.simulate_click(
                gpui_kit::point(px(point.0), px(point.1)),
                Default::default(),
            );
            assert_eq!(clicks.borrow().as_slice(), &[key]);
        }
        // Frozen row 0 ends at 22.5. The boundary gap belongs to the strip;
        // scrolling rows behind it cannot receive a click through that gap.
        clicks.borrow_mut().clear();
        cx.simulate_click(gpui_kit::point(px(70.), px(23.)), Default::default());
        assert!(
            clicks.borrow().is_empty(),
            "body must be clipped before hit testing"
        );
        let nodes = cx.update(|w, _| snapshots(w));
        assert_eq!(
            nodes
                .iter()
                .filter(|n| n.role() == Some(Role::Grid))
                .count(),
            1
        );
        let corner = nodes.iter().find(|n| n.label() == Some("0:0")).unwrap();
        assert_eq!(corner.bounds().origin, gpui_kit::point(px(2.), px(2.)));
        let seen = view.read_with(cx, |v, _| v.seen.borrow().clone());
        let mut unique = HashSet::new();
        // A draw may contain multiple internal passes; the corner must occur
        // exactly as often as any other consistently present cell.
        let corner_count = seen.iter().filter(|c| c.0 == 0 && c.1 == 0).count();
        for &(r, c, fr, fc) in &seen {
            assert_eq!(fr, r == 0);
            assert_eq!(fc, c == 0);
            unique.insert((r, c));
        }
        assert_eq!(seen.len(), unique.len() * corner_count);
        let before = view.read_with(cx, |v, _| v.scroll.precise_offset());
        cx.simulate_event(gpui_kit::ScrollWheelEvent {
            position: gpui_kit::point(px(10.), px(10.)),
            delta: gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(-10.), px(-10.))),
            modifiers: Default::default(),
            touch_phase: gpui_kit::TouchPhase::Moved,
        });
        cx.update(|w, cx| w.draw(cx).clear(cx));
        assert_eq!(
            view.read_with(cx, |v, _| v.scroll.precise_offset()),
            before - gpui_kit::point(10., 10.)
        );
    }
}

#[gpui_kit::test]
fn frozen_grid_alignment_uses_body_clearance_without_mutating_axis_layout(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| harness(cx, 100, false));
    cx.update(|w, cx| w.draw(cx).clear(cx));
    let rows = view.read_with(cx, |v, _| v.rows.clone());
    for align in [
        VirtualizerAlign::Start,
        VirtualizerAlign::Center,
        VirtualizerAlign::End,
        VirtualizerAlign::Nearest,
    ] {
        rows.update(cx, |s, cx| {
            s.scroll_to_key(
                50usize.into(),
                VirtualizerScrollOptions::default().align(align),
            )
            .unwrap();
            cx.notify();
        });
        cx.update(|w, cx| w.draw(cx).clear(cx));
        rows.read_with(cx, |s, _| {
            let v = s.viewport().unwrap();
            let item = s.snapshot(v).unwrap().item(50).unwrap();
            assert!(
                item.start() - v.offset() >= 24. - 0.01,
                "body target behind frozen strip"
            );
            assert!(item.start() + f64::from(item.extent()) - v.offset() <= 100.01);
        });
    }
    let before = view.read_with(cx, |v, _| v.scroll.precise_offset());
    rows.update(cx, |s, cx| {
        s.scroll_to_key(
            0usize.into(),
            VirtualizerScrollOptions::default().align(VirtualizerAlign::Start),
        )
        .unwrap();
        cx.notify();
    });
    cx.update(|w, cx| w.draw(cx).clear(cx));
    assert_eq!(
        view.read_with(cx, |v, _| v.scroll.precise_offset()),
        before,
        "already frozen key must not reset offset"
    );
    rows.update(cx, |s, cx| {
        s.scroll_to_offset(300., VirtualizerScrollOptions::default())
            .unwrap();
        cx.notify();
    });
    cx.update(|w, cx| w.draw(cx).clear(cx));
    assert_eq!(
        view.read_with(cx, |v, _| -v.scroll.precise_offset().y),
        300.
    );
    view.update(cx, |v, cx| {
        v.frozen = (0, 0);
        cx.notify();
    });
    rows.update(cx, |s, cx| {
        s.scroll_to_key(
            50usize.into(),
            VirtualizerScrollOptions::default().align(VirtualizerAlign::Start),
        )
        .unwrap();
        cx.notify();
    });
    cx.update(|w, cx| w.draw(cx).clear(cx));
    assert_eq!(
        view.read_with(cx, |v, _| -v.scroll.precise_offset().y),
        1102.
    );
}

#[gpui_kit::test]
fn frozen_grid_huge_prefix_and_hidden_target_remain_bounded(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| harness(cx, 1_000_000, false));
    view.update(cx, |v, cx| {
        v.frozen = (usize::MAX, usize::MAX);
        cx.notify();
    });
    cx.update(|w, cx| w.draw(cx).clear(cx));
    assert!(view.read_with(cx, |v, _| v.seen.borrow().len()) < 100);
    let rows = view.read_with(cx, |v, _| v.rows.clone());
    rows.update(cx, |s, cx| {
        s.scroll_to_key(
            900_000usize.into(),
            VirtualizerScrollOptions::default().align(VirtualizerAlign::Start),
        )
        .unwrap();
        cx.notify();
    });
    cx.update(|w, cx| w.draw(cx).clear(cx));
    assert!(
        !rows.read_with(cx, |s, _| s.has_scroll_command()),
        "hidden frozen target must terminate"
    );
    assert_eq!(
        view.read_with(cx, |v, _| v.scroll.precise_offset()),
        gpui_kit::point(0., 0.)
    );
    rows.update(cx, |s, cx| {
        s.replace_source(
            source(0, 0),
            VirtualizerSizing::fixed(px(20.5)),
            VirtualizerChange::Replace {
                preserve_anchor: false,
            },
        )
        .unwrap();
        cx.notify();
    });
    cx.update(|w, cx| w.draw(cx).clear(cx));
    rows.read_with(cx, |s, _| {
        assert!(s.snapshot(s.viewport().unwrap()).unwrap().is_empty())
    });
}

#[gpui_kit::test]
fn frozen_grid_focus_survives_toggle_and_geometry_change(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| harness(cx, 100, true));
    cx.update(|w, cx| w.draw(cx).clear(cx));
    let focus = view.read_with(cx, |v, _| v.focus.clone());
    cx.update(|w, cx| focus.focus(w, cx));
    view.update(cx, |v, cx| {
        v.scroll
            .set_precise_offset(gpui_kit::point(-1_000., -1_000.));
        cx.notify();
    });
    cx.update(|w, cx| w.draw(cx).clear(cx));
    assert!(cx.update(|w, _| focus.is_focused(w)));
    view.update(cx, |v, cx| {
        v.frozen = (0, 0);
        cx.notify();
    });
    cx.update(|w, cx| w.draw(cx).clear(cx));
    assert!(cx.update(|w, _| focus.is_focused(w)));
    let rows = view.read_with(cx, |v, _| v.rows.clone());
    rows.update(cx, |s, cx| {
        s.update_sizes(&[yue_component::VirtualizerSizeUpdate::new(0usize, px(25.5))])
            .unwrap();
        cx.notify();
    });
    view.update(cx, |v, cx| {
        v.frozen = (1, 1);
        cx.notify();
    });
    cx.update(|w, cx| w.draw(cx).clear(cx));
    assert!(cx.update(|w, _| focus.is_focused(w)));
    assert_eq!(
        cx.update(|w, _| snapshots(w))
            .iter()
            .filter(|n| n.label() == Some("0:0"))
            .count(),
        1
    );
}

#[gpui_kit::test]
fn frozen_grid_zero_body_area_rejects_inaccessible_body_target(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| harness(cx, 100, false));
    view.update(cx, |v, cx| {
        v.frozen = (8, 1);
        cx.notify();
    });
    cx.update(|w, cx| w.draw(cx).clear(cx));
    let rows = view.read_with(cx, |v, _| v.rows.clone());
    rows.update(cx, |s, cx| {
        s.scroll_to_key(50usize.into(), VirtualizerScrollOptions::default())
            .unwrap();
        cx.notify();
    });
    cx.update(|w, cx| w.draw(cx).clear(cx));
    assert!(!rows.read_with(cx, |s, _| s.has_scroll_command()));
    assert_eq!(
        view.read_with(cx, |v, _| v.scroll.precise_offset()),
        gpui_kit::point(0., 0.)
    );
}
