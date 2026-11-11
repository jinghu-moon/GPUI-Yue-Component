use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{AppContext as _, Entity, TestAppContext};
use std::rc::Rc;
use yue_component::{
    TableColumn, TableCommitOutcome, TableDelegate, TableError, TableRows, TableSource, TableView,
    YueTableState,
};

#[path = "table/benchmark.rs"]
mod benchmark;
#[path = "table/layout.rs"]
mod layout;

// Deliberately not Clone: accepting a snapshot must not clone business rows.
struct Record {
    key: &'static str,
    value: i32,
}
struct Delegate;
impl TableDelegate for Delegate {
    type Source = TableRows<Record>;
}
fn source(revision: u64, keys: &[&'static str]) -> Rc<TableRows<Record>> {
    Rc::new(
        TableRows::try_new(
            "orders",
            revision,
            keys.iter()
                .map(|key| Record {
                    key,
                    value: revision as i32,
                })
                .collect::<Vec<_>>()
                .into(),
            |row: &Record| row.key.into(),
        )
        .unwrap(),
    )
}
fn view(source: Rc<TableRows<Record>>) -> TableView<TableRows<Record>> {
    TableView::new(
        source,
        vec![TableColumn::text("value", "数值", |row: &Record| {
            row.value.to_string()
        })],
    )
}
#[test]
fn source_rejects_empty_and_duplicate_keys_without_copying_records() {
    let rows: Rc<[Record]> =
        vec![Record { key: "a", value: 0 }, Record { key: "a", value: 1 }].into();
    assert!(matches!(
        TableRows::try_new("orders", 1, rows.clone(), |r: &Record| r.key.into()),
        Err(TableError::DuplicateRowKey(_))
    ));
    assert!(matches!(
        TableRows::try_new("orders", 1, rows.clone(), |_: &Record| "".into()),
        Err(TableError::EmptyKey)
    ));
    let valid = TableRows::try_new("orders", 1, rows.clone(), |r: &Record| {
        r.value.to_string().into()
    })
    .unwrap();
    assert!(std::ptr::eq(
        valid.row(valid.row_key(0).unwrap()).unwrap(),
        &rows[0]
    ));
}
#[gpui_kit::test]
fn failed_commit_keeps_snapshot_identity_and_navigation(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let delegate = cx.new(|_| Delegate);
        let state: Entity<YueTableState<Delegate>> = cx.new(|cx| YueTableState::new(&delegate, cx));
        let initial = source(1, &["a", "b"]);
        state.update(cx, |state, cx| {
            state.commit_view(view(initial.clone()), None, cx).unwrap();
        });
        let accepted = state.read(cx).view().unwrap().clone();
        let invalid = TableView::new(
            source(2, &["b", "a"]),
            vec![
                TableColumn::text("same", "一", |r: &Record| r.value.to_string()),
                TableColumn::text("same", "二", |r: &Record| r.value.to_string()),
            ],
        );
        state.update(cx, |s, cx| {
            assert!(matches!(
                s.commit_view(invalid, None, cx),
                Err(TableError::DuplicateColumnKey(_))
            ))
        });
        assert!(Rc::ptr_eq(state.read(cx).view().unwrap(), &accepted));
        assert!(Rc::ptr_eq(
            state.read(cx).view().unwrap().source(),
            &initial
        ));
        assert_eq!(
            accepted
                .source()
                .row(accepted.source().row_key(0).unwrap())
                .unwrap()
                .value,
            1
        );
        let stale = view(source(3, &["b", "a"])).based_on(0);
        state.update(cx, |s, cx| {
            assert_eq!(
                s.commit_view(stale, None, cx).unwrap_err(),
                TableError::StaleView
            )
        });
        assert!(Rc::ptr_eq(state.read(cx).view().unwrap(), &accepted));
        state.update(cx, |s, cx| {
            assert_eq!(
                s.commit_view(accepted.as_ref().clone(), None, cx).unwrap(),
                TableCommitOutcome::Unchanged
            )
        });
    });
}

use gpui_kit::base::{
    Root,
    test_support::{ElementSnapshot, snapshots},
};
use gpui_kit::{
    Context, ElementId, Focusable as _, IntoElement, ParentElement as _, Render, Role,
    SharedString, Styled as _, Subscription, VisualTestContext, Window, div, px, rems,
};
use std::cell::Cell;
use yue_component::{
    Accent, TableAlign, TableBorders, TableEvent, TableLoadState, TableRenderMode, TableRowKey,
    TableSelectionMode, TableSize, TableTextOverflow, ThemeMode, YueTable, YueTheme,
};

