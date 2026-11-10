use super::*;
use yue_component::{TableColumnKey, TableHeaderGroup, TablePin};

pub(super) fn configure(
    p: &Entity<Probe>,
    cx: &mut VisualTestContext,
    update: impl FnOnce(Vec<TableColumn<UiRow>>) -> Vec<TableColumn<UiRow>>,
) {
    let state = state(p, cx);
    let old = state.read_with(cx, |s, _| s.view().unwrap().clone());
    let columns = update(old.columns().to_vec());
    state.update(cx, |s, cx| {
        s.commit_view(old.as_ref().clone().with_columns(columns), None, cx)
            .unwrap()
    });
    draw(cx);
    draw(cx);
}
#[gpui_kit::test]
fn constraints_responsive_restore_and_atomic_group_rejection(cx: &mut TestAppContext) {
    let (p, cx) = open(cx, 20, false);
    configure(&p, cx, |_| {
        vec![
            TableColumn::text("name", "Name", |r: &UiRow| r.key.clone()).width(rems(4.)),
            TableColumn::text("note", "Note", |r: &UiRow| r.note.clone())
                .min_width(rems(8.))
                .max_width(rems(20.))
                .flex(1.),
            TableColumn::text("amount", "Amount", |r: &UiRow| r.amount.to_string())
                .width(rems(10.))
                .responsive_priority(1),
            TableColumn::text("hidden", "Hidden", |r: &UiRow| r.key.clone()).visible(false),
        ]
    });
    let state = state(&p, cx);
    let key = TableColumnKey::try_new("note").unwrap();
    assert_eq!(
        state.read_with(cx, |s, _| s.column_width(&key)),
        Some(px(320.))
    );
    p.update(cx, |p, cx| {
        p.host_width = rems(19.);
        cx.notify();
    });
    draw(cx);
    draw(cx);
    assert!(
        !nodes(cx)
            .iter()
            .any(|n| n.path().contains(&ElementId::from("amount")) && n.role() == Some(Role::Cell))
    );
    assert_eq!(
        state.read_with(cx, |s, _| s.column_width(&key)),
        Some(px(192.))
    );
    p.update(cx, |p, cx| {
        p.host_width = rems(64.);
        cx.notify();
    });
    draw(cx);
    draw(cx);
    assert!(
        nodes(cx)
            .iter()
            .any(|n| n.path().contains(&ElementId::from("amount")) && n.role() == Some(Role::Cell))
    );
    assert!(
        !nodes(cx)
            .iter()
            .any(|n| n.path().contains(&ElementId::from("hidden")))
    );
    let old = state.read_with(cx, |s, _| s.view().unwrap().clone());
    let group = TableHeaderGroup::new("identity", "Identity");
    let mut cols = old.columns().to_vec();
    cols[0] = cols[0]
        .clone()
        .header_path(vec![group.clone()])
        .pin(TablePin::Start);
    cols[1] = cols[1].clone().header_path(vec![group]);
    assert!(
        state
            .update(cx, |s, cx| s.commit_view(
                old.as_ref().clone().with_columns(cols),
                None,
                cx
            ))
            .is_err()
    );
    assert!(state.read_with(cx, |s, _| Rc::ptr_eq(s.view().unwrap(), &old)));
}

