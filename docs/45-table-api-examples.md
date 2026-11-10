# YueTable：拟定 API 与接入示例

日期：2026-11-10。配套 [Table 主设计](45-table.md)。**第 3 节已更新为 M1 可编译消费者；第 1／2／4／5 节保留完整目标的后续草案，不代表当前 API。** 已实现签名和与草案的差异以 [M1 使用指南](45-table-m1.md) 为准；本地／远程查询例留到 M3 编译和交互验收。

三库依据见[完善记录](research/2026-11-10-table-design-hardening.md)：保留“稳定 key＋列定义＋数据”的基础入口；本地查询、远程查询和分页分开说明；没有复制 DOM、React hook 或多份受控排序状态。这里只给三个入口流程，复杂结构和非法组合沿主设计第 17 节。

## 1. 最小公共合同

### 1.1 不可变来源与呈现 delegate

```rust,ignore
pub trait TableSource: 'static {
    type Row: 'static;
    fn scope(&self) -> &SharedString;
    fn revision(&self) -> u64;
    fn len(&self) -> usize;
    fn row_key(&self, source_position: usize) -> Option<&TableRowKey>;
    fn position_of(&self, key: &TableRowKey) -> Option<usize>;
    fn row(&self, key: &TableRowKey) -> Option<&Self::Row>;
}

pub trait TableDelegate: Sized + 'static {
    type Row: 'static;
    type Source: TableSource<Row = Self::Row>;
    fn render_cell(
        &mut self,
        row: &Self::Row,
        column: &TableColumn<Self::Row>,
        cell: &TableCellContext,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        column.render(row, cell, window, cx)
    }
}
```

Row 借用绑定本次 source 读取，不返回跨帧的 `&Row` 或借用它的元素。默认 delegate 调列 renderer；复杂业务可覆写，但不改变几何、不发查询。Source 只保存业务记录和索引，Input Entity 等编辑状态由宿主另按 cell key 持有。Source 不要求 Send／Sync；后台工作显式转换为可发送纯数据。

| 拟定入口 | 明确合同 |
| --- | --- |
| `TableRows::try_new(scope, revision, Rc<[R]>, key_of) -> Result<Self, TableError>` | 共享现有 slice、一次建 key 索引并拒绝空／重复 key，不要求 R: Clone；scope 标识账户／结果域，不用 query 文本充当身份 |
| `TableView::new(Rc<S>, Rc<[TableColumn<S::Row>]>, TableDataMode)` | 产生候选；初始无选择、无分页、无展开、Ready。Local 的 total／投影由接受流程派生，Remote 必须提供明确 total 状态 |
| `view.with_source(source)`／`with_query(query)`／`with_selection(selection)` | 从当前快照构造候选，保留其他领域及基础 revision；只克隆共享引用和改变的小状态，不复制完整业务 Row |
| `view.with_total(TableTotal)`／`with_load(TableLoadState)` | Remote 的 Known／Unknown 与加载事实；不把 Unknown 写成 0；query 与当前结果来源不一致时须显式处于刷新状态并保留来源标签 |
| `state.view() -> Option<&Rc<TableView<D::Source>>>` | 未绑定时 None；接受后返回唯一事实快照，reader 无副作用 |
| `state.commit_view(candidate, response_to, window, cx) -> Result<TableCommitOutcome, TableError>` | Applied／Unchanged；先验证候选与票据，最后一次替换；失败不修改旧 source、view、会话或滚动 |
| `state.mark_pending(id, window, cx)`／`reject_request(id, reason, window, cx)` | 返回 Result；仅处理仍有效的本实例请求，静默结算等待；旧失败不清新 pending |
| `state.request_query(query, TableQuerySource, window, cx)` | 生成正常 QueryRequested，不直接改 accepted query；用于分页适配、具名刷新／重试，与静默 commit 分开 |

所有 reader 从同一 accepted view 返回 source／query／selection／columns／total／load／revision。普通来源替换仍受候选基础 revision 约束；初始化候选仅用于未绑定 state。业务 source 换新并不自动代表业务选择、展开或列偏好应清空。

### 1.2 typed 列与异构存储

`TableColumn<R>::value(key, title, read: Fn(&R) -> V)` 返回只在构造时持有 V 的 builder；V: Clone＋'static，Row 本身不要求 Clone。`display_with(Fn(&V) -> SharedString)` 或 `render_with(...)` 明确呈现；`sortable(true)` 要求 V: Ord，也可用 `compare_by(Fn(&V, &V) -> Ordering)` 定义数值／日期／空值规则。`build()` 返回 Result，缺少 renderer 或非法配置不生成半合法列。

