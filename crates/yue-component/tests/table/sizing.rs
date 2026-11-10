use super::*;
use yue_component::TableRowSizing;

#[gpui_kit::test]
fn known_row_heights_follow_keys_after_reordering_and_reject_incomplete_snapshots(
    cx: &mut TestAppContext,
) {
    let (probe, cx) = open(cx, 40, false);
    let state = state(&probe, cx);
    cx.update(|_, cx| {
        state.update(cx, |s, cx| {
            let view = s.view().unwrap();
            let heights = (0..40).map(|i| {
                (
                    TableRowKey::try_new(format!("row-{i}")).unwrap(),
                    rems(if i == 0 { 6. } else { 3. }),
                )
            });
            s.commit_view(
                view.as_ref()
                    .clone()
                    .with_row_sizing(TableRowSizing::known(heights).unwrap()),
                None,
                cx,
            )
            .unwrap();
        })
    });
    draw(cx);
    // Cell excludes the row's one-pixel bottom separator.
    close(cell(cx, "row-0", "name").bounds().size.height, px(95.));
    close(cell(cx, "row-1", "name").bounds().size.height, px(47.));
    cx.update(|_, cx| {
        state.update(cx, |s, cx| {
            let current = s.view().unwrap().clone();
            assert!(
                s.commit_view(
                    current
                        .as_ref()
                        .clone()
                        .with_row_sizing(TableRowSizing::known([]).unwrap()),
                    None,
                    cx
                )
                .is_err()
            );
            assert!(Rc::ptr_eq(s.view().unwrap(), &current));
        })
    });
    // Reorder the actual source and verify the same keys retain their heights.
    cx.update(|_, cx| {
        state.update(cx, |s, cx| {
            let old = s.view().unwrap();
            s.commit_view(
                TableView::new(ui_source(40, 2, true), old.shared_columns().clone())
                    .with_row_sizing(old.row_sizing().clone()),
                None,
                cx,
            )
            .unwrap();
            s.scroll_to_row(
                &TableRowKey::try_new("row-0").unwrap(),
                yue_component::VirtualizerScrollOptions::default(),
                cx,
            )
            .unwrap();
        })
    });
    for _ in 0..3 {
        draw(cx);
    }
    close(cell(cx, "row-0", "name").bounds().size.height, px(95.));
    close(cell(cx, "row-1", "name").bounds().size.height, px(47.));
    assert!(TableRowSizing::measured(rems(0.)).is_err());
}

#[gpui_kit::test]
fn measured_rows_wrap_align_short_cells_and_resize_without_losing_anchor(cx: &mut TestAppContext) {
    let (probe, cx) = open(cx, 1000, false);
    let state = state(&probe, cx);
    let configure = |cx: &mut VisualTestContext, width: f32| {
        cx.update(|_, cx| {
            state.update(cx, |s, cx| {
                let current = s.view().unwrap().as_ref().clone();
                let cols: Rc<[_]> = current
                    .columns()
                    .iter()
                    .map(|col| {
                        if col.key().as_str() == "note" {
                            col.clone().width(rems(width))
                        } else {
                            col.clone()
                        }
                    })
                    .collect::<Vec<_>>()
                    .into();
                s.commit_view(
                    current
                        .with_columns(cols)
                        .with_row_sizing(TableRowSizing::measured(rems(2.5)).unwrap()),
                    None,
                    cx,
                )
                .unwrap();
            })
        });
        for _ in 0..4 {
            draw(cx);
        }
    };
    configure(cx, 5.);
    let tall = cell(cx, "row-0", "note").bounds();
    assert!(
        tall.size.height > px(40.),
        "natural wrapped height must replace estimate: {tall:?}"
    );
    let short = cell(cx, "row-0", "name").bounds();
    close(tall.size.height, short.size.height);
    let content = nodes(cx)
        .into_iter()
        .find(|n| {
            n.path().contains(&row_id("row-0"))
                && n.path().contains(&ElementId::from("name"))
                && n.path().last() == Some(&ElementId::from("cell-content"))
        })
        .unwrap();
    close(content.bounds().center().y, short.center().y);
    configure(cx, 30.);
    assert!(cell(cx, "row-0", "note").bounds().size.height < tall.size.height);
}

#[gpui_kit::test]
fn measured_deep_anchor_reflow_revises_content_and_settles_without_redraw_loop(
    cx: &mut TestAppContext,
) {
    let (probe, cx) = open(cx, 20_000, false);
    let state = state(&probe, cx);
    cx.update(|_, cx| {
        state.update(cx, |s, cx| {
            let next = s
                .view()
                .unwrap()
                .as_ref()
                .clone()
                .with_row_sizing(TableRowSizing::measured(rems(2.5)).unwrap());
            s.commit_view(next, None, cx).unwrap();
            s.scroll_to_row(
                &TableRowKey::try_new("row-10000").unwrap(),
                yue_component::VirtualizerScrollOptions::default(),
                cx,
            )
            .unwrap();
        })
    });
    for _ in 0..5 {
        draw(cx);
    }
    let parts = nodes(cx);
    let anchor = (9900..=10000)
        .map(|i| format!("row-{i}"))
        .find(|key| {
            parts.iter().any(|n| {
                n.path().contains(&row_id(key))
                    && n.role() == Some(Role::Cell)
                    && n.bounds().top() <= px(40.)
                    && n.bounds().bottom() > px(40.)
            })
        })
        .expect("a real row intersects the clear edge below the frozen header");
    let before = cell(cx, &anchor, "name").bounds().top();
    cx.update(|_, cx| {
        state.update(cx, |s, cx| {
            let old = s.view().unwrap();
            let rows = (0..20_000)
                .map(|i| UiRow {
                    key: format!("row-{i}").into(),
                    note: if i < 10_005 {
                        "新修订：多行内容\n第二行\n第三行\n第四行"
                    } else {
                        "短"
                    }
                    .into(),
                    amount: i,
                })
                .collect::<Vec<_>>();
            let source = Rc::new(
                TableRows::try_new("test", 2, rows.into(), |r: &UiRow| r.key.clone()).unwrap(),
            );
            let next = TableView::new(source, old.shared_columns().clone())
                .with_row_sizing(old.row_sizing().clone());
            s.commit_view(next, None, cx).unwrap();
        })
    });
    for _ in 0..6 {
        draw(cx);
    }
    let after = cell(cx, &anchor, "name").bounds().top();
    close(before, after);
    assert!(cell(cx, &anchor, "note").bounds().size.height > px(40.));
    let offset = state.read_with(cx, |s, _| s.scroll_handle().precise_offset());
    for _ in 0..8 {
        draw(cx);
    }
    assert_eq!(
        offset,
        state.read_with(cx, |s, _| s.scroll_handle().precise_offset())
    );
    assert!(
        nodes(cx)
            .iter()
            .filter(|n| n.role() == Some(Role::Cell))
            .count()
            < 180
    );
}
