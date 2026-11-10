use super::*;
use yue_component::{
    TableColumnDraft, TableColumnKey, TableColumnPreference, TableColumnPreferences,
    TableMovePlacement, TablePin,
};
fn col(key: &str) -> TableColumnKey {
    TableColumnKey::try_new(key.to_owned()).unwrap()
}
fn columns() -> Vec<TableColumn<Record>> {
    ["a", "b", "c"]
        .into_iter()
        .map(|k| TableColumn::text(k, k, |r: &Record| r.value.to_string()))
        .collect()
}
#[test]
fn preferences_migrate_by_meaning_preserve_raw_width_and_protect_permissions() {
    let view = TableView::new(source(1, &["a"]), columns());
    let mut draft = TableColumnDraft::new(&view);
    draft.set_width(&col("b"), gpui_kit::rems(100.)).unwrap();
    draft.set_visible(&col("c"), false).unwrap();
    draft.set_pin(&col("b"), TablePin::End).unwrap();
    draft.set_order(vec![col("c"), col("b"), col("a")]).unwrap();
    assert!(view.column_preferences().values().is_empty());
    let configured = draft.validate_for(&view).unwrap();
    assert_eq!(configured.columns()[0].key(), &col("c"));
    assert_eq!(
        configured.column_preferences().values()[&col("b")].width(),
        Some(gpui_kit::rems(100.))
    );
    let mut changed = columns();
    changed[1] = changed[1].clone().semantic_version(2);
    changed[2] = changed[2].clone().allowed(false);
    let updated = configured.with_columns(changed);
    assert_eq!(
        updated.column_preferences().values()[&col("b")].width(),
        None
    );
    assert!(
        !updated
            .columns()
            .iter()
            .find(|c| c.key() == &col("c"))
            .unwrap()
            .is_visible()
    );
    assert!(draft.validate_for(&updated).is_err());
    let forbidden = TableColumnPreferences::new(updated.column_schema())
        .with_column(col("c"), TableColumnPreference::new(1).with_visible(true));
    assert!(forbidden.apply_to(&updated).is_err());
    let mut reset = TableColumnDraft::new(&view);
    reset.reset();
    assert!(
        reset
            .validate_for(&view)
            .unwrap()
            .column_preferences()
            .values()
            .values()
            .all(|p| p.width().is_none())
    );
    assert!(
        draft
            .validate_for(&view.clone().with_column_schema(9))
            .is_err()
    );
}
#[test]
fn order_cannot_move_protected_columns_and_order_only_snapshots_keep_versions() {
    let mut cols = columns();
    cols[1] = cols[1].clone().configurable(false);
    let view = TableView::new(source(1, &["a"]), cols);
    assert!(
        TableColumnPreferences::new(1)
            .with_order(vec![col("b"), col("a"), col("c")])
            .apply_to(&view)
            .is_err()
    );
    let saved = TableColumnPreferences::new(1)
        .with_order(vec![col("c"), col("b"), col("a")])
        .apply_to(&view)
        .unwrap();
    let mut changed = columns();
    changed[0] = changed[0].clone().semantic_version(2);
    changed[1] = changed[1].clone().configurable(false);
    let migrated = saved.with_columns(changed);
    assert_eq!(migrated.columns()[1].key(), &col("b"));
    assert_eq!(
        migrated.column_preferences().values()[&col("a")].semantic_version(),
        2
    );
}
#[test]
fn row_reorder_shares_non_clone_records_and_releases_old_source() {
    let original = source(1, &["a", "b", "c"]);
    let key = original.row_key(0).unwrap().clone();
    let pointer = original.row(&key).unwrap() as *const Record;
    let weak = Rc::downgrade(&original);
    let keys = ["c", "a", "b"].map(|k| yue_component::TableRowKey::try_new(k).unwrap());
    let moved = original.reordered(2, &keys).unwrap();
    assert_eq!(moved.row(&key).unwrap() as *const Record, pointer);
    drop(original);
    assert!(weak.upgrade().is_none());
    assert_eq!(moved.position_of(&key), Some(1));
    assert!(
        moved
            .reordered(3, &[key.clone(), key.clone(), key])
            .is_err()
    );
}
#[gpui_kit::test]
fn tail_tickets_deduplicate_reject_bad_append_retry_and_isolate_old_results(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| {
        let delegate = cx.new(|_| Delegate);
        let state = cx.new(|cx| YueTableState::new(&delegate, cx));
        state.update(cx, |s, cx| {
            s.commit_view(view(source(1, &[])), None, cx).unwrap();
            let first = s.request_tail("cursor-1", cx).unwrap().unwrap();
            assert_eq!(first.range(), 0..0);
            assert!(s.request_tail("cursor-1", cx).unwrap().is_none());
            s.reject_range(&first, cx).unwrap();
            let retry = s.retry_range(cx).unwrap().unwrap();
            assert_ne!(retry.id(), first.id());
            assert!(s.reject_range(&first, cx).is_err());
            s.commit_range(&retry, view(source(2, &["a", "b"])), cx)
                .unwrap();
            assert!(s.request_tail("cursor-1", cx).unwrap().is_none());
            let next = s.request_tail("cursor-2", cx).unwrap().unwrap();
            assert!(
                s.commit_range(&next, view(source(3, &["b", "a", "c"])), cx)
                    .is_err()
            );
            assert_eq!(s.view().unwrap().source().len(), 2);
            s.commit_range(&next, view(source(3, &["a", "b", "c"])), cx)
                .unwrap();
            assert!(s.reject_range(&retry, cx).is_err());
        });
    });
}
#[gpui_kit::test]
fn configuration_is_one_controlled_request_with_stale_and_queue_guards(cx: &mut TestAppContext) {
    use std::cell::RefCell;
    cx.update(|cx| {
        let delegate = cx.new(|_| Delegate);
        let state = cx.new(|cx| YueTableState::new(&delegate, cx));
        let events = Rc::new(RefCell::new(Vec::new()));
        let capture = events.clone();
        let subscription = cx.subscribe(&state, move |_, e: &yue_component::TableEvent, _| {
            if let yue_component::TableEvent::ColumnConfigurationRequested(r) = e {
                capture.borrow_mut().push(r.clone());
            }
        });
        state.update(cx, |s, cx| {
            s.commit_view(TableView::new(source(1, &["a"]), columns()), None, cx)
                .unwrap();
        });
        let initial = state.read(cx).view().unwrap().clone();
        let mut draft = TableColumnDraft::new(&initial);
        draft.set_visible(&col("c"), false).unwrap();
        state.update(cx, |s, cx| {
            s.request_column_configuration(&draft, cx).unwrap();
        });
        cx.defer(move |cx| {
            let _keep_alive = subscription;
            assert_eq!(events.borrow().len(), 1);
            let request = events.borrow()[0].clone();
            state.update(cx, |s, cx| {
                assert!(Rc::ptr_eq(s.view().unwrap(), &initial));
                s.commit_view(
                    request.apply_to(s.view().unwrap()).unwrap(),
                    Some(request.id()),
                    cx,
                )
                .unwrap();
                assert!(request.apply_to(s.view().unwrap()).is_err());
                let draft = TableColumnDraft::new(s.view().unwrap());
                for _ in 0..64 {
                    s.request_column_configuration(&draft, cx).unwrap();
                }
                assert!(matches!(
                    s.request_column_configuration(&draft, cx),
                    Err(TableError::TooManyPendingRequests)
                ));
                assert!(matches!(
                    s.request_column_move(&col("a"), &col("b"), TableMovePlacement::After, cx),
                    Err(TableError::TooManyPendingRequests)
                ));
            });
        });
    });
    cx.run_until_parked();
}