`.filter_scalar(Fn(&V, &V) -> bool)` 声明 scalar 载荷；Set／Range／Missing 由相应显式 schema 声明，不按控件字符串猜值。`filter_key() -> TableFilterKey<V>` 仅在已声明筛选后取得，携带 column key 和列语义身份。`TableQuery::with_filter(&key, Option<TableFilterValue<V>>) -> Result<Self, TableError>` 在写入时检查形状；None 清除该列过滤，Scalar(0)／Scalar(false) 都是有效值。提交时再次检查该 key 是否属于当前列声明／语义版本。

内部在 V 仍明确时捕获 read、比较、校验、相等判断及 renderer，擦除为 `TableColumn<R>`；调用者不使用 Any／downcast。`TableColumn::display(key, title, renderer)` 用于 Button、Checkbox 等非数据列，不伪造 accessor。`TableFilterValue<V>` 的 Scalar／Set／Range／Missing 只是类型化载荷，各列 schema 决定允许组合；API 不保证任意 V 都支持所有算子。

```rust,ignore
fn zero_amount_query() -> Result<TableQuery, TableError> {
    let amount = TableColumn::<Order>::value("amount", "金额（分）", |r| r.amount)
        .display_with(|value| value.to_string().into())
        .filter_scalar(|actual, expected| actual == expected);
    let key: TableFilterKey<i64> = amount.filter_key();
    // 实际页面须将 amount.build()? 放入同一次声明的 columns。
    TableQuery::paged(1, 20)?.with_filter(&key, Some(TableFilterValue::Scalar(0)))
}
```

此片段说明类型关系，不直接把它返回的 query 用到另一套同名列上。真实页面保留同一次列声明生成的 filter key；列语义变化后旧 key 明确拒绝。数值不经显示文本往返；0／false、缺失、空字符串和空集合各按 schema 处理。

### 1.3 请求载荷、重基与错误

`TableEvent::QueryRequested(TableQueryRequest)` 与 `SelectionRequested(TableSelectionRequest)` 的 payload 均提供 `id()`、`before()`、`target()`、`base_revision()` 和 `apply_to(&accepted_view)`。`target` 用于说明当时意图；接受时通过 apply_to 在当前合法 owner 上应用 typed 意图，而非把旧完整集合覆盖新选择。不同领域的修订可以保留，冲突或过期同域请求依既定策略拒绝；apply_to 不发布。

`TableEvent::request_id()` 返回 Option：观察事件为 None。payload 可 clone 后交异步处理，但不得延长旧 renderer／快照的无界寿命。TableError 至少涵盖 Unbound、InvalidQuery、InvalidSource、InvalidColumns、InvalidSpan、StaleRequest、UnsupportedCombination 和适配错误；带说明的 InvalidQuery 可用 SharedString，正式字段私有与 reader 规则沿主设计。拒绝原因是用户可理解的文本，不把 Debug 错误当最终产品文案。

本附录远程例在等待时禁止新的 UI 修改请求，保留浏览滚动；通过宿主明确替换请求的场景仍要由 query／mount 票据拦截旧完成。允许连续输入的生产页面可采用最新绝对查询替代策略，但必须另验连续 toggle／分页意图，不只复制 before 快照。

## 2. 共享业务模型与列

以下代码被后三例共同使用。共享 import 覆盖全部例子，Table 新入口从 crate 根导出；没有内部模块路径。fixture 的 stable key 在数据进入来源前创建，示例不使用行 index 做身份。

