# YueTable：实际 API 与接入示例

日期：2026-11-11。配套 [Table 主设计](45-table.md)、[M1](45-table-m1.md)、[M2](45-table-m2.md) 及 [M3](45-table-m3.md)。本页只列当前实现；后续阶段合同仍在主设计，不用伪签名冒充可用 API。

## 1. 共享与受控合同

`TableSource` 关联 `Row`，提供 `scope`、`revision`、`len`、`row_key`、`position_of`、`row` 与 `contains_key`。`TableRows::try_new(scope, revision, Rc<[R]>, key_of)` 建立索引，拒绝空／重复 key，不要求 Row: Clone。`TableDelegate` 只声明 Source，可覆写呈现；构造为 `YueTableState::new(&delegate, cx)`。

`TableView::new(Rc<S>, columns)` 形成候选，默认 Remote、已知 source.len、Ready、无分页和选择；从 `state.view()` 克隆候选保留基础 revision。`commit_view(candidate, response_to, cx)` 原子接受，`reject_request(id, cx)` 只结束当前有效票据。查询请求不直接改 accepted 值，`is_current_request(id)` 用于在响应转换前隔离旧完成；最终 commit 仍重验。改变 source 递增 source revision，外部替换使旧请求失效。

| 领域 | 当前入口 |
| --- | --- |
| 查询 | `view.with_query(query)`；`state.request_query(query, source, cx)`、`request_sort`、`request_filter`、`request_page`、`request_refresh` |
| 结果与加载 | `view.with_result(TableDataMode, TableTotal)`、`load_state(TableLoadState)`；Known(0) 和 Unknown 分开 |
| 选择 | `selection_mode`、`selection`、`disabled_rows`；SelectionRequested 的 `apply_to` 在最新视图上重基；页内全选保留页外选择 |
| 列 | `with_columns`、`shared_columns`；clone 原列保留查询语义身份，重新声明同名规则使旧筛选和等待请求失效 |
| 本地 | `TableLocalQuery::execute(source, columns, query, revision)` 返回 `TableView<TableProjection<S>>`，过滤→稳定多排序→分页，total 是分页前结果数 |
| 远程 | `TableRemotePage::new(page_source, known_keys)` 只借用当前页 Row，known key 域用于跨页选择，不伪造 records；组件不重排或切片服务器返回页 |
| 分页 | `sync_table_pagination(&view, &pager, window, cx)` 静默同步；仅值变化才 set_value，无关更新保留跳页输入草稿；Unknown 不接已知总数分页器 |

## 2. typed 列

`TableColumn::value(key, title, accessor)` 捕获 typed V，V: Clone + PartialEq + PartialOrd + 'static。必须提供 `display_with` 或 `render_with` 后 build；异构列容器中调用者无需 Any 或 downcast。默认 `.sortable(true)` 要求 V: Ord；浮点或业务空值排序显式 `compare_by` 提供全序。`.filterable(true)` 开启相等 Scalar／Set 与闭区间 Range，`.filter_scalar` 自定义字面文本规则，`.missing_when` 明确缺失定义。

构造列时取 `filter_key()`，与同一次列声明一起保留。`query.with_filter(&key, Some(TableFilterValue::Scalar(0)))` 保留 0，false 同理；None 清除，Set 空集合表示零匹配，Missing 与空字符串不同。非法区间在执行／提交前拒绝。筛选与排序默认归第一页，显式 with_page 可覆盖；页码一基、page_size 非零，Local 自动夹到末页。

多排序按有序条目稳定比较，同值保留 source 次序；Shift 点击追加列，切换已有列方向不改变优先级。排序控件与导航键只发请求，等待期间显示 accepted 状态。Alt+S 排当前活动列；不可排序列拒绝请求。

## 3. 例 A：最小有限视口表格（M1）

本节完整程序已迁入 [examples/minimal/src/bin/table.rs](../examples/minimal/src/bin/table.rs)，使用实际公共 API，通过消费者编译。它只依赖 `yue-component`，从其 re-export 获取 GPUI；不开放尚未实现的查询 API。

