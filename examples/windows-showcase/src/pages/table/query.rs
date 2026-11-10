use super::*;
use gpui_kit::base::input::InputState;
use yue_component::{
    ButtonVariant, DatePickerOptions, DatePickerSelection, DatePickerValue, InputSize,
    PaginationChange, PaginationValue, TableDataMode, TableFilterKey, TableFilterValue,
    TableLocalQuery, TableProjection, TableQuery, TableQueryRequest, TableTotal, TreeNode,
    TreeSelectEvent, TreeSelectMode, TreeSelectValue, YueDatePicker, YueDatePickerState,
    YueDropdownMenu, YueInput, YueMenuItem, YuePagination, YuePaginationState, YueTreeSelect,
    YueTreeSelectState, sync_table_pagination,
};

struct QueryOrders;
impl TableDelegate for QueryOrders {
    type Source = TableProjection<TableRows<Order>>;
}

pub(super) struct QueryExample {
    _delegate: Entity<QueryOrders>,
    table: Entity<YueTableState<QueryOrders>>,
    pager: Entity<YuePaginationState>,
    source: Rc<TableRows<Order>>,
    columns: Rc<[TableColumn<Order>]>,
    name_key: TableFilterKey<SharedString>,
    amount_key: TableFilterKey<i32>,
    done_key: TableFilterKey<bool>,
    date_key: TableFilterKey<chrono::NaiveDate>,
    dates: Entity<YueDatePickerState>,
    name: Entity<InputState>,
    lower: Entity<InputState>,
    upper: Entity<InputState>,
    tree: Entity<YueTreeSelectState>,
    remote: bool,
    filter_open: bool,
    requests: usize,
    waiting: Option<TableQueryRequest>,
    old: Option<TableQueryRequest>,
    message: SharedString,
    summary: SharedString,
    filtered_sum: i32,
    _subscriptions: Vec<Subscription>,
}
impl QueryExample {
    pub(super) fn new(remote: bool, w: &mut Window, cx: &mut Context<Self>) -> Self {
        let source = records(137, 1, false);
        let name_col = TableColumn::value("name", "负责人", |r: &Order| r.name.clone())
            .display_with(|v| v.clone())
            .sortable(true)
            .filter_scalar(|a, b| a.contains(b.as_ref()));
        let name_key = name_col.filter_key();
        let amount_col = TableColumn::value("amount", "金额（元）", |r: &Order| r.amount)
            .display_with(|v| v.to_string().into())
            .sortable(true)
            .filterable(true);
        let amount_key = amount_col.filter_key();
        let done_col = TableColumn::value("done", "已核验", |r: &Order| r.done)
            .display_with(|v| if *v { "已核验" } else { "待核验" }.into())
            .sortable(true)
            .filterable(true);
        let done_key = done_col.filter_key();
        let date_col = TableColumn::value("date", "创建日期", |r: &Order| {
            let ix = r
                .key
                .strip_prefix("order-")
                .unwrap()
                .parse::<u32>()
                .unwrap();
            chrono::NaiveDate::from_ymd_opt(2026, 1 + ix % 12, 1 + ix % 28).unwrap()
        })
        .display_with(|v| v.to_string().into())
        .sortable(true)
        .filterable(true);
        let date_key = date_col.filter_key();
        let columns: Rc<[_]> = vec![
            TableColumn::text("number", "工单", |r: &Order| r.key.clone()).width(rems(9.)),
            name_col.build().unwrap().width(rems(12.)),
            amount_col
                .build()
                .unwrap()
                .width(rems(10.))
                .align(TableAlign::End),
            done_col.build().unwrap().width(rems(10.)),
            date_col.build().unwrap().width(rems(11.)),
        ]
        .into();
        let delegate = cx.new(|_| QueryOrders);
        let table = cx.new(|cx| YueTableState::new(&delegate, cx));
        table.update(cx, |s, cx| {
            s.commit_view(
                TableLocalQuery::execute(
                    source.clone(),
                    columns.clone(),
                    TableQuery::paged(1, 10).unwrap(),
                    1,
                )
                .unwrap()
                .with_result(
                    if remote {
                        TableDataMode::Remote
                    } else {
                        TableDataMode::Local
                    },
                    TableTotal::Known(137),
                )
                .selection_mode(TableSelectionMode::Multiple),
                None,
                cx,
            )
            .unwrap()
        });
        let pager =
            cx.new(|cx| YuePaginationState::new(PaginationValue::new(137, 1, 10).unwrap(), w, cx));
        let name = cx.new(|cx| InputState::new(w, cx).placeholder("负责人名称，按字面匹配"));
        let lower = cx.new(|cx| InputState::new(w, cx).placeholder("最低金额"));
        let upper = cx.new(|cx| InputState::new(w, cx).placeholder("最高金额"));
        let dates = cx.new(|cx| {
            YueDatePickerState::new(
                DatePickerOptions::new("query-date-range").selection(DatePickerSelection::Range),
                w,
                cx,
            )
            .unwrap()
        });
        let tree = cx.new(|cx| {
            let mut state = YueTreeSelectState::new(TreeSelectMode::Single, w, cx).unwrap();
            state
                .set_nodes(
                    vec![TreeNode::new("status", "核验状态").children([
                        TreeNode::new("yes", "已核验"),
                        TreeNode::new("no", "待核验"),
                    ])],
                    cx,
                )
                .unwrap();
            state.set_leaves_only(true, cx).unwrap();
            state
        });
        let subscriptions = vec![
            cx.subscribe_in(&table, w, |s, _, event: &TableEvent, w, cx| match event {
                TableEvent::QueryRequested(request) => {
                    if !s.table.read(cx).is_current_request(request.id()) {
                        return;
                    }
                    s.requests += 1;
                    if s.remote {
                        s.old = s.waiting.replace(request.clone());
                        let current = s.table.read(cx).view().unwrap().clone();
                        s.table.update(cx, |state, cx| {
                            state
                                .commit_view(
                                    current
                                        .as_ref()
                                        .clone()
                                        .load_state(TableLoadState::Refreshing),
                                    None,
                                    cx,
                                )
                                .unwrap()
                        });
                        s.message = "正在等待结果，旧行和已接受查询继续显示".into();
                        s.sync_pager(w, cx);
                    } else {
                        s.accept_query(request, w, cx);
                    }
                    cx.notify();
                }
                TableEvent::SelectionRequested(request) => {
                    let current = s.table.read(cx).view().unwrap().clone();
                    s.table.update(cx, |state, cx| {
                        state
                            .commit_view(
                                request.apply_to(&current).unwrap(),
                                Some(request.id()),
                                cx,
                            )
                            .unwrap()
                    });
                    s.update_summary(cx);
                    cx.notify();
                }
                _ => {}
            }),
            cx.subscribe_in(&pager, w, |s, _, event: &PaginationChange, _, cx| {
                let value = event.value();
                let _ = s.table.update(cx, |state, cx| {
                    state.request_page(value.page(), value.page_size(), cx)
                });
            }),
            cx.subscribe_in(&tree, w, |s, _, event: &TreeSelectEvent, w, cx| {
                if let TreeSelectEvent::Change(change) = event {
                    let selected = match change.value() {
                        TreeSelectValue::Single(key) => key.as_deref().map(|key| key == "yes"),
                        _ => None,
                    };
                    s.sync_tree(w, cx);
                    let _ = s.table.update(cx, |state, cx| {
                        state.request_filter(
                            &s.done_key,
                            selected.map(TableFilterValue::Scalar),
                            cx,
                        )
                    });
                }
            }),
        ];
        let filtered_sum = (0..source.len())
            .map(|i| source.row(source.row_key(i).unwrap()).unwrap().amount)
            .sum();
        let mut result = Self {
            _delegate: delegate,
            table,
            pager,
            source,
            columns,
            name_key,
            amount_key,
            done_key,
            date_key,
            dates,
            name,
            lower,
            upper,
            tree,
            remote,
            filter_open: false,
            requests: 0,
            waiting: None,
            old: None,
            message: "".into(),
            summary: "本页、筛选结果及选择的金额汇总在接受查询后更新".into(),
            filtered_sum,
            _subscriptions: subscriptions,
        };
        result.update_summary(cx);
        result
    }
    fn sync_pager(&self, w: &mut Window, cx: &mut Context<Self>) {
        let view = self.table.read(cx).view().unwrap().clone();
        sync_table_pagination(&view, &self.pager, w, cx).unwrap();
        self.sync_tree(w, cx);
    }
    fn sync_tree(&self, w: &mut Window, cx: &mut Context<Self>) {
        let query = self.table.read(cx).view().unwrap().query();
        let selected = match query.filter(&self.done_key) {
            Some(TableFilterValue::Scalar(done)) => Some(if *done { "yes" } else { "no" }.into()),
            _ => None,
        };
        let value = TreeSelectValue::Single(selected);
        self.tree.update(cx, |state, cx| {
            if state.value(cx) != value {
                state.set_value(value, w, cx).unwrap();
            }
        });
    }
    fn accept_query(
        &mut self,
        request: &TableQueryRequest,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.table.read(cx).is_current_request(request.id()) {
            self.message = "已忽略过期结果".into();
            return;
        }
        let current = self.table.read(cx).view().unwrap().clone();
        let next = request.apply_to(&current).and_then(|query| {
            TableLocalQuery::execute_with_summary(
                self.source.clone(),
                self.columns.clone(),
                query,
                current.source().revision() + 1,
                |source, keys| {
                    keys.iter()
                        .filter_map(|key| source.row(key))
                        .map(|r| r.amount)
                        .sum::<i32>()
                },
            )
        });
        match next {
            Ok((next, sum)) => {
                self.filtered_sum = sum;
                let total = next.total();
                let next = next
                    .with_result(
                        if self.remote {
                            TableDataMode::Remote
                        } else {
                            TableDataMode::Local
                        },
                        total,
                    )
                    .selection_mode(current.selection_kind())
                    .selection(current.selected_keys().iter().cloned());
                self.table.update(cx, |s, cx| {
                    s.commit_view(next, Some(request.id()), cx).unwrap()
                });
                self.message = "查询已接受".into();
            }
            Err(_) => {
                let _ = self
                    .table
                    .update(cx, |s, cx| s.reject_request(request.id(), cx));
                self.message = "查询未接受，保留旧结果".into();
            }
        }
        self.sync_pager(w, cx);
        self.update_summary(cx);
    }
    fn update_summary(&mut self, cx: &mut Context<Self>) {
        let view = self.table.read(cx).view().unwrap();
        let page_sum: i32 = (0..view.source().len())
            .map(|i| {
                view.source()
                    .row(view.source().row_key(i).unwrap())
                    .unwrap()
                    .amount
            })
            .sum();
        let selected_sum: i32 = view
            .selected_keys()
            .iter()
            .filter_map(|key| self.source.row(key))
            .map(|r| r.amount)
            .sum();
        let filtered_sum = self.filtered_sum;
        self.summary = format!(
            "金额合计：本页 {page_sum} 元 · 筛选结果 {filtered_sum} 元 · 已选 {selected_sum} 元"
        )
        .into();
    }
    fn apply_filters(&mut self, cx: &mut Context<Self>) {
        let name = self.name.read(cx).value();
        let low = self.lower.read(cx).value();
        let high = self.upper.read(cx).value();
        let amount = match (low.is_empty(), high.is_empty()) {
            (true, true) => None,
            (false, false) => match (low.parse::<i32>(), high.parse::<i32>()) {
                (Ok(start), Ok(end)) if start <= end => {
                    Some(TableFilterValue::Range { start, end })
                }
                _ => {
                    self.message = "请输入合法的金额范围".into();
                    cx.notify();
                    return;
                }
            },
            _ => {
                self.message = "请同时填写最低和最高金额".into();
                cx.notify();
                return;
            }
        };
        let query = self
            .table
            .read(cx)
            .view()
            .unwrap()
            .query()
            .clone()
            .with_filter(
                &self.name_key,
                (!name.is_empty()).then_some(TableFilterValue::Scalar(name)),
            )
            .unwrap()
            .with_filter(&self.amount_key, amount)
            .unwrap()
            .with_filter(
                &self.date_key,
                match self.dates.read(cx).value() {
                    DatePickerValue::Range(start, end) => Some(TableFilterValue::Range {
                        start: start.start_date(),
                        end: end.start_date(),
                    }),
                    _ => None,
                },
            )
            .unwrap();
        let _ = self.table.update(cx, |s, cx| {
            s.request_query(query, yue_component::TableQuerySource::Filter, cx)
        });
        self.filter_open = false;
        cx.notify();
    }
}
impl Render for QueryExample {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let yes = cx.weak_entity();
        let no = yes.clone();
        let clear = yes.clone();
        let view = self.table.read(cx).view().unwrap().clone();
        let selected = view.selected_keys().len();
        let page_selected = (0..view.source().len())
            .filter(|i| {
                view.selected_keys()
                    .contains(view.source().row_key(*i).unwrap())
            })
            .count();
        let mut controls = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(
                YueDropdownMenu::new(
                    "query-status",
                    theme,
                    YueButton::new("query-status-button", "核验状态"),
                )
                .use_default_theme()
                .items([
                    YueMenuItem::new("已核验").on_click(move |_, _, cx| {
                        let _ = yes.update(cx, |s, cx| {
                            let _ = s.table.update(cx, |state, cx| {
                                state.request_filter(
                                    &s.done_key,
                                    Some(TableFilterValue::Scalar(true)),
                                    cx,
                                )
                            });
                        });
                    }),
                    YueMenuItem::new("待核验").on_click(move |_, _, cx| {
                        let _ = no.update(cx, |s, cx| {
                            let _ = s.table.update(cx, |state, cx| {
                                state.request_filter(
                                    &s.done_key,
                                    Some(TableFilterValue::Scalar(false)),
                                    cx,
                                )
                            });
                        });
                    }),
                    YueMenuItem::new("清除状态条件").on_click(move |_, _, cx| {
                        let _ = clear.update(cx, |s, cx| {
                            let _ = s.table.update(cx, |state, cx| {
                                state.request_filter(&s.done_key, None, cx)
                            });
                        });
                    }),
                ]),
            )
            .child(
                YueTreeSelect::new("query-tree-status", &self.tree)
                    .accessibility_label("按核验状态筛选"),
            )
            .child(
                YueButton::new("query-custom", "名称与金额筛选…").on_click(cx.listener(
                    |s, _, w, cx| {
                        s.filter_open = !s.filter_open;
                        if s.filter_open {
                            let query = s.table.read(cx).view().unwrap().query().clone();
                            let text = match query.filter(&s.name_key) {
                                Some(TableFilterValue::Scalar(v)) => v.clone(),
                                _ => "".into(),
                            };
                            s.name.update(cx, |input, cx| input.set_value(text, w, cx));
                            let (low, high) = match query.filter(&s.amount_key) {
                                Some(TableFilterValue::Range { start, end }) => {
                                    (start.to_string(), end.to_string())
                                }
                                _ => (String::new(), String::new()),
                            };
                            s.lower.update(cx, |input, cx| input.set_value(low, w, cx));
                            s.upper.update(cx, |input, cx| input.set_value(high, w, cx));
                            let dates = match query.filter(&s.date_key) {
                                Some(TableFilterValue::Range { start, end }) => {
                                    DatePickerValue::date_range(*start, *end)
                                }
                                _ => DatePickerValue::Empty,
                            };
                            s.dates
                                .update(cx, |state, cx| state.set_value(dates, w, cx).unwrap());
                        }
                        cx.notify();
                    },
                )),
            )
            .child(
                YueButton::new("query-refresh", "刷新").on_click(cx.listener(|s, _, _, cx| {
                    let _ = s.table.update(cx, |state, cx| state.request_refresh(cx));
                })),
            )
            .child(
                YueButton::new("query-clear-selection", "清空选择")
                    .disabled(selected == 0)
                    .on_click(cx.listener(|s, _, _, cx| {
                        let _ = s
                            .table
                            .update(cx, |state, cx| state.request_clear_selection(cx));
                    })),
            )
            .child(
                YueButton::new("query-batch", "批量核验已选工单")
                    .disabled(selected == 0 || self.waiting.is_some())
                    .on_click(cx.listener(|s, _, w, cx| {
                        let current = s.table.read(cx).view().unwrap().clone();
                        let rows = (0..s.source.len())
                            .map(|ix| {
                                let key = s.source.row_key(ix).unwrap();
                                let row = s.source.row(key).unwrap();
                                Order {
                                    key: row.key.clone(),
                                    name: row.name.clone(),
                                    note: row.note.clone(),
                                    amount: row.amount,
                                    done: row.done || current.selected_keys().contains(key),
                                }
                            })
                            .collect::<Vec<_>>();
                        s.source = Rc::new(
                            TableRows::try_new(
                                s.source.scope().clone(),
                                s.source.revision() + 1,
                                rows.into(),
                                |r: &Order| r.key.clone(),
                            )
                            .unwrap(),
                        );
                        let (next, sum) = TableLocalQuery::execute_with_summary(
                            s.source.clone(),
                            s.columns.clone(),
                            current.query().clone(),
                            current.source().revision() + 1,
                            |source, keys| {
                                keys.iter()
                                    .filter_map(|key| source.row(key))
                                    .map(|r| r.amount)
                                    .sum::<i32>()
                            },
                        )
                        .unwrap();
                        s.filtered_sum = sum;
                        let total = next.total();
                        let next = next
                            .with_result(current.data_mode(), total)
                            .selection_mode(current.selection_kind())
                            .selection(current.selected_keys().iter().cloned());
                        s.table
                            .update(cx, |state, cx| state.commit_view(next, None, cx).unwrap());
                        s.message =
                            format!("已按稳定 key 核验 {} 项", current.selected_keys().len())
                                .into();
                        s.sync_pager(w, cx);
                        s.update_summary(cx);
                        cx.notify();
                    })),
            );
        if self.remote {
            controls = controls
                .child(
                    YueButton::new("query-complete", "完成最新请求")
                        .disabled(self.waiting.is_none())
                        .on_click(cx.listener(|s, _, w, cx| {
                            if let Some(r) = s.waiting.take() {
                                s.accept_query(&r, w, cx);
                            }
                            cx.notify();
                        })),
                )
                .child(
                    YueButton::new("query-late", "完成旧请求")
                        .disabled(self.old.is_none())
                        .on_click(cx.listener(|s, _, w, cx| {
                            if let Some(r) = s.old.take() {
                                s.accept_query(&r, w, cx);
                            }
                            cx.notify();
                        })),
                )
                .child(
                    YueButton::new("query-fail", "模拟失败")
                        .disabled(self.waiting.is_none())
                        .on_click(cx.listener(|s, _, w, cx| {
                            if let Some(r) = s.waiting.take()
                                && s.table.read(cx).is_current_request(r.id())
                            {
                                s.table.update(cx, |state, cx| {
                                    state.reject_request(r.id(), cx).unwrap();
                                    let old = state.view().unwrap().as_ref().clone().load_state(
                                        TableLoadState::Failed("查询失败，可重新刷新".into()),
                                    );
                                    state.commit_view(old, None, cx).unwrap();
                                });
                                s.sync_pager(w, cx);
                            }
                            cx.notify();
                        })),
                );
        }
        let mut content = div().flex().flex_col().gap_3().min_w_0().child(controls);
        if self.filter_open {
            content = content.child(
                div()
                    .id("query-filter-panel")
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(
                        YueInput::new("query-name", theme, &self.name)
                            .use_default_theme()
                            .accessibility_label("负责人名称")
                            .width(rems(16.))
                            .size(InputSize::Small),
                    )
                    .child(
                        YueInput::new("query-min", theme, &self.lower)
                            .use_default_theme()
                            .accessibility_label("最低金额")
                            .width(rems(10.))
                            .size(InputSize::Small),
                    )
                    .child(
                        YueInput::new("query-max", theme, &self.upper)
                            .use_default_theme()
                            .accessibility_label("最高金额")
                            .width(rems(10.))
                            .size(InputSize::Small),
                    )
                    .child(
                        YueDatePicker::new(&self.dates)
                            .size(InputSize::Small)
                            .accessibility_label("创建日期范围"),
                    )
                    .child(
                        YueButton::new("query-apply", "应用筛选")
                            .on_click(cx.listener(|s, _, _, cx| s.apply_filters(cx))),
                    )
                    .child(
                        YueButton::new("query-cancel", "取消")
                            .variant(ButtonVariant::Ghost)
                            .on_click(cx.listener(|s, _, _, cx| {
                                s.filter_open = false;
                                cx.notify();
                            })),
                    ),
            );
        }
        content
            .child(format!("已选 {selected} 项，其中当前页 {page_selected} 项"))
            .child(self.summary.clone())
            .child(self.message.clone())
            .child(
                YueTable::new("query-table", &self.table)
                    .height(rems(18.))
                    .accessibility_label(if self.remote {
                        "远程查询工单"
                    } else {
                        "本地查询工单"
                    }),
            )
            .child(
                YuePagination::new("query-pagination", &self.pager)
                    .show_total(true)
                    .page_size_selector(true)
                    .show_jumper(true),
            )
            .child(if self.remote {
                "此例由宿主模拟远程结果，可主动完成旧请求验证隔离；Table 不再次处理返回页。"
            } else {
                "点击列标题排序；Shift 加入多排序。选择跨页保留，表头全选仅作用于当前页。"
            })
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use crate::pages::table::tests::{click, draw};
    use gpui_kit::base::{Root, test_support::snapshots};
    use gpui_kit::{ElementId, Modifiers, TestAppContext, VisualTestContext, px, size};
    use yue_component::{Accent, ThemeMode, YueTheme};

