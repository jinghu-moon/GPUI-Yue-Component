use super::super::*;
use gpui_kit::{Axis, ScrollHandle, rems};
use yue_component::{ScrollbarVisibility, YueScrollbar, YueSegmented, YueSegmentedOption};

pub(crate) struct ScrollbarPage {
    handles: [ScrollHandle; 3],
    visibility: ScrollbarVisibility,
    short: bool,
    dark: bool,
}
impl ScrollbarPage {
    pub(crate) fn new(_: &mut Context<Self>) -> Self {
        Self {
            handles: Default::default(),
            visibility: ScrollbarVisibility::Auto,
            short: false,
            dark: false,
        }
    }
}
impl Render for ScrollbarPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let sample_theme = if self.dark {
            YueTheme::new(ThemeMode::Dark, Accent::Azure)
        } else {
            theme
        };
        let policies = YueSegmented::new(
            "scrollbar-visibility",
            theme,
            match self.visibility {
                ScrollbarVisibility::Auto => "auto",
                ScrollbarVisibility::Hover => "hover",
                ScrollbarVisibility::Always => "always",
            },
        )
        .label("滚动条显示策略")
        .options(vec![
            YueSegmentedOption::new("auto", "自动"),
            YueSegmentedOption::new("hover", "悬停视口"),
            YueSegmentedOption::new("always", "常显"),
        ])
        .on_change(cx.listener(|page, value: &SharedString, _, cx| {
            page.visibility = match value.as_ref() {
                "hover" => ScrollbarVisibility::Hover,
                "always" => ScrollbarVisibility::Always,
                _ => ScrollbarVisibility::Auto,
            };
            cx.notify();
        }));
        let mut examples = div().w_full().flex().flex_col().gap_6();
        for (i, title, description, axis) in [
            (
                0,
                "纵向内容",
                "靠近右边缘唤醒；点击滑块外的透明区域翻页",
                Some(Axis::Vertical),
            ),
            (
                1,
                "横向内容",
                "拖动底部滑块，查看末列内容",
                Some(Axis::Horizontal),
            ),
            (
                2,
                "双轴内容",
                "同一视口共享一个滚动条实例；右下角自动避让",
                None,
            ),
        ] {
            let handle = &self.handles[i];
            let wide = axis != Some(Axis::Vertical) && !self.short;
            let tall = axis != Some(Axis::Horizontal) && !self.short;
            let content = div()
                .flex()
                .flex_col()
                .flex_none()
                .w(if wide { rems(70.) } else { rems(14.) })
                .children((0..if tall { 30 } else { 2 }).map(|row| {
                    div()
                        .h(rems(2.5))
                        .flex_none()
                        .px_4()
                        .flex()
                        .items_center()
                        .justify_between()
                        .border_b_1()
                        .border_color(gpui_kit::rgb(sample_theme.border_default()))
                        .child(format!("记录 {:02} · 内容区域", row + 1))
                        .when(wide, |d| d.child(format!("末列 {:02}", row + 1)))
                }));
            let bar = match axis {
                Some(Axis::Vertical) => YueScrollbar::vertical(("scrollbar-sample", i), handle),
                Some(Axis::Horizontal) => YueScrollbar::horizontal(("scrollbar-sample", i), handle),
                _ => YueScrollbar::both(("scrollbar-sample", i), handle),
            }
            .visibility(self.visibility)
            .theme(sample_theme);
            let viewport = div()
                .relative()
                .w_full()
                .h(rems(12.))
                .min_w_0()
                .bg(gpui_kit::rgb(sample_theme.surface()))
                .text_color(gpui_kit::rgb(sample_theme.text_primary()))
                .child(
                    div()
                        .id(("scrollbar-viewport", i))
                        .size_full()
                        .overflow_scroll()
                        .track_scroll(handle)
                        .child(content),
                )
                .child(bar);
            examples = examples.child(section(theme, title, description, viewport));
        }
        div().debug_selector(|| "scrollbar-content".into()).w_full().min_w_0()
            .flex().flex_col().gap_6().p_8()
            .child(div().text_2xl().child("Scrollbar 滚动条"))
            .child(div().text_color(gpui_kit::rgb(theme.text_secondary()))
                .child("只显示悬浮滑块，所有状态均无可见轨道。滚动、拖动与键盘导航由原内容视口负责。"))
            .child(policies)
            .child(div().flex().flex_wrap().gap_2()
                .child(YueButton::new("scrollbar-short", if self.short { "恢复溢出内容" } else { "切换短内容" })
                    .on_click(cx.listener(|p,_,_,cx| { p.short = !p.short; cx.notify(); })))
                .child(YueButton::new("scrollbar-dark", if self.dark { "跟随当前主题" } else { "深色示例" })
                    .on_click(cx.listener(|p,_,_,cx| { p.dark = !p.dark; cx.notify(); })))
                .child(YueButton::new("scrollbar-reset", "回到起点")
                    .on_click(cx.listener(|p,_,_,cx| { for h in &p.handles { h.set_offset(Default::default()); } cx.notify(); }))))
            .child(examples)
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::base::test_support::snapshots;
    use gpui_kit::{Modifiers, TestAppContext, VisualTestContext};

    fn draw(cx: &mut VisualTestContext) {
        cx.update(|w, cx| {
            w.refresh();
            w.draw(cx).clear(cx);
        });
    }
    fn click(cx: &mut VisualTestContext, label: &str) {
        let at = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.label() == Some(label))
                .unwrap()
                .bounds()
                .center()
        });
        cx.simulate_click(at, Modifiers::default());
        draw(cx);
    }
    #[gpui_kit::test]
    fn showcase_policies_short_content_and_theme_keep_the_original_owners(cx: &mut TestAppContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap()
        });
        let (showcase, cx) = cx.add_window_view(Showcase::new);
        cx.update(|w, _| w.activate_window());
        showcase.update(cx, |s, cx| {
            s.page = ShowcasePage::Scrollbar;
            cx.notify();
        });
        draw(cx);
        click(cx, "常显");
        let page = showcase.read_with(cx, |s, _| s.scrollbar_demo.clone());
        assert_eq!(
            page.read_with(cx, |p, _| p.visibility),
            ScrollbarVisibility::Always
        );
        page.update(cx, |p, cx| {
            p.handles[0].set_offset(gpui_kit::point(gpui_kit::px(0.), gpui_kit::px(-80.)));
            cx.notify();
        });
        draw(cx);
        click(cx, "深色示例");
        assert!(page.read_with(cx, |p, _| p.dark));
        assert_eq!(
            page.read_with(cx, |p, _| p.handles[0].offset().y),
            gpui_kit::px(-80.)
        );
        click(cx, "切换短内容");
        assert!(page.read_with(cx, |p, _| p.short));
        for i in 0..3 {
            assert_eq!(
                page.read_with(cx, |p, _| p.handles[i].max_offset()),
                Default::default()
            );
        }
        click(cx, "恢复溢出内容");
        assert!(!page.read_with(cx, |p, _| p.short));
        click(cx, "悬停视口");
        assert_eq!(
            page.read_with(cx, |p, _| p.visibility),
            ScrollbarVisibility::Hover
        );
        click(cx, "回到起点");
        for i in 0..3 {
            assert_eq!(
                page.read_with(cx, |p, _| p.handles[i].offset()),
                Default::default()
            );
        }
    }
}
