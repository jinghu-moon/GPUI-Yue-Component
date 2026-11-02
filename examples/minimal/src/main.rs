//! External-style consumer: a window, a controlled switch and a command.
use yue_component::gpui_kit::{
    self, AppContext as _, Context, IntoElement, ParentElement as _, Render, Styled as _, Window,
    WindowOptions, div,
};
use yue_component::{Accent, ThemeMode, YueButton, YueSwitch, YueTheme};

#[derive(Default)]
struct Example {
    enabled: bool,
    saves: usize,
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        let theme = yue_component::current_theme(cx);
        div()
            .flex()
            .flex_col()
            .gap_4()
            .p_6()
            .bg(gpui_kit::rgb(theme.page()))
            .text_color(gpui_kit::rgb(theme.text_primary()))
            .child(
                YueSwitch::new("enabled", theme, self.enabled, "启用")
                    .use_default_theme()
                    .on_change(move |next, _, _, cx| {
                        owner.update(cx, |this, cx| {
                            this.enabled = next;
                            cx.notify();
                        });
                    }),
            )
            .child(
                YueButton::new("save", "保存").on_click(cx.listener(|this, _, _, cx| {
                    this.saves += 1;
                    cx.notify();
                })),
            )
            .child(format!("已保存 {} 次", self.saves))
    }
}

fn main() {
    gpui_kit::application().run(|cx| {
        yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure))
            .expect("initialize Yue");
        gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
            cx.new(|_| Example::default())
        })
        .expect("open example");
    });
}