struct UiRow {
    key: SharedString,
    note: SharedString,
    amount: i32,
}
struct UiDelegate {
    calls: Rc<Cell<usize>>,
}
impl TableDelegate for UiDelegate {
    type Source = TableRows<UiRow>;
    fn render_cell(
        &mut self,
        row: &UiRow,
        column: &TableColumn<UiRow>,
        cell: &yue_component::TableCellContext,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        self.calls.set(self.calls.get() + 1);
        column.render(row, cell, w, cx)
    }
}
fn ui_source(count: usize, revision: u64, reverse: bool) -> Rc<TableRows<UiRow>> {
    let mut rows: Vec<_> = (0..count)
        .map(|i| UiRow {
            key: format!("row-{i}").into(),
            note: if i == 0 {
                "多行说明：一段较长的中文内容，用于验证同排短文本在容器中的垂直居中和自然换行。"
            } else {
                "短内容"
            }
            .into(),
            amount: i as i32,
        })
        .collect();
    if reverse {
        rows.reverse();
    }
    Rc::new(TableRows::try_new("test", revision, rows.into(), |r: &UiRow| r.key.clone()).unwrap())
}
struct Probe {
    _delegate: Entity<UiDelegate>,
    state: Entity<YueTableState<UiDelegate>>,
    _subscription: Subscription,
    calls: Rc<Cell<usize>>,
    requests: usize,
    defer_resize: bool,
    deferred_resize: Option<yue_component::TableColumnResizeRequest>,
    size: TableSize,
    borders: TableBorders,
    mode: TableRenderMode,
    disabled: bool,
    accept: bool,
    actions: Rc<Cell<usize>>,
    host_width: gpui_kit::Rems,
    frozen: (usize, usize),
    footer_fixed: bool,
    row_drag: yue_component::TableRowDrag,
    tail: Option<(SharedString, gpui_kit::Rems, bool)>,
}
impl Probe {
    fn new(count: usize, twelve: bool, cx: &mut Context<Self>) -> Self {
        let calls = Rc::new(Cell::new(0));
        let delegate = cx.new(|_| UiDelegate {
            calls: calls.clone(),
        });
        let state = cx.new(|cx| YueTableState::new(&delegate, cx));
        let actions = Rc::new(Cell::new(0));
        let command = actions.clone();
        let columns: Vec<_> = if twelve {
            (0..12)
                .map(|i| {
                    TableColumn::text(format!("col-{i}"), format!("Column {i}"), |r: &UiRow| {
                        r.amount.to_string()
                    })
                    .width(rems(8.))
                })
                .collect()
        } else {
            vec![
                TableColumn::text("name", "项目", |r: &UiRow| r.key.clone())
                    .width(rems(7.))
                    .footer("合计"),
                TableColumn::text("note", "说明", |r: &UiRow| r.note.clone())
                    .width(rems(12.))
                    .overflow(TableTextOverflow::Wrap),
                TableColumn::text("amount", "数量", |r: &UiRow| r.amount.to_string())
                    .width(rems(7.))
                    .align(TableAlign::Center)
                    .footer("0"),
                TableColumn::custom("action", "", move |_: &UiRow, cell, _, _| {
                    let command = command.clone();
                    let button = yue_component::YueButton::new("view-record", "查看")
                        .size(match cell.size() {
                            TableSize::Small => yue_component::ButtonSize::Small,
                            TableSize::Medium => yue_component::ButtonSize::Medium,
                            TableSize::Large => yue_component::ButtonSize::Large,
                        })
                        .disabled(cell.is_disabled() || cell.is_busy())
                        .tab_stop(cell.is_active())
                        .on_click(move |_, _, _| command.set(command.get() + 1));
                    if let Some(focus) = cell.control_focus() {
                        button.track_focus(focus)
                    } else {
                        button
                    }
                })
                .width(rems(6.))
                .interactive(true),
            ]
        };
        let mode = if twelve {
            TableSelectionMode::None
        } else {
            TableSelectionMode::Multiple
        };
        let disabled = if count > 1 && !twelve {
            vec![TableRowKey::try_new("row-1").unwrap()]
        } else {
            vec![]
        };
        state.update(cx, |s, cx| {
            s.commit_view(
                TableView::new(ui_source(count, 1, false), columns)
                    .selection_mode(mode)
                    .disabled_rows(disabled),
                None,
                cx,
            )
            .unwrap()
        });
        let subscription = cx.subscribe(&state, |p, state, event: &TableEvent, cx| {
            if let TableEvent::RowMoveRequested(request) = event {
                p.requests += 1;
                let view = state.read(cx).view().unwrap().clone();
                state.update(cx, |s, cx| {
                    s.commit_view(
                        request.apply_to_rows(&view).unwrap(),
                        Some(request.id()),
                        cx,
                    )
                    .unwrap();
                });
            }
            if let TableEvent::ColumnMoveRequested(request) = event {
                p.requests += 1;
                let view = state.read(cx).view().unwrap().clone();
                state.update(cx, |s, cx| {
                    s.commit_view(request.apply_to(&view).unwrap(), Some(request.id()), cx)
                        .unwrap();
                });
            }
            if let TableEvent::ColumnResizeRequested(request) = event {
                p.requests += 1;
                if p.defer_resize {
                    p.deferred_resize = Some(request.clone());
                    return;
                }
                let view = state.read(cx).view().unwrap().clone();
                state.update(cx, |s, cx| {
                    if p.accept {
                        s.commit_view(request.apply_to(&view).unwrap(), Some(request.id()), cx)
                            .unwrap();
                    } else {
                        s.reject_request(request.id(), cx).unwrap();
                    }
                });
            }
            if let TableEvent::SelectionRequested(request) = event {
                p.requests += 1;
                let view = state.read(cx).view().unwrap().clone();
                state.update(cx, |s, cx| {
                    if p.accept {
                        s.commit_view(request.apply_to(&view).unwrap(), Some(request.id()), cx)
                            .unwrap();
                    } else {
                        s.reject_request(request.id(), cx).unwrap();
                    }
                });
            }
        });
        Self {
            _delegate: delegate,
            state,
            _subscription: subscription,
            calls,
            requests: 0,
            defer_resize: false,
            deferred_resize: None,
            size: TableSize::Medium,
            borders: TableBorders::Grid,
            mode: TableRenderMode::Rows,
            disabled: false,
            accept: true,
            actions,
            host_width: rems(64.),
            frozen: (0, 0),
            footer_fixed: false,
            row_drag: yue_component::TableRowDrag::None,
            tail: None,
        }
    }
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(self.host_width).h(rems(32.)).child(
            YueTable::new("probe-table", &self.state)
                .accessibility_label("测试工单")
                .height(rems(28.))
                .size(self.size)
                .borders(self.borders)
                .render_mode(self.mode)
                .disabled(self.disabled)
                .frozen_rows(self.frozen.0, self.frozen.1)
                .footer_fixed(self.footer_fixed)
                .row_drag(self.row_drag)
                .when_some(self.tail.clone(), |table, (cursor, distance, more)| {
                    table.tail_loading(cursor, distance, more)
                }),
        )
    }
}
fn draw(cx: &mut VisualTestContext) {
    #[cfg(feature = "table-profile")]
    let _perf = gpui_kit::perf_stats::scope("fixture.draw_effects");
    cx.run_until_parked();
    cx.update(|w, cx| {
        w.refresh();
        w.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}
fn open(
    cx: &mut TestAppContext,
    count: usize,
    twelve: bool,
) -> (Entity<Probe>, &mut VisualTestContext) {
    cx.update(|cx| {
        yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap()
    });
    let mut probe = None;
    let (_, cx) = cx.add_window_view(|w, cx| {
        let p = cx.new(|cx| Probe::new(count, twelve, cx));
        probe = Some(p.clone());
        Root::new(p, w, cx)
    });
    cx.update(|w, _| w.resize(gpui_kit::size(px(1152.), px(768.))));
    draw(cx);
    (probe.unwrap(), cx)
}
fn nodes(cx: &mut VisualTestContext) -> Vec<ElementSnapshot> {
    cx.update(|w, _| snapshots(w))
}
fn row_id(key: &str) -> ElementId {
    (
        (ElementId::from("table-record"), "test").into(),
        key.to_owned(),
    )
        .into()
}
fn cell(cx: &mut VisualTestContext, row: &str, column: &str) -> ElementSnapshot {
    let id = if row == "table-header" || row == "table-footer" {
        row.to_owned().into()
    } else {
        row_id(row)
    };
    nodes(cx)
        .into_iter()
        .find(|n| {
            n.path().contains(&id)
                && n.path().last() == Some(&ElementId::from(column.to_owned()))
                && matches!(n.role(), Some(Role::Cell | Role::ColumnHeader))
        })
        .unwrap_or_else(|| panic!("missing cell {row}/{column}"))
}
fn state(p: &Entity<Probe>, cx: &VisualTestContext) -> Entity<YueTableState<UiDelegate>> {
    p.read_with(cx, |p, _| p.state.clone())
}
fn close(a: gpui_kit::Pixels, b: gpui_kit::Pixels) {
    assert!((a - b).abs() < px(0.6), "{a:?} != {b:?}");
}

#[gpui_kit::test]
fn shared_header_tracks_and_rem_density_geometry(cx: &mut TestAppContext) {
    let (p, cx) = open(cx, 100, false);
    for rem in [12., 16., 24.] {
        cx.update(|w, _| w.set_rem_size(px(rem)));
        for size in [TableSize::Small, TableSize::Medium, TableSize::Large] {
            p.update(cx, |p, cx| {
                p.size = size;
                cx.notify();
            });
            draw(cx);
            for column in ["name", "note", "amount"] {
                let head = cell(cx, "table-header", column).bounds();
                let body = cell(cx, "row-0", column).bounds();
                close(head.left(), body.left());
                close(head.size.width, body.size.width);
                assert!(body.size.height > px(0.));
            }
        }
    }
    let s = state(&p, cx);
    cx.update(|_, cx| {
        s.update(cx, |s, cx| {
            s.scroll_to_row(
                &TableRowKey::try_new("row-90").unwrap(),
                Default::default(),
                cx,
            )
            .unwrap()
        })
    });
    draw(cx);
    close(cell(cx, "table-header", "name").bounds().top(), px(0.));
    assert!(cell(cx, "row-90", "amount").bounds().size.height > px(0.));
}

#[gpui_kit::test]
fn selection_intents_skip_disabled_rows_and_commits_are_silent(cx: &mut TestAppContext) {
    let (p, cx) = open(cx, 40, false);
    let s = state(&p, cx);
    cx.update(|w, cx| s.focus_handle(cx).focus(w, cx));
    cx.simulate_keystrokes("down space");
    draw(cx);
    assert_eq!(
        s.read_with(cx, |s, _| s.view().unwrap().selected_keys().len()),
        1
    );
    cx.simulate_keystrokes("shift-down shift-down");
    draw(cx);
    let selected = s.read_with(cx, |s, _| s.view().unwrap().selected_keys().clone());
    assert_eq!(selected.len(), 2);
    assert!(!selected.contains(&TableRowKey::try_new("row-1").unwrap()));
    let before = p.read_with(cx, |p, _| p.requests);
    cx.update(|_, cx| {
        s.update(cx, |s, cx| {
            let view = s.view().unwrap().as_ref().clone();
            assert_eq!(
                s.commit_view(view, None, cx).unwrap(),
                TableCommitOutcome::Unchanged
            );
        })
    });
    draw(cx);
    assert_eq!(p.read_with(cx, |p, _| p.requests), before);
    cx.simulate_keystrokes("ctrl-a");
    draw(cx);
    assert_eq!(
        s.read_with(cx, |s, _| s.view().unwrap().selected_keys().len()),
        39
    );
    p.update(cx, |p, cx| {
        p.accept = false;
        cx.notify();
    });
    cx.simulate_keystrokes("space");
    draw(cx);
    assert_eq!(
        s.read_with(cx, |s, _| s.view().unwrap().selected_keys().len()),
        39
    );
    assert!(s.read_with(cx, |s, _| s.pending_request().is_none()));
}

#[gpui_kit::test]
fn static_multiline_rows_align_and_column_footer_is_not_a_record(cx: &mut TestAppContext) {
    let (p, cx) = open(cx, 4, false);
    p.update(cx, |p, cx| {
        p.mode = TableRenderMode::Static;
        cx.notify();
    });
    draw(cx);
    let note = cell(cx, "row-0", "note").bounds();
    let short = cell(cx, "row-0", "amount").bounds();
    close(note.top(), short.top());
    close(note.size.height, short.size.height);
    let content = nodes(cx)
        .into_iter()
        .find(|n| {
            n.path().contains(&row_id("row-0"))
                && n.path().contains(&ElementId::from("amount"))
                && n.path().last() == Some(&ElementId::from("cell-content"))
        })
        .unwrap()
        .bounds();
    close(content.center().y, short.center().y);
    let footer = cell(cx, "table-footer", "amount").bounds();
    assert!(footer.top() >= cell(cx, "row-3", "amount").bounds().bottom() - px(0.5));
    let s = state(&p, cx);
    cx.update(|_, cx| {
        s.update(cx, |s, cx| {
            s.request_select_all(true, cx).unwrap();
        })
    });
    draw(cx);
    assert_eq!(
        s.read_with(cx, |s, _| s.view().unwrap().selected_keys().len()),
        3
    );
}

#[gpui_kit::test]
fn tb01_working_set_is_bounded_and_large_jumps_render_current_records(cx: &mut TestAppContext) {
    let (p, cx) = open(cx, 1000, true);
    let s = state(&p, cx);
    let calls = p.read_with(cx, |p, _| p.calls.clone());
    calls.set(0);
    draw(cx);
    let small_calls = calls.get();
    cx.update(|_, cx| {
        s.update(cx, |s, cx| {
            let columns = s.view().unwrap().shared_columns().clone();
            s.commit_view(
                TableView::new(ui_source(100_000, 2, false), columns),
                None,
                cx,
            )
            .unwrap();
        })
    });
    draw(cx);
    calls.set(0);
    draw(cx);
    let large_calls = calls.get();
    assert!(
        small_calls > 0 && small_calls < 600,
        "small render calls: {small_calls}"
    );
    assert!(
        large_calls <= small_calls + 24,
        "small={small_calls}, large={large_calls}"
    );
    eprintln!("TB01 headless render calls: 1,000x12={small_calls}, 100,000x12={large_calls}");
    cx.update(|_, cx| {
        s.update(cx, |s, cx| {
            s.scroll_to_row(
                &TableRowKey::try_new("row-99990").unwrap(),
                Default::default(),
                cx,
            )
            .unwrap()
        })
    });
    draw(cx);
    let parts = nodes(cx);
    assert!(
        parts
            .iter()
            .any(|n| n.path().contains(&row_id("row-99990")))
    );
    assert!(!parts.iter().any(|n| n.path().contains(&row_id("row-0"))));
    assert!(
        parts
            .iter()
            .filter(|n| n.role() == Some(Role::Cell))
            .count()
            < 600
    );
}

#[gpui_kit::test]
fn embedded_button_owns_click_and_space_without_selecting_row(cx: &mut TestAppContext) {
    let (p, cx) = open(cx, 40, false);
    let button = nodes(cx)
        .into_iter()
        .find(|n| {
            n.path().contains(&row_id("row-0"))
                && n.role() == Some(Role::Button)
                && n.label() == Some("查看")
        })
        .unwrap();
    cx.simulate_click(button.bounds().center(), Default::default());
    draw(cx);
    assert_eq!(p.read_with(cx, |p, _| p.actions.get()), 1);
    assert_eq!(p.read_with(cx, |p, _| p.requests), 0);
    let keystroke = gpui_kit::Keystroke::parse("space").unwrap();
    cx.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui_kit::KeyUpEvent { keystroke });
    draw(cx);
    assert_eq!(p.read_with(cx, |p, _| p.actions.get()), 2);
    assert_eq!(p.read_with(cx, |p, _| p.requests), 0);
    assert!(nodes(cx).iter().any(|n| n.path().contains(&row_id("row-0"))
        && n.role() == Some(Role::Button)
        && n.focused() == Some(true)));
}

#[gpui_kit::test]
fn busy_disabled_and_empty_states_keep_selection_requests_out(cx: &mut TestAppContext) {
    let (p, cx) = open(cx, 40, false);
    let s = state(&p, cx);
    cx.update(|w, cx| s.focus_handle(cx).focus(w, cx));
    cx.update(|_, cx| {
        s.update(cx, |s, cx| {
            let next = s
                .view()
                .unwrap()
                .as_ref()
                .clone()
                .load_state(TableLoadState::Refreshing);
            s.commit_view(next, None, cx).unwrap();
            assert!(
                s.request_toggle(&TableRowKey::try_new("row-0").unwrap(), false, cx)
                    .is_err()
            );
        })
    });
    draw(cx);
    assert!(
        nodes(cx)
            .iter()
            .any(|n| n.path().contains(&row_id("row-0")))
    );
    p.update(cx, |p, cx| {
        p.disabled = true;
        cx.notify();
    });
    draw(cx);
    cx.simulate_keystrokes("space ctrl-a");
    draw(cx);
    assert_eq!(p.read_with(cx, |p, _| p.requests), 0);
    cx.update(|_, cx| {
        s.update(cx, |s, cx| {
            let cols = s.view().unwrap().shared_columns().clone();
            s.commit_view(
                TableView::new(ui_source(0, 2, false), cols).load_state(TableLoadState::Loading),
                None,
                cx,
            )
            .unwrap();
        })
    });
    draw(cx);
    assert!(!nodes(cx).iter().any(|n| n.role() == Some(Role::Cell)));
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(300));
    draw(cx);
    assert!(nodes(cx).iter().any(|n| n.label() == Some("正在加载数据")));
}

