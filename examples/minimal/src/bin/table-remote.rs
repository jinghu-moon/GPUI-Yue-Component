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