```rust
//! Minimal consumer of the public Table API; no private modules or component layer.
use std::rc::Rc;
use yue_component::gpui_kit::{
    self, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, SharedString,
    Styled as _, Window, WindowOptions, div, rems,
};
use yue_component::{
    Accent, TableColumn, TableDelegate, TableRows, TableView, ThemeMode, YueTable, YueTableState,
    YueTheme,
};

struct Order {
    key: SharedString,
    title: SharedString,
    amount: i32,
}
struct Orders;
impl TableDelegate for Orders {
    type Source = TableRows<Order>;
}
struct Example {
    _delegate: Entity<Orders>,
    table: Entity<YueTableState<Orders>>,
}
impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let source = Rc::new(
            TableRows::try_new(
                "orders",
                1,
                vec![
                    Order {
                        key: "a".into(),
                        title: "设计稿".into(),
                        amount: 0,
                    },
                    Order {
                        key: "b".into(),
                        title: "实现与验证".into(),
                        amount: 1200,
                    },
                ]
                .into(),
                |row: &Order| row.key.clone(),
            )
            .unwrap(),
        );
        let columns = vec![
            TableColumn::text("title", "事项", |r: &Order| r.title.clone()).width(rems(18.)),
            TableColumn::text("amount", "金额", |r: &Order| r.amount.to_string()).width(rems(10.)),
        ];
        let delegate = cx.new(|_| Orders);
        let table = cx.new(|cx| YueTableState::new(&delegate, cx));
        table.update(cx, |s, cx| {
            s.commit_view(TableView::new(source, columns), None, cx)
                .unwrap()
        });
        Self {
            _delegate: delegate,
            table,
        }
    }
}
impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().p_4().child(
            YueTable::new("orders", &self.table)
                .height(rems(20.))
                .accessibility_label("订单事项"),
        )
    }
}
fn main() {
    gpui_kit::application().run(|cx| {
        yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| cx.new(Example::new)).unwrap();
    });
}
```

运行 `cargo run -p yue-minimal --bin table`；自动化挂载与边界验收见 [M1 实施记录](research/2026-11-10-table-m1-implementation.md)。原设计中独立 Row 关联类型、构造／commit 必传 Window、第三个 TableDataMode 参数在 M1 予以精简；后续阶段不得让当前有效的最小程序失效。

## 4. 例 B：本地查询＋受控分页（M3）

源文件：[table-local.rs](../examples/minimal/src/bin/table-local.rs)。