#[gpui_kit::test]
fn pinned_lanes_tail_rows_single_controls_and_narrow_restore(cx: &mut TestAppContext) {
    let (p, cx) = open(cx, 1000, false);
    p.update(cx, |p, cx| {
        p.host_width = rems(32.);
        p.frozen = (1, 1);
        p.footer_fixed = true;
        cx.notify();
    });
    configure(&p, cx, |mut cols| {
        cols[0] = cols[0].clone().pin(TablePin::Start);
        cols[1] = cols[1].clone().width(rems(20.));
        cols[3] = cols[3].clone().pin(TablePin::End);
        cols
    });
    let state = state(&p, cx);
    let before = cell(cx, "row-0", "name").bounds();
    state.update(cx, |s, _| {
        s.scroll_handle()
            .set_offset(gpui_kit::point(px(-170.), px(-500.)))
    });
    draw(cx);
    assert_eq!(cell(cx, "row-0", "name").bounds().origin, before.origin);
    let action = cell(cx, "row-0", "action");
    assert!((action.bounds().right() - px(512.)).abs() < px(1.));
    let all = nodes(cx);
    let buttons: Vec<_> = all
        .iter()
        .filter(|n| n.path().contains(&row_id("row-0")) && n.role() == Some(Role::Button))
        .collect();
    assert_eq!(buttons.len(), 1);
    cx.simulate_click(buttons[0].bounds().center(), Default::default());
    draw(cx);
    assert_eq!(p.read_with(cx, |p, _| p.actions.get()), 1);
    let tail = cell(cx, "row-999", "name");
    let foot = cell(cx, "table-footer", "name");
    assert!((tail.bounds().bottom() - foot.bounds().top()).abs() < px(1.));
    p.update(cx, |p, cx| {
        p.host_width = rems(14.);
        cx.notify();
    });
    draw(cx);
    draw(cx);
    let end = TableColumnKey::try_new("action").unwrap();
    assert_eq!(
        state.read_with(cx, |s, _| s.effective_pin(&end)),
        Some(TablePin::None)
    );
    p.update(cx, |p, cx| {
        p.host_width = rems(32.);
        cx.notify();
    });
    draw(cx);
    draw(cx);
    assert_eq!(
        state.read_with(cx, |s, _| s.effective_pin(&end)),
        Some(TablePin::End)
    );
}

#[gpui_kit::test]
fn grouped_headers_share_widths_and_shallow_leaves_span_bands(cx: &mut TestAppContext) {
    let (p, cx) = open(cx, 20, false);
    configure(&p, cx, |mut cols| {
        let group = TableHeaderGroup::new("identity", "Identity");
        cols[0] = cols[0].clone().header_path(vec![group.clone()]);
        cols[1] = cols[1]
            .clone()
            .header_path(vec![group, TableHeaderGroup::new("detail", "Details")]);
        cols
    });
    let all = nodes(cx);
    let g = all
        .iter()
        .find(|n| n.role() == Some(Role::ColumnHeader) && n.label() == Some("Identity"))
        .unwrap();
    assert_eq!(g.column_span(), Some(2));
    assert!(
        (g.bounds().size.width
            - (cell(cx, "row-0", "name").bounds().size.width
                + cell(cx, "row-0", "note").bounds().size.width))
            .abs()
            < px(1.)
    );
    assert_eq!(cell(cx, "table-header", "name").row_span(), Some(2));
    assert_eq!(cell(cx, "table-header", "amount").row_span(), Some(3));
    assert_eq!(cell(cx, "row-0", "name").row_index(), Some(3));
    let header = cell(cx, "table-header", "note");
    assert!((header.bounds().bottom() - cell(cx, "row-0", "note").bounds().top()).abs() < px(1.));
}