```rust,ignore
use gpui_kit::*;
use std::{future::Future, pin::Pin, rc::Rc};
use yue_component::{
    PaginationChange, PaginationValue, TableColumn, TableDataMode, TableDelegate, TableError,
    TableEvent, TableFilterKey, TableFilterValue, TableLoadState, TableQuery, TableQueryRequest,
    TableQuerySource, TableRequestId, TableRowKey, TableRows, TableSource, TableTotal, TableView,
    YuePagination, YuePaginationState, YueTable, YueTableState,
};

struct Order {
    id: TableRowKey,
    name: SharedString,
    amount: i64,
}
struct OrderDelegate;
impl TableDelegate for OrderDelegate {
    type Row = Order;
    type Source = TableRows<Order>;
}
type OrderView = TableView<TableRows<Order>>;
type OrderState = YueTableState<OrderDelegate>;

fn orders() -> Result<Rc<TableRows<Order>>, TableError> {
    let rows: Rc<[Order]> = vec![
        Order {
            id: TableRowKey::new("o-41")?,
            name: "显示器".into(),
            amount: 125000,
        },
        Order {
            id: TableRowKey::new("o-42")?,
            name: "维护服务".into(),
            amount: 0,
        },
        Order {
            id: TableRowKey::new("o-43")?,
            name: "键盘".into(),
            amount: 32000,
        },
    ]
    .into();
    Ok(Rc::new(TableRows::try_new("orders", 1, rows, |r| {
        r.id.clone()
    })?))
}
fn columns(interactive: bool) -> Result<Rc<[TableColumn<Order>]>, TableError> {
    Ok(vec![
        TableColumn::<Order>::value("name", "名称", |r| r.name.clone())
            .display_with(|v| v.clone())
            .sortable(interactive)
            .build()?,
        TableColumn::<Order>::value("amount", "金额（分）", |r| r.amount)
            .display_with(|v| v.to_string().into())
            .sortable(interactive)
            .build()?,
    ]
    .into())
}
fn accepted(state: &Entity<OrderState>, cx: &App) -> Result<Rc<OrderView>, TableError> {
    state.read(cx).view().cloned().ok_or(TableError::Unbound)
}
fn pagination(view: &OrderView) -> Result<PaginationValue, TableError> {
    let TableTotal::Known(total) = view.total() else {
        return Err(TableError::InvalidQuery("当前分页器要求已知总数".into()));
    };
    let page = view
        .query()
        .page()
        .ok_or_else(|| TableError::InvalidQuery("此示例要求分页查询".into()))?;
    PaginationValue::new(total, page.number(), page.size())
        .map_err(|e| TableError::InvalidQuery(e.to_string().into()))
}
```

`TableView::data_mode()` 返回构造时的 Local／Remote 模式；`TableView::query()` 返回 &TableQuery。`with_*` 接受 &self 并返回保留基础 revision 的候选，不能原地修改 accepted Rc。TableQuery::default 是无分页；paged／with_page 使用一基页码与非零 page_size，非法输入返回错误。TableView::total 返回轻量值，query.page 返回 Option<TablePage>；Local 接受流程规范页码与派生 total，Remote 不重新处理返回页。这里的列宽、边框和 Medium 外观使用默认值，避免最小例先写一屏样式参数。

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

目标：排序与分页只改变一个 query；Local 接受流程处理过滤→排序→分页，renderer 不做第二次切片。数据例沿用三条记录，每页两条便于核对。选择和 typed 筛选可在后续单功能例中开启，不先混入本例。

```rust,ignore
struct LocalOrders {
    _delegate: Entity<OrderDelegate>,
    table: Entity<OrderState>,
    pager: Entity<YuePaginationState>,
    error: SharedString,
    _subscriptions: Vec<Subscription>,
}
impl LocalOrders {
    fn create(w: &mut Window, cx: &mut App) -> Result<Entity<Self>, TableError> {
        let delegate = cx.new(|_| OrderDelegate);
        let table = cx.new(|cx| YueTableState::new(&delegate, w, cx));
        let initial = TableView::new(orders()?, columns(true)?, TableDataMode::Local)
            .with_query(TableQuery::paged(1, 2)?);
        table.update(cx, |s, cx| s.commit_view(initial, None, w, cx))?;
        let view = accepted(&table, cx)?;
        let value = pagination(view.as_ref())?;
        let pager = cx.new(|cx| YuePaginationState::new(value, w, cx));
        Ok(cx.new(|cx| {
            let table_sub = cx.subscribe_in(
                &table,
                w,
                |this: &mut Self, _, event: &TableEvent, w, cx| {
                    if let TableEvent::QueryRequested(request) = event {
                        if let Err(error) = this.apply_query(request, w, cx) {
                            let reason: SharedString = error.to_string().into();
                            let settled = this.table.update(cx, |s, cx| {
                                s.reject_request(request.id(), reason.clone(), w, cx)
                            });
                            if settled.is_ok() {
                                this.error = reason;
                                cx.notify();
                            }
                        }
                    }
                },
            );
            let pager_sub = cx.subscribe_in(
                &pager,
                w,
                |this: &mut Self, _, event: &PaginationChange, w, cx| {
                    let requested = accepted(&this.table, cx).and_then(|view| {
                        view.query()
                            .with_page(event.value().page(), event.value().page_size())
                    });
                    let result = requested.and_then(|query| {
                        this.table.update(cx, |s, cx| {
                            s.request_query(query, TableQuerySource::Pagination, w, cx)
                        })
                    });
                    if let Err(error) = result {
                        this.error = error.to_string().into();
                        cx.notify();
                    }
                },
            );
            Self {
                _delegate: delegate,
                table,
                pager,
                error: "".into(),
                _subscriptions: vec![table_sub, pager_sub],
            }
        }))
    }
    fn apply_query(
        &mut self,
        request: &TableQueryRequest,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), TableError> {
        let before = accepted(&self.table, cx)?;
        let query = request.apply_to(&before)?;
        let candidate = before.with_query(query);
        self.table.update(cx, |s, cx| {
            s.commit_view(candidate, Some(request.id()), w, cx)
        })?;
        let view = accepted(&self.table, cx)?;
        let next = pagination(view.as_ref())?;
        self.pager.update(cx, |p, cx| {
            if p.value() != next {
                p.set_value(next, w, cx);
            }
        });
        if !self.error.is_empty() {
            self.error = "".into();
            cx.notify();
        }
        Ok(())
    }
}
impl Render for LocalOrders {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .min_w_0()
            .min_h_0()
            .gap_3()
            .child(self.error.clone())
            .child(
                YueTable::new("local-orders", &self.table)
                    .height(rems(24.0))
                    .accessibility_label("本地订单查询"),
            )
            .child(YuePagination::new("local-orders-pages", &self.pager))
    }
}
```

