use super::*;
use yue_component::{
    TableDataMode, TableFilterValue, TableLocalQuery, TableProjection, TableQuery, TableSort,
    TableSortDirection, TableTotal,
};

struct QueryRecord {
    key: SharedString,
    name: SharedString,
    amount: i64,
    done: bool,
    optional: Option<i64>,
    date: chrono::NaiveDate,
    metadata: QueryMetadata,
    category: usize,
}
struct QueryMetadata {
    priority: usize,
}
fn query_source(count: usize) -> Rc<TableRows<QueryRecord>> {
    Rc::new(
        TableRows::try_new(
            "query-orders",
            1,
            (0..count)
                .map(|i| QueryRecord {
                    key: format!("order-{i}").into(),
                    name: format!("设备{}.[*]", i % 13).into(),
                    amount: (i % 19) as i64,
                    done: i % 3 == 0,
                    optional: (i % 4 != 0).then_some(i as i64),
                    date: chrono::NaiveDate::from_ymd_opt(
                        2026,
                        1 + (i % 12) as u32,
                        1 + (i % 28) as u32,
                    )
                    .unwrap(),
                    metadata: QueryMetadata { priority: i % 5 },
                    category: i % 7,
                })
                .collect::<Vec<_>>()
                .into(),
            |r: &QueryRecord| r.key.clone(),
        )
        .unwrap(),
    )
}
#[test]
fn typed_local_query_matches_independent_oracle_and_never_clones_records() {
    let source = query_source(250_000);
    let amount = TableColumn::value("amount", "Amount", |r: &QueryRecord| r.amount)
        .display_with(|v| v.to_string().into())
        .sortable(true)
        .filterable(true);
    let amount_key = amount.filter_key();
    let done = TableColumn::value("done", "Done", |r: &QueryRecord| r.done)
        .display_with(|v| v.to_string().into())
        .sortable(true)
        .filterable(true);
    let done_key = done.filter_key();
    let name = TableColumn::value("name", "Name", |r: &QueryRecord| r.name.clone())
        .display_with(|v| v.clone())
        .filter_scalar(|actual, expected| actual.contains(expected.as_ref()));
    let name_key = name.filter_key();
    let optional = TableColumn::value("optional", "Optional", |r: &QueryRecord| r.optional)
        .display_with(|v| format!("{v:?}").into())
        .missing_when(Option::is_none);
    let missing_key = optional.filter_key();
    let date = TableColumn::value("date", "日期", |r: &QueryRecord| r.date)
        .display_with(|v| v.to_string().into())
        .sortable(true)
        .filterable(true);
    let date_key = date.filter_key();
    let priority = TableColumn::value("priority", "优先级", |r: &QueryRecord| {
        r.metadata.priority
    })
    .display_with(|v| v.to_string().into())
    .sortable(true)
    .filterable(true);
    let priority_key = priority.filter_key();
    let columns: Rc<[_]> = vec![
        amount.build().unwrap(),
        done.build().unwrap(),
        name.build().unwrap(),
        optional.build().unwrap(),
        date.build().unwrap(),
        priority.build().unwrap(),
        TableColumn::value("category", "分类", |r: &QueryRecord| r.category)
            .display_with(|v| v.to_string().into())
            .sortable(true)
            .build()
            .unwrap(),
        TableColumn::text("key", "编号", |r: &QueryRecord| r.key.clone()),
    ]
    .into();
    for mode in 0..8 {
        let mut query = TableQuery::paged(2, 37).unwrap().with_sorts(vec![
            TableSort::new(amount_key.column().clone(), TableSortDirection::Descending),
            TableSort::new(done_key.column().clone(), TableSortDirection::Ascending),
        ]);
        query = match mode {
            0 => query,
            1 => query
                .with_filter(&amount_key, Some(TableFilterValue::Scalar(0)))
                .unwrap(),
            2 => query
                .with_filter(&done_key, Some(TableFilterValue::Scalar(false)))
                .unwrap(),
            3 => query
                .with_filter(
                    &amount_key,
                    Some(TableFilterValue::Range { start: 4, end: 9 }),
                )
                .unwrap()
                .with_filter(&done_key, Some(TableFilterValue::Scalar(false)))
                .unwrap(),
            4 => query
                .with_filter(&name_key, Some(TableFilterValue::Scalar(".[*]".into())))
                .unwrap(),
            5 => query
                .with_filter(&missing_key, Some(TableFilterValue::Missing))
                .unwrap(),
            6 => query
                .with_filter(
                    &date_key,
                    Some(TableFilterValue::Range {
                        start: chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
                        end: chrono::NaiveDate::from_ymd_opt(2026, 5, 31).unwrap(),
                    }),
                )
                .unwrap(),
            _ => query
                .with_filter(&priority_key, Some(TableFilterValue::Set(vec![0, 3])))
                .unwrap(),
        }
        .with_page(2, 37)
        .unwrap();
        let view =
            TableLocalQuery::execute(source.clone(), columns.clone(), query, 2 + mode).unwrap();
        let mut oracle: Vec<usize> = (0..250_000)
            .filter(|i| match mode {
                1 => i % 19 == 0,
                2 => i % 3 != 0,
                3 => (4..=9).contains(&(i % 19)) && i % 3 != 0,
                5 => i % 4 == 0,
                6 => (2..=4).contains(&(i % 12)),
                7 => i % 5 == 0 || i % 5 == 3,
                _ => true,
            })
            .collect();
        oracle.sort_by_key(|i| (std::cmp::Reverse(i % 19), i % 3 == 0));
        assert_eq!(view.total(), TableTotal::Known(oracle.len()));
        assert_eq!(view.data_mode(), TableDataMode::Local);
        assert_eq!(
            (0..view.source().len())
                .map(|i| view.source().row_key(i).unwrap().as_str().to_owned())
                .collect::<Vec<_>>(),
            oracle
                .iter()
                .skip(37)
                .take(37)
                .map(|i| format!("order-{i}"))
                .collect::<Vec<_>>()
        );
        let key = view.source().row_key(0).unwrap();
        assert!(std::ptr::eq(
            view.source().row(key).unwrap(),
            source.row(key).unwrap()
        ));
        assert_eq!(source.row_key(0).unwrap().as_str(), "order-0");
    }
    let invalid = TableQuery::default()
        .with_filter(
            &amount_key,
            Some(TableFilterValue::Range { start: 10, end: 0 }),
        )
        .unwrap();
    assert!(TableLocalQuery::execute(source.clone(), columns.clone(), invalid, 100).is_err());
    let replacement = TableColumn::value("amount", "Amount", |r: &QueryRecord| r.amount)
        .display_with(|v| v.to_string().into())
        .filterable(true)
        .build()
        .unwrap();
    let old = TableQuery::default()
        .with_filter(&amount_key, Some(TableFilterValue::Scalar(0)))
        .unwrap();
    assert!(
        TableLocalQuery::execute(source, vec![replacement].into(), old, 101).is_err(),
        "same key with different semantics rejects the old filter"
    );
}
struct QueryDelegate;
impl TableDelegate for QueryDelegate {
    type Source = TableProjection<TableRows<QueryRecord>>;
}
#[gpui_kit::test]
fn pending_query_survives_width_changes_but_not_rule_replacement(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let delegate = cx.new(|_| QueryDelegate);
        let state = cx.new(|cx| YueTableState::new(&delegate, cx));
        let make = || {
            TableColumn::value("amount", "Amount", |r: &QueryRecord| r.amount)
                .display_with(|v| v.to_string().into())
                .sortable(true)
                .build()
                .unwrap()
        };
        let columns: Rc<[_]> = vec![make()].into();
        let key = columns[0].key().clone();
        state.update(cx, |s, cx| {
            s.commit_view(
                TableLocalQuery::execute(query_source(10), columns, TableQuery::default(), 1)
                    .unwrap(),
                None,
                cx,
            )
            .unwrap();
            let id = s.request_sort(&key, false, cx).unwrap();
            let current = s.view().unwrap();
            let columns: Rc<[_]> = vec![current.columns()[0].clone().width(rems(20.))].into();
            s.commit_view(current.as_ref().clone().with_columns(columns), None, cx)
                .unwrap();
            assert!(s.is_current_request(id));
            let columns: Rc<[_]> = vec![make()].into();
            s.commit_view(
                s.view().unwrap().as_ref().clone().with_columns(columns),
                None,
                cx,
            )
            .unwrap();
            assert!(!s.is_current_request(id));
            assert!(s.reject_request(id, cx).is_err());
        });
    });
}