    fn open(
        cx: &mut TestAppContext,
        remote: bool,
    ) -> (Entity<QueryExample>, &mut VisualTestContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap()
        });
        let mut entity = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            let view = cx.new(|cx| QueryExample::new(remote, w, cx));
            entity = Some(view.clone());
            Root::new(view, w, cx)
        });
        cx.update(|w, _| w.resize(size(px(1500.), px(1100.))));
        draw(cx);
        (entity.unwrap(), cx)
    }
    fn sort(cx: &mut VisualTestContext, label: &str, shift: bool) {
        let point = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.label().is_some_and(|v| v.starts_with(label)))
                .unwrap()
                .bounds()
                .center()
        });
        cx.simulate_click(
            point,
            Modifiers {
                shift,
                ..Default::default()
            },
        );
        draw(cx);
    }
    fn click_label(cx: &mut VisualTestContext, label: &str) {
        let point = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.label() == Some(label))
                .unwrap()
                .bounds()
                .center()
        });
        cx.simulate_click(point, Default::default());
        draw(cx);
    }
    #[gpui_kit::test]
    fn menus_tree_filters_cross_page_selection_and_batch_are_wired(cx: &mut TestAppContext) {
        let (example, cx) = open(cx, false);
        click_label(cx, "选择当前页的可选行");
        assert_eq!(
            example.read_with(cx, |s, cx| s
                .table
                .read(cx)
                .view()
                .unwrap()
                .selected_keys()
                .len()),
            10
        );
        click(cx, "query-batch");
        assert!(example.read_with(cx, |s, _| {
            s.source
                .row(&TableRowKey::try_new("order-0").unwrap())
                .unwrap()
                .done
        }));
        click(cx, "query-status-button");
        click_label(cx, "待核验");
        assert!(cx.update(|w, _| {
            snapshots(w).iter().any(|n| {
                n.label() == Some("选择当前页的可选行")
                    && n.checked() == Some(false)
                    && n.indeterminate() != Some(true)
            })
        }));
        assert_eq!(
            example.read_with(cx, |s, cx| s
                .table
                .read(cx)
                .view()
                .unwrap()
                .query()
                .filter(&s.done_key)
                .cloned()),
            Some(TableFilterValue::Scalar(false))
        );
        click_label(cx, "展开树选项");
        click_label(cx, "展开节点");
        click_label(cx, "已核验");
        assert_eq!(
            example.read_with(cx, |s, cx| s
                .table
                .read(cx)
                .view()
                .unwrap()
                .query()
                .filter(&s.done_key)
                .cloned()),
            Some(TableFilterValue::Scalar(true))
        );
        assert_eq!(
            example.read_with(cx, |s, cx| s
                .table
                .read(cx)
                .view()
                .unwrap()
                .selected_keys()
                .len()),
            10
        );
        click(cx, "query-clear-selection");
        assert!(example.read_with(cx, |s, cx| {
            s.table.read(cx).view().unwrap().selected_keys().is_empty()
        }));
    }
    #[gpui_kit::test]
    fn local_query_real_sort_draft_cancel_paging_and_bulk(cx: &mut TestAppContext) {
        let (example, cx) = open(cx, false);
        sort(cx, "按金额", false);
        sort(cx, "按已核验", true);
        sort(cx, "按金额", true);
        assert_eq!(
            example.read_with(cx, |s, cx| {
                let view = s.table.read(cx).view().unwrap();
                (
                    s.requests,
                    view.query().sorts()[0].column().as_str().to_owned(),
                    view.query().sorts()[0].direction(),
                )
            }),
            (
                3,
                "amount".into(),
                yue_component::TableSortDirection::Descending
            )
        );
        click(cx, "query-custom");
        click(cx, "query-name");
        cx.simulate_input("不存在的名称");
        click(cx, "query-cancel");
        assert_eq!(example.read_with(cx, |s, _| s.requests), 3);
        click(cx, "query-custom");
        assert!(example.read_with(cx, |s, cx| s.name.read(cx).value().is_empty()));
        click(cx, "query-name");
        cx.simulate_input("不存在的名称");
        click(cx, "query-apply");
        assert!(example.read_with(cx, |s, cx| {
            s.table.read(cx).view().unwrap().source().is_empty()
        }));
        click(cx, "query-custom");
        click(cx, "query-name");
        cx.simulate_keystrokes("ctrl-a backspace");
        click(cx, "query-apply");
        assert_eq!(
            example.read_with(cx, |s, cx| s.table.read(cx).view().unwrap().total()),
            TableTotal::Known(137)
        );
        // The actual pager button, rather than the owner's request method.
        let next: ElementId = (ElementId::from("query-pagination"), "next").into();
        let point = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.path().last() == Some(&next))
                .unwrap()
                .bounds()
                .center()
        });
        cx.simulate_click(point, Default::default());
        draw(cx);
        assert_eq!(
            example.read_with(cx, |s, cx| s.pager.read(cx).value().page()),
            2
        );
        let jump: ElementId = (ElementId::from("query-pagination"), "jump").into();
        let point = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.path().contains(&jump) && n.role() == Some(gpui_kit::Role::TextInput))
                .unwrap()
                .bounds()
                .center()
        });
        cx.simulate_click(point, Default::default());
        cx.simulate_keystrokes("ctrl-a");
        cx.simulate_input("9");
        // An unrelated publication while the real jump Input still has focus.
        example.update(cx, |s, cx| {
            let current = s.table.read(cx).view().unwrap().as_ref().clone();
            s.table.update(cx, |state, cx| {
                state.commit_view(current, None, cx).unwrap()
            });
        });
        cx.update(|w, cx| {
            example.update(cx, |s, cx| s.sync_pager(w, cx));
        });
        draw(cx);
        assert!(cx.update(|w, _| {
            snapshots(w)
                .iter()
                .any(|n| n.path().contains(&jump) && n.value() == Some("9"))
        }));
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert_eq!(
            example.read_with(cx, |s, cx| s.pager.read(cx).value().page()),
            9
        );
    }
    #[gpui_kit::test]
    fn remote_refresh_late_success_failure_and_retry_use_real_buttons(cx: &mut TestAppContext) {
        let (example, cx) = open(cx, true);
        click(cx, "query-refresh");
        click(cx, "query-refresh");
        let pending = example.read_with(cx, |s, cx| s.table.read(cx).pending_request());
        click(cx, "query-late");
        assert_eq!(
            example.read_with(cx, |s, cx| s.table.read(cx).pending_request()),
            pending
        );
        assert_eq!(
            example.read_with(cx, |s, cx| s.table.read(cx).view().unwrap().source().len()),
            10
        );
        click(cx, "query-fail");
        assert!(example.read_with(cx, |s, cx| matches!(
            s.table.read(cx).view().unwrap().load(),
            TableLoadState::Failed(_)
        )));
        click(cx, "query-refresh");
        click(cx, "query-complete");
        assert!(example.read_with(cx, |s, cx| s.table.read(cx).pending_request().is_none()));
        assert_eq!(example.read_with(cx, |s, _| s.requests), 3);
        assert_eq!(
            example.read_with(cx, |s, cx| s.table.read(cx).view().unwrap().load().clone()),
            TableLoadState::Ready
        );
    }
}