`request_query` 返回 Result<()>；它发请求而不直接发布，保留订阅句柄以接收唯一事件。commit 的 Applied 由 Table 自身调度必要失效，Unchanged 只结算；宿主不要为相同成功值额外 notify，只有自己的错误文本等真正改变时通知。无变化分页值不调用 set_value，所以在内容／选择更新时跳页 Input 草稿仍在。分页同步在成功接受后同次 App 更新内进行，不再发 QueryRequested。错误例保持旧行、页码、total，错误文字由真实页面替换为可恢复动作与准确文案。

## 5. 例 C：远程请求，成功／失败／卸载结算（M3）

此例复用上节已存在的 GPUI 接线方式，业务传输通过显式参数注入。`OrdersService` 返回启动错误或一个本地 Future；真实 SDK 负责 URL、鉴权、协议和值编码，本文不伪造一个通用 HTTP API。响应的 source 包含服务器已处理的当前页，total 指结果域；初始视图由宿主提供，不把首次加载写进 render。

```rust,ignore
struct OrdersResponse {
    source: Rc<TableRows<Order>>,
    total: usize,
}
type OrdersFuture = Pin<Box<dyn Future<Output = Result<OrdersResponse, SharedString>>>>;
type OrdersService = Rc<dyn Fn(TableQuery) -> Result<OrdersFuture, SharedString>>;

struct RemoteOrders {
    _delegate: Entity<OrderDelegate>,
    table: Entity<OrderState>,
    service: OrdersService,
    task: Option<Task<()>>,
    generation: u64,
    error: SharedString,
    _subscriptions: Vec<Subscription>,
}
impl RemoteOrders {
    fn create(
        initial: OrderView,
        service: OrdersService,
        w: &mut Window,
        cx: &mut App,
    ) -> Result<Entity<Self>, TableError> {
        if initial.data_mode() != TableDataMode::Remote {
            return Err(TableError::InvalidQuery("此示例要求 Remote 模式".into()));
        }
        // commit 校验 total、当前页与 source 一致。
        let delegate = cx.new(|_| OrderDelegate);
        let table = cx.new(|cx| YueTableState::new(&delegate, w, cx));
        table.update(cx, |s, cx| s.commit_view(initial, None, w, cx))?;
        Ok(cx.new(|cx| {
            let subscription = cx.subscribe_in(
                &table,
                w,
                |this: &mut Self, _, event: &TableEvent, w, cx| {
                    if let TableEvent::QueryRequested(request) = event {
                        if let Err(error) = this.begin(request, w, cx) {
                            this.fail(request.id(), error.to_string().into(), w, cx);
                        }
                    }
                },
            );
            Self {
                _delegate: delegate,
                table,
                service,
                task: None,
                generation: 0,
                error: "".into(),
                _subscriptions: vec![subscription],
            }
        }))
    }
    fn fail(
        &mut self,
        id: TableRequestId,
        reason: SharedString,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let settled = self
            .table
            .update(cx, |s, cx| s.reject_request(id, reason.clone(), w, cx));
        if settled.is_ok() {
            self.error = reason;
            cx.notify();
        }
    }
    fn begin(
        &mut self,
        request: &TableQueryRequest,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), TableError> {
        let view = accepted(&self.table, cx)?;
        let query = request.apply_to(view.as_ref())?;
        self.table
            .update(cx, |s, cx| s.mark_pending(request.id(), w, cx))?;
        self.generation += 1;
        let generation = self.generation;
        self.task = None; // 取消自有等待；不宣称撤销已发到服务器的业务操作。
        self.error = "".into();
        let future = match (self.service)(query.clone()) {
            Ok(future) => future,
            Err(reason) => {
                self.fail(request.id(), reason, w, cx);
                return Ok(());
            }
        };
        let id = request.id();
        self.task = Some(cx.spawn_in(w, async move |weak, cx| {
            let result = future.await;
            let _ = weak.update_in(cx, |this, w, cx| {
                if this.generation != generation {
                    return;
                }
                this.task = None;
                match result {
                    Err(reason) => this.fail(id, reason, w, cx),
                    Ok(response) => {
                        let applied = accepted(&this.table, cx).and_then(|before| {
                            let candidate = before
                                .with_source(response.source)
                                .with_query(query)
                                .with_total(TableTotal::Known(response.total))
                                .with_load(TableLoadState::Ready);
                            this.table
                                .update(cx, |s, cx| s.commit_view(candidate, Some(id), w, cx))
                        });
                        if let Err(error) = applied {
                            this.fail(id, error.to_string().into(), w, cx);
                        }
                        cx.notify();
                    }
                }
            });
        }));
        cx.notify();
        Ok(())
    }
}
impl Render for RemoteOrders {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .min_w_0()
            .min_h_0()
            .gap_3()
            .child(self.error.clone())
            .child(
                YueTable::new("remote-orders", &self.table)
                    .height(rems(24.0))
                    .disabled(self.task.is_some())
                    .accessibility_label("服务器订单查询"),
            )
    }
}
```

