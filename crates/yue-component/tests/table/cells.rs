use super::*;
use gpui_kit::{InteractiveElement as _, StatefulInteractiveElement as _};
use yue_component::{TableCellKey, TableSpan, TableSpanPlan, VirtualizerScrollOptions};
fn key(r: usize, c: usize) -> TableCellKey {
    TableCellKey::new(
        TableRowKey::try_new(format!("row-{r}")).unwrap(),
        yue_component::TableColumnKey::try_new(format!("col-{c}")).unwrap(),
    )
}
#[gpui_kit::test]
fn cells_builds_bounded_two_axis_workset_and_locates_both_axes(cx: &mut TestAppContext) {
    let (p, cx) = open(cx, 50_000, true);
    p.update(cx, |p, cx| {
        p.mode = TableRenderMode::Cells;
        p.frozen = (1, 1);
        cx.notify();
    });
    layout::configure(&p, cx, |_| {
        (0..256)
            .map(|c| {
                TableColumn::text(
                    format!("col-{c}"),
                    format!("Column {c}"),
                    move |r: &UiRow| format!("{}:{c}", r.key),
                )
                .width(rems(5. + (c % 3) as f32))
                .pin(if c < 2 {
                    yue_component::TablePin::Start
                } else if c >= 254 {
                    yue_component::TablePin::End
                } else {
                    yue_component::TablePin::None
                })
            })
            .collect()
    });
    let s = state(&p, cx);
    for (r, c) in [(0, 0), (25_000, 128), (49_998, 250), (120, 8), (0, 0)] {
        p.update(cx, |p, _| p.calls.set(0));
        s.update(cx, |s, cx| {
            s.scroll_to_cell(&key(r, c), VirtualizerScrollOptions::default(), cx)
                .unwrap()
        });
        draw(cx);
        draw(cx);
        let count = p.read_with(cx, |p, _| p.calls.get());
        assert!(count < 1000, "built {count} cells");
        let target = cell(cx, &format!("row-{r}"), &format!("col-{c}"));
        assert_eq!(target.row_index(), Some(r + 1));
        assert_eq!(target.column_index(), Some(c));
        assert!(target.bounds().size.width > px(0.));
        let left = cell(cx, &format!("row-{r}"), "col-0");
        let right = cell(cx, &format!("row-{r}"), "col-255");
        assert!(left.bounds().right() <= target.bounds().right());
        assert!(right.bounds().right() <= px(1025.));
    }
}
#[gpui_kit::test]
fn sparse_spans_reject_offscreen_overlap_and_render_one_origin(cx: &mut TestAppContext) {
    let (p, cx) = open(cx, 1000, true);
    p.update(cx, |p, cx| {
        p.mode = TableRenderMode::Cells;
        cx.notify();
    });
    draw(cx);
    let s = state(&p, cx);
    let old = s.read_with(cx, |s, _| s.view().unwrap().clone());
    s.update(cx, |s, cx| {
        s.commit_view(
            old.as_ref()
                .clone()
                .with_spans(TableSpanPlan::new(vec![TableSpan::new(key(10, 2), 5, 3)])),
            None,
            cx,
        )
        .unwrap();
    });
    s.update(cx, |s, cx| {
        s.scroll_to_cell(&key(12, 3), VirtualizerScrollOptions::default(), cx)
            .unwrap()
    });
    draw(cx);
    draw(cx);
    let origin = cell(cx, "row-10", "col-2");
    assert_eq!(origin.row_span(), Some(5));
    assert_eq!(origin.column_span(), Some(3));
    close(origin.bounds().size.width, px(384.));
    close(origin.bounds().size.height, px(199.));
    assert!(
        !nodes(cx)
            .iter()
            .any(|n| n.path().contains(&row_id("row-12"))
                && n.path().last() == Some(&ElementId::from("col-3"))
                && n.role() == Some(Role::Cell))
    );
    let accepted = s.read_with(cx, |s, _| s.view().unwrap().clone());
    s.update(cx, |s, cx| {
        for bad in [
            vec![
                TableSpan::new(key(900, 2), 2, 3),
                TableSpan::new(key(901, 3), 3, 2),
            ],
            vec![TableSpan::new(key(999, 2), 2, 1)],
            vec![TableSpan::new(key(900, 11), 1, 2)],
        ] {
            assert!(
                s.commit_view(
                    accepted
                        .as_ref()
                        .clone()
                        .with_spans(TableSpanPlan::new(bad)),
                    None,
                    cx
                )
                .is_err()
            );
            assert!(Rc::ptr_eq(s.view().unwrap(), &accepted));
        }
    });
}
#[gpui_kit::test]
fn rows_colspan_uses_shared_tracks_without_duplicate_content(cx: &mut TestAppContext) {
    let (p, cx) = open(cx, 40, true);
    let s = state(&p, cx);
    s.update(cx, |s, cx| {
        let old = s.view().unwrap().as_ref().clone();
        s.commit_view(
            old.with_spans(TableSpanPlan::new(vec![TableSpan::new(key(0, 1), 1, 3)])),
            None,
            cx,
        )
        .unwrap();
    });
    draw(cx);
    draw(cx);
    let origin = cell(cx, "row-0", "col-1");
    assert_eq!(origin.column_span(), Some(3));
    close(origin.bounds().size.width, px(384.));
    assert!(
        !nodes(cx).iter().any(|n| n.path().contains(&row_id("row-0"))
            && n.path().last() == Some(&ElementId::from("col-2"))
            && n.role() == Some(Role::Cell))
    );
}
struct HostProbe {
    probes: Vec<Entity<Probe>>,
    scroll: gpui_kit::ScrollHandle,
    inset: gpui_kit::Rems,
}
impl Render for HostProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut content = div()
            .w(rems(80.))
            .flex()
            .flex_col()
            .gap_4()
            .child(div().h(rems(10.)).flex_shrink_0());
        for p in &self.probes {
            let state = p.read(cx).state.clone();
            content = content
                .child(
                    YueTable::new(("host-table", p.entity_id()), &state)
                        .scroll_host(&self.scroll, self.inset, rems(0.))
                        .footer_fixed(true),
                )
                .child(div().h(rems(10.)).flex_shrink_0());
        }
        div()
            .id("page-owner")
            .w(rems(64.))
            .h(rems(30.))
            .overflow_scroll()
            .track_scroll(&self.scroll)
            .child(content)
    }
}
#[gpui_kit::test]
fn two_host_tables_share_owner_without_offscreen_clamp_and_respect_chrome(cx: &mut TestAppContext) {
    cx.update(|cx| {
        yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap()
    });
    let mut host = None;
    let (_, cx) = cx.add_window_view(|w, cx| {
        let h = cx.new(|cx| HostProbe {
            probes: vec![
                cx.new(|cx| Probe::new(40, false, cx)),
                cx.new(|cx| Probe::new(40, false, cx)),
            ],
            scroll: gpui_kit::ScrollHandle::new(),
            inset: rems(2.),
        });
        host = Some(h.clone());
        Root::new(h, w, cx)
    });
    cx.update(|w, _| w.resize(gpui_kit::size(px(1152.), px(768.))));
    draw(cx);
    draw(cx);
    let host = host.unwrap();
    let scroll = host.read_with(cx, |h, _| h.scroll.clone());
    let (second, second_id) = host.read_with(cx, |h, cx| {
        let p = &h.probes[1];
        (
            p.read(cx).state.clone(),
            ElementId::from(("host-table", p.entity_id())),
        )
    });
    second.update(cx, |s, cx| {
        s.scroll_to_row(
            &TableRowKey::try_new("row-20").unwrap(),
            VirtualizerScrollOptions::default(),
            cx,
        )
        .unwrap()
    });
    draw(cx);
    draw(cx);
    let ns = nodes(cx);
    let target = ns
        .iter()
        .find(|n| {
            n.path().contains(&second_id)
                && n.path().contains(&row_id("row-20"))
                && n.path().last() == Some(&ElementId::from("name"))
                && n.role() == Some(Role::Cell)
        })
        .unwrap();
    assert!(target.bounds().top() >= px(32.) && target.bounds().bottom() <= px(480.));

    for y in [400., 1100., 1900., 2300., 200., 0.] {
        scroll.set_offset(gpui_kit::point(px(0.), px(-y)));
        draw(cx);
        draw(cx);
        close(scroll.offset().y, px(-y));
        let ns = nodes(cx);
        let headers: Vec<_> = ns
            .iter()
            .filter(|n| {
                n.role() == Some(Role::ColumnHeader)
                    && n.path().last() == Some(&ElementId::from("name"))
            })
            .collect();
        assert!(
            headers
                .iter()
                .filter(|n| n.bounds().top() >= px(32.) && n.bounds().top() < px(480.))
                .count()
                <= 1
        );
    }
    scroll.set_offset(gpui_kit::point(px(0.), px(-400.)));
    draw(cx);
    draw(cx);
    let ns = nodes(cx);
    let h = ns
        .iter()
        .find(|n| {
            n.role() == Some(Role::ColumnHeader)
                && n.bounds().top() < px(480.)
                && n.path().last() == Some(&ElementId::from("name"))
        })
        .unwrap();
    close(h.bounds().top(), px(32.));
    host.update(cx, |h, cx| {
        h.inset = rems(4.);
        cx.notify();
    });
    draw(cx);
    draw(cx);
    let ns = nodes(cx);
    let h = ns
        .iter()
        .find(|n| {
            n.role() == Some(Role::ColumnHeader)
                && n.bounds().top() < px(480.)
                && n.path().last() == Some(&ElementId::from("name"))
        })
        .unwrap();
    close(h.bounds().top(), px(64.));
    host.update(cx, |h, cx| {
        h.probes.remove(0);
        cx.notify();
    });
    draw(cx);
    draw(cx);
    close(scroll.offset().y, px(-400.));
}
#[gpui_kit::test]
fn cross_pin_group_has_one_semantic_header_and_two_visual_fragments(cx: &mut TestAppContext) {
    let (p, cx) = open(cx, 40, true);
    p.update(cx, |p, cx| {
        p.mode = TableRenderMode::Cells;
        cx.notify();
    });
    layout::configure(&p, cx, |mut columns| {
        let g = yue_component::TableHeaderGroup::new("group", "Shared");
        columns[0] = columns[0]
            .clone()
            .header_path(vec![g.clone()])
            .pin(yue_component::TablePin::Start);
        columns[1] = columns[1].clone().header_path(vec![g]);
        columns
    });
    let ns = nodes(cx);
    assert_eq!(
        ns.iter()
            .filter(|n| n.label() == Some("Shared") && n.role() == Some(Role::ColumnHeader))
            .count(),
        1
    );
    let h = cell(cx, "table-header", "col-2");
    assert_eq!(h.row_span(), Some(2));
    close(h.bounds().size.height, px(79.));
}
struct RangeDelegate;
impl TableDelegate for RangeDelegate {
    type Source = yue_component::TableRangeRows<i32>;
}
#[gpui_kit::test]
fn range_tickets_isolate_queries_owners_retries_and_placeholders(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let d = cx.new(|_| RangeDelegate);
        let a = cx.new(|cx| YueTableState::new(&d, cx));
        let b = cx.new(|cx| YueTableState::new(&d, cx));
        let src = Rc::new(yue_component::TableRangeRows::<i32>::new("query-1", 1, 1000).unwrap());
        let cols = vec![TableColumn::text("v", "Value", |r: &i32| r.to_string())];
        for s in [&a, &b] {
            s.update(cx, |s, cx| {
                s.commit_view(
                    TableView::new(src.clone(), cols.clone())
                        .selection_mode(TableSelectionMode::Multiple),
                    None,
                    cx,
                )
                .unwrap()
            });
        }
        let first = a.update(cx, |s, cx| s.request_range(0..20, cx).unwrap().unwrap());
        assert!(
            a.update(cx, |s, cx| s.request_range(2..10, cx).unwrap())
                .is_none()
        );
        let second = a.update(cx, |s, cx| s.request_range(100..120, cx).unwrap().unwrap());
        assert_eq!(
            a.update(cx, |s, cx| s.reject_range(&first, cx)),
            Err(TableError::StaleRequest)
        );
        assert_eq!(
            b.update(cx, |s, cx| s.reject_range(&second, cx)),
            Err(TableError::StaleRequest)
        );
        a.update(cx, |s, cx| s.reject_range(&second, cx).unwrap());
        let retry = a.update(cx, |s, cx| s.retry_range(cx).unwrap().unwrap());
        assert_ne!(second.id(), retry.id());
        let loaded = Rc::new(
            src.with_records(2, (100..120).map(|i| (i, i as i32)))
                .unwrap(),
        );
        assert_eq!(loaded.loaded_len(), 20);
        let candidate =
            TableView::new(loaded, cols.clone()).selection_mode(TableSelectionMode::Multiple);
        assert_eq!(
            a.update(cx, |s, cx| s.commit_range(&second, candidate.clone(), cx)),
            Err(TableError::StaleRequest)
        );
        a.update(cx, |s, cx| s.commit_range(&retry, candidate, cx).unwrap());
        a.update(cx, |s, cx| {
            let request = s.request_select_all(true, cx).unwrap();
            s.reject_request(request, cx).unwrap();
            assert!(
                s.request_toggle(&TableRowKey::try_new("slot-0").unwrap(), false, cx)
                    .is_err()
            );
        });
        let late = a.update(cx, |s, cx| s.request_range(0..20, cx).unwrap().unwrap());
        a.update(cx, |s, cx| {
            s.commit_view(
                TableView::new(
                    Rc::new(yue_component::TableRangeRows::<i32>::new("query-2", 1, 1000).unwrap()),
                    cols,
                ),
                None,
                cx,
            )
            .unwrap()
        });
        assert_eq!(
            a.update(cx, |s, cx| s.reject_range(&late, cx)),
            Err(TableError::StaleRequest)
        );
    });
}
#[gpui_kit::test]
#[ignore = "opt-in TB02 release size baseline; tools/sample-table.py --workload tb02"]
fn tb02_cells_frame_sample(cx: &mut TestAppContext) {
    use std::time::Instant;
    let count: usize = std::env::var("YUE_TABLE_BENCH_ROWS")
        .unwrap()
        .parse()
        .unwrap();
    assert!(count == 1000 || count == 50_000);
    let start = Instant::now();
    let (p, cx) = open(cx, count, true);
    p.update(cx, |p, cx| {
        p.mode = TableRenderMode::Cells;
        p.frozen = (1, 1);
        cx.notify();
    });
    layout::configure(&p, cx, |_| {
        (0..256)
            .map(|c| {
                TableColumn::text(
                    format!("col-{c}"),
                    format!("Column {c}"),
                    move |r: &UiRow| format!("{}:{c}", r.key),
                )
                .width(rems(5. + (c % 3) as f32))
                .pin(if c < 2 {
                    yue_component::TablePin::Start
                } else if c >= 254 {
                    yue_component::TablePin::End
                } else {
                    yue_component::TablePin::None
                })
            })
            .collect()
    });
    let s = state(&p, cx);
    s.update(cx, |s, cx| {
        let v = s.view().unwrap().as_ref().clone();
        let sizes = (0..count).map(|r| {
            (
                TableRowKey::try_new(format!("row-{r}")).unwrap(),
                rems(if r + 1 == count {
                    2.5
                } else {
                    2.5 + (r % 3) as f32 * 0.5
                }),
            )
        });
        s.commit_view(
            v.with_row_sizing(yue_component::TableRowSizing::known(sizes).unwrap())
                .with_spans(TableSpanPlan::new(
                    (0..16)
                        .map(|i| TableSpan::new(key(10 + i * 20, 10 + i * 12), 3, 2))
                        .collect::<Vec<_>>(),
                )),
            None,
            cx,
        )
        .unwrap();
    });
    draw(cx);
    draw(cx);
    let mount = start.elapsed().as_nanos();
    let calls = p.read_with(cx, |p, _| p.calls.clone());
    cx.update(|w, cx| {
        w.set_rem_size(px(16.));
        cx.set_reduce_motion(true);
    });
    let step = |frame: usize, cx: &mut VisualTestContext| {
        let sample = cx.update(|w, cx| {
            s.read(cx).scroll_handle().set_offset(gpui_kit::point(
                px(-(((frame * 97) % 16_000) as f32)),
                px(-(((frame * 37) % 8000) as f32)),
            ));
            w.refresh();
            calls.set(0);
            let start = Instant::now();
            w.draw(cx).clear(cx);
            (start.elapsed().as_nanos(), calls.get())
        });
        cx.run_until_parked();
        sample
    };
    for frame in 0..120 {
        step(frame, cx);
    }
    println!("TABLE_MOUNT,rows={count},ns={mount}");
    for frame in 0..1000 {
        let (ns, renders) = step(frame + 120, cx);
        assert!(renders > 0 && renders < 600);
        println!("TABLE_FRAME,rows={count},frame={frame},ns={ns},renders={renders}");
    }
    for sample in 0..10 {
        let (_, allocations, bytes) = super::allocation::measure(|| step(sample + 2000, cx));
        println!(
            "TABLE_ALLOC,rows={count},sample={sample},allocations={allocations},bytes={bytes}"
        );
    }
    println!(
        "TABLE_MATRIX,rows={count},columns=256,sizing=Known,pins=2+2,frozen_rows=1+1,spans=16"
    );
}
