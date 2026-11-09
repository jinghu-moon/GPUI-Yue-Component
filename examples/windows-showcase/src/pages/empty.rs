use super::super::*;
use gpui_kit::{Axis, FocusHandle, rems};
use std::path::PathBuf;
use yue_component::{
    EmptySize, Icon, YueAvatar, YueAvatarGroup, YueEmpty, YueLink, YueList, YueListItem,
};

pub(crate) struct EmptyPage {
    project: bool,
    filtered: bool,
    failed_image: bool,
    narrow: bool,
    refreshes: usize,
    toolbar_focus: FocusHandle,
}

impl EmptyPage {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        Self {
            project: false,
            filtered: false,
            failed_image: false,
            narrow: true,
            refreshes: 0,
            toolbar_focus: cx.focus_handle().tab_stop(true),
        }
    }
}

fn actions() -> gpui_kit::Div {
    div().flex().flex_wrap().items_center().gap_2()
}

impl Render for EmptyPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let empty =
            if self.filtered {
                YueEmpty::new("empty-projects")
                    .icon(outline::Search)
                    .title("未找到匹配项目")
                    .description("当前筛选条件没有匹配结果，可以清除筛选后重试。")
                    .actions(YueButton::new("empty-clear-filter", "清除筛选").on_click(
                        cx.listener(|page, _, window, cx| {
                            page.filtered = false;
                            page.toolbar_focus.focus(window, cx);
                            cx.notify();
                        }),
                    ))
            } else {
                YueEmpty::new("empty-projects")
                    .title("还没有项目")
                    .description("创建项目后，会显示在这里。")
                    .actions(
                        YueButton::new("empty-create", "创建示例项目").on_click(cx.listener(
                            |page, _, window, cx| {
                                page.project = true;
                                page.toolbar_focus.focus(window, cx);
                                cx.notify();
                            },
                        )),
                    )
            };
        let mut list = YueList::new("empty-project-list")
            .accessibility_label("示例项目")
            .bordered(true)
            .empty(empty.size(EmptySize::Small).inset(false));
        if self.project && !self.filtered {
            list =
                list.child(
                    YueListItem::new("empty-project-row")
                        .content("Yue 示例项目")
                        .extra(YueButton::new("empty-delete-project", "移除").on_click(
                            cx.listener(|page, _, window, cx| {
                                page.project = false;
                                page.toolbar_focus.focus(window, cx);
                                cx.notify();
                            }),
                        )),
                );
        }
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(if self.failed_image {
            "assets/empty-missing.svg"
        } else {
            "assets/empty-box.svg"
        });
        let basic = div()
            .w_full()
            .flex()
            .flex_wrap()
            .items_start()
            .gap_6()
            .child(
                div()
                    .flex_1()
                    .min_w(rems(10.))
                    .child(YueEmpty::new("empty-default")),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(rems(10.))
                    .child(YueEmpty::new("empty-small").size(EmptySize::Small)),
            );
        let custom = div()
            .w_full()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                YueEmpty::new("empty-search")
                    .icon(outline::Search)
                    .description("未找到匹配内容，试试其他关键词"),
            )
            .child(
                YueEmpty::new("empty-warning")
                    .media(
                        Icon::new(outline::AlertCircle).size(rems(2.)).color_value(
                            theme
                                .token(yue_component::theme::keys::WARNING_SOLID)
                                .unwrap()
                                .rgba()
                                .unwrap(),
                        ),
                    )
                    .description("No archived data. Reload after reconnecting."),
            );
        let image =
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueEmpty::new("empty-image")
                        .image(source)
                        .hide_description()
                        .actions(YueButton::new("empty-refresh", "刷新示例").on_click(
                            cx.listener(|page, _, _, cx| {
                                page.refreshes += 1;
                                cx.notify();
                            }),
                        )),
                )
                .child(
                    actions()
                        .child(
                            YueButton::new(
                                "empty-image-failure",
                                if self.failed_image {
                                    "恢复图片"
                                } else {
                                    "模拟图片失败"
                                },
                            )
                            .on_click(cx.listener(|page, _, _, cx| {
                                page.failed_image = !page.failed_image;
                                cx.notify();
                            })),
                        )
                        .child(readout(theme, format!("已刷新 {} 次", self.refreshes))),
                );
        let team = YueEmpty::new("empty-team")
            .media(YueAvatarGroup::new("empty-team-media", theme).children([
                YueAvatar::new("empty-team-a", theme).name("林"),
                YueAvatar::new("empty-team-b", theme).name("周"),
                YueAvatar::new("empty-team-c", theme).name("吴"),
            ]))
            .title("团队还没有共享项目")
            .description_content(
                div()
                    .flex()
                    .flex_wrap()
                    .justify_center()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .id("empty-team-help-copy")
                            .role(gpui_kit::Role::Label)
                            .aria_label("了解如何创建和共享项目")
                            .child("了解如何创建和共享项目"),
                    )
                    .child(YueLink::new(
                        "empty-help",
                        "GPUI Kit 文档",
                        "https://gpui-kit.com/docs",
                    )),
            );
        let layout = div().w_full().min_w_0().flex().flex_col().items_start().gap_3()
            .child(YueButton::new("empty-width", if self.narrow { "展开区域" } else { "收窄区域" })
                .on_click(cx.listener(|page, _, _, cx| { page.narrow = !page.narrow; cx.notify(); })))
            .child(div().w(if self.narrow { rems(15.) } else { rems(30.) }).max_w_full()
                .child(YueEmpty::new("empty-horizontal").layout(Axis::Horizontal)
                    .size(EmptySize::Small).inset(false).icon(outline::Search)
                    .title("没有匹配结果")
                    .description("尝试缩短关键词，或检查筛选条件。No matching results; try another keyword or remove the active filters.")
                    .actions(actions()
                        .child(YueButton::new("empty-layout-small", "收窄区域")
                            .on_click(cx.listener(|page, _, _, cx| { page.narrow = true; cx.notify(); })))
                        .child(YueButton::new("empty-layout-wide", "展开区域")
                            .on_click(cx.listener(|page, _, _, cx| { page.narrow = false; cx.notify(); }))))));
        let projects = div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                actions()
                    .child(
                        YueButton::new(
                            "empty-project-toolbar",
                            if self.project {
                                "清空示例项目"
                            } else {
                                "添加示例项目"
                            },
                        )
                        .track_focus(&self.toolbar_focus)
                        .on_click(cx.listener(|page, _, _, cx| {
                            page.project = !page.project;
                            page.filtered = false;
                            cx.notify();
                        })),
                    )
                    .child(
                        YueButton::new(
                            "empty-filter",
                            if self.filtered {
                                "清除筛选"
                            } else {
                                "筛选无结果"
                            },
                        )
                        .disabled(!self.project)
                        .on_click(cx.listener(|page, _, _, cx| {
                            page.filtered = !page.filtered;
                            cx.notify();
                        })),
                    ),
            )
            .child(list);
        div()
            .debug_selector(|| "empty-content".into())
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_8()
            .p_8()
            .child(div().text_2xl().child("Empty 空状态"))
            .child(section(
                theme,
                "基本用法",
                "默认图标与说明；两档尺寸共用主题和排版规则",
                basic,
            ))
            .child(section(
                theme,
                "自定义图标和文案",
                "普通无结果使用中性色；自定义媒体负责自身颜色和语义",
                custom,
            ))
            .child(section(
                theme,
                "自定义图片",
                "透明本地插画使用 Contain；切换加载失败可检查同尺寸回退",
                image,
            ))
            .child(section(
                theme,
                "自定义媒体与富内容",
                "媒体保留固有宽度，外部帮助链接保留自己的焦点和语义",
                team,
            ))
            .child(section(
                theme,
                "布局与长文案",
                "收窄内容区域后文字自然换行；动作由传入的容器负责换行",
                layout,
            ))
            .child(section(
                theme,
                "List 空态与实际操作",
                "创建第一项、移除最后一项、筛选无结果；宿主将消失控件的焦点交还工具栏",
                projects,
            ))
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::base::test_support::{ElementSnapshot, snapshots};
    use gpui_kit::{ElementId, Modifiers, TestAppContext, VisualTestContext, point, px};

    fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
    }

    fn node(cx: &mut VisualTestContext, id: &'static str) -> ElementSnapshot {
        cx.update(|window, _| {
            snapshots(window)
                .into_iter()
                .find(|n| n.path().last() == Some(&ElementId::from(id)))
                .unwrap_or_else(|| panic!("missing {id}"))
        })
    }

    fn click(cx: &mut VisualTestContext, id: &'static str) {
        let target = node(cx, id).bounds().center();
        let height = cx.update(|w, _| w.viewport_size().height);
        if target.y < px(0.) || target.y > height {
            cx.simulate_event(gpui_kit::ScrollWheelEvent {
                position: point(target.x, height / 2.),
                delta: gpui_kit::ScrollDelta::Pixels(point(px(0.), height / 2. - target.y)),
                touch_phase: gpui_kit::TouchPhase::Moved,
                ..Default::default()
            });
            draw(cx);
        }
        let position = node(cx, id).bounds().center();
        cx.simulate_click(position, Modifiers::default());
        draw(cx);
    }

    #[gpui_kit::test]
    fn first_item_last_item_and_filter_recovery_return_focus_to_the_toolbar(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap()
        });
        let (view, cx) = cx.add_window_view(Showcase::new);
        view.update(cx, |view, cx| {
            view.page = ShowcasePage::Empty;
            cx.notify();
        });
        draw(cx);
        click(cx, "empty-create");
        assert!(view.read_with(cx, |v, cx| v.empty_demo.read(cx).project));
        assert_eq!(node(cx, "empty-project-toolbar").focused(), Some(true));
        click(cx, "empty-delete-project");
        assert!(!view.read_with(cx, |v, cx| v.empty_demo.read(cx).project));
        assert_eq!(node(cx, "empty-project-toolbar").focused(), Some(true));
        click(cx, "empty-project-toolbar");
        click(cx, "empty-filter");
        assert!(view.read_with(cx, |v, cx| v.empty_demo.read(cx).filtered));
        click(cx, "empty-clear-filter");
        assert!(!view.read_with(cx, |v, cx| v.empty_demo.read(cx).filtered));
        assert_eq!(node(cx, "empty-project-toolbar").focused(), Some(true));
        assert_eq!(
            node(cx, "empty-delete-project").role(),
            Some(gpui_kit::Role::Button)
        );
    }
}
