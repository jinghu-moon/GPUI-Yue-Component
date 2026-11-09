use super::super::*;
use gpui_kit::{Entity, ScrollHandle, rems};
use yue_component::{
    TreeCheckMode, TreeNode, TreeSelectionMode, YueResizable, YueResizablePanel, YueResizableState,
    YueScrollbar, YueTree, YueTreeState,
};

pub(crate) struct ResizablePage {
    states: [Entity<YueResizableState>; 4],
    tree: Entity<YueTreeState>,
    editor: Entity<gpui_kit::base::input::InputState>,
    logs: ScrollHandle,
    disabled: bool,
    dark: bool,
    commits: usize,
}
impl ResizablePage {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let tree = cx.new(|cx| {
            let mut state =
                YueTreeState::new(TreeSelectionMode::Single, TreeCheckMode::None, window, cx);
            state
                .set_nodes(
                    vec![TreeNode::new("workspace", "Yue 工作空间").children([
                        TreeNode::new("src", "src").children([
                            TreeNode::new("main", "main.rs"),
                            TreeNode::new("theme", "theme.rs"),
                            TreeNode::new("layout", "layout.rs"),
                        ]),
                        TreeNode::new("docs", "设计文档").children([
                            TreeNode::new("resizable", "Resizable 分栏"),
                            TreeNode::new("scrollbar", "Scrollbar 滚动条"),
                        ]),
                    ])],
                    cx,
                )
                .unwrap();
            state
                .set_expanded_keys(vec!["workspace".into(), "src".into(), "docs".into()], cx)
                .unwrap();
            state
        });
        Self {
            states: std::array::from_fn(|_| cx.new(|_| YueResizableState::default())),
            tree,
            editor: cx.new(|cx| {
                gpui_kit::base::input::InputState::new(window, cx)
                    .default_value("这里的输入内容会在调整布局和切换页面后保留。")
            }),
            logs: ScrollHandle::default(),
            disabled: false,
            dark: false,
            commits: 0,
        }
    }
}
fn pane(theme: YueTheme, title: &'static str, body: impl IntoElement) -> impl IntoElement {
    div()
        .size_full()
        .min_w_0()
        .min_h_0()
        .flex()
        .flex_col()
        .bg(gpui_kit::rgb(theme.surface()))
        .text_color(gpui_kit::rgb(theme.text_primary()))
        .child(
            div()
                .h_10()
                .flex_none()
                .px_4()
                .flex()
                .items_center()
                .text_sm()
                .border_b_1()
                .border_color(gpui_kit::rgb(theme.border_default()))
                .child(title),
        )
        .child(div().flex_1().min_w_0().min_h_0().child(body))
}
impl Render for ResizablePage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let sample = if self.dark {
            YueTheme::new(ThemeMode::Dark, theme.accent())
        } else {
            theme
        };
        let basic = YueResizable::horizontal("basic-split", &self.states[0]).theme(sample).disabled(self.disabled)
            .on_resize(cx.listener(|p, _, _, cx| { p.commits += 1; cx.notify(); }))
            .child(YueResizablePanel::new("collection").label("集合与详情")
                .initial_size(rems(16.)).min_size(rems(9.)).max_size(rems(25.))
                .child(pane(sample, "集合", div().p_4().child("拖动右侧边界，分配集合与详情的宽度。"))))
            .child(YueResizablePanel::new("detail").label("详情")
                .min_size(rems(12.)).child(pane(sample, "详情", div().p_4().flex().flex_col().gap_3()
                    .child("Tab 聚焦分隔器，再按方向键；Shift 加大步长，Home／End 到达约束边界。")
                    .child(YueInput::new("split-editor", sample, &self.editor)))));
        let vertical = YueResizable::vertical("vertical-split", &self.states[1])
            .theme(sample)
            .disabled(self.disabled)
            .child(
                YueResizablePanel::new("preview")
                    .label("预览与输出")
                    .initial_size(rems(8.))
                    .min_size(rems(5.))
                    .child(pane(
                        sample,
                        "预览",
                        div().p_4().child("上下分栏：使用上／下键调整高度。"),
                    )),
            )
            .child(
                YueResizablePanel::new("output")
                    .label("输出")
                    .min_size(rems(5.))
                    .child(pane(
                        sample,
                        "输出",
                        div()
                            .p_4()
                            .child("双击分隔线或按 Enter，恢复前一面板的默认高度。"),
                    )),
            );
        let log_body = div()
            .relative()
            .size_full()
            .min_h_0()
            .child(
                div()
                    .id("split-log-viewport")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.logs)
                    .child(div().flex().flex_col().px_4().children((0..30).map(|ix| {
                        div()
                            .h_7()
                            .flex_none()
                            .text_sm()
                            .child(format!("{:02}  分栏布局已准备好", ix + 1))
                    }))),
            )
            .child(YueScrollbar::vertical("split-log-scrollbar", &self.logs).theme(sample));
        let centre = YueResizable::vertical("work-output-split", &self.states[3])
            .theme(sample)
            .disabled(self.disabled)
            .child(
                YueResizablePanel::new("work")
                    .label("工作区与日志")
                    .min_size(rems(7.))
                    .child(pane(
                        sample,
                        "工作区",
                        div()
                            .p_4()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child("导航宽度在容器改变时保持；中间区域分配剩余空间。")
                            .child("左右边界和中间上下边界分别使用独立尺寸状态。"),
                    )),
            )
            .child(
                YueResizablePanel::new("logs")
                    .label("日志")
                    .initial_size(rems(7.))
                    .min_size(rems(5.))
                    .child(pane(sample, "日志", log_body)),
            );
        let nested = YueResizable::horizontal("workspace-split", &self.states[2])
            .theme(sample)
            .disabled(self.disabled)
            .child(
                YueResizablePanel::new("navigation")
                    .label("导航与工作区")
                    .initial_size(rems(12.))
                    .min_size(rems(9.))
                    .max_size(rems(22.))
                    .fixed(true)
                    .child(pane(
                        sample,
                        "文件导航",
                        YueTree::new("split-tree", &self.tree)
                            .theme(sample)
                            .show_lines(true)
                            .height(rems(18.).to_pixels(window.rem_size())),
                    )),
            )
            .child(
                YueResizablePanel::new("centre")
                    .label("工作区")
                    .min_size(rems(14.))
                    .child(centre),
            )
            .child(
                YueResizablePanel::new("inspector")
                    .label("属性")
                    .initial_size(rems(11.))
                    .min_size(rems(9.))
                    .max_size(rems(20.))
                    .child(pane(
                        sample,
                        "属性",
                        div()
                            .p_4()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child("类型：Rust 项目")
                            .child("布局：嵌套分栏")
                            .child("状态：可调整"),
                    )),
            );
        div()
            .debug_selector(|| "resizable-content".into())
            .w_full()
            .min_w_0()
            .p_8()
            .flex()
            .flex_col()
            .gap_6()
            .child(div().text_2xl().child("Resizable 可调整分栏"))
            .child(
                div()
                    .text_color(gpui_kit::rgb(theme.text_secondary()))
                    .child(
                        "拖动分隔线调整空间；双击或 Enter 复位。每块面板保留自己的内容与滚动位置。",
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        YueButton::new(
                            "split-toggle",
                            if self.disabled {
                                "启用分栏调整"
                            } else {
                                "禁用分栏调整"
                            },
                        )
                        .on_click(cx.listener(|p, _, _, cx| {
                            p.disabled = !p.disabled;
                            cx.notify();
                        })),
                    )
                    .child(
                        YueButton::new("split-reset", "恢复默认布局").on_click(cx.listener(
                            |p, _, _, cx| {
                                for s in &p.states {
                                    s.update(cx, |s, cx| s.reset_layout(cx));
                                }
                                cx.notify();
                            },
                        )),
                    )
                    .child(
                        YueButton::new(
                            "split-theme",
                            if self.dark {
                                "跟随当前主题"
                            } else {
                                "深色示例"
                            },
                        )
                        .on_click(cx.listener(|p, _, _, cx| {
                            p.dark = !p.dark;
                            cx.notify();
                        })),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .child(format!("基础示例已提交 {} 次调整", self.commits)),
            )
            .child(section(
                theme,
                "左右分栏",
                "集合与详情 · 双侧尺寸约束 · 输入内容保留",
                div().w_full().h(rems(13.)).child(basic),
            ))
            .child(section(
                theme,
                "上下分栏",
                "上／下键调整；前一面板默认高 8rem",
                div().w_full().h(rems(16.)).child(vertical),
            ))
            .child(section(
                theme,
                "嵌套工作空间",
                "固定导航 + 独立工作区／日志 + 属性；内容复用 Tree 与 Scrollbar",
                div().w_full().h(rems(22.)).child(nested),
            ))
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::base::{Root, test_support::snapshots};
    use gpui_kit::{Modifiers, Role, TestAppContext, VisualTestContext, px, size};
    fn draw(cx: &mut VisualTestContext) {
        for _ in 0..3 {
            cx.update(|w, cx| {
                w.draw(cx).clear(cx);
            });
            cx.run_until_parked();
        }
    }
    fn click(cx: &mut VisualTestContext, label: &str) {
        let at = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|s| s.label() == Some(label))
                .expect(label)
                .bounds()
                .center()
        });
        cx.simulate_click(at, Modifiers::default());
        draw(cx);
    }
    #[gpui_kit::test]
    fn nested_examples_keep_inputs_and_independent_owners_while_controls_work(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap()
        });
        let mut saved = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            w.activate_window();
            let page = cx.new(|cx| ResizablePage::new(w, cx));
            saved = Some(page.clone());
            Root::new(page, w, cx)
        });
        cx.simulate_resize(size(px(1280.), px(1800.)));
        draw(cx);
        let page = saved.unwrap();
        let states = page.read_with(cx, |p, _| p.states.clone());
        assert!(
            states
                .iter()
                .all(|s| !s.read_with(cx, |s, _| s.sizes().is_empty()))
        );
        let before: Vec<_> = states
            .iter()
            .map(|s| s.read_with(cx, |s, _| s.sizes().clone()))
            .collect();
        let editor = page.read_with(cx, |p, _| p.editor.clone());
        let value = editor.read_with(cx, |s, _| s.value());
        let at = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|s| s.role() == Some(Role::Splitter) && s.label() == Some("详情"))
                .unwrap()
                .bounds()
                .center()
        });
        cx.simulate_click(at, Modifiers::default());
        draw(cx);
        cx.simulate_keystrokes("right");
        draw(cx);
        assert_ne!(states[0].read_with(cx, |s, _| s.sizes().clone()), before[0]);
        for ix in 1..4 {
            assert_eq!(
                states[ix].read_with(cx, |s, _| s.sizes().clone()),
                before[ix]
            );
        }
        assert_eq!(editor.read_with(cx, |s, _| s.value()), value);
        click(cx, "深色示例");
        assert!(page.read_with(cx, |p, _| p.dark));
        assert_eq!(editor.read_with(cx, |s, _| s.value()), value);
        click(cx, "禁用分栏调整");
        assert!(cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .filter(|s| s.role() == Some(Role::Splitter))
                .all(|s| s.disabled() == Some(true))
        }));
        click(cx, "启用分栏调整");
        click(cx, "恢复默认布局");
        // The window was resized after mounting. Reset restores declared
        // defaults at its current size, rather than the old proportional split.
        assert_eq!(states[0].read_with(cx, |s, _| s.sizes()[0]), px(256.));
        assert_eq!(states[1].read_with(cx, |s, _| s.sizes()[0]), px(128.));
        assert_eq!(states[2].read_with(cx, |s, _| s.sizes()[0]), px(192.));
        assert_eq!(states[2].read_with(cx, |s, _| s.sizes()[2]), px(176.));
        assert_eq!(states[3].read_with(cx, |s, _| s.sizes()[1]), px(112.));
    }
}