struct PipelineProbe {
    _delegate: Entity<QueryDelegate>,
    state: Entity<YueTableState<QueryDelegate>>,
    calls: Rc<std::cell::Cell<usize>>,
    _subscription: gpui_kit::Subscription,
}
impl PipelineProbe {
    fn new(cx: &mut gpui_kit::Context<Self>) -> Self {
        let calls = Rc::new(std::cell::Cell::new(0));
        let count = calls.clone();
        let column = TableColumn::value("amount", "Amount", |r: &QueryRecord| r.amount)
            .display_with(|v| v.to_string().into())
            .filter_scalar(move |_, _| {
                count.set(count.get() + 1);
                true
            });
        let filter = column.filter_key();
        let delegate = cx.new(|_| QueryDelegate);
        let state = cx.new(|cx| YueTableState::new(&delegate, cx));
        let query = TableQuery::default()
            .with_filter(&filter, Some(TableFilterValue::Scalar(0)))
            .unwrap();
        state.update(cx, |s, cx| {
            s.commit_view(
                TableLocalQuery::execute(
                    query_source(1000),
                    vec![column.build().unwrap()].into(),
                    query,
                    1,
                )
                .unwrap()
                .selection_mode(TableSelectionMode::Multiple),
                None,
                cx,
            )
            .unwrap()
        });
        let subscription = cx.subscribe(&state, |s: &mut Self, _, event: &TableEvent, cx| {
            if let TableEvent::SelectionRequested(request) = event {
                let current = s.state.read(cx).view().unwrap();
                let next = request.apply_to(current).unwrap();
                s.state.update(cx, |state, cx| {
                    state.commit_view(next, Some(request.id()), cx).unwrap()
                });
            }
        });
        calls.set(0);
        Self {
            _delegate: delegate,
            state,
            calls,
            _subscription: subscription,
        }
    }
}
impl gpui_kit::Render for PipelineProbe {
    fn render(
        &mut self,
        _: &mut gpui_kit::Window,
        _: &mut gpui_kit::Context<Self>,
    ) -> impl gpui_kit::IntoElement {
        yue_component::YueTable::new("pipeline", &self.state).height(rems(20.))
    }
}
#[gpui_kit::test]
fn scrolling_selection_and_theme_never_repeat_query_predicates(cx: &mut TestAppContext) {
    use gpui_kit::Focusable as _;
    cx.update(|cx| {
        yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap()
    });
    let mut probe = None;
    let (_, cx) = cx.add_window_view(|w, cx| {
        let view = cx.new(PipelineProbe::new);
        probe = Some(view.clone());
        Root::new(view, w, cx)
    });
    let probe = probe.unwrap();
    draw(cx);
    let state = probe.read_with(cx, |s, _| s.state.clone());
    for offset in [0., -200., -3000., -100., 0.] {
        cx.update(|w, cx| {
            state
                .read(cx)
                .scroll_handle()
                .set_offset(gpui_kit::point(px(0.), px(offset)));
            state.read(cx).focus_handle(cx).focus(w, cx);
            w.refresh();
        });
        draw(cx);
    }
    cx.simulate_keystrokes("ctrl-a");
    draw(cx);
    assert_eq!(
        state.read_with(cx, |s, _| s.view().unwrap().selected_keys().len()),
        1000
    );
    cx.update(|w, cx| {
        w.set_rem_size(px(24.));
        yue_component::set_theme(cx, YueTheme::new(ThemeMode::Dark, Accent::Azure));
    });
    draw(cx);
    assert_eq!(probe.read_with(cx, |s, _| s.calls.get()), 0);
}

