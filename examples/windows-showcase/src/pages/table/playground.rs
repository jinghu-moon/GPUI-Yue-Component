use super::*;

pub(super) struct Playground {
    delegate: Entity<Orders>,
    table: Entity<YueTableState<Orders>>,
    layout: Entity<YueTableState<Orders>>,
    large: Option<Entity<YueTableState<Orders>>>,
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
impl Playground {
    pub(super) fn release_large(&mut self, cx: &mut Context<Self>) {
        if self.large.take().is_some() {
            cx.notify();
        }
    }

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
impl Playground {
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
impl Render for Playground {
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
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_5()
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
        content
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use crate::pages::table::tests::{click, draw};
    use gpui_kit::base::Root;
    use gpui_kit::{TestAppContext, px, size};
    use yue_component::{Accent, ThemeMode, YueTheme};
    #[gpui_kit::test]
    fn table_page_controls_update_real_tables(cx: &mut TestAppContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap()
        });
        let mut page = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            let view = cx.new(|cx| Playground::new(w, cx));
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