#[gpui_kit::test]
fn horizontal_scroll_moves_body_and_header_once(cx: &mut TestAppContext) {
    let (p, cx) = open(cx, 1000, true);
    let s = state(&p, cx);
    let before = cell(cx, "table-header", "col-0").bounds();
    cx.update(|_, cx| {
        s.read(cx)
            .scroll_handle()
            .set_offset(gpui_kit::point(px(-400.), px(-160.)))
    });
    draw(cx);
    let head = cell(cx, "table-header", "col-0").bounds();
    close(head.left(), before.left() - px(400.));
    close(head.top(), before.top());
    let row = nodes(cx)
        .into_iter()
        .find(|n| {
            n.role() == Some(Role::Cell) && n.path().last() == Some(&ElementId::from("col-0"))
        })
        .unwrap();
    close(head.left(), row.bounds().left());
    close(head.size.width, row.bounds().size.width);
}

#[gpui_kit::test]
fn foreign_tickets_stale_bases_and_regressing_sources_cannot_publish(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let delegate = cx.new(|_| Delegate);
        let a = cx.new(|cx| YueTableState::new(&delegate, cx));
        let b = cx.new(|cx| YueTableState::new(&delegate, cx));
        for state in [&a, &b] {
            state.update(cx, |s, cx| {
                s.commit_view(
                    view(source(1, &["a", "b"])).selection_mode(TableSelectionMode::Multiple),
                    None,
                    cx,
                )
                .unwrap()
            });
        }
        let key = TableRowKey::try_new("a").unwrap();
        let a_ticket = a.update(cx, |s, cx| s.request_toggle(&key, false, cx).unwrap());
        let b_ticket = b.update(cx, |s, cx| s.request_toggle(&key, false, cx).unwrap());
        assert_ne!(a_ticket, b_ticket);
        b.update(cx, |s, cx| {
            let accepted = s.view().unwrap().clone();
            assert_eq!(
                s.commit_view(accepted.as_ref().clone(), Some(a_ticket), cx)
                    .unwrap_err(),
                TableError::StaleRequest
            );
            assert_eq!(s.pending_request(), Some(b_ticket));
            assert_eq!(
                s.commit_view(view(source(0, &["b", "a"])), None, cx)
                    .unwrap_err(),
                TableError::StaleSource
            );
            assert!(Rc::ptr_eq(s.view().unwrap(), &accepted));
        });
    });
}