#[test]
fn typed_column_validation_empty_recovery_and_page_clamping() {
    assert!(
        TableColumn::value("a", "A", |r: &QueryRecord| r.amount)
            .build()
            .is_err()
    );
    let column = TableColumn::value("a", "A", |r: &QueryRecord| r.amount)
        .display_with(|v| v.to_string().into())
        .filterable(true);
    let key = column.filter_key();
    let columns: Rc<[_]> = vec![column.build().unwrap()].into();
    let source = query_source(99);
    let query = TableQuery::paged(99, 10)
        .unwrap()
        .with_filter(&key, Some(TableFilterValue::Set(vec![])))
        .unwrap();
    let empty = TableLocalQuery::execute(source.clone(), columns.clone(), query, 2).unwrap();
    assert_eq!(empty.total(), TableTotal::Known(0));
    assert_eq!(empty.query().page().unwrap().number(), 1);
    assert!(empty.source().is_empty());
    let recovered = TableLocalQuery::execute(
        source,
        columns,
        empty
            .query()
            .clone()
            .clear_filters()
            .with_page(99, 10)
            .unwrap(),
        3,
    )
    .unwrap();
    assert_eq!(recovered.total(), TableTotal::Known(99));
    assert_eq!(recovered.query().page().unwrap().number(), 10);
    assert_eq!(recovered.source().len(), 9);
    let calls = std::cell::Cell::new(0);
    let (page, sum) = TableLocalQuery::execute_with_summary(
        query_source(99),
        recovered.shared_columns().clone(),
        TableQuery::paged(2, 10).unwrap(),
        4,
        |source, keys| {
            calls.set(calls.get() + 1);
            keys.iter()
                .map(|key| source.row(key).unwrap().amount)
                .sum::<i64>()
        },
    )
    .unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(page.source().len(), 10);
    assert_eq!(sum, (0..99).map(|i| i % 19).sum::<i64>());
}