```rust
//! Local query consumer. All rows stay in the host; projections borrow them.
use std::rc::Rc;
use yue_component::gpui_kit::{
    self, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, SharedString,
    Styled as _, Subscription, Window, WindowOptions, div, rems,
};
use yue_component::{
    Accent, PaginationChange, PaginationValue, TableColumn, TableDelegate, TableEvent,
    TableLocalQuery, TableProjection, TableQuery, TableRows, TableSource, ThemeMode, YuePagination,
    YuePaginationState, YueTable, YueTableState, YueTheme, sync_table_pagination,
};
struct Order {
    key: SharedString,
    amount: i32,
}
struct Orders;
impl TableDelegate for Orders {
    type Source = TableProjection<TableRows<Order>>;
}
struct Example {
    _delegate: Entity<Orders>,
    table: Entity<YueTableState<Orders>>,
    pager: Entity<YuePaginationState>,
    source: Rc<TableRows<Order>>,
    columns: Rc<[TableColumn<Order>]>,
    _subscriptions: Vec<Subscription>,
}
impl Example {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let source = Rc::new(
            TableRows::try_new(
                "local-orders",
                1,
                (0..37)
                    .map(|i| Order {
                        key: format!("order-{i}").into(),
                        amount: i % 7,
                    })
                    .collect::<Vec<_>>()
                    .into(),
                |r: &Order| r.key.clone(),
            )
            .unwrap(),
        );
        let columns: Rc<[_]> = vec![
            TableColumn::text("id", "订单", |r: &Order| r.key.clone()),
            TableColumn::value("amount", "金额", |r: &Order| r.amount)
                .display_with(|v| v.to_string().into())
                .sortable(true)
                .build()
                .unwrap(),
        ]
        .into();
        let delegate = cx.new(|_| Orders);
        let table = cx.new(|cx| YueTableState::new(&delegate, cx));
        table.update(cx, |s, cx| {
            s.commit_view(
                TableLocalQuery::execute(
                    source.clone(),
                    columns.clone(),
                    TableQuery::paged(1, 10).unwrap(),
                    1,
                )
                .unwrap(),
                None,
                cx,
            )
            .unwrap()
        });
        let pager =
            cx.new(|cx| YuePaginationState::new(PaginationValue::new(37, 1, 10).unwrap(), w, cx));
        let subscriptions = vec![
            cx.subscribe_in(&table, w, |s: &mut Self, _, event: &TableEvent, w, cx| {
                if let TableEvent::QueryRequested(r) = event {
                    if !s.table.read(cx).is_current_request(r.id()) {
                        return;
                    }
                    let current = s.table.read(cx).view().unwrap().clone();
                    let query = r.apply_to(&current).unwrap();
                    let next = TableLocalQuery::execute(
                        s.source.clone(),
                        s.columns.clone(),
                        query,
                        current.source().revision() + 1,
                    )
                    .unwrap();
                    s.table.update(cx, |state, cx| {
                        state.commit_view(next, Some(r.id()), cx).unwrap()
                    });
                    let accepted = s.table.read(cx).view().unwrap().clone();
                    sync_table_pagination(&accepted, &s.pager, w, cx).unwrap();
                    cx.notify();
                }
            }),
            cx.subscribe_in(
                &pager,
                w,
                |s: &mut Self, _, event: &PaginationChange, _, cx| {
                    let value = event.value();
                    s.table.update(cx, |state, cx| {
                        state
                            .request_page(value.page(), value.page_size(), cx)
                            .unwrap()
                    });
                },
            ),
        ];
        Self {
            _delegate: delegate,
            table,
            pager,
            source,
            columns,
            _subscriptions: subscriptions,
        }
    }
}
impl Render for Example {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                YueTable::new("orders", &self.table)
                    .height(rems(24.))
                    .accessibility_label("本地订单"),
            )
            .child(
                YuePagination::new("pages", &self.pager)
                    .show_total(true)
                    .show_jumper(true),
            )
    }
}
fn main() {
    gpui_kit::application().run(|cx| {
        yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
        gpui_kit::open_window(WindowOptions::default(), cx, |w, cx| {
            cx.new(|cx| Example::new(w, cx))
        })
        .unwrap();
    });
}
```

## 5. 例 C：远程页、失败与刷新（M3）

源文件：[table-remote.rs](../examples/minimal/src/bin/table-remote.rs)。

