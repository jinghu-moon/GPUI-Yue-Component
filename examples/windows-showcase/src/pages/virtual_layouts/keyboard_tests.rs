//! Exercise the production Grid with real headless input, not owner mutations.
use super::*;
use gpui_kit::{TestAppContext, VisualTestContext, size};

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| {
        w.refresh();
        w.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}

fn cell(cx: &mut VisualTestContext, row: usize, column: usize) {
    let point = cx.update(|w, _| {
        gpui_kit::base::test_support::snapshots(w)
            .into_iter()
            .find(|n| n.row_index() == Some(row) && n.column_index() == Some(column))
            .expect("requested cell is mounted")
            .bounds()
            .center()
    });
    cx.simulate_click(point, Default::default());
    draw(cx);
}

fn selected(cx: &mut VisualTestContext) -> (usize, usize) {
    cx.update(|w, _| {
        let nodes = gpui_kit::base::test_support::snapshots(w);
        let cells = nodes
            .iter()
            .filter(|n| n.role() == Some(Role::Cell))
            .collect::<Vec<_>>();
        assert!(cells.len() < 200, "cursor retention must remain sparse");
        let selected = cells
            .into_iter()
            .filter(|n| n.selected() == Some(true))
            .collect::<Vec<_>>();
        assert_eq!(selected.len(), 1, "one logical cell cursor");
        (
            selected[0].row_index().unwrap(),
            selected[0].column_index().unwrap(),
        )
    })
}

fn focused(cx: &mut VisualTestContext) -> bool {
    cx.update(|w, _| {
        gpui_kit::base::test_support::snapshots(w).iter().any(|n| {
            n.path().last() == Some(&ElementId::from("showcase-grid")) && n.focused() == Some(true)
        })
    })
}

fn press(cx: &mut VisualTestContext, key: &str, expected: (usize, usize)) {
    cx.simulate_keystrokes(key);
    draw(cx);
    assert_eq!(selected(cx), expected, "after {key}");
    assert!(focused(cx), "Grid must keep keyboard ownership after {key}");
}

fn open(cx: &mut TestAppContext) -> (Entity<VirtualLayoutsPage>, &mut VisualTestContext) {
    cx.update(|cx| {
        yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
        cx.set_reduce_motion(true);
    });
    let mut view = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let page = cx.new(|cx| VirtualLayoutsPage::new_with_focus(window, cx));
        view = Some(page.clone());
        gpui_kit::base::Root::new(page, window, cx)
    });
    cx.update(|window, _| window.activate_window());
    (view.unwrap(), cx)
}

#[gpui_kit::test]
fn grid_keyboard_arrows_edges_and_corners(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    cx.simulate_resize(size(px(1_400.), px(1_000.)));
    draw(cx);
    cell(cx, 1, 1);
    assert!(focused(cx), "click enters the single Grid tab stop");
    assert_eq!(selected(cx), (1, 1));
    press(cx, "right", (1, 2));
    press(cx, "down", (2, 2));
    press(cx, "home", (2, 0));
    press(cx, "left", (2, 0));
    press(cx, "end", (2, 999));
    press(cx, "right", (2, 999));
    press(cx, "ctrl-end", (999_999, 999));
    press(cx, "down", (999_999, 999));
    press(cx, "ctrl-home", (0, 0));
    assert!(view.read_with(cx, |p, cx| {
        [&p.rows, &p.columns]
            .into_iter()
            .all(|axis| axis.read(cx).viewport().unwrap().offset() == 0.)
    }));
    press(cx, "up", (0, 0));
}

#[gpui_kit::test]
fn grid_keyboard_rtl_and_merged_cells(cx: &mut TestAppContext) {
    let (_, cx) = open(cx);
    cx.simulate_resize(size(px(1_400.), px(1_000.)));
    draw(cx);
    super::tests::click(cx, "grid-span");
    cell(cx, 1, 1);
    press(cx, "right", (1, 4));
    press(cx, "left", (1, 1));
    press(cx, "down", (3, 1));
    press(cx, "up", (1, 1));
    super::tests::click(cx, "grid-direction");
    assert!(!focused(cx), "toolbar interaction must own focus");
    cell(cx, 1, 1);
    press(cx, "left", (1, 4));
    press(cx, "right", (1, 1));
    press(cx, "home", (1, 0));
    press(cx, "end", (1, 999));
}

#[gpui_kit::test]
fn grid_keyboard_ignores_modifiers_and_releases_tab_focus(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    cx.simulate_resize(size(px(1_400.), px(1_000.)));
    draw(cx);
    cell(cx, 1, 1);
    for key in ["shift-right", "alt-down", "ctrl-right", "ctrl-shift-end"] {
        press(cx, key, (1, 1));
    }
    cx.simulate_keystrokes("shift-tab");
    draw(cx);
    assert!(!focused(cx), "Grid must not trap Tab navigation");
    assert_eq!(selected(cx), (1, 1));
    cx.simulate_keystrokes("tab");
    draw(cx);
    assert!(focused(cx), "Tab must re-enter the Grid");
    press(cx, "right", (1, 2));
    super::tests::click(cx, "grid-last");
    cx.simulate_keystrokes("left");
    draw(cx);
    assert!(!focused(cx), "toolbar navigation must not steal Grid focus");
    assert_eq!(selected(cx), (1, 2));
    cx.simulate_keystrokes("tab");
    draw(cx);
    assert!(focused(cx));
    assert_eq!(selected(cx), (1, 2));
    draw(cx);
    let offsets = view.read_with(cx, |p, cx| {
        (
            p.rows.read(cx).viewport().unwrap().offset(),
            p.columns.read(cx).viewport().unwrap().offset(),
            p.rows.read(cx).has_scroll_command(),
            p.columns.read(cx).has_scroll_command(),
        )
    });
    assert!(
        offsets.0 < 72. && offsets.1 < 432.,
        "Tab entry must reveal the retained logical cursor after toolbar scrolling: {offsets:?}"
    );
}

#[gpui_kit::test]
fn grid_keyboard_span_toggle_normalizes_cursor_and_tab_unmount_keeps_keys(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    cx.simulate_resize(size(px(1_400.), px(1_000.)));
    draw(cx);
    cell(cx, 2, 3);
    super::tests::click(cx, "grid-span");
    assert_eq!(selected(cx), (1, 1));
    assert!(!focused(cx));
    view.update(cx, |p, cx| {
        p.active = 1;
        cx.notify();
    });
    draw(cx);
    view.update(cx, |p, cx| {
        p.active = 0;
        cx.notify();
    });
    draw(cx);
    assert_eq!(selected(cx), (1, 1));
    cell(cx, 1, 1);
    press(cx, "right", (1, 4));
}

#[gpui_kit::test]
fn grid_keyboard_page_uses_body_extent_and_survives_resize(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    cx.simulate_resize(size(px(1_400.), px(1_000.)));
    draw(cx);
    cell(cx, 1, 1);
    let rows = view.read_with(cx, |p, cx| {
        ((f32::from(p.rows.read(cx).viewport().unwrap().extent()) - 36.) / 36.).floor() as usize
    });
    press(cx, "pagedown", (1 + rows.max(1), 1));
    press(cx, "pageup", (1, 1));
    press(cx, "ctrl-end", (999_999, 999));
    cx.simulate_resize(size(px(1_000.), px(800.)));
    cx.update(|w, _| w.set_rem_size(px(20.)));
    draw(cx);
    assert!(focused(cx));
    assert_eq!(selected(cx), (999_999, 999));
    press(cx, "left", (999_999, 998));
    press(cx, "ctrl-home", (0, 0));
}