struct RemoteDelegate;
impl TableDelegate for RemoteDelegate {
    type Source = yue_component::TableRemotePage<TableRows<Record>>;
}
#[gpui_kit::test]
fn remote_page_keeps_selected_keys_without_fabricating_records_and_totals_are_atomic(
    cx: &mut TestAppContext,
) {
    let (state, events, _sub, _delegate) = cx.update(|cx| {
        let delegate = cx.new(|_| RemoteDelegate);
        let state = cx.new(|cx| YueTableState::new(&delegate, cx));
        let outside = TableRowKey::try_new("outside").unwrap();
        let page = Rc::new(yue_component::TableRemotePage::new(
            source(1, &["a", "b"]),
            [outside.clone()],
        ));
        let columns: Rc<[_]> = vec![TableColumn::text("v", "Value", |r: &Record| {
            r.value.to_string()
        })]
        .into();
        let base = TableView::new(page.clone(), columns)
            .with_query(TableQuery::paged(2, 2).unwrap())
            .with_result(TableDataMode::Remote, TableTotal::Known(4))
            .selection_mode(TableSelectionMode::Multiple)
            .selection([outside.clone()]);
        state.update(cx, |s, cx| {
            s.commit_view(base, None, cx).unwrap();
            assert!(page.contains_key(&outside));
            assert!(page.row(&outside).is_none());
            let accepted = s.view().unwrap().clone();
            let bad = accepted
                .as_ref()
                .clone()
                .with_result(TableDataMode::Remote, TableTotal::Known(0));
            assert!(s.commit_view(bad, None, cx).is_err());
            assert!(Rc::ptr_eq(s.view().unwrap(), &accepted));
            s.commit_view(
                accepted
                    .as_ref()
                    .clone()
                    .with_result(TableDataMode::Remote, TableTotal::Unknown),
                None,
                cx,
            )
            .unwrap();
            assert_eq!(s.view().unwrap().total(), TableTotal::Unknown);
        });
        let events = Rc::new(std::cell::RefCell::new(Vec::new()));
        let capture = events.clone();
        let sub = cx.subscribe(&state, move |_, e: &TableEvent, _| {
            if let TableEvent::SelectionRequested(r) = e {
                capture.borrow_mut().push(r.clone());
            }
        });
        state.update(cx, |s, cx| {
            s.request_select_all(true, cx).unwrap();
        });
        // Explicit event flushing is performed by the TestAppContext boundary.
        (state, events, sub, delegate)
    });
    cx.update(|cx| {
        let request = events.borrow()[0].clone();
        state.update(cx, |s, cx| {
            let next = request.apply_to(s.view().unwrap()).unwrap();
            assert_eq!(next.selected_keys().len(), 3);
            s.commit_view(next, Some(request.id()), cx).unwrap();
            s.request_select_all(false, cx).unwrap();
        });
    });
    cx.update(|cx| {
        let request = events.borrow()[1].clone();
        state.update(cx, |s, cx| {
            let next = request.apply_to(s.view().unwrap()).unwrap();
            assert_eq!(next.selected_keys().len(), 1);
            assert!(
                next.selected_keys()
                    .contains(&TableRowKey::try_new("outside").unwrap())
            );
            s.commit_view(next, Some(request.id()), cx).unwrap();
        });
    });
}
#[gpui_kit::test]
fn query_tickets_reject_old_success_failure_and_keep_unrelated_selection(cx: &mut TestAppContext) {
    let (state, events, _subscription, source, columns, filter, first, second, _delegate) = cx
        .update(|cx| {
            let delegate = cx.new(|_| QueryDelegate);
            let state = cx.new(|cx| YueTableState::new(&delegate, cx));
            let amount = TableColumn::value("amount", "Amount", |r: &QueryRecord| r.amount)
                .display_with(|v| v.to_string().into())
                .sortable(true)
                .filterable(true);
            let filter = amount.filter_key();
            let columns: Rc<[_]> = vec![amount.build().unwrap()].into();
            let source = query_source(100);
            state.update(cx, |s, cx| {
                s.commit_view(
                    TableLocalQuery::execute(
                        source.clone(),
                        columns.clone(),
                        TableQuery::paged(1, 10).unwrap(),
                        1,
                    )
                    .unwrap(),
                    None,
                    cx,
                )
                .unwrap()
            });
            let events = Rc::new(std::cell::RefCell::new(Vec::new()));
            let capture = events.clone();
            let _sub = cx.subscribe(&state, move |_, event: &TableEvent, _| {
                if let TableEvent::QueryRequested(r) = event {
                    capture.borrow_mut().push(r.clone());
                }
            });
            let first = state.update(cx, |s, cx| {
                s.request_filter(&filter, Some(TableFilterValue::Scalar(0)), cx)
                    .unwrap()
            });
            let second = state.update(cx, |s, cx| {
                s.request_sort(filter.column(), false, cx).unwrap()
            });
            assert!(!state.read(cx).is_current_request(first));
            assert!(state.read(cx).is_current_request(second));
            // Event delivery is settled before deriving responses.
            (
                state, events, _sub, source, columns, filter, first, second, delegate,
            )
        });
    cx.update(|cx| {
        let events = events.borrow();
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[1].target().filter(&filter),
            Some(&TableFilterValue::Scalar(0))
        );
        let accepted = state.read(cx).view().unwrap().clone();
        let old_result = TableLocalQuery::execute(
            source.clone(),
            columns.clone(),
            events[0].apply_to(&accepted).unwrap(),
            2,
        )
        .unwrap();
        assert!(
            state
                .update(cx, |s, cx| s.commit_view(old_result, Some(first), cx))
                .is_err()
        );
        assert!(Rc::ptr_eq(state.read(cx).view().unwrap(), &accepted));
        assert!(state.read(cx).is_current_request(second));
        let result =
            TableLocalQuery::execute(source, columns, events[1].apply_to(&accepted).unwrap(), 2)
                .unwrap();
        state.update(cx, |s, cx| s.commit_view(result, Some(second), cx).unwrap());
        let final_view = state.read(cx).view().unwrap().clone();
        assert!(
            state
                .update(cx, |s, cx| s.reject_request(first, cx))
                .is_err()
        );
        assert!(Rc::ptr_eq(state.read(cx).view().unwrap(), &final_view));
        assert!(state.read(cx).pending_request().is_none());
    });
}