#[gpui_kit::test]
fn two_selection_intents_in_one_update_rebase_without_losing_first_action(cx: &mut TestAppContext) {
    let (p, cx) = open(cx, 40, false);
    let s = state(&p, cx);
    cx.update(|_, cx| {
        s.update(cx, |s, cx| {
            s.request_toggle(&TableRowKey::try_new("row-0").unwrap(), false, cx)
                .unwrap();
            s.request_toggle(&TableRowKey::try_new("row-2").unwrap(), false, cx)
                .unwrap();
        })
    });
    draw(cx);
    assert_eq!(p.read_with(cx, |p, _| p.requests), 2);
    assert_eq!(
        s.read_with(cx, |s, _| s.view().unwrap().selected_keys().len()),
        2
    );
    assert!(s.read_with(cx, |s, _| s.pending_request().is_none()));
}

#[gpui_kit::test]
fn enter_and_escape_use_real_cell_control_focus_and_single_selection_uses_radio(
    cx: &mut TestAppContext,
) {
    let (p, cx) = open(cx, 40, false);
    let s = state(&p, cx);
    cx.update(|w, cx| s.focus_handle(cx).focus(w, cx));
    for key in ["down", "right", "right", "right", "enter"] {
        cx.simulate_keystrokes(key);
        draw(cx);
    }
    draw(cx);
    assert!(nodes(cx).iter().any(|n| n.path().contains(&row_id("row-0"))
        && n.role() == Some(Role::Button)
        && n.focused() == Some(true)));
    cx.simulate_keystrokes("escape");
    draw(cx);
    assert!(cx.update(|w, cx| s.focus_handle(cx).is_focused(w)));
    cx.update(|_, cx| {
        s.update(cx, |s, cx| {
            let view = s
                .view()
                .unwrap()
                .as_ref()
                .clone()
                .selection_mode(TableSelectionMode::Single);
            s.commit_view(view, None, cx).unwrap();
        })
    });
    draw(cx);
    assert!(
        nodes(cx)
            .iter()
            .any(|n| n.path().contains(&row_id("row-0")) && n.role() == Some(Role::RadioButton))
    );
    assert!(!nodes(cx).iter().any(|n| n.role() == Some(Role::CheckBox)));
}

