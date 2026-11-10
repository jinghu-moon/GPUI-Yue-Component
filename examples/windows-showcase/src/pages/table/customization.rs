use super::*;
use yue_component::{
    TableMovePlacement, TableQuery, TableRangeRequest, TableRowDrag, TableSort, TableSortDirection,
};
struct Items;
impl TableDelegate for Items {
    type Source = TableRows<usize>;
}
pub(super) struct CustomizationExample {
    _delegate: Entity<Items>,
    table: Entity<YueTableState<Items>>,
    tail: Entity<YueTableState<Items>>,
    settings: Entity<super::configuration::ColumnSettings<Items>>,
    _subscriptions: Vec<Subscription>,
    row: bool,
    sorted: bool,
    cursor: usize,
    pending: Option<TableRangeRequest>,
    failed: bool,
    message: SharedString,
}
fn rows(scope: &str, revision: u64, len: usize) -> Rc<TableRows<usize>> {
    Rc::new(
        TableRows::try_new(
            scope,
            revision,
            (0..len).collect::<Vec<_>>().into(),
            |i: &usize| format!("item-{i}").into(),
        )
        .unwrap(),
    )
}
fn columns() -> Vec<TableColumn<usize>> {
    vec![
        TableColumn::text("drag", "移动", |_: &usize| "")
            .width(rems(4.))
            .row_drag_handle(true)
            .configurable(false),
        TableColumn::value("number", "编号", |i: &usize| *i)
            .display_with(|i| i.to_string().into())
            .sortable(true)
            .build()
            .unwrap()
            .width(rems(9.))
            .reorderable(true)
            .resizable(true),
        TableColumn::text("title", "标题", |i: &usize| format!("待办任务 {}", i + 1))
            .width(rems(16.))
            .reorderable(true)
            .resizable(true),
        TableColumn::text("note", "说明", |i: &usize| format!("记录 {i} 的说明"))
            .width(rems(16.))
            .reorderable(true)
            .resizable(true),
    ]
}
impl CustomizationExample {
    pub(super) fn new(cx: &mut Context<Self>) -> Self {
        let delegate = cx.new(|_| Items);
        let table = cx.new(|cx| YueTableState::new(&delegate, cx));
        let tail = cx.new(|cx| YueTableState::new(&delegate, cx));
        table.update(cx, |s, cx| {
            s.commit_view(
                TableView::new(rows("move-demo", 1, 1000), columns()),
                None,
                cx,
            )
            .unwrap()
        });
        tail.update(cx, |s, cx| {
            s.commit_view(
                TableView::new(
                    rows("tail-demo", 1, 30),
                    columns().into_iter().skip(1).collect::<Vec<_>>(),
                ),
                None,
                cx,
            )
            .unwrap()
        });
        let settings = cx.new(|cx| super::configuration::ColumnSettings::new(&table, cx));
        let subscriptions = vec![
            cx.subscribe(&table, |s, table, event: &TableEvent, cx| {
                let old = table.read(cx).view().unwrap().clone();
                let next = match event {
                    TableEvent::RowMoveRequested(r) => {
                        r.apply_to_rows(&old).map(|next| (next, r.id()))
                    }
                    TableEvent::ColumnMoveRequested(r) => {
                        r.apply_to(&old).map(|next| (next, r.id()))
                    }
                    TableEvent::ColumnResizeRequested(r) => {
                        r.apply_to(&old).map(|next| (next, r.id()))
                    }
                    // This example rejects active sort requests to keep its current order.
                    TableEvent::QueryRequested(r) => {
                        let _ = table.update(cx, |t, cx| t.reject_request(r.id(), cx));
                        s.message = "请使用排序开关观察重排拒绝；此例保持原数据顺序".into();
                        cx.notify();
                        return;
                    }
                    _ => return,
                };
                match next {
                    Ok((next, id)) => {
                        table.update(cx, |t, cx| t.commit_view(next, Some(id), cx).unwrap());
                        s.message = "宿主已接受，一次请求改变一次真实顺序".into();
                    }
                    Err(e) => s.message = e.to_string().into(),
                }
                cx.notify();
            }),
            cx.subscribe(&tail, |s, _, event: &TableEvent, cx| {
                if let TableEvent::RangeRequested(r) = event {
                    s.pending = Some(r.clone());
                    s.failed = false;
                    s.message = "已到尾部：模拟远程等待，可完成或失败".into();
                    cx.notify();
                }
            }),
        ];
        Self {
            _delegate: delegate,
            table,
            tail,
            settings,
            _subscriptions: subscriptions,
            row: false,
            sorted: false,
            cursor: 0,
            pending: None,
            failed: false,
            message: "拖动手柄或用空格＋方向键＋Enter；Escape取消".into(),
        }
    }
    fn append(&mut self, cx: &mut Context<Self>) {
        let Some(request) = self.pending.take() else {
            return;
        };
        let old = self.tail.read(cx).view().unwrap().clone();
        let next = TableView::new(
            rows(
                "tail-demo",
                old.source().revision() + 1,
                old.source().len() + 30,
            ),
            old.declared_columns().to_vec(),
        );
        if self
            .tail
            .update(cx, |s, cx| s.commit_range(&request, next, cx))
            .is_ok()
        {
            self.cursor += 1;
            self.message = "追加30条，旧前缀及滚动位置保留".into();
        }
        cx.notify();
    }
}
impl Render for CustomizationExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let head: Vec<_> = {
            let view = self.table.read(cx).view().unwrap();
            (0..6)
                .filter_map(|i| view.source().row_key(i).map(|k| k.as_str().to_owned()))
                .collect()
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        YueButton::new(
                            "row-drag-mode",
                            if self.row {
                                "整行拖动已开启"
                            } else {
                                "启用整行拖动"
                            },
                        )
                        .on_click(cx.listener(|s, _, _, cx| {
                            s.row = !s.row;
                            cx.notify();
                        })),
                    )
                    .child(
                        YueButton::new(
                            "row-drag-sort",
                            if self.sorted {
                                "关闭排序限制"
                            } else {
                                "模拟排序激活（拒绝行重排）"
                            },
                        )
                        .on_click(cx.listener(|s, _, _, cx| {
                            s.sorted = !s.sorted;
                            s.table.update(cx, |t, cx| {
                                let old = t.view().unwrap().as_ref().clone();
                                let q = if s.sorted {
                                    TableQuery::default().with_sorts(vec![TableSort::new(
                                        yue_component::TableColumnKey::try_new("number").unwrap(),
                                        TableSortDirection::Ascending,
                                    )])
                                } else {
                                    TableQuery::default()
                                };
                                t.commit_view(old.with_query(q), None, cx).unwrap();
                            });
                            cx.notify();
                        })),
                    )
                    .child(
                        YueButton::new("move-first-after-third", "将首行移到第三行后").on_click(
                            cx.listener(|s, _, _, cx| {
                                let old = s.table.read(cx).view().unwrap().clone();
                                let a = old.source().row_key(0).unwrap().clone();
                                let b = old.source().row_key(2).unwrap().clone();
                                if let Err(e) = s.table.update(cx, |t, cx| {
                                    t.request_row_move(&a, &b, TableMovePlacement::After, cx)
                                }) {
                                    s.message = e.to_string().into();
                                    cx.notify();
                                }
                            }),
                        ),
                    ),
            )
            .child(self.settings.clone())
            .child(format!("实际前六行：{}", head.join(" → ")))
            .child(
                YueTable::new("custom-table", &self.table)
                    .height(rems(20.))
                    .row_drag(if self.row {
                        TableRowDrag::Row
                    } else {
                        TableRowDrag::Handle
                    }),
            )
            .child("未知总量／尾游标追加：到尾部后只发一次请求，失败需要手动重试。三批后结束。")
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        YueButton::new("tail-go-end", "滚动到末尾").on_click(cx.listener(
                            |s, _, _, cx| {
                                let key = {
                                    let v = s.tail.read(cx).view().unwrap();
                                    v.source().row_key(v.source().len() - 1).unwrap().clone()
                                };
                                let _ = s.tail.update(cx, |s, cx| {
                                    s.scroll_to_row(&key, Default::default(), cx)
                                });
                            },
                        )),
                    )
                    .child(
                        YueButton::new("tail-complete", "完成追加")
                            .disabled(self.pending.is_none())
                            .on_click(cx.listener(|s, _, _, cx| s.append(cx))),
                    )
                    .child(
                        YueButton::new("tail-fail", "模拟失败")
                            .disabled(self.pending.is_none())
                            .on_click(cx.listener(|s, _, _, cx| {
                                if let Some(r) = s.pending.take() {
                                    let _ = s.tail.update(cx, |s, cx| s.reject_range(&r, cx));
                                    s.failed = true;
                                    s.message = "加载失败；旧行继续可用".into();
                                    cx.notify();
                                }
                            })),
                    )
                    .child(
                        YueButton::new("tail-retry", "重试追加")
                            .disabled(!self.failed)
                            .on_click(cx.listener(|s, _, _, cx| {
                                let _ = s.tail.update(cx, |s, cx| s.retry_range(cx));
                            })),
                    ),
            )
            .child(
                YueTable::new("tail-table", &self.tail)
                    .height(rems(14.))
                    .tail_loading(format!("cursor-{}", self.cursor), rems(3.), self.cursor < 3),
            )
            .child(self.message.clone())
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use crate::pages::table::tests::{click, draw};
    use gpui_kit::base::{Root, test_support::snapshots};
    use gpui_kit::{Role, TestAppContext, VisualTestContext, px, size};
    fn open(cx: &mut TestAppContext) -> (Entity<CustomizationExample>, &mut VisualTestContext) {
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
        let mut entity = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            let view = cx.new(CustomizationExample::new);
            entity = Some(view.clone());
            Root::new(view, w, cx)
        });
        cx.update(|w, _| w.resize(size(px(1600.), px(2200.))));
        draw(cx);
        (entity.unwrap(), cx)
    }
    fn toggle_note(cx: &mut VisualTestContext) {
        let point = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.role() == Some(Role::CheckBox) && n.label() == Some("说明"))
                .unwrap()
                .bounds()
                .center()
        });
        cx.simulate_click(point, Default::default());
        draw(cx);
    }
    #[gpui_kit::test]
    fn settings_cancel_confirm_and_row_move_use_real_controls(cx: &mut TestAppContext) {
        let (p, cx) = open(cx);
        click(cx, "table-column-settings");
        toggle_note(cx);
        click(cx, "dialog-cancel");
        assert!(p.read_with(cx, |p, cx| {
            p.table
                .read(cx)
                .view()
                .unwrap()
                .columns()
                .iter()
                .find(|c| c.key().as_str() == "note")
                .unwrap()
                .is_visible()
        }));
        click(cx, "table-column-settings");
        toggle_note(cx);
        click(cx, "dialog-confirm");
        assert!(!p.read_with(cx, |p, cx| {
            p.table
                .read(cx)
                .view()
                .unwrap()
                .columns()
                .iter()
                .find(|c| c.key().as_str() == "note")
                .unwrap()
                .is_visible()
        }));
        click(cx, "move-first-after-third");
        assert_eq!(
            p.read_with(cx, |p, cx| p
                .table
                .read(cx)
                .view()
                .unwrap()
                .source()
                .row_key(2)
                .unwrap()
                .as_str()
                .to_owned()),
            "item-0"
        );
        click(cx, "row-drag-sort");
        let revision = p.read_with(cx, |p, cx| p.table.read(cx).view().unwrap().revision());
        click(cx, "move-first-after-third");
        assert_eq!(
            p.read_with(cx, |p, cx| p.table.read(cx).view().unwrap().revision()),
            revision
        );
        assert!(p.read_with(cx, |p, _| p.message.contains("UnsupportedOperation")));
    }
    #[gpui_kit::test]
    fn tail_append_failure_retry_keeps_prefix_and_old_source_releases(cx: &mut TestAppContext) {
        let (p, cx) = open(cx);
        click(cx, "tail-go-end");
        draw(cx);
        draw(cx);
        assert!(p.read_with(cx, |p, _| p.pending.is_some()));
        let weak = p.read_with(cx, |p, cx| {
            Rc::downgrade(p.tail.read(cx).view().unwrap().source())
        });
        click(cx, "tail-fail");
        draw(cx);
        assert!(p.read_with(cx, |p, _| p.pending.is_none() && p.failed));
        click(cx, "tail-retry");
        draw(cx);
        assert!(p.read_with(cx, |p, _| p.pending.is_some()));
        click(cx, "tail-complete");
        draw(cx);
        draw(cx);
        assert_eq!(
            p.read_with(cx, |p, cx| p.tail.read(cx).view().unwrap().source().len()),
            60
        );
        assert_eq!(
            p.read_with(cx, |p, cx| p
                .tail
                .read(cx)
                .view()
                .unwrap()
                .source()
                .row_key(0)
                .unwrap()
                .as_str()
                .to_owned()),
            "item-0"
        );
        assert!(weak.upgrade().is_none());
        assert!(
            p.read_with(cx, |p, _| p.pending.is_none()),
            "new data moves the tail away, so no request loop"
        );
    }
}
