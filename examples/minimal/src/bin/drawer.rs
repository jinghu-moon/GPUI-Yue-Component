//! Public Drawer consumer, using Base Root and existing Yue controls.
use yue_component::gpui_kit::{
    self, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, Styled as _,
    Window, WindowOptions, div,
};
use yue_component::{
    Accent, DrawerFooter, ThemeMode, YueButton, YueDrawer, YueDrawerState, YueTheme,
};
struct Example {
    drawer: Entity<YueDrawerState>,
}
impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let state = self.drawer.clone();
        div()
            .size_full()
            .p_4()
            .child(
                YueButton::text("open", theme, "查看详情…").on_click(move |_, w, cx| {
                    let _ = state.update(cx, |s, cx| s.open(w, cx));
                }),
            )
            .child(
                YueDrawer::new("details", theme, &self.drawer)
                    .title("项目详情")
                    .footer(DrawerFooter::None)
                    .resizable(true)
                    .content(|_, _| {
                        div()
                            .child("内容由宿主提供；关闭后状态仍由宿主持有。")
                            .into_any_element()
                    }),
            )
    }
}
fn main() {
    gpui_kit::application().run(|cx| {
        yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|cx| Example {
                drawer: cx.new(YueDrawerState::new),
            })
        })
        .unwrap();
    });
}