#[gpui_kit::test]
fn theme_and_content_updates_keep_header_body_geometry_coherent(cx: &mut TestAppContext) {
    let (p, cx) = open(cx, 40, false);
    let s = state(&p, cx);
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        for accent in [Accent::Azure, Accent::Neutral] {
            cx.update(|_, cx| yue_component::set_theme(cx, YueTheme::new(mode, accent)));
            draw(cx);
            close(
                cell(cx, "table-header", "name").bounds().left(),
                cell(cx, "row-0", "name").bounds().left(),
            );
        }
    }
    let old = cell(cx, "row-0", "amount").bounds();
    cx.update(|_, cx| {
        s.update(cx, |s, cx| {
            let view = s
                .view()
                .unwrap()
                .as_ref()
                .clone()
                .load_state(TableLoadState::Failed("保留旧结果".into()));
            s.commit_view(view, None, cx).unwrap();
        })
    });
    draw(cx);
    close(
        old.size.width,
        cell(cx, "row-0", "amount").bounds().size.width,
    );
    assert!(
        nodes(cx)
            .iter()
            .any(|n| n.path().last() == Some(&ElementId::from("table-error")))
    );
}

#[gpui_kit::test]
fn focused_control_retains_only_its_row_and_deleted_record_repairs_focus(cx: &mut TestAppContext) {
    let (p, cx) = open(cx, 1000, false);
    let s = state(&p, cx);
    cx.update(|w, cx| s.focus_handle(cx).focus(w, cx));
    for key in ["down", "right", "right", "right", "enter"] {
        cx.simulate_keystrokes(key);
        draw(cx);
    }
    cx.update(|_, cx| {
        s.read(cx)
            .scroll_handle()
            .set_offset(gpui_kit::point(px(0.), px(-20_000.)))
    });
    draw(cx);
    let parts = nodes(cx);
    assert_eq!(
        parts
            .iter()
            .filter(|n| n.path().contains(&row_id("row-0")) && n.role() == Some(Role::Button))
            .count(),
        1
    );
    assert!(
        parts
            .iter()
            .filter(|n| n.role() == Some(Role::Cell))
            .count()
            < 150
    );
    let source = s.read_with(cx, |s, _| Rc::downgrade(s.view().unwrap().source()));
    cx.update(|_, cx| {
        s.update(cx, |s, cx| {
            let columns = s.view().unwrap().shared_columns().clone();
            s.commit_view(TableView::new(ui_source(0, 2, false), columns), None, cx)
                .unwrap();
        })
    });
    draw(cx);
    assert!(source.upgrade().is_none());
    assert!(cx.update(|w, cx| s.focus_handle(cx).is_focused(w)));
    assert!(
        nodes(cx)
            .iter()
            .any(|n| n.path().contains(&ElementId::from("table-header")))
    );
}

