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
