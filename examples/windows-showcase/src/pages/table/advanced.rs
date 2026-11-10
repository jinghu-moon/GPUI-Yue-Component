use gpui_kit::{
    AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _,
    Render, ScrollHandle, StatefulInteractiveElement as _, Styled as _, Subscription, Window, div,
    rems,
};
use std::rc::Rc;
use yue_component::{
    TableCellKey, TableColumn, TableColumnKey, TableDelegate, TableEvent, TablePin, TableRangeRows,
    TableRenderMode, TableRowKey, TableRows, TableSource, TableSpan, TableSpanPlan, TableView,
    YueButton, YueScrollbar, YueTable, YueTableState,
};
struct Matrix;
impl TableDelegate for Matrix {
    type Source = TableRows<usize>;
}
struct Range;
impl TableDelegate for Range {
    type Source = TableRangeRows<usize>;
}
pub(super) struct AdvancedExample {
    _matrix: Entity<Matrix>,
    _range: Entity<Range>,
    grid: Entity<YueTableState<Matrix>>,
    static_spans: Entity<YueTableState<Matrix>>,
    host_tables: Vec<Entity<YueTableState<Matrix>>>,
    range: Entity<YueTableState<Range>>,
    host: ScrollHandle,
    _subscription: Subscription,
    fail_next: bool,
    message: String,
}
fn matrix_rows(scope: &str, len: usize) -> Rc<TableRows<usize>> {
    Rc::new(
        TableRows::try_new(
            scope,
            1,
            (0..len).collect::<Vec<_>>().into(),
            |r: &usize| format!("r-{r}").into(),
        )
        .unwrap(),
    )
}
fn columns(count: usize) -> Vec<TableColumn<usize>> {
    (0..count)
        .map(|i| {
            TableColumn::text(
                format!("c-{i}"),
                format!("字段 {}", i + 1),
                move |r: &usize| format!("{r} · {i}"),
            )
            .width(rems(7.))
            .pin(if i == 0 {
                TablePin::Start
            } else if i + 1 == count {
                TablePin::End
            } else {
                TablePin::None
            })
        })
        .collect()
}
impl AdvancedExample {
    pub(super) fn new(cx: &mut Context<Self>) -> Self {
        let matrix = cx.new(|_| Matrix);
        let range = cx.new(|_| Range);
        let grid = cx.new(|cx| YueTableState::new(&matrix, cx));
        grid.update(cx, |s, cx| {
            s.commit_view(
                TableView::new(matrix_rows("advanced-grid", 50_000), columns(256)).with_spans(
                    TableSpanPlan::new(vec![TableSpan::new(
                        TableCellKey::new(
                            TableRowKey::try_new("r-2").unwrap(),
                            TableColumnKey::try_new("c-2").unwrap(),
                        ),
                        3,
                        3,
                    )]),
                ),
                None,
                cx,
            )
            .unwrap()
        });
        let static_spans = cx.new(|cx| YueTableState::new(&matrix, cx));
        static_spans.update(cx, |s, cx| {
            s.commit_view(
                TableView::new(
                    matrix_rows("static-spans", 6),
                    vec![
                        TableColumn::text("c-0", "自然高度合并", |r: &usize| {
                            if *r == 0 {
                                "长内容参与自然行高，覆盖区域只保留一个内容与交互。".repeat(8)
                            } else {
                                r.to_string()
                            }
                        })
                        .width(rems(8.))
                        .overflow(yue_component::TableTextOverflow::Wrap),
                        TableColumn::text("c-1", "次列", |r: &usize| r.to_string()).width(rems(8.)),
                        TableColumn::text("c-2", "邻列居中", |r: &usize| format!("记录 {r}"))
                            .width(rems(10.)),
                    ],
                )
                .with_spans(TableSpanPlan::new(vec![TableSpan::new(
                    TableCellKey::new(
                        TableRowKey::try_new("r-0").unwrap(),
                        TableColumnKey::try_new("c-0").unwrap(),
                    ),
                    3,
                    2,
                )])),
                None,
                cx,
            )
            .unwrap()
        });
        let host_tables = (0..2)
            .map(|i| {
                let state = cx.new(|cx| YueTableState::new(&matrix, cx));
                state.update(cx, |s, cx| {
                    s.commit_view(
                        TableView::new(matrix_rows(&format!("affix-{i}"), 24), columns(8)),
                        None,
                        cx,
                    )
                    .unwrap()
                });
                state
            })
            .collect();
        let ranges = cx.new(|cx| YueTableState::new(&range, cx));
        ranges.update(cx, |s, cx| {
            s.commit_view(
                TableView::new(
                    Rc::new(TableRangeRows::new("range-demo-query-1", 1, 10_000).unwrap()),
                    columns(8),
                ),
                None,
                cx,
            )
            .unwrap()
        });
        let sub = cx.subscribe(&ranges, |example, state, event: &TableEvent, cx| {
            if let TableEvent::RangeRequested(request) = event {
                if example.fail_next {
                    example.fail_next = false;
                    state.update(cx, |s, cx| {
                        let _ = s.reject_range(request, cx);
                    });
                    example.message = "本次加载失败，点击重试；旧数据保留".into();
                    cx.notify();
                    return;
                }
                let old = state.read(cx).view().unwrap().clone();
                let next = old
                    .source()
                    .with_records(old.source().revision() + 1, request.range().map(|i| (i, i)))
                    .unwrap();
                let view = TableView::new(Rc::new(next), old.shared_columns().clone());
                state.update(cx, |s, cx| {
                    let _ = s.commit_range(request, view, cx);
                });
                example.message = format!(
                    "已加载 {} / 10000 行",
                    state.read(cx).view().unwrap().source().loaded_len()
                );
                cx.notify();
            }
        });
        Self {
            _matrix: matrix,
            _range: range,
            grid,
            static_spans,
            host_tables,
            range: ranges,
            host: ScrollHandle::new(),
            _subscription: sub,
            fail_next: false,
            message: "滚动时按真实可见范围加载，未加载位置不参与选择".into(),
        }
    }
}
impl Render for AdvancedExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut affix = div().flex().flex_col().gap_4();
        for (i, table) in self.host_tables.iter().enumerate() {
            affix = affix
                .child(div().child(format!("页面内表格 {}", i + 1)))
                .child(
                    YueTable::new(("affix", i), table)
                        .scroll_host(&self.host, rems(0.), rems(0.))
                        .footer_fixed(true),
                )
                .child(div().h(rems(5.)).flex_shrink_0());
        }
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child("双轴工作集 · 50000 × 256 · 中部 3×3 合并格")
            .child(
                YueTable::new("matrix", &self.grid)
                    .render_mode(TableRenderMode::Cells)
                    .height(rems(24.))
                    .frozen_rows(1, 1),
            )
            .child("短表自然高度 rowspan：长内容撑开被跨行，邻列仍居中")
            .child(
                YueTable::new("static-spans", &self.static_spans)
                    .render_mode(TableRenderMode::Static),
            )
            .child("同一面板内两张表：滚动表头吸顶，离开自身范围解除")
            .child(
                div()
                    .id("affix-panel")
                    .relative()
                    .h(rems(26.))
                    .overflow_scroll()
                    .track_scroll(&self.host)
                    .child(affix)
                    .child(YueScrollbar::both("affix-scroll", &self.host)),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        YueButton::new("fail-next-range", "下一次范围加载模拟失败").on_click(
                            cx.listener(|s, _, _, cx| {
                                s.fail_next = true;
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        YueButton::new("retry-range", "重试加载").on_click(cx.listener(
                            |s, _, _, cx| {
                                s.range.update(cx, |s, cx| {
                                    let _ = s.retry_range(cx);
                                });
                            },
                        )),
                    ),
            )
            .child(self.message.clone())
            .child(
                YueTable::new("range-table", &self.range)
                    .range_loading(12)
                    .height(rems(20.)),
            )
    }
}
#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use crate::pages::table::tests::draw;
    use gpui_kit::base::Root;
    use gpui_kit::{TestAppContext, px, size};
    #[gpui_kit::test]
    fn advanced_real_viewport_loads_ranges_and_releases_sources(cx: &mut TestAppContext) {
        cx.update(|cx| {
            yue_component::init(
                cx,
                yue_component::YueTheme::new(
                    yue_component::ThemeMode::Light,
                    yue_component::Accent::Azure,
                ),
            )
            .unwrap()
        });
        let mut example = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            let e = cx.new(AdvancedExample::new);
            example = Some(e.clone());
            Root::new(e, w, cx)
        });
        let e = example.unwrap();
        cx.update(|w, _| w.resize(size(px(1600.), px(3000.))));
        draw(cx);
        draw(cx);
        assert!(
            e.read_with(cx, |e, cx| e
                .range
                .read(cx)
                .view()
                .unwrap()
                .source()
                .loaded_len())
                > 0
        );
        let old = e.read_with(cx, |e, cx| {
            Rc::downgrade(e.range.read(cx).view().unwrap().source())
        });
        e.update(cx, |e, cx| {
            e.fail_next = true;
            e.range
                .read(cx)
                .scroll_handle()
                .set_offset(gpui_kit::point(px(0.), px(-3000.)));
            cx.notify();
        });
        draw(cx);
        draw(cx);
        assert!(e.read_with(cx, |e, _| e.message.contains("失败")));
        e.update(cx, |e, cx| {
            e.range.update(cx, |s, cx| {
                s.retry_range(cx).unwrap();
            })
        });
        draw(cx);
        draw(cx);
        assert!(e.read_with(cx, |e, _| e.message.contains("已加载")));
        assert!(old.upgrade().is_none());
    }
}