```rust
//! Remote-page consumer. The server is simulated; Table receives only one page.
use std::rc::Rc;
use yue_component::gpui_kit::{
    self, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, SharedString,
    Styled as _, Subscription, Window, WindowOptions, div, rems,
};
use yue_component::{
    Accent, PaginationChange, PaginationValue, TableColumn, TableDataMode, TableDelegate,
    TableEvent, TableLoadState, TableQuery, TableQueryRequest, TableRemotePage, TableRows,
    TableSource, TableTotal, TableView, ThemeMode, YueButton, YuePagination, YuePaginationState,
    YueTable, YueTableState, YueTheme, sync_table_pagination,
};
struct Order {
    key: SharedString,
    amount: i32,
}
struct Orders;
impl TableDelegate for Orders {
    type Source = TableRemotePage<TableRows<Order>>;
}
fn server_page(
    query: TableQuery,
    revision: u64,
    columns: Rc<[TableColumn<Order>]>,
) -> TableView<TableRemotePage<TableRows<Order>>> {
    let page = query.page().unwrap();
    let start = (page.number() - 1) * page.size();
    let rows = (start..(start + page.size()).min(37))
        .map(|i| Order {
            key: format!("order-{i}").into(),
            amount: i as i32,
        })
        .collect::<Vec<_>>();
    let source = Rc::new(
        TableRows::try_new("remote-orders", revision, rows.into(), |r: &Order| {
            r.key.clone()
        })
        .unwrap(),
    );
    TableView::new(Rc::new(TableRemotePage::new(source, [])), columns)
        .with_query(query)
        .with_result(TableDataMode::Remote, TableTotal::Known(37))
}
struct Example {
    _delegate: Entity<Orders>,
    table: Entity<YueTableState<Orders>>,
    pager: Entity<YuePaginationState>,
    waiting: Option<TableQueryRequest>,
    _subscriptions: Vec<Subscription>,
}
impl Example {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let columns: Rc<[_]> = vec![
            TableColumn::text("id", "订单", |r: &Order| r.key.clone()),
            TableColumn::text("amount", "金额", |r: &Order| r.amount.to_string()),
        ]
        .into();
        let delegate = cx.new(|_| Orders);
        let table = cx.new(|cx| YueTableState::new(&delegate, cx));
        table.update(cx, |s, cx| {
            s.commit_view(
                server_page(TableQuery::paged(1, 10).unwrap(), 1, columns),
                None,
                cx,
            )
            .unwrap()
        });
        let pager =
            cx.new(|cx| YuePaginationState::new(PaginationValue::new(37, 1, 10).unwrap(), w, cx));
        let subscriptions = vec![
            cx.subscribe_in(&table, w, |s: &mut Self, _, event: &TableEvent, w, cx| {
                if let TableEvent::QueryRequested(r) = event {
                    if !s.table.read(cx).is_current_request(r.id()) {
                        return;
                    }
                    s.waiting = Some(r.clone());
                    let old = s
                        .table
                        .read(cx)
                        .view()
                        .unwrap()
                        .as_ref()
                        .clone()
                        .load_state(TableLoadState::Refreshing);
                    s.table
                        .update(cx, |state, cx| state.commit_view(old, None, cx).unwrap());
                    s.sync(w, cx);
                    cx.notify();
                }
            }),
            cx.subscribe_in(
                &pager,
                w,
                |s: &mut Self, _, event: &PaginationChange, _, cx| {
                    let value = event.value();
                    s.table.update(cx, |state, cx| {
                        state
                            .request_page(value.page(), value.page_size(), cx)
                            .unwrap()
                    });
                },
            ),
        ];
        Self {
            _delegate: delegate,
            table,
            pager,
            waiting: None,
            _subscriptions: subscriptions,
        }
    }
    fn sync(&self, w: &mut Window, cx: &mut Context<Self>) {
        let view = self.table.read(cx).view().unwrap().clone();
        sync_table_pagination(&view, &self.pager, w, cx).unwrap();
    }
}
impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child("翻页后点击完成，模拟远程响应；等待和失败均保留旧页")
            .child(
                YueButton::new("complete", "完成请求")
                    .disabled(self.waiting.is_none())
                    .on_click(cx.listener(|s, _, w, cx| {
                        if let Some(r) = s.waiting.take() {
                            // Check before converting or publishing response-derived values.
                            if !s.table.read(cx).is_current_request(r.id()) {
                                return;
                            }
                            let current = s.table.read(cx).view().unwrap().clone();
                            let query = r.apply_to(&current).unwrap();
                            let next = server_page(
                                query,
                                current.source().revision() + 1,
                                current.shared_columns().clone(),
                            );
                            s.table.update(cx, |state, cx| {
                                state.commit_view(next, Some(r.id()), cx).unwrap()
                            });
                            s.sync(w, cx);
                            cx.notify();
                        }
                    })),
            )
            .child(
                YueButton::new("fail", "模拟失败")
                    .disabled(self.waiting.is_none())
                    .on_click(cx.listener(|s, _, w, cx| {
                        if let Some(r) = s.waiting.take() {
                            if !s.table.read(cx).is_current_request(r.id()) {
                                return;
                            }
                            s.table.update(cx, |state, cx| {
                                state.reject_request(r.id(), cx).unwrap();
                                let old =
                                    state.view().unwrap().as_ref().clone().load_state(
                                        TableLoadState::Failed("加载失败，请重试".into()),
                                    );
                                state.commit_view(old, None, cx).unwrap();
                            });
                            s.sync(w, cx);
                            cx.notify();
                        }
                    })),
            )
            .child(
                YueButton::new("retry", "刷新").on_click(cx.listener(|s, _, _, cx| {
                    s.table
                        .update(cx, |state, cx| state.request_refresh(cx).unwrap());
                })),
            )
            .child(
                YueTable::new("orders", &self.table)
                    .height(rems(24.))
                    .accessibility_label("远程订单"),
            )
            .child(
                YuePagination::new("pages", &self.pager)
                    .show_total(true)
                    .show_jumper(true),
            )
    }
}
fn main() {
    gpui_kit::application().run(|cx| {
        yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
        gpui_kit::open_window(WindowOptions::default(), cx, |w, cx| {
            cx.new(|cx| Example::new(w, cx))
        })
        .unwrap();
    });
}
```