宿主首次已有数据时的入口如下；尚无数据时显式构造空 source 与 Loading 输入，首次请求由宿主生命周期发起，不能依赖 render 触发。

```rust,ignore
fn remote_example(
    service: OrdersService,
    w: &mut Window,
    cx: &mut App,
) -> Result<Entity<RemoteOrders>, TableError> {
    let initial = TableView::new(orders()?, columns(true)?, TableDataMode::Remote)
        .with_query(TableQuery::paged(1, 3)?)
        .with_total(TableTotal::Known(3));
    RemoteOrders::create(initial, service, w, cx)
}
```

此例等待时 query 尚未接受，保持旧排序／页码和结果，mark_pending 驱动等待反馈；成功一次接受 query＋source＋total，并结算 pending，失败保持旧快照。它与“先接受 query、旧数据标为 Refreshing”的另一种显式策略互斥，不能混用来源标签。Remote 的分页 UI 可直接复用例 B 的分页适配，前提是 Known total；Unknown 使用具名加载入口。

generation 只保护宿主任务；commit／reject 仍必须验证 Table 的实例、mount、请求及结果域票据，不能把宿主整数当作全部保护。卸载后弱引用更新失败即停止；合法重挂载也必须由 Table 的 mount 校验拦截旧结果。传输取消由 SDK 提供，不引入 Table HTTP 客户端。

完整工作流验收使用注入的可控 service：立即成功、启动返回错误、异步失败、A/B 倒序完成、旧失败、错误 total、卸载／同 state 重挂载，再检查全部任务完成后的最终状态。单凭例中 generation 或一个 Promise 已完成不能证明通过。

## 6. 示例组织和布局

主设计第 22.2 节的独立示例逐步增加能力；基础例同时支持“查看代码／重置当前例”，综合工单例负责验证组合，不把全部功能塞进一个属性实验场。

```text
有限视口结果表                 自然高度 Card 小表
标题 + 主要动作                Card 标题
筛选／工具区                   自然表头
列对齐表头                     有界小集合，自然行高
正文视口 + overlay 滑块        可选 summary
列对齐 summary                 Card footer
整表 footer / Pagination
```

布局顺序、宽窄退化与控制来源属于设计验收；示例说明应告诉读者“谁持有值、发生几次请求、失败保留什么”。生产界面不显示 revision、采样门禁或内部坐标，测试控制面板只放 Showcase 开发区。API 变化时更新同一份示例，并以消费者编译结果替代本文的“未编译”标记；在此之前不发布可直接运行的承诺。