#[gpui_kit::test]
#[ignore = "opt-in TB04 query pipeline; run prebuilt release executable with tools/sample-table-query.py"]
fn tb04_query_pipeline_sample(cx: &mut TestAppContext) {
    #[cfg(feature = "table-profile")]
    let capture = super::profile::start();
    use std::time::Instant;
    let source = query_source(250_000);
    let amount = TableColumn::value("amount", "Amount", |r: &QueryRecord| r.amount)
        .display_with(|v| v.to_string().into())
        .sortable(true)
        .filterable(true);
    let amount_key = amount.filter_key();
    let done = TableColumn::value("done", "Done", |r: &QueryRecord| r.done)
        .display_with(|v| v.to_string().into())
        .sortable(true)
        .filterable(true);
    let done_key = done.filter_key();
    let name = TableColumn::value("name", "Name", |r: &QueryRecord| r.name.clone())
        .display_with(|v| v.clone())
        .filter_scalar(|a, b| a.contains(b.as_ref()));
    let name_key = name.filter_key();
    let optional = TableColumn::value("optional", "Optional", |r: &QueryRecord| r.optional)
        .display_with(|v| format!("{v:?}").into())
        .missing_when(Option::is_none);
    let optional_key = optional.filter_key();
    let date = TableColumn::value("date", "Date", |r: &QueryRecord| r.date)
        .display_with(|v| v.to_string().into())
        .filterable(true);
    let date_key = date.filter_key();
    let priority = TableColumn::value("priority", "Priority", |r: &QueryRecord| {
        r.metadata.priority
    })
    .display_with(|v| v.to_string().into())
    .filterable(true);
    let priority_key = priority.filter_key();
    let columns: Rc<[_]> = vec![
        amount.build().unwrap(),
        done.build().unwrap(),
        name.build().unwrap(),
        optional.build().unwrap(),
        date.build().unwrap(),
        priority.build().unwrap(),
        TableColumn::text("key", "Key", |r: &QueryRecord| r.key.clone()),
        TableColumn::text("category", "Category", |r: &QueryRecord| {
            r.category.to_string()
        }),
    ]
    .into();
    let delegate = cx.update(|cx| cx.new(|_| QueryDelegate));
    let state = cx.update(|cx| cx.new(|cx| YueTableState::new(&delegate, cx)));
    let mut revision = 0;
    for mode in 0..11 {
        let base = TableQuery::default().with_sorts(vec![
            TableSort::new(amount_key.column().clone(), TableSortDirection::Descending),
            TableSort::new(done_key.column().clone(), TableSortDirection::Ascending),
        ]);
        let query = match mode {
            0 => base,
            1 => base
                .with_filter(&amount_key, Some(TableFilterValue::Scalar(0)))
                .unwrap(),
            2 => base
                .with_filter(&done_key, Some(TableFilterValue::Scalar(false)))
                .unwrap(),
            3 => base
                .with_filter(
                    &amount_key,
                    Some(TableFilterValue::Range { start: 4, end: 9 }),
                )
                .unwrap()
                .with_filter(&done_key, Some(TableFilterValue::Scalar(false)))
                .unwrap(),
            4 => base
                .with_filter(&name_key, Some(TableFilterValue::Scalar(".[*]".into())))
                .unwrap(),
            5 => base
                .with_filter(&optional_key, Some(TableFilterValue::Missing))
                .unwrap(),
            6 => base
                .with_filter(
                    &date_key,
                    Some(TableFilterValue::Range {
                        start: chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
                        end: chrono::NaiveDate::from_ymd_opt(2026, 5, 31).unwrap(),
                    }),
                )
                .unwrap(),
            7 => base
                .with_filter(&priority_key, Some(TableFilterValue::Set(vec![0, 3])))
                .unwrap(),
            8 => base
                .with_filter(&amount_key, Some(TableFilterValue::Scalar(-1)))
                .unwrap(),
            9 => base
                .with_filter(&amount_key, Some(TableFilterValue::Scalar(-1)))
                .unwrap()
                .clear_filters(),
            _ => base.with_page(2, 37).unwrap(),
        };
        let mut oracle: Vec<usize> = (0..250_000)
            .filter(|i| match mode {
                1 => i % 19 == 0,
                2 => i % 3 != 0,
                3 => (4..=9).contains(&(i % 19)) && i % 3 != 0,
                5 => i % 4 == 0,
                6 => (2..=4).contains(&(i % 12)),
                7 => i % 5 == 0 || i % 5 == 3,
                8 => false,
                _ => true,
            })
            .collect();
        oracle.sort_by_key(|i| (std::cmp::Reverse(i % 19), i % 3 == 0));
        let total = oracle.len();
        if mode == 10 {
            oracle = oracle.into_iter().skip(37).take(37).collect();
        }
        let expected = oracle
            .iter()
            .map(|i| format!("order-{i}"))
            .collect::<Vec<_>>();
        // Full ordering oracle is checked outside every measured interval.
        for sample in 0..6 {
            revision += 1;
            let start = Instant::now();
            let (view, stages) = TableLocalQuery::execute_profiled(
                source.clone(),
                columns.clone(),
                query.clone(),
                revision,
            )
            .unwrap();
            let query_ns = start.elapsed().as_nanos();
            assert_eq!(view.total(), TableTotal::Known(total));
            assert_eq!(
                (0..view.source().len())
                    .map(|i| view.source().row_key(i).unwrap().as_str().to_owned())
                    .collect::<Vec<_>>(),
                expected
            );
            let accepted_ns = cx.update(|cx| {
                let start = Instant::now();
                state.update(cx, |s, cx| s.commit_view(view, None, cx).unwrap());
                start.elapsed().as_nanos()
            });
            println!(
                "TABLE_QUERY,mode={mode},sample={sample},query={query_ns},validate={},filter={},sort={},projection={},accept={accepted_ns},total={total},rows={}",
                stages.validation.as_nanos(),
                stages.filter.as_nanos(),
                stages.sort.as_nanos(),
                stages.projection.as_nanos(),
                expected.len()
            );
        }
    }
    #[cfg(feature = "table-profile")]
    super::profile::finish(capture);
}
