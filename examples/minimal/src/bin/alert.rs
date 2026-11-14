//! Inline controlled Alert using only the public API.
use yue_component::gpui_kit::{
    self, AppContext as _, Context, IntoElement, ParentElement as _, Render, Styled as _, Window,
    WindowOptions, div,
};
use yue_component::{Accent, AlertKind, ThemeMode, YueAlert, YueButton, YueTheme};
struct Example {
    visible: bool,
}
impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let close = cx.listener(|s, _: &yue_component::AlertCloseReason, _, cx| {
            s.visible = false;
            cx.notify();
        });
        div()
            .p_4()
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                YueAlert::new("sync-error", "同步失败")
                    .kind(AlertKind::Error)
                    .description("连接中断，本地编辑已保留。可以重试或关闭此提示。")
                    .visible(self.visible)
                    .closable(true)
                    .on_close(move |r, w, cx| close(&r, w, cx)),
            )
            .child(
                YueButton::new("restore", "重新显示").on_click(cx.listener(|s, _, _, cx| {
                    s.visible = true;
                    cx.notify();
                })),
            )
    }
}
fn main() {
    gpui_kit::application().run(|cx| {
        yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| Example { visible: true })
        })
        .unwrap();
    });
}
