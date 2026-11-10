use super::*;
use std::{sync::Arc, time::Instant};
use yue_component::{TableOutline, TableOutlineRow, TableOutlineSource, TableRowSizing, TreeNode};
struct ComplexDelegate {
    images: Vec<Arc<gpui_kit::RenderImage>>,
    calls: Rc<Cell<usize>>,
}
impl TableDelegate for ComplexDelegate {
    type Source = TableRows<UiRow>;
    fn render_cell(
        &mut self,
        row: &UiRow,
        col: &TableColumn<UiRow>,
        cell: &yue_component::TableCellContext,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        self.calls.set(self.calls.get() + 1);
        match col.key().as_str() {
            "avatar" => yue_component::YueAvatar::new("avatar", yue_component::current_theme(cx))
                .use_default_theme()
                .size(yue_component::AvatarSize::Small)
                .src(self.images[row.amount as usize % 512].clone())
                .into_any_element(),
            "status" => yue_component::YueTag::new(
                "status",
                if row.amount % 2 == 0 {
                    "已核验"
                } else {
                    "待核验"
                },
            )
            .into_any_element(),
            "action" => yue_component::YueButton::new("open", "查看")
                .size(yue_component::ButtonSize::Small)
                .tab_stop(cell.is_active())
                .on_click(|_, _, _| {})
                .into_any_element(),
            _ => col.render(row, cell, w, cx),
        }
    }
}
struct TreeDelegate {
    calls: Rc<Cell<usize>>,
}
impl TableDelegate for TreeDelegate {
    type Source = TableOutlineSource<TableRows<UiRow>>;
    fn render_cell(
        &mut self,
        row: &TableOutlineRow,
        col: &TableColumn<TableOutlineRow>,
        cell: &yue_component::TableCellContext,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        self.calls.set(self.calls.get() + 1);
        col.render(row, cell, w, cx)
    }
}
struct Host<D: TableDelegate> {
    _delegate: Entity<D>,
    state: Entity<YueTableState<D>>,
}
impl<D: TableDelegate> Render for Host<D> {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(rems(64.))
            .h(rems(32.))
            .child(YueTable::new("complex", &self.state).height(rems(28.)))
    }
}
fn run<D: TableDelegate>(
    cx: &mut TestAppContext,
    count: usize,
    build: impl FnOnce(&mut gpui_kit::App) -> (Entity<D>, TableView<D::Source>),
    calls: Rc<Cell<usize>>,
    started: Instant,
) {
    cx.update(|cx| {
        yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Neutral)).unwrap();
        cx.set_reduce_motion(true);
    });
    let (delegate, view) = cx.update(build);
    let state = cx.new(|cx| YueTableState::new(&delegate, cx));
    state.update(cx, |s, cx| s.commit_view(view, None, cx).unwrap());
    let (_, cx) = cx.add_window_view(|w, cx| {
        let host = cx.new(|_| Host {
            _delegate: delegate,
            state: state.clone(),
        });
        Root::new(host, w, cx)
    });
    cx.update(|w, _| {
        w.resize(gpui_kit::size(px(1152.), px(768.)));
        w.set_rem_size(px(16.));
    });
    for _ in 0..6 {
        draw(cx);
    }
    println!(
        "TABLE_MOUNT,rows={count},ns={}",
        started.elapsed().as_nanos()
    );
    let step = |frame: usize, cx: &mut VisualTestContext| {
        let sample = cx.update(|w, cx| {
            state
                .read(cx)
                .scroll_handle()
                .set_offset(gpui_kit::point(px(0.), px(-(((frame * 37) % 8000) as f32))));
            w.refresh();
            calls.set(0);
            let begin = Instant::now();
            w.draw(cx).clear(cx);
            (begin.elapsed().as_nanos(), calls.get())
        });
        cx.run_until_parked();
        sample
    };
    for frame in 0..120 {
        step(frame, cx);
    }
    for frame in 0..1000 {
        let (ns, renders) = step(frame + 120, cx);
        assert!(ns > 0 && renders > 0 && renders < 600);
        println!("TABLE_FRAME,rows={count},frame={frame},ns={ns},renders={renders}");
    }
    // Allocation attribution is a separate interval, after all timing samples.
    for sample in 0..10 {
        let (_, allocations, bytes) = allocation::measure(|| step(1120 + sample, cx));
        assert!(allocations > 0 && bytes > 0);
        println!(
            "TABLE_ALLOC,rows={count},sample={sample},allocations={allocations},bytes={bytes}"
        );
    }
    let owner = state.read_with(cx, |s, _| s.scroll_handle().precise_offset());
    let parts = nodes(cx);
    let body = parts
        .iter()
        .filter(|n| n.role() == Some(Role::Cell))
        .collect::<Vec<_>>();
    assert!(!body.is_empty() && body.len() < 600);
    assert!(
        body.iter()
            .any(|n| n.bounds().top() < px(448.) && n.bounds().bottom() > px(40.))
    );
    let before = calls.get();
    cx.run_until_parked();
    assert_eq!(
        before,
        calls.get(),
        "passive idle must not build more cells"
    );
    assert_eq!(
        owner,
        state.read_with(cx, |s, _| s.scroll_handle().precise_offset())
    );
}
#[gpui_kit::test]
#[ignore = "opt-in TB03 release baseline, tools/sample-table.py --workload tb03"]
fn tb03_measured_frame_sample(cx: &mut TestAppContext) {
    let count: usize = std::env::var("YUE_TABLE_BENCH_ROWS")
        .unwrap()
        .parse()
        .unwrap();
    assert!(count == 1000 || count == 20_000);
    let calls = Rc::new(Cell::new(0));
    let counter = calls.clone();
    let started = Instant::now();
    run(
        cx,
        count,
        move |cx| {
            let images = (0..512)
                .map(|i| {
                    let pixels = image::RgbaImage::from_fn(64, 64, |x, y| {
                        image::Rgba([i as u8, (x * 4) as u8, (y * 4) as u8, 255])
                    });
                    Arc::new(gpui_kit::RenderImage::new(vec![image::Frame::new(pixels)]))
                })
                .collect::<Vec<_>>();
            let bytes: usize = images.iter().map(|i| i.as_bytes(0).unwrap().len()).sum();
            assert_eq!(bytes, 512 * 64 * 64 * 4);
            println!(
                "TABLE_RESOURCE,pool=512,width=64,height=64,pixel_bytes={bytes},gpu_bytes=unmeasured"
            );
            let delegate = cx.new(|_| ComplexDelegate {
                images,
                calls: counter,
            });
            let rows = (0..count)
                .map(|i| UiRow {
                    key: format!("complex-{i}").into(),
                    amount: i as i32,
                    note: match i % 3 {
                        0 => "一行说明",
                        1 => "两行中文内容\nSecond line",
                        _ => "四行内容\n第二行\nthird line\n第四行",
                    }
                    .into(),
                })
                .collect::<Vec<_>>();
            let source = Rc::new(
                TableRows::try_new("tb03-v1", 1, rows.into(), |r: &UiRow| r.key.clone()).unwrap(),
            );
            let cols = [
                "key", "note", "amount", "status", "avatar", "action", "owner", "extra",
            ]
            .into_iter()
            .map(|key| {
                TableColumn::text(key, key, move |r: &UiRow| {
                    if key == "note" {
                        r.note.clone()
                    } else if key == "amount" {
                        r.amount.to_string().into()
                    } else {
                        r.key.clone()
                    }
                })
                .width(rems(if key == "note" { 14. } else { 7. }))
                .overflow(TableTextOverflow::Wrap)
            })
            .collect::<Vec<_>>();
            (
                delegate,
                TableView::new(source, cols)
                    .with_row_sizing(TableRowSizing::measured(rems(2.5)).unwrap()),
            )
        },
        calls,
        started,
    );
}
#[gpui_kit::test]
#[ignore = "opt-in TB06 release baseline, tools/sample-table.py --workload tb06"]
fn tb06_tree_frame_sample(cx: &mut TestAppContext) {
    let count: usize = std::env::var("YUE_TABLE_BENCH_ROWS")
        .unwrap()
        .parse()
        .unwrap();
    assert!(count == 1000 || count == 50_000);
    let calls = Rc::new(Cell::new(0));
    let counter = calls.clone();
    let started = Instant::now();
    run(
        cx,
        count,
        move |cx| {
            let source = ui_source(count, 1, false);
            let mut outline = TableOutline::new(source);
            let roots = (0..count / 100)
                .map(|r| {
                    TreeNode::new(format!("row-{}", r * 100), format!("Root {r}")).children(
                        (1..100).map(|c| {
                            TreeNode::new(format!("row-{}", r * 100 + c), format!("Child {c}"))
                        }),
                    )
                })
                .collect();
            outline.set_tree(roots).unwrap();
            for r in 0..10 {
                outline
                    .toggle_tree(&TableRowKey::try_new(format!("row-{}", r * 100)).unwrap())
                    .unwrap();
            }
            let snapshot = Rc::new(outline.snapshot(1, None).unwrap());
            assert_eq!(snapshot.len(), count / 100 + 990);
            if count == 50_000 {
                assert_eq!(snapshot.len(), 1490);
            }
            println!(
                "TABLE_TREE,nodes={count},roots={},visible={}",
                count / 100,
                snapshot.len()
            );
            let cols = (0..8)
                .map(|i| {
                    TableColumn::text(
                        format!("c{i}"),
                        format!("Column {i}"),
                        |r: &TableOutlineRow| {
                            format!("{} / {}", r.depth(), r.record_key().as_str())
                        },
                    )
                    .width(rems(8.))
                })
                .collect::<Vec<_>>();
            (
                cx.new(|_| TreeDelegate { calls: counter }),
                TableView::new(snapshot, cols),
            )
        },
        calls,
        started,
    );
}
