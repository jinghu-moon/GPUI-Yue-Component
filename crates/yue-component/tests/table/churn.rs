use super::*;
use std::time::Instant;

/// TB05 measures full immutable snapshot publication separately from preparation
/// and draw/effects. It does not claim that TableRows offers partial storage.
#[gpui_kit::test]
#[ignore = "opt-in TB05 snapshot churn; TABLE_CHURN_MEASURED selects the independent variant"]
fn tb05_snapshot_churn(cx: &mut TestAppContext) {
    #[cfg(feature = "table-profile")]
    let capture = super::profile::start();
    let measured = std::env::var("TABLE_CHURN_MEASURED").is_ok_and(|v| v == "1");
    let (p, cx) = open(cx, 10_000, true);
    let observed = Rc::new(Cell::new(-1));
    layout::configure(&p, cx, |_| {
        (0..16)
            .map(|i| {
                let observed = observed.clone();
                TableColumn::text(
                    format!("col-{i}"),
                    format!("Field {i}"),
                    move |r: &UiRow| {
                        if r.key.as_ref() == "row-0" {
                            observed.set(r.amount);
                        }
                        if i == 0 {
                            r.note.clone()
                        } else {
                            r.amount.to_string().into()
                        }
                    },
                )
                .width(rems(8.))
                .overflow(if measured {
                    TableTextOverflow::Wrap
                } else {
                    TableTextOverflow::Ellipsis
                })
            })
            .collect()
    });
    let s = state(&p, cx);
    if measured {
        s.update(cx, |s, cx| {
            let v = s
                .view()
                .unwrap()
                .as_ref()
                .clone()
                .with_row_sizing(yue_component::TableRowSizing::measured(rems(2.5)).unwrap());
            s.commit_view(v, None, cx).unwrap();
        });
        draw(cx);
        draw(cx);
    }
    let mut history = Vec::new();
    for step in 0..100 {
        let current = s.read_with(cx, |s, _| s.view().unwrap().clone());
        history.push(Rc::downgrade(current.source()));
        let start = Instant::now();
        let changed = |i: usize| i == 0 || i == 9000 || (128..192).contains(&i);
        let records = (0..current.source().len())
            .map(|i| {
                let r = current
                    .source()
                    .row(current.source().row_key(i).unwrap())
                    .unwrap();
                UiRow {
                    key: r.key.clone(),
                    note: if changed(i) {
                        if step % 2 == 0 {
                            "内容更新".into()
                        } else {
                            "长文本更新与换行反馈；".repeat(8).into()
                        }
                    } else {
                        r.note.clone()
                    },
                    amount: if changed(i) { step + 1 } else { r.amount },
                }
            })
            .collect::<Vec<_>>();
        let source = Rc::new(
            TableRows::try_new(
                "test",
                current.source().revision() + 1,
                records.into(),
                |r: &UiRow| r.key.clone(),
            )
            .unwrap(),
        );
        let prepare = start.elapsed().as_nanos();
        let next = TableView::new(source, current.declared_columns().to_vec())
            .with_row_sizing(current.row_sizing().clone());
        let start = Instant::now();
        s.update(cx, |s, cx| {
            s.commit_view(next, None, cx).unwrap();
        });
        let accept = start.elapsed().as_nanos();
        drop(current);
        let start = Instant::now();
        draw(cx);
        draw(cx);
        let draw_effects = start.elapsed().as_nanos();
        assert_eq!(
            observed.get(),
            step + 1,
            "visible cell must read the latest accepted record"
        );
        assert!(
            history.iter().all(|s| s.upgrade().is_none()),
            "completed frames must release all historical sources"
        );
        println!(
            "TABLE_CHURN,measured={},step={step},prepare_ns={prepare},accept_ns={accept},draw_effects_ns={draw_effects},historical_sources=0",
            usize::from(measured)
        );
    }
    let calls = p.read_with(cx, |p, _| p.calls.get());
    cx.run_until_parked();
    assert_eq!(
        p.read_with(cx, |p, _| p.calls.get()),
        calls,
        "idle must not rebuild cells"
    );
    #[cfg(feature = "table-profile")]
    super::profile::finish(capture);
}
