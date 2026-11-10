use super::*;
use yue_component::{
    TableOutline, TableOutlineRow, TableOutlineSource, TableRowSizing, TreeLoadTicket, TreeNode,
    YueDescriptionItem, YueDescriptions,
};

struct OutlineOrders {
    host: gpui_kit::WeakEntity<StructureExample>,
}
impl TableDelegate for OutlineOrders {
    type Source = TableOutlineSource<TableRows<Order>>;
    fn render_detail(
        &mut self,
        row: &TableOutlineRow,
        _: gpui_kit::Pixels,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        self.host
            .read_with(cx, |s, _| {
                s.detail_renders.set(s.detail_renders.get() + 1);
                let record = s.source.row(row.record_key()).unwrap();
                let Some((_, nested)) = s.nested.get(row.record_key()) else {
                    return div().into_any_element();
                };
                div()
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(YueDescriptions::new("record-details").columns(2).items([
                        YueDescriptionItem::new("owner", "负责人").text(record.name.clone()),
                        YueDescriptionItem::new("amount", "金额").text(record.amount.to_string()),
                        YueDescriptionItem::new("note", "说明").text(record.note.clone()),
                    ]))
                    .child("关联工单（独立子表滚动区域）")
                    .child(YueTable::new("nested-orders", nested).height(rems(8.)))
                    .into_any_element()
            })
            .unwrap_or_else(|_| div().into_any_element())
    }
}
pub(super) struct StructureExample {
    _delegate: Entity<OutlineOrders>,
    table: Entity<YueTableState<OutlineOrders>>,
    nested:
        std::collections::BTreeMap<TableRowKey, (Entity<Orders>, Entity<YueTableState<Orders>>)>,
    detail_renders: std::cell::Cell<usize>,
    outline: TableOutline<TableRows<Order>>,
    source: Rc<TableRows<Order>>,
    revision: u64,
    ticket: Option<TreeLoadTicket>,
    cascade: bool,
    page: Option<yue_component::TablePage>,
    message: SharedString,
    _subscription: Subscription,
}
fn key(ix: usize) -> TableRowKey {
    TableRowKey::try_new(format!("order-{ix}")).unwrap()
}
impl StructureExample {
    pub(super) fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let source = records(24, 1, false);
        let mut outline = TableOutline::new(source.clone())
            .with_check_mode(yue_component::TreeCheckMode::Cascade)
            .unwrap();
        outline
            .set_tree(vec![
                TreeNode::new("order-0", "设备巡检").children([
                    TreeNode::new("order-1", "主轴检查"),
                    TreeNode::new("order-2", "温升记录")
                        .children([TreeNode::new("order-3", "温度明细")]),
                ]),
                TreeNode::new("order-4", "待加载分组").unloaded(),
                TreeNode::new("order-8", "其他工单"),
            ])
            .unwrap();
        outline
            .set_details((0..24).map(|ix| {
                (
                    key(ix),
                    TableRowKey::try_new(format!("detail-order-{ix}")).unwrap(),
                )
            }))
            .unwrap();
        let weak = cx.weak_entity();
        let delegate = cx.new(|_| OutlineOrders { host: weak });
        let table = cx.new(|cx| YueTableState::new(&delegate, cx));
        let host = cx.weak_entity();
        let details = host.clone();
        let amounts = source.clone();
        let columns: Rc<[_]> = vec![
            TableColumn::custom(
                "name",
                "工单结构",
                move |row: &TableOutlineRow, cell, _, _| {
                    let host = host.clone();
                    let row_key = row.record_key().clone();
                    let depth = row.depth();
                    let expanded = row.is_expanded();
                    let branch = row.is_branch();
                    let name = row.record_key().as_str().to_owned();
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .pl(rems(depth as f32))
                        .children(branch.then(|| {
                            let button = YueButton::new(
                                "tree-disclosure",
                                if expanded { "折叠" } else { "展开" },
                            )
                            .accessibility_label(format!(
                                "{}{}",
                                if expanded { "折叠" } else { "展开" },
                                name
                            ))
                            .size(yue_component::ButtonSize::Small)
                            .tab_stop(cell.is_active())
                            .on_click(move |_, _, cx| {
                                let _ = host.update(cx, |s, cx| {
                                    s.outline.toggle_tree(&row_key).unwrap();
                                    if s.outline
                                        .tree()
                                        .unwrap()
                                        .node(row_key.as_str())
                                        .is_some_and(|node| {
                                            matches!(
                                                node.child_state(),
                                                yue_component::TreeChildren::Unloaded
                                            )
                                        })
                                    {
                                        s.ticket = s.outline.request_children(&row_key).unwrap();
                                        s.message = "节点加载中；可模拟失败后重试".into();
                                    }
                                    s.publish(cx);
                                });
                            });
                            if let Some(focus) = cell.control_focus() {
                                button.track_focus(focus)
                            } else {
                                button
                            }
                        }))
                        .child(name)
                },
            )
            .width(rems(27.))
            .interactive(true),
            TableColumn::text("amount", "金额", move |row: &TableOutlineRow| {
                amounts.row(row.record_key()).unwrap().amount.to_string()
            })
            .width(rems(9.)),
            TableColumn::custom(
                "details",
                "详情",
                move |row: &TableOutlineRow, cell, _, _| {
                    let host = details.clone();
                    let key = row.record_key().clone();
                    let button = YueButton::new("detail-disclosure", "切换详情")
                        .size(yue_component::ButtonSize::Small)
                        .tab_stop(cell.is_active())
                        .on_click(move |_, _, cx| {
                            let _ = host.update(cx, |s, cx| {
                                s.toggle_detail(&key, cx);
                                s.publish(cx);
                            });
                        });
                    if let Some(focus) = cell.control_focus() {
                        button.track_focus(focus)
                    } else {
                        button
                    }
                },
            )
            .interactive(true)
            .width(rems(10.)),
        ]
        .into();
        table.update(cx, |s, cx| {
            s.commit_view(
                TableView::new(Rc::new(outline.snapshot(1, None).unwrap()), columns)
                    .selection_mode(TableSelectionMode::Multiple)
                    .with_row_sizing(TableRowSizing::measured(rems(2.5)).unwrap()),
                None,
                cx,
            )
            .unwrap()
        });
        let subscription = cx.subscribe(&table, |s: &mut Self, _, e: &TableEvent, cx| {
            if let TableEvent::SelectionRequested(r) = e {
                let view = s.table.read(cx).view().unwrap();
                let next = if s.cascade {
                    s.outline.apply_selection(r, view)
                } else {
                    r.apply_to(view)
                };
                let Ok(next) = next else {
                    s.message = "该分组尚未完整加载，暂不能级联选择".into();
                    s.table
                        .update(cx, |state, cx| state.reject_request(r.id(), cx).unwrap());
                    cx.notify();
                    return;
                };
                s.table.update(cx, |state, cx| {
                    state.commit_view(next, Some(r.id()), cx).unwrap()
                });
            }
        });
        Self {
            _delegate: delegate,
            table,
            nested: Default::default(),
            detail_renders: Default::default(),
            outline,
            source,
            revision: 1,
            ticket: None,
            cascade: false,
            page: None,
            message: "树展开与详情分别控制；关闭详情不创建内容".into(),
            _subscription: subscription,
        }
    }
    fn toggle_detail(&mut self, key: &TableRowKey, cx: &mut Context<Self>) {
        if !self.outline.toggle_detail(key).unwrap() {
            self.nested.remove(key);
            return;
        }
        let nested_delegate = cx.new(|_| Orders);
        let nested = cx.new(|cx| YueTableState::new(&nested_delegate, cx));
        nested.update(cx, |s, cx| {
            s.commit_view(
                TableView::new(
                    records(12, 1, false),
                    vec![
                        TableColumn::text("key", "关联编号", |r: &Order| r.key.clone()),
                        TableColumn::text("note", "处理说明", |r: &Order| r.note.clone())
                            .width(rems(26.)),
                    ],
                ),
                None,
                cx,
            )
            .unwrap()
        });
        self.nested.insert(key.clone(), (nested_delegate, nested));
    }
    fn publish(&mut self, cx: &mut Context<Self>) {
        self.revision += 1;
        let current = self.table.read(cx).view().unwrap().clone();
        let projection = self.outline.snapshot(self.revision, self.page).unwrap();
        let total = projection.root_total();
        let source = Rc::new(projection);
        let mut next = TableView::new(source, current.shared_columns().clone())
            .selection_mode(current.selection_kind())
            .selection(current.selected_keys().iter().cloned())
            .with_row_sizing(TableRowSizing::measured(rems(2.5)).unwrap());
        next = next.with_result(
            yue_component::TableDataMode::Remote,
            yue_component::TableTotal::Known(total),
        );
        if let Some(page) = self.page {
            next = next
                .with_query(yue_component::TableQuery::paged(page.number(), page.size()).unwrap());
        }
        self.table
            .update(cx, |s, cx| s.commit_view(next, None, cx).unwrap());
        cx.notify();
    }
}
impl Render for StructureExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .min_w_0()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        YueButton::new("outline-filter", "只看温度明细").on_click(cx.listener(
                            |s, _, _, cx| {
                                s.page = None;
                                s.outline.filter_tree("温度", None).unwrap();
                                s.publish(cx);
                            },
                        )),
                    )
                    .child(
                        YueButton::new("outline-clear", "清除树筛选").on_click(cx.listener(
                            |s, _, _, cx| {
                                s.page = None;
                                s.outline.filter_tree("", None).unwrap();
                                s.publish(cx);
                            },
                        )),
                    )
                    .child(
                        YueButton::new("outline-load", "完成节点加载")
                            .disabled(self.ticket.is_none())
                            .on_click(cx.listener(|s, _, _, cx| {
                                if let Some(ticket) = s.ticket.take() {
                                    s.outline
                                        .apply_children(
                                            &ticket,
                                            vec![
                                                TreeNode::new("order-5", "加载的工单"),
                                                TreeNode::new("order-6", "加载的记录"),
                                            ],
                                            s.source.clone(),
                                        )
                                        .unwrap();
                                    s.message = "节点加载完成".into();
                                    s.publish(cx);
                                }
                            })),
                    )
                    .child(
                        YueButton::new("outline-fail", "模拟节点失败")
                            .disabled(self.ticket.is_none())
                            .on_click(cx.listener(|s, _, _, cx| {
                                if let Some(ticket) = s.ticket.take() {
                                    s.outline
                                        .reject_children(&ticket, "暂时无法加载".into())
                                        .unwrap();
                                    s.message = "节点加载失败；展开该节点重试".into();
                                    cx.notify();
                                }
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        YueButton::new(
                            "outline-cascade",
                            if self.cascade {
                                "关闭父子级联"
                            } else {
                                "开启父子级联"
                            },
                        )
                        .on_click(cx.listener(|s, _, _, cx| {
                            s.cascade = !s.cascade;
                            cx.notify();
                        })),
                    )
                    .child(YueButton::new("outline-page1", "根分页：第 1 页").on_click(
                        cx.listener(|s, _, _, cx| {
                            s.page = Some(yue_component::TablePage::new(1, 2).unwrap());
                            s.publish(cx);
                        }),
                    ))
                    .child(YueButton::new("outline-page2", "根分页：第 2 页").on_click(
                        cx.listener(|s, _, _, cx| {
                            s.page = Some(yue_component::TablePage::new(2, 2).unwrap());
                            s.publish(cx);
                        }),
                    ))
                    .child(
                        YueButton::new("outline-all", "显示全部根").on_click(cx.listener(
                            |s, _, _, cx| {
                                s.page = None;
                                s.publish(cx);
                            },
                        )),
                    ),
            )
            .child(self.message.clone())
            .child(
                YueTable::new("outline-table", &self.table)
                    .height(rems(28.))
                    .accessibility_label("工单树与详情"),
            )
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use crate::pages::table::tests::{click, draw};
    use gpui_kit::base::{Root, test_support::snapshots};
    use gpui_kit::{ElementId, TestAppContext, VisualTestContext, px, size};
    fn open(cx: &mut TestAppContext) -> (Entity<StructureExample>, &mut VisualTestContext) {
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
            let view = cx.new(|cx| StructureExample::new(w, cx));
            entity = Some(view.clone());
            Root::new(view, w, cx)
        });
        cx.update(|w, _| w.resize(size(px(1500.), px(1200.))));
        draw(cx);
        (entity.unwrap(), cx)
    }
    fn disclosure(cx: &mut VisualTestContext, ix: usize, id: &str) {
        let row: ElementId = (
            (
                ElementId::from("table-record"),
                SharedString::from("showcase-orders"),
            )
                .into(),
            format!("order-{ix}"),
        )
            .into();
        let center = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| {
                    n.path().contains(&row)
                        && n.path().last() == Some(&ElementId::from(id.to_owned()))
                })
                .unwrap()
                .bounds()
                .center()
        });
        cx.simulate_click(center, Default::default());
        for _ in 0..4 {
            draw(cx);
        }
    }
    #[gpui_kit::test]
    fn root_paging_and_opt_in_cascade_keep_collapsed_descendants(cx: &mut TestAppContext) {
        let (example, cx) = open(cx);
        click(cx, "outline-cascade");
        disclosure(cx, 0, "select");
        assert_eq!(
            example.read_with(cx, |s, cx| s
                .table
                .read(cx)
                .view()
                .unwrap()
                .selected_keys()
                .len()),
            4
        );
        disclosure(cx, 0, "tree-disclosure");
        assert_eq!(
            example.read_with(cx, |s, cx| s
                .table
                .read(cx)
                .view()
                .unwrap()
                .selected_keys()
                .len()),
            4
        );
        click(cx, "outline-page2");
        assert_eq!(
            example.read_with(cx, |s, cx| s
                .table
                .read(cx)
                .view()
                .unwrap()
                .source()
                .row_key(0)
                .unwrap()
                .as_str()
                .to_owned()),
            "order-8"
        );
        assert_eq!(
            example.read_with(cx, |s, cx| s
                .table
                .read(cx)
                .view()
                .unwrap()
                .selected_keys()
                .len()),
            4
        );
        click(cx, "outline-page1");
        // An unloaded subtree cannot be marked as fully selected.
        disclosure(cx, 4, "select");
        assert_eq!(
            example.read_with(cx, |s, cx| s
                .table
                .read(cx)
                .view()
                .unwrap()
                .selected_keys()
                .len()),
            4
        );
        assert!(example.read_with(cx, |s, cx| s.table.read(cx).pending_request().is_none()));
    }
    #[gpui_kit::test]
    fn tree_filter_lazy_retry_and_independent_detail_owners(cx: &mut TestAppContext) {
        let (example, cx) = open(cx);
        assert_eq!(example.read_with(cx, |s, _| s.detail_renders.get()), 0);
        assert!(example.read_with(cx, |s, _| s.nested.is_empty()));
        disclosure(cx, 0, "tree-disclosure");
        assert_eq!(
            example.read_with(cx, |s, cx| s
                .table
                .read(cx)
                .view()
                .unwrap()
                .source()
                .record_len()),
            5
        );
        disclosure(cx, 0, "detail-disclosure");
        let first = example.read_with(cx, |s, _| s.nested[&key(0)].1.clone());
        assert!(example.read_with(cx, |s, _| s.detail_renders.get() > 0));
        disclosure(cx, 1, "detail-disclosure");
        let second = example.read_with(cx, |s, _| s.nested[&key(1)].1.clone());
        assert_ne!(first.entity_id(), second.entity_id());
        cx.update(|_, cx| {
            first
                .read(cx)
                .scroll_handle()
                .set_offset(gpui_kit::point(px(0.), px(-80.)))
        });
        draw(cx);
        assert_eq!(
            second.read_with(cx, |s, _| s.scroll_handle().offset().y),
            px(0.)
        );
        let weak = first.downgrade();
        drop(first);
        disclosure(cx, 0, "detail-disclosure");
        assert!(
            weak.upgrade().is_none(),
            "closing a detail releases only its nested owner"
        );
        assert!(example.read_with(cx, |s, _| s.nested.contains_key(&key(1))));
        disclosure(cx, 1, "detail-disclosure");
        click(cx, "outline-filter");
        assert_eq!(
            example.read_with(cx, |s, cx| s
                .table
                .read(cx)
                .view()
                .unwrap()
                .source()
                .record_len()),
            3
        );
        click(cx, "outline-clear");
        disclosure(cx, 4, "tree-disclosure");
        assert!(example.read_with(cx, |s, _| s.ticket.is_some()));
        click(cx, "outline-fail");
        disclosure(cx, 4, "tree-disclosure");
        click(cx, "outline-load");
        // Retry may toggle disclosure closed; opening again reveals accepted children.
        if !example.read_with(cx, |s, _| s.outline.tree().unwrap().is_expanded("order-4")) {
            disclosure(cx, 4, "tree-disclosure");
        }
        assert!(example.read_with(cx, |s, cx| {
            s.table
                .read(cx)
                .view()
                .unwrap()
                .source()
                .position_of(&key(5))
                .is_some()
        }));
    }
}
