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