## 6. 示例布局和范围

三个消费者均只依赖 yue-component，从其 re-export 获取 GPUI。运行 `cargo run -p yue-minimal --bin table-local` 或 `--bin table-remote`。Remote 示例用按钮模拟响应完成与失败，没有声称连接真实网络；生产宿主把同样票据检查放在所有成功、失败、finally 与发布派生值之前，并按自己的任务所有权取消卸载任务。

综合交互位于 [Showcase 查询示例](../examples/windows-showcase/src/pages/table/query.rs)：普通 Dropdown 筛选、TreeSelect 叶子筛选、自定义名称／数值范围草稿、跨页选择及批量核验、明确标注本页／筛选结果／已选范围的汇总。完整模拟服务端拥有 137 条数据，所以能显示完整汇总；只加载一页的实际 Remote 消费者不能据该页声称完整域汇总。

有限视口表格由一个 Base owner 管正文滚动，Pagination 位于视口外；外层 flex 子项保持 min_w_0／min_h_0。查询与汇总在接受路径执行，render 只组合已有值和当前窗口记录。M4 的实际接口与接线见 [复杂记录指南](45-table-m4.md)及生产 Showcase 的 [编辑示例](../examples/windows-showcase/src/pages/table/editing.rs)、[树与详情示例](../examples/windows-showcase/src/pages/table/structure.rs)。M5 Cells／span、M6 重排另分阶段实现和验收。

## 7. M4 接入要点

自然行高在 TableView 中声明 `with_row_sizing(TableRowSizing::measured(rems(2.5))?)`；Known 接受 `(TableRowKey, Rems)` 序列。正文选择、页根总数与显示行数使用不同合同，不将详情行包装成业务记录。

`TableOutline<S>` 是宿主持有的结构适配器，Table 的 Source 改为 `TableOutlineSource<S>`，Row 是 `TableOutlineRow` 元数据；通过 `source.record(row)` 借用业务 Row。宿主每次接受树／详情变化后发布新的 `snapshot(revision, page)`。根分页时同步 TableQuery 页码及 root_total；折叠和分页不清除合法已选 key。级联默认关闭，opt-in 后调用 outline.apply_selection，而非应用普通 Toggle 再私自递归 children。

`TableDelegate::render_detail(row, width, window, context)` 返回现有组件组合。详情中的 retained 控件／子表按业务 key 在宿主创建和释放；禁止将同一个 Entity 同时挂到多个打开的详情。

编辑接线保持三种身份：宿主 retained 控件值、Table 编辑 token、业务 source row revision。控件事件比较实际值后使旧保存失效；保存入口先 flush 最新值。先 request_save，验证通过再 accept_edit_validation，只有 Saving 中的当前 token 可 commit_edit。示例实际复用 Form／Field；不要将包含真实输入的 row 变成每帧重建的临时 Entity。