fn setup_drag(p: &Entity<Probe>, cx: &mut VisualTestContext) {
    p.update(cx, |p, cx| {
        p.row_drag = yue_component::TableRowDrag::Handle;
        cx.notify();
    });
    layout::configure(p, cx, |_| {
        vec![
            TableColumn::text("drag", "移动", |_: &UiRow| "")
                .width(rems(4.))
                .row_drag_handle(true),
            TableColumn::text("name", "Name", |r: &UiRow| r.key.clone())
                .width(rems(12.))
                .reorderable(true),
            TableColumn::text("note", "Note", |r: &UiRow| r.note.clone())
                .width(rems(12.))
                .reorderable(true),
        ]
    });
    let s = state(p, cx);
    s.update(cx, |s, cx| {
        let v = s.view().unwrap().as_ref().clone().disabled_rows(vec![]);
        s.commit_view(v, None, cx).unwrap();
    });
    draw(cx);
}
fn pointer(
    cx: &mut VisualTestContext,
    start: gpui_kit::Point<gpui_kit::Pixels>,
    end: gpui_kit::Point<gpui_kit::Pixels>,
) {
    cx.simulate_event(gpui_kit::MouseDownEvent {
        position: start,
        button: gpui_kit::MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    // Release has a different absolute coordinate from the last motion.
    cx.simulate_event(gpui_kit::MouseMoveEvent {
        position: start + gpui_kit::point(px(0.), px(5.)),
        pressed_button: Some(gpui_kit::MouseButton::Left),
        ..Default::default()
    });
    cx.simulate_event(gpui_kit::MouseUpEvent {
        position: end,
        button: gpui_kit::MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    draw(cx);
}
#[gpui_kit::test]
fn pointer_final_coordinate_static_and_cells_reorder_once_and_keep_record_identity(
    cx: &mut TestAppContext,
) {
    let (p, cx) = open(cx, 20, false);
    setup_drag(&p, cx);
    let s = state(&p, cx);
    for mode in [
        TableRenderMode::Rows,
        TableRenderMode::Cells,
        TableRenderMode::Static,
    ] {
        p.update(cx, |p, cx| {
            p.mode = mode;
            cx.notify();
        });
        draw(cx);
        let old = s.read_with(cx, |s, _| s.view().unwrap().clone());
        let a = old.source().row_key(0).unwrap().as_str().to_owned();
        let b = old.source().row_key(3).unwrap().as_str().to_owned();
        s.update(cx, |s, cx| {
            s.scroll_to_row(
                &yue_component::TableRowKey::try_new(a.clone()).unwrap(),
                Default::default(),
                cx,
            )
            .unwrap_or(())
        });
        draw(cx);
        draw(cx);
        let start = cell(cx, &a, "drag").bounds().center();
        let mut end = cell(cx, &b, "drag").bounds().center();
        end.y += px(3.);
        let before = p.read_with(cx, |p, _| p.requests);
        pointer(cx, start, end);
        assert_eq!(p.read_with(cx, |p, _| p.requests), before + 1);
        let new = s.read_with(cx, |s, _| s.view().unwrap().clone());
        let key = yue_component::TableRowKey::try_new(a).unwrap();
        assert_eq!(new.source().position_of(&key), Some(3));
        assert!(std::ptr::eq(
            old.source().row(&key).unwrap(),
            new.source().row(&key).unwrap()
        ));
    }
}
#[gpui_kit::test]
fn keyboard_moves_and_escape_lost_button_and_query_change_cancel_without_commit(
    cx: &mut TestAppContext,
) {
    let (p, cx) = open(cx, 1000, false);
    setup_drag(&p, cx);
    let s = state(&p, cx);
    cx.update(|w, cx| s.focus_handle(cx).focus(w, cx));
    cx.simulate_keystrokes("down");
    draw(cx);
    cx.simulate_keystrokes("enter");
    draw(cx);
    for key in ["space", "down", "down", "enter"] {
        let keystroke = gpui_kit::Keystroke::parse(key).unwrap();
        cx.simulate_event(gpui_kit::KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(gpui_kit::KeyUpEvent { keystroke });
        draw(cx);
    }
    assert_eq!(p.read_with(cx, |p, _| p.requests), 1);
    let keystroke = gpui_kit::Keystroke::parse("space").unwrap();
    cx.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui_kit::KeyUpEvent { keystroke });
    for _ in 0..30 {
        cx.simulate_keystrokes("down");
        draw(cx);
        draw(cx);
    }
    assert!(s.read_with(cx, |s, _| s.scroll_handle().offset().y < px(0.)));
    let remote = cell(cx, "row-32", "drag").bounds();
    assert!(remote.top() >= px(39.) && remote.bottom() <= px(449.));
    cx.simulate_keystrokes("escape");
    draw(cx);
    assert_eq!(p.read_with(cx, |p, _| p.requests), 1);
    s.update(cx, |s, cx| {
        s.scroll_to_row(
            &TableRowKey::try_new("row-0").unwrap(),
            Default::default(),
            cx,
        )
        .unwrap()
    });
    draw(cx);
    draw(cx);
    let start = cell(cx, "row-0", "drag").bounds().center();
    for cancel in ["escape", "lost-button", "query"] {
        let before = p.read_with(cx, |p, _| p.requests);
        cx.simulate_event(gpui_kit::MouseDownEvent {
            position: start,
            button: gpui_kit::MouseButton::Left,
            click_count: 1,
            ..Default::default()
        });
        cx.simulate_event(gpui_kit::MouseMoveEvent {
            position: start + gpui_kit::point(px(0.), px(45.)),
            pressed_button: Some(gpui_kit::MouseButton::Left),
            ..Default::default()
        });
        match cancel {
            "escape" => cx.simulate_keystrokes("escape"),
            "lost-button" => cx.simulate_event(gpui_kit::MouseMoveEvent {
                position: start,
                pressed_button: None,
                ..Default::default()
            }),
            _ => {
                s.update(cx, |s, cx| {
                    let v = s
                        .view()
                        .unwrap()
                        .as_ref()
                        .clone()
                        .with_query(yue_component::TableQuery::paged(1, 1000).unwrap());
                    s.commit_view(v, None, cx).unwrap();
                });
            }
        }
        cx.simulate_event(gpui_kit::MouseUpEvent {
            position: start + gpui_kit::point(px(0.), px(120.)),
            button: gpui_kit::MouseButton::Left,
            click_count: 1,
            ..Default::default()
        });
        draw(cx);
        assert_eq!(p.read_with(cx, |p, _| p.requests), before);
    }
}

#[gpui_kit::test]
fn columns_move_with_final_pointer_and_reject_pin_group_protected_and_noop(
    cx: &mut TestAppContext,
) {
    let (p, cx) = open(cx, 20, true);
    setup_drag(&p, cx);
    let s = state(&p, cx);
    let button = nodes(cx)
        .into_iter()
        .find(|n| n.role() == Some(Role::Button) && n.label() == Some("移动列 name"))
        .unwrap();
    let mut end = cell(cx, "table-header", "note").bounds().center();
    end.x += px(20.);
    pointer(cx, button.bounds().center(), end);
    assert_eq!(p.read_with(cx, |p, _| p.requests), 1);
    assert_eq!(
        s.read_with(cx, |s, _| s.view().unwrap().columns()[2].key().clone()),
        col("name")
    );
    s.update(cx, |s, cx| {
        assert!(
            s.request_column_move(&col("note"), &col("name"), TableMovePlacement::Before, cx)
                .unwrap()
                .is_none()
        );
        let v = s.view().unwrap().as_ref().clone();
        let mut c = v.declared_columns().to_vec();
        c[1] = c[1].clone().pin(TablePin::Start);
        s.commit_view(v.with_columns(c), None, cx).unwrap();
        assert!(
            s.request_column_move(&col("name"), &col("note"), TableMovePlacement::After, cx)
                .is_err()
        );
        assert!(
            s.request_column_move(&col("name"), &col("drag"), TableMovePlacement::Before, cx)
                .is_err()
        );
    });
    layout::configure(&p, cx, |mut c| {
        c[1] = c[1]
            .clone()
            .pin(TablePin::None)
            .header_path(vec![yue_component::TableHeaderGroup::new("g", "Group")]);
        c
    });
    s.update(cx, |s, cx| {
        assert!(
            s.request_column_move(&col("name"), &col("note"), TableMovePlacement::Before, cx)
                .is_err()
        )
    });
    layout::configure(&p, cx, |_| {
        (0..24)
            .map(|i| {
                TableColumn::text(format!("wide-{i}"), format!("Wide {i}"), |r: &UiRow| {
                    r.key.clone()
                })
                .width(rems(8.))
                .reorderable(true)
            })
            .collect()
    });
    for _ in 0..30 {
        if nodes(cx)
            .iter()
            .any(|n| n.label() == Some("移动列 wide-0") && n.focused() == Some(true))
        {
            break;
        }
        cx.simulate_keystrokes("tab");
        draw(cx);
    }
    assert!(
        nodes(cx)
            .iter()
            .any(|n| n.label() == Some("移动列 wide-0") && n.focused() == Some(true))
    );
    let keystroke = gpui_kit::Keystroke::parse("space").unwrap();
    cx.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui_kit::KeyUpEvent { keystroke });
    for _ in 0..12 {
        cx.simulate_keystrokes("right");
        draw(cx);
    }
    assert!(s.read_with(cx, |s, _| s.scroll_handle().offset().x < px(0.)));
    let target = cell(cx, "table-header", "wide-12").bounds();
    assert!(target.left() >= -px(1.) && target.right() <= px(1025.));
    cx.simulate_keystrokes("escape");
    draw(cx);
    assert_eq!(p.read_with(cx, |p, _| p.requests), 1);
}
#[gpui_kit::test]
fn drag_edge_uses_same_scroll_owner_and_escape_stops_pending_frames(cx: &mut TestAppContext) {
    let (p, cx) = open(cx, 50_000, true);
    setup_drag(&p, cx);
    let s = state(&p, cx);
    let handle = s.read_with(cx, |s, _| s.scroll_handle().clone());
    let start = cell(cx, "row-0", "drag").bounds().center();
    let region = nodes(cx)
        .into_iter()
        .find(|n| n.role() == Some(Role::Table))
        .unwrap()
        .bounds();
    let edge = gpui_kit::point(start.x, region.bottom() - px(4.));
    cx.simulate_event(gpui_kit::MouseDownEvent {
        position: start,
        button: gpui_kit::MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    cx.simulate_event(gpui_kit::MouseMoveEvent {
        position: edge,
        pressed_button: Some(gpui_kit::MouseButton::Left),
        ..Default::default()
    });
    assert!(handle.offset().y < px(0.));
    // The source can disappear while the root capture still owns cancellation.
    handle.set_offset(gpui_kit::point(px(0.), -px(5000.)));
    cx.simulate_keystrokes("escape");
    draw(cx);
    let cancelled = handle.offset();
    draw(cx);
    draw(cx);
    assert_eq!(handle.offset(), cancelled);
    cx.simulate_event(gpui_kit::MouseUpEvent {
        position: edge,
        button: gpui_kit::MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    draw(cx);
    assert_eq!(p.read_with(cx, |p, _| p.requests), 0);
}
#[gpui_kit::test]
fn tail_observes_real_viewport_and_failure_never_automatically_retries(cx: &mut TestAppContext) {
    use std::cell::RefCell;
    let (p, cx) = open(cx, 1000, true);
    let s = state(&p, cx);
    let events = Rc::new(RefCell::new(Vec::new()));
    let capture = events.clone();
    let _subscription = cx.update(|_, cx| {
        cx.subscribe(&s, move |_, e: &TableEvent, _| {
            if let TableEvent::RangeRequested(r) = e {
                capture.borrow_mut().push(r.clone());
            }
        })
    });
    p.update(cx, |p, cx| {
        p.tail = Some(("cursor-1".into(), rems(2.), true));
        p.frozen = (1, 1);
        p.footer_fixed = true;
        cx.notify();
    });
    draw(cx);
    draw(cx);
    assert!(events.borrow().is_empty());
    s.update(cx, |s, cx| {
        s.scroll_to_row(
            &TableRowKey::try_new("row-998").unwrap(),
            Default::default(),
            cx,
        )
        .unwrap()
    });
    draw(cx);
    draw(cx);
    assert_eq!(events.borrow().len(), 1);
    let request = events.borrow()[0].clone();
    s.update(cx, |s, cx| s.reject_range(&request, cx).unwrap());
    draw(cx);
    draw(cx);
    assert_eq!(events.borrow().len(), 1);
    s.update(cx, |s, cx| {
        s.retry_range(cx).unwrap().unwrap();
    });
    draw(cx);
    assert_eq!(events.borrow().len(), 2);
    assert_ne!(events.borrow()[0].id(), events.borrow()[1].id());
    p.update(cx, |p, cx| {
        p.tail = Some(("cursor-2".into(), rems(2.), false));
        cx.notify();
    });
    draw(cx);
    draw(cx);
    assert_eq!(events.borrow().len(), 2);
}

#[gpui_kit::test]
fn capabilities_and_complete_builder_validation_match_accepted_configuration(
    cx: &mut TestAppContext,
) {
    let (p, cx) = open(cx, 20, true);
    setup_drag(&p, cx);
    let s = state(&p, cx);
    cx.update(|_, cx| {
        let capabilities = YueTable::new("capabilities", &s)
            .row_drag(yue_component::TableRowDrag::Handle)
            .capabilities(cx)
            .unwrap();
        assert!(capabilities.row_reordering());
        assert!(capabilities.column_reordering());
        assert!(capabilities.measured_rows());
        assert!(!capabilities.rowspan());
        let cells = YueTable::new("capabilities", &s)
            .render_mode(TableRenderMode::Cells)
            .capabilities(cx)
            .unwrap();
        assert!(cells.rowspan());
        assert!(!cells.measured_rows());
        assert!(
            !YueTable::new("capabilities", &s)
                .readonly(true)
                .capabilities(cx)
                .unwrap()
                .row_reordering()
        );
        assert!(
            YueTable::new("capabilities", &s)
                .range_loading(4)
                .tail_loading("cursor", rems(1.), true)
                .validate(cx)
                .is_err()
        );
        assert!(
            YueTable::new("capabilities", &s)
                .render_mode(TableRenderMode::Static)
                .scroll_host(&gpui_kit::ScrollHandle::new(), rems(0.), rems(0.))
                .validate(cx)
                .is_err()
        );
        assert!(matches!(
            YueTable::new("capabilities", &s)
                .tail_loading(" ", rems(1.), true)
                .validate(cx),
            Err(TableError::EmptyKey)
        ));
        assert!(
            YueTable::new("capabilities", &s)
                .render_mode(TableRenderMode::Static)
                .range_loading(4)
                .validate(cx)
                .is_err()
        );
        s.update(cx, |s, cx| {
            let next = s
                .view()
                .unwrap()
                .as_ref()
                .clone()
                .with_query(yue_component::TableQuery::paged(1, 20).unwrap());
            s.commit_view(next, None, cx).unwrap();
        });
        assert!(
            YueTable::new("capabilities", &s)
                .render_mode(TableRenderMode::Static)
                .frozen_rows(1, 0)
                .validate(cx)
                .is_err()
        );
        assert!(
            YueTable::new("capabilities", &s)
                .render_mode(TableRenderMode::Static)
                .footer_fixed(true)
                .validate(cx)
                .is_err()
        );
        let accepted = s.read(cx).view().unwrap().clone();
        assert!(
            YueTable::new("capabilities", &s)
                .range_loading(4)
                .validate(cx)
                .is_err()
        );
        assert!(
            YueTable::new("capabilities", &s)
                .tail_loading("cursor", rems(1.), true)
                .validate(cx)
                .is_err()
        );
        assert!(Rc::ptr_eq(s.read(cx).view().unwrap(), &accepted));
    });
}
