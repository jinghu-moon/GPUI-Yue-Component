#[path = "table/configuration.rs"]
mod configuration;
#[path = "table/customization.rs"]
mod customization;
#[path = "table/temporal.rs"]
mod temporal;
use crate::scroll::ScrollableElement as _;
#[path = "table/advanced.rs"]
mod advanced;
#[path = "table/editing.rs"]
mod editing;
#[path = "table/query.rs"]
mod query;
#[path = "table/structure.rs"]
mod structure;
use gpui_kit::{
    AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _,
    Render, SharedString, Styled as _, Subscription, Window, div, rems,
};
use std::rc::Rc;
use yue_component::{
    AvatarSize, TableAlign, TableBorders, TableColumn, TableDelegate, TableEvent, TableLoadState,
    TableRenderMode, TableRowKey, TableRows, TableSelectionMode, TableSize, TableSource,
    TableTextOverflow, TableView, TagTone, YueAvatar, YueButton, YueCard, YueCardHeader, YueTable,
    YueTableState, YueTag,
};

struct Order {
    key: SharedString,
    name: SharedString,
    note: SharedString,
    amount: i32,
    done: bool,
}
struct Orders;
impl TableDelegate for Orders {
    type Source = TableRows<Order>;
}

pub(crate) struct TablePage {
    delegate: Entity<Orders>,
    table: Entity<YueTableState<Orders>>,
    layout: Entity<YueTableState<Orders>>,
    large: Option<Entity<YueTableState<Orders>>>,
    queries: Option<(Entity<query::QueryExample>, Entity<query::QueryExample>)>,
    structure: Option<Entity<structure::StructureExample>>,
    editing: Option<Entity<editing::EditingExample>>,
    temporal: Option<Entity<temporal::TemporalExample>>,
    advanced: Option<Entity<advanced::AdvancedExample>>,
    customization: Option<Entity<customization::CustomizationExample>>,
    size: TableSize,
    borders: TableBorders,
    striped: bool,
    advanced_layout: bool,
    grouped: bool,
    show_done: bool,
    static_mode: bool,
    narrow: bool,
    reversed: bool,
    revision: u64,
    subscriptions: Vec<Subscription>,
    message: SharedString,
}
fn records(count: usize, revision: u64, reverse: bool) -> Rc<TableRows<Order>> {
    let mut records: Vec<_> = (0..count)
        .map(|i| Order {
            key: format!("order-{i}").into(),
            name: format!("项目 {}", i + 1).into(),
            note: match i % 3 {
                0 => "待处理的工单；此处演示中文长文本、换行与完整内容提示。",
                1 => "https://example.com/orders/very-long-record-identifier-2026",
                _ => "已核验",
            }
            .into(),
            amount: if i % 7 == 0 { 0 } else { (i as i32 + 1) * 100 },
            done: i % 3 == 2,
        })
        .collect();
    if reverse {
        records.reverse();
    }
    Rc::new(
        TableRows::try_new("showcase-orders", revision, records.into(), |r: &Order| {
            r.key.clone()
        })
        .unwrap(),
    )
}
fn columns(wrap: bool) -> Rc<[TableColumn<Order>]> {
    vec![
        TableColumn::text("number", "工单", |r: &Order| r.key.clone())
            .width(rems(9.))
            .footer("当前结果"),
        TableColumn::custom("owner", "负责人", |r: &Order, _, _, cx| {
            let theme = yue_component::current_theme(cx);
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    YueAvatar::new("avatar", theme)
                        .name("岳")
                        .size(AvatarSize::Small),
                )
                .child(r.name.clone())
        })
        .width(rems(11.)),
        TableColumn::text("note", "说明", |r: &Order| r.note.clone())
            .width(rems(18.))
            .overflow(if wrap {
                TableTextOverflow::Wrap
            } else {
                TableTextOverflow::Ellipsis
            }),
        TableColumn::text("amount", "金额（元）", |r: &Order| {
            r.amount.to_string()
        })
        .width(rems(8.))
        .align(TableAlign::End),
        TableColumn::custom("status", "状态", |r: &Order, _, _, _| {
            YueTag::new("status", if r.done { "已完成" } else { "处理中" }).tone(if r.done {
                TagTone::Success
            } else {
                TagTone::Primary
            })
        })
        .width(rems(8.)),
        TableColumn::text("done", "已核验", |r: &Order| r.done.to_string()).width(rems(7.)),
        TableColumn::custom("action", "", |r: &Order, cell, _, _| {
            let key = r.key.clone();
            let button = YueButton::new("view-order", "复制编号")
                .size(match cell.size() {
                    TableSize::Small => yue_component::ButtonSize::Small,
                    TableSize::Medium => yue_component::ButtonSize::Medium,
                    TableSize::Large => yue_component::ButtonSize::Large,
                })
                .disabled(cell.is_disabled() || cell.is_busy())
                .tab_stop(cell.is_active())
                .on_click(move |_, _, cx| {
                    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(key.to_string()))
                });
            if let Some(focus) = cell.control_focus() {
                button.track_focus(focus)
            } else {
                button
            }
        })
        .width(rems(8.))
        .interactive(true),
    ]
    .into()
}
impl TablePage {
    pub(crate) fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let delegate = cx.new(|_| Orders);
        let table = cx.new(|cx| YueTableState::new(&delegate, cx));
        let layout = cx.new(|cx| YueTableState::new(&delegate, cx));
        table.update(cx, |s, cx| {
            s.commit_view(
                TableView::new(records(40, 1, false), columns(false))
                    .selection_mode(TableSelectionMode::Multiple)
                    .disabled_rows([TableRowKey::try_new("order-1").unwrap()]),
                None,
                cx,
            )
            .unwrap()
        });
        layout.update(cx, |s, cx| {
            s.commit_view(
                TableView::new(records(4, 1, false), columns(true)),
                None,
                cx,
            )
            .unwrap()
        });
        let subscription = cx.subscribe(&table, |page, table, event: &TableEvent, cx| {
            if let TableEvent::ColumnResizeRequested(request) = event {
                let view = table.read(cx).view().unwrap().clone();
                table.update(cx, |s, cx| {
                    if let Err(error) = request
                        .apply_to(&view)
                        .and_then(|v| s.commit_view(v, Some(request.id()), cx))
                    {
                        let _ = s.reject_request(request.id(), cx);
                        page.message = format!("列宽未修改：{error}").into();
                    }
                });
                cx.notify();
            }
            if let TableEvent::SelectionRequested(request) = event {
                let accepted = table.read(cx).view().unwrap().clone();
                let next = request.apply_to(&accepted);
                table.update(cx, |s, cx| {
                    match next.and_then(|next| s.commit_view(next, Some(request.id()), cx)) {
                        Ok(_) => {
                            page.message =
                                format!("已选择 {} 条工单", s.view().unwrap().selected_keys().len())
                                    .into()
                        }
                        Err(error) => {
                            let _ = s.reject_request(request.id(), cx);
                            page.message = error.to_string().into();
                        }
                    }
                });
                cx.notify();
            }
        });
        Self {
            delegate,
            table,
            layout,
            large: None,
            queries: None,
            structure: None,
            editing: None,
            temporal: None,
            advanced: None,
            customization: None,
            size: TableSize::Medium,
            borders: TableBorders::Rows,
            striped: false,
            advanced_layout: false,
            grouped: false,
            show_done: true,
            static_mode: false,
            narrow: false,
            reversed: false,
            revision: 1,
            subscriptions: vec![subscription],
            message:
                "方向键导航；空格选择，Shift 扩选，Ctrl+A 全选。Enter／F2 进入操作，Escape 返回。"
                    .into(),
        }
    }
    fn large(&mut self, cx: &mut Context<Self>) {
        if self.large.is_some() {
            self.large = None;
            cx.notify();
            return;
        }
        // TB01 shape: 100,000 rows x 12 ordinary text columns. Prepared only on request.
        let columns: Vec<_> = (0..12)
            .map(|i| {
                TableColumn::text(
                    format!("field-{i}"),
                    format!("字段 {}", i + 1),
                    move |r: &Order| {
                        if i == 0 {
                            r.key.clone()
                        } else {
                            format!("{} / {}", r.amount, i).into()
                        }
                    },
                )
                .width(rems(8.))
            })
            .collect();
        let state = cx.new(|cx| YueTableState::new(&self.delegate, cx));
        state.update(cx, |s, cx| {
            s.commit_view(
                TableView::new(records(100_000, 1, false), columns),
                None,
                cx,
            )
            .unwrap()
        });
        self.large = Some(state);
        cx.notify();
    }
}
impl TablePage {
    fn sync_layout_columns(&mut self, reset: bool, cx: &mut Context<Self>) {
        let current = self.table.read(cx).view().unwrap().clone();
        let mut next = if reset {
            columns(false).to_vec()
        } else {
            current.columns().to_vec()
        };
        for c in &mut next {
            let key = c.key().as_str().to_owned();
            let pin = if self.advanced_layout {
                match key.as_str() {
                    "number" => yue_component::TablePin::Start,
                    "action" => yue_component::TablePin::End,
                    _ => yue_component::TablePin::None,
                }
            } else {
                yue_component::TablePin::None
            };
            *c = c
                .clone()
                .pin(pin)
                .resizable(self.advanced_layout)
                .min_width(rems(5.))
                .max_width(rems(30.));
            if reset && self.advanced_layout && key == "note" {
                *c = c.clone().min_width(rems(12.)).flex(1.);
            }
            if key == "done" {
                *c = c.clone().visible(self.show_done).responsive_priority(10);
            }
            let path = if self.grouped {
                match key.as_str() {
                    "owner" | "note" => {
                        vec![yue_component::TableHeaderGroup::new("details", "工单信息")]
                    }
                    "amount" | "status" | "done" => vec![yue_component::TableHeaderGroup::new(
                        "progress",
                        "金额与进度",
                    )],
                    _ => vec![],
                }
            } else {
                vec![]
            };
            *c = c.clone().header_path(path);
        }
        self.table.update(cx, |s, cx| {
            s.commit_view(current.as_ref().clone().with_columns(next), None, cx)
                .unwrap()
        });
        cx.notify();
    }
}
impl Render for TablePage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _ = &self.subscriptions;
        let controls = div()
            .flex()
            .flex_wrap()
            .gap_2()
            .child(
                YueButton::new("table-size", format!("密度：{:?}", self.size)).on_click(
                    cx.listener(|s, _, _, cx| {
                        s.size = match s.size {
                            TableSize::Small => TableSize::Medium,
                            TableSize::Medium => TableSize::Large,
                            TableSize::Large => TableSize::Small,
                        };
                        cx.notify();
                    }),
                ),
            )
            .child(
                YueButton::new("table-borders", format!("边框：{:?}", self.borders)).on_click(
                    cx.listener(|s, _, _, cx| {
                        s.borders = match s.borders {
                            TableBorders::None => TableBorders::Rows,
                            TableBorders::Rows => TableBorders::Grid,
                            TableBorders::Grid => TableBorders::None,
                        };
                        cx.notify();
                    }),
                ),
            )
            .child(
                YueButton::new("table-stripes", "切换斑马纹").on_click(cx.listener(
                    |s, _, _, cx| {
                        s.striped = !s.striped;
                        cx.notify();
                    },
                )),
            )
            .child(
                YueButton::new("table-narrow", "切换窄容器").on_click(cx.listener(
                    |s, _, _, cx| {
                        s.narrow = !s.narrow;
                        cx.notify();
                    },
                )),
            )
            .child(
                YueButton::new(
                    "table-mode",
                    if self.static_mode {
                        "切换虚拟视口"
                    } else {
                        "切换自然高度"
                    },
                )
                .on_click(cx.listener(|s, _, _, cx| {
                    s.static_mode = !s.static_mode;
                    cx.notify();
                })),
            )
            .child(
                YueButton::new("table-single", "切换单选／多选").on_click(cx.listener(
                    |s, _, _, cx| {
                        s.table.update(cx, |state, cx| {
                            let view = state.view().unwrap().as_ref().clone();
                            let next = if view.selection_kind() == TableSelectionMode::Single {
                                TableSelectionMode::Multiple
                            } else {
                                TableSelectionMode::Single
                            };
                            state
                                .commit_view(view.selection([]).selection_mode(next), None, cx)
                                .unwrap();
                            s.message = if next == TableSelectionMode::Single {
                                "单选"
                            } else {
                                "多选"
                            }
                            .into();
                        });
                        cx.notify();
                    },
                )),
            )
            .child(
                YueButton::new("table-reverse", "反转数据顺序").on_click(cx.listener(
                    |s, _, _, cx| {
                        s.reversed = !s.reversed;
                        s.revision += 1;
                        s.table.update(cx, |state, cx| {
                            let old = state.view().unwrap();
                            let candidate = TableView::new(
                                records(40, s.revision, s.reversed),
                                old.shared_columns().clone(),
                            )
                            .selection_mode(TableSelectionMode::Multiple)
                            .selection(old.selected_keys().iter().cloned());
                            state.commit_view(candidate, None, cx).unwrap();
                        });
                        cx.notify();
                    },
                )),
            );
        let states = div()
            .flex()
            .flex_wrap()
            .gap_2()
            .children(
                [
                    ("table-ready", "显示数据", TableLoadState::Ready),
                    ("table-refresh", "保留结果刷新", TableLoadState::Refreshing),
                    (
                        "table-fail",
                        "模拟失败",
                        TableLoadState::Failed("刷新失败；当前工单仍可查看，请重试。".into()),
                    ),
                ]
                .into_iter()
                .map(|(id, label, load)| {
                    YueButton::new(id, label).on_click(cx.listener(move |s, _, _, cx| {
                        s.table.update(cx, |state, cx| {
                            let view = state
                                .view()
                                .unwrap()
                                .as_ref()
                                .clone()
                                .load_state(load.clone());
                            state.commit_view(view, None, cx).unwrap();
                        });
                        cx.notify();
                    }))
                }),
            )
            .child(
                YueButton::new("table-initial", "初次加载").on_click(cx.listener(|s, _, _, cx| {
                    s.revision += 1;
                    s.table.update(cx, |state, cx| {
                        let cols = state.view().unwrap().shared_columns().clone();
                        state
                            .commit_view(
                                TableView::new(records(0, s.revision, false), cols)
                                    .load_state(TableLoadState::Loading),
                                None,
                                cx,
                            )
                            .unwrap();
                    });
                    cx.notify();
                })),
            )
            .child(
                YueButton::new("table-empty", "清空／恢复").on_click(cx.listener(|s, _, _, cx| {
                    s.revision += 1;
                    s.table.update(cx, |state, cx| {
                        let old = state.view().unwrap();
                        let count = if old.source().is_empty() { 40 } else { 0 };
                        let view = TableView::new(
                            records(count, s.revision, false),
                            old.shared_columns().clone(),
                        )
                        .selection_mode(TableSelectionMode::Multiple);
                        state.commit_view(view, None, cx).unwrap();
                    });
                    cx.notify();
                })),
            )
            .child(
                YueButton::new(
                    "table-large",
                    if self.large.is_some() {
                        "卸载大数据示例"
                    } else {
                        "加载 10 万行 × 12 列"
                    },
                )
                .on_click(cx.listener(|s, _, _, cx| s.large(cx))),
            );
        let mut content = div()
            .p_6()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_5()
            .child(div().text_2xl().child("Table 表格"))
            .child("同类记录的字段比较；滚动、选择与数据更新保留稳定身份。")
            .child(controls)
            .child(states)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        YueButton::new(
                            "table-advanced-layout",
                            if self.advanced_layout {
                                "还原默认布局"
                            } else {
                                "演示固定与伸缩布局"
                            },
                        )
                        .on_click(cx.listener(|s, _, _, cx| {
                            s.advanced_layout = !s.advanced_layout;
                            s.static_mode = false;
                            s.sync_layout_columns(true, cx);
                        })),
                    )
                    .child(
                        YueButton::new(
                            "table-grouped",
                            if self.grouped {
                                "取消分组表头"
                            } else {
                                "分组表头"
                            },
                        )
                        .on_click(cx.listener(|s, _, _, cx| {
                            s.grouped = !s.grouped;
                            s.sync_layout_columns(false, cx);
                        })),
                    )
                    .child(
                        YueButton::new(
                            "table-column-visibility",
                            if self.show_done {
                                "隐藏核验列"
                            } else {
                                "显示核验列"
                            },
                        )
                        .on_click(cx.listener(|s, _, _, cx| {
                            s.show_done = !s.show_done;
                            s.sync_layout_columns(false, cx);
                        })),
                    ),
            )
            .child(div().id("table-message").child(self.message.clone()))
            .child(
                div()
                    .w(if self.narrow { rems(28.) } else { rems(76.) })
                    .max_w_full()
                    .child(
                        YueCard::new("table-orders-card")
                            .header(YueCardHeader::new("工单"))
                            .body_inset(false)
                            .child(
                                YueTable::new("orders-table", &self.table)
                                    .accessibility_label("工单列表")
                                    .size(self.size)
                                    .borders(self.borders)
                                    .striped(self.striped)
                                    .height(rems(22.))
                                    .frozen_rows(
                                        usize::from(self.advanced_layout),
                                        usize::from(self.advanced_layout),
                                    )
                                    .footer_fixed(self.advanced_layout)
                                    .render_mode(if self.static_mode && !self.advanced_layout {
                                        TableRenderMode::Static
                                    } else {
                                        TableRenderMode::Rows
                                    }),
                            ),
                    ),
            )
            .child(
                YueCard::new("table-layout-card")
                    .header(YueCardHeader::new("自然高度与多行对齐"))
                    .body_inset(false)
                    .child(
                        YueTable::new("layout-table", &self.layout)
                            .accessibility_label("工单内容与状态")
                            .borders(TableBorders::Grid)
                            .render_mode(TableRenderMode::Static),
                    ),
            );
        if let Some(large) = &self.large {
            content = content.child(
                YueCard::new("table-large-card")
                    .header(YueCardHeader::new("100,000 行 × 12 列"))
                    .body_inset(false)
                    .child(
                        YueTable::new("large-table", large)
                            .accessibility_label("大数据工单")
                            .height(rems(28.)),
                    ),
            );
        }
        content = content.child(
            YueButton::new(
                "table-query-examples",
                if self.queries.is_some() {
                    "收起查询示例"
                } else {
                    "本地与远程查询示例"
                },
            )
            .on_click(cx.listener(|s, _, w, cx| {
                s.queries = if s.queries.is_some() {
                    None
                } else {
                    Some((
                        cx.new(|cx| query::QueryExample::new(false, w, cx)),
                        cx.new(|cx| query::QueryExample::new(true, w, cx)),
                    ))
                };
                cx.notify();
            })),
        );
        content = content.child(
            YueButton::new(
                "table-structure-examples",
                if self.structure.is_some() {
                    "收起结构示例"
                } else {
                    "树、详情与嵌套子表…"
                },
            )
            .on_click(cx.listener(|s, _, w, cx| {
                s.structure = if s.structure.is_some() {
                    None
                } else {
                    Some(cx.new(|cx| structure::StructureExample::new(w, cx)))
                };
                cx.notify();
            })),
        );
        if let Some(structure) = &self.structure {
            content = content.child(
                YueCard::new("table-structure-card")
                    .header(YueCardHeader::new("树与展开详情"))
                    .child(structure.clone()),
            );
        }
        content = content.child(
            YueButton::new(
                "table-editing-examples",
                if self.editing.is_some() {
                    "收起编辑示例"
                } else {
                    "单元格与整行编辑…"
                },
            )
            .on_click(cx.listener(|s, _, w, cx| {
                s.editing = if s.editing.is_some() {
                    None
                } else {
                    Some(cx.new(|cx| editing::EditingExample::new(w, cx)))
                };
                s.temporal = if s.editing.is_some() {
                    Some(cx.new(|cx| temporal::TemporalExample::new(w, cx)))
                } else {
                    None
                };
                cx.notify();
            })),
        );
        if let Some(editing) = &self.editing {
            content = content.child(
                YueCard::new("table-editing-card")
                    .header(YueCardHeader::new("真实控件编辑与保存"))
                    .child(editing.clone()),
            );
        }
        if let Some(temporal) = &self.temporal {
            content = content.child(
                YueCard::new("table-temporal-card")
                    .header(YueCardHeader::new("日期与时间编辑"))
                    .child(temporal.clone()),
            );
        }
        content = content.child(
            YueButton::new(
                "advanced-table-toggle",
                if self.advanced.is_some() {
                    "收起双轴、合并与范围加载示例"
                } else {
                    "双轴、合并、宿主吸顶与范围加载…"
                },
            )
            .on_click(cx.listener(|s, _, _, cx| {
                s.advanced = if s.advanced.is_some() {
                    None
                } else {
                    Some(cx.new(advanced::AdvancedExample::new))
                };
                cx.notify();
            })),
        );
        if let Some(advanced) = &self.advanced {
            content = content.child(
                YueCard::new("table-advanced-card")
                    .header(YueCardHeader::new("高级布局与范围加载"))
                    .child(advanced.clone()),
            );
        }
        content = content.child(
            YueButton::new(
                "table-customization-toggle",
                if self.customization.is_some() {
                    "收起重排和列配置"
                } else {
                    "重排、列配置与尾游标加载…"
                },
            )
            .on_click(cx.listener(|s, _, _, cx| {
                s.customization = if s.customization.is_some() {
                    None
                } else {
                    Some(cx.new(customization::CustomizationExample::new))
                };
                cx.notify();
            })),
        );
        if let Some(example) = &self.customization {
            content = content.child(
                YueCard::new("table-customization-card")
                    .header(YueCardHeader::new("重排、列设置与尾部追加"))
                    .child(example.clone()),
            );
        }
        if let Some((local, remote)) = &self.queries {
            content = content
                .child(
                    YueCard::new("table-local-query-card")
                        .header(YueCardHeader::new("本地查询与跨页选择"))
                        .child(local.clone()),
                )
                .child(
                    YueCard::new("table-remote-query-card")
                        .header(YueCardHeader::new("远程查询与过期恢复"))
                        .child(remote.clone()),
                );
        }
        div()
            .id("table-content")
            .debug_selector(|| "table-content".into())
            .size_full()
            .min_w_0()
            .min_h_0()
            .child(content)
            .overflow_y_scrollbar()
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::base::{Root, test_support::snapshots};
    use gpui_kit::{ElementId, TestAppContext, VisualTestContext, px, size};
    use yue_component::{Accent, ThemeMode, YueTheme};
    pub(super) fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|w, cx| {
            w.refresh();
            w.draw(cx).clear(cx);
        });
        cx.run_until_parked();
    }
    pub(super) fn click(cx: &mut VisualTestContext, id: &str) {
        let point = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.path().last() == Some(&ElementId::from(id.to_owned())))
                .unwrap()
                .bounds()
                .center()
        });
        cx.simulate_click(point, Default::default());
        draw(cx);
    }
    #[gpui_kit::test]
    fn table_page_controls_update_real_tables(cx: &mut TestAppContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap()
        });
        let mut page = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            let view = cx.new(|cx| TablePage::new(w, cx));
            page = Some(view.clone());
            Root::new(view, w, cx)
        });
        let page = page.unwrap();
        cx.update(|w, _| w.resize(size(px(1800.), px(1600.))));
        draw(cx);
        for id in [
            "table-size",
            "table-borders",
            "table-stripes",
            "table-narrow",
            "table-mode",
            "table-single",
            "table-reverse",
        ] {
            click(cx, id);
        }
        assert_eq!(page.read_with(cx, |p, _| p.size), TableSize::Large);
        assert!(page.read_with(cx, |p, _| p.static_mode && p.striped && p.narrow));
        assert!(page.read_with(cx, |p, cx| {
            p.table
                .read(cx)
                .view()
                .unwrap()
                .source()
                .row_key(0)
                .unwrap()
                .as_str()
                == "order-39"
        }));
        click(cx, "table-refresh");
        assert_eq!(
            page.read_with(cx, |p, cx| p.table.read(cx).view().unwrap().load().clone()),
            TableLoadState::Refreshing
        );
        click(cx, "table-fail");
        click(cx, "table-ready");
        click(cx, "table-empty");
        assert!(page.read_with(cx, |p, cx| {
            p.table.read(cx).view().unwrap().source().is_empty()
        }));
        click(cx, "table-empty");
        assert_eq!(
            page.read_with(cx, |p, cx| p.table.read(cx).view().unwrap().source().len()),
            40
        );
        click(cx, "table-large");
        assert_eq!(
            page.read_with(cx, |p, cx| p
                .large
                .as_ref()
                .unwrap()
                .read(cx)
                .view()
                .unwrap()
                .source()
                .len()),
            100_000
        );
        click(cx, "table-large");
        assert!(page.read_with(cx, |p, _| p.large.is_none()));
    }
}