#[gpui_kit::test]
fn native_semantics_use_zero_based_coordinates_and_finite_viewport_counts(cx: &mut TestAppContext) {
    let (p, cx) = open(cx, 40, false);
    let parts = nodes(cx);
    let table = parts
        .iter()
        .find(|n| n.role() == Some(Role::Table))
        .unwrap();
    assert_eq!(table.row_count(), Some(42)); // 40 records + header + column footer.
    assert_eq!(table.column_count(), Some(5)); // Four business columns + selection.
    assert_eq!(cell(cx, "table-header", "name").column_index(), Some(1));
    assert!(parts.iter().any(
        |n| n.path().last() == Some(&ElementId::from("table-header")) && n.row_index() == Some(0)
    ));
    assert!(
        parts
            .iter()
            .any(|n| n.path().last() == Some(&row_id("row-0"))
                && n.role() == Some(Role::Row)
                && n.row_index() == Some(1))
    );
    p.update(cx, |p, cx| {
        p.disabled = true;
        cx.notify();
    });
    draw(cx);
    assert!(
        nodes(cx)
            .iter()
            .any(|n| n.role() == Some(Role::Table) && n.disabled() == Some(true))
    );
}

#[gpui_kit::test]
fn request_waits_are_bounded_and_old_rejection_does_not_clear_newer_pending(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| {
        let delegate = cx.new(|_| Delegate);
        let state = cx.new(|cx| YueTableState::new(&delegate, cx));
        state.update(cx, |s, cx| {
            s.commit_view(
                view(source(1, &["a", "b"])).selection_mode(TableSelectionMode::Multiple),
                None,
                cx,
            )
            .unwrap();
            let key = TableRowKey::try_new("a").unwrap();
            let first = s.request_toggle(&key, false, cx).unwrap();
            let mut last = first;
            for _ in 1..64 {
                last = s.request_toggle(&key, false, cx).unwrap();
            }
            assert_eq!(
                s.request_toggle(&key, false, cx).unwrap_err(),
                TableError::TooManyPendingRequests
            );
            s.reject_request(first, cx).unwrap();
            assert_eq!(s.pending_request(), Some(last));
            assert_eq!(
                s.reject_request(first, cx).unwrap_err(),
                TableError::StaleRequest
            );
        });
    });
}