#[gpui_kit::test]
fn resize_pointer_final_coordinate_keyboard_release_and_rejection(cx: &mut TestAppContext) {
    let (p, cx) = open(cx, 20, false);
    configure(&p, cx, |mut cols| {
        cols[0] = cols[0]
            .clone()
            .resizable(true)
            .min_width(rems(5.))
            .max_width(rems(14.));
        cols
    });
    let at = nodes(cx)
        .into_iter()
        .find(|n| n.role() == Some(Role::Splitter))
        .unwrap()
        .bounds()
        .center();
    cx.simulate_event(gpui_kit::MouseDownEvent {
        position: at,
        button: gpui_kit::MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
        first_mouse: false,
    });
    cx.simulate_event(gpui_kit::MouseMoveEvent {
        position: at + gpui_kit::point(px(20.), px(0.)),
        pressed_button: Some(gpui_kit::MouseButton::Left),
        modifiers: Default::default(),
    });
    draw(cx);
    assert_eq!(p.read_with(cx, |p, _| p.requests), 0);
    cx.simulate_event(gpui_kit::MouseUpEvent {
        position: at + gpui_kit::point(px(40.), px(0.)),
        button: gpui_kit::MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
    });
    draw(cx);
    assert_eq!(p.read_with(cx, |p, _| p.requests), 1);
    assert_eq!(cell(cx, "row-0", "name").bounds().size.width, px(152.));
    let stroke = gpui_kit::Keystroke::parse("right").unwrap();
    for held in [false, true, true] {
        cx.simulate_event(gpui_kit::KeyDownEvent {
            keystroke: stroke.clone(),
            is_held: held,
            prefer_character_input: false,
        });
    }
    draw(cx);
    assert_eq!(p.read_with(cx, |p, _| p.requests), 1);
    cx.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: stroke.clone(),
    });
    draw(cx);
    assert_eq!(p.read_with(cx, |p, _| p.requests), 2);
    let accepted = cell(cx, "row-0", "name").bounds().size.width;
    p.update(cx, |p, _| p.accept = false);
    cx.simulate_keystrokes("right");
    cx.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: stroke.clone(),
    });
    draw(cx);
    assert_eq!(
        cell(cx, "row-0", "name").bounds().size.width,
        accepted,
        "owner rejection restores accepted widths"
    );
    assert_eq!(p.read_with(cx, |p, _| p.requests), 3);
    cx.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: stroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    draw(cx);
    cx.simulate_keystrokes("escape");
    draw(cx);
    cx.simulate_event(gpui_kit::KeyUpEvent { keystroke: stroke });
    draw(cx);
    assert_eq!(
        p.read_with(cx, |p, _| p.requests),
        3,
        "Escape and late keyup cannot commit"
    );
    assert_eq!(cell(cx, "row-0", "name").bounds().size.width, accepted);
}

#[gpui_kit::test]
fn deferred_resize_draws_accepted_width_until_host_commits(cx: &mut TestAppContext) {
    let (p, cx) = open(cx, 20, false);
    configure(&p, cx, |mut cols| {
        cols[0] = cols[0].clone().resizable(true);
        cols
    });
    p.update(cx, |p, _| p.defer_resize = true);
    let before = cell(cx, "row-0", "name").bounds().size.width;
    let at = nodes(cx)
        .into_iter()
        .find(|n| n.role() == Some(Role::Splitter))
        .unwrap()
        .bounds()
        .center();
    cx.simulate_event(gpui_kit::MouseDownEvent {
        position: at,
        button: gpui_kit::MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
        first_mouse: false,
    });
    cx.simulate_event(gpui_kit::MouseMoveEvent {
        position: at + gpui_kit::point(px(24.), px(0.)),
        pressed_button: Some(gpui_kit::MouseButton::Left),
        modifiers: Default::default(),
    });
    draw(cx);
    close(
        cell(cx, "row-0", "name").bounds().size.width,
        before + px(24.),
    );
    cx.simulate_event(gpui_kit::MouseUpEvent {
        position: at + gpui_kit::point(px(40.), px(0.)),
        button: gpui_kit::MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
    });
    draw(cx);
    close(cell(cx, "row-0", "name").bounds().size.width, before);
    let request = p.update(cx, |p, _| p.deferred_resize.take().unwrap());
    assert_eq!(p.read_with(cx, |p, _| p.requests), 1);
    let state = state(&p, cx);
    let accepted = state.read_with(cx, |s, _| s.view().unwrap().clone());
    state.update(cx, |s, cx| {
        s.commit_view(request.apply_to(&accepted).unwrap(), Some(request.id()), cx)
            .unwrap()
    });
    draw(cx);
    close(
        cell(cx, "row-0", "name").bounds().size.width,
        before + px(40.),
    );
    assert!(state.read_with(cx, |s, _| s.pending_request().is_none()));
}