struct CheckedSource {
    rows: Rc<TableRows<Record>>,
    malformed: u8,
}
impl TableSource for CheckedSource {
    type Row = Record;
    fn scope(&self) -> &SharedString {
        self.rows.scope()
    }
    fn revision(&self) -> u64 {
        10 + u64::from(self.malformed)
    }
    fn len(&self) -> usize {
        self.rows.len()
    }
    fn row_key(&self, position: usize) -> Option<&TableRowKey> {
        self.rows
            .row_key(if self.malformed == 1 { 0 } else { position })
    }
    fn position_of(&self, key: &TableRowKey) -> Option<usize> {
        self.rows
            .position_of(key)
            .map(|p| if self.malformed == 2 { 1 - p } else { p })
    }
    fn row(&self, key: &TableRowKey) -> Option<&Record> {
        if self.malformed == 3 {
            None
        } else {
            self.rows.row(key)
        }
    }
}
struct CheckedDelegate;
impl TableDelegate for CheckedDelegate {
    type Source = CheckedSource;
}
#[gpui_kit::test]
fn custom_source_validation_rejects_duplicate_keys_bad_reverse_and_missing_records(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| {
        let delegate = cx.new(|_| CheckedDelegate);
        let state = cx.new(|cx| YueTableState::new(&delegate, cx));
        state.update(cx, |s, cx| {
            let rows = source(1, &["a", "b"]);
            let columns: Rc<[TableColumn<Record>]> =
                vec![TableColumn::text("value", "数值", |r: &Record| {
                    r.value.to_string()
                })]
                .into();
            s.commit_view(
                TableView::new(
                    Rc::new(CheckedSource {
                        rows: rows.clone(),
                        malformed: 0,
                    }),
                    columns.clone(),
                ),
                None,
                cx,
            )
            .unwrap();
            let accepted = s.view().unwrap().clone();
            for malformed in 1..=3 {
                let candidate = TableView::new(
                    Rc::new(CheckedSource {
                        rows: rows.clone(),
                        malformed,
                    }),
                    columns.clone(),
                );
                assert!(s.commit_view(candidate, None, cx).is_err());
                assert!(Rc::ptr_eq(s.view().unwrap(), &accepted));
            }
        });
    });
}

#[gpui_kit::test]
fn replacing_source_releases_history_and_rejects_old_ticket(cx: &mut TestAppContext) {
    let (p, cx) = open(cx, 40, false);
    let s = state(&p, cx);
    p.update(cx, |p, _| p.accept = false);
    let old = s.read_with(cx, |s, _| Rc::downgrade(s.view().unwrap().source()));
    let mut ticket = None;
    cx.update(|_, cx| {
        s.update(cx, |s, cx| {
            ticket = Some(
                s.request_toggle(&TableRowKey::try_new("row-0").unwrap(), false, cx)
                    .unwrap(),
            );
        })
    });
    draw(cx);
    cx.update(|_, cx| {
        s.update(cx, |s, cx| {
            let columns = s.view().unwrap().shared_columns().clone();
            s.commit_view(TableView::new(ui_source(40, 2, true), columns), None, cx)
                .unwrap();
            let next = s.view().unwrap().as_ref().clone();
            assert_eq!(
                s.commit_view(next, ticket, cx).unwrap_err(),
                TableError::StaleRequest
            );
        })
    });
    draw(cx);
    assert!(
        old.upgrade().is_none(),
        "old frame/source must be released after replacement"
    );
}

#[path = "table/outline.rs"]
mod outline;
#[path = "table/query.rs"]
mod query;
#[path = "table/sizing.rs"]
mod sizing;

#[path = "table/edit.rs"]
mod edit;

#[path = "table/complex.rs"]
mod complex;

#[path = "table/allocation.rs"]
mod allocation;

#[path = "table/cells.rs"]
mod cells;

#[path = "table/customization.rs"]
mod customization;

#[path = "table/churn.rs"]
mod churn;

#[cfg(feature = "table-profile")]
#[path = "table/profile.rs"]
mod profile;
