use super::super::*;
use yue_component::{
    BreadcrumbCollapse, BreadcrumbEllipsis, BreadcrumbNavigate, BreadcrumbSeparator, YueBreadcrumb,
    YueBreadcrumbItem, YueSegmented, YueSegmentedOption,
};

pub(crate) struct BreadcrumbPage {
    depth: usize,
    collapsed: bool,
    icons: bool,
    disabled: bool,
    readonly: bool,
    current_clickable: bool,
    slash: bool,
    expanded_ends: bool,
    word_ellipsis: bool,
    activations: usize,
    last: SharedString,
}

impl BreadcrumbPage {
    pub(crate) fn new(_: &mut Context<Self>) -> Self {
        Self {
            depth: 8,
            collapsed: true,
            icons: false,
            disabled: false,
            readonly: false,
            current_clickable: false,
            slash: false,
            expanded_ends: true,
            word_ellipsis: false,
            activations: 0,
            last: "尚未导航".into(),
        }
    }
}
fn items(count: usize, icons: bool, disabled: bool) -> Vec<YueBreadcrumbItem> {
    [
        "首页",
        "项目",
        "Yue Component",
        "设计规范",
        "组件",
        "导航",
        "Breadcrumb",
        "当前示例",
    ]
    .into_iter()
    .take(count)
    .enumerate()
    .map(|(i, label)| {
        let item =
            YueBreadcrumbItem::new(format!("section-{i}"), label).disabled(disabled && i == 3);
        if icons {
            if i == 0 {
                item.icon(outline::Home)
            } else {
                item.icon(outline::Folder)
            }
        } else {
            item
        }
    })
    .collect()
}
fn column() -> gpui_kit::Div {
    div().w_full().flex().flex_col().items_start().gap_4()
}
fn row() -> gpui_kit::Div {
    div().flex().items_center().flex_wrap().gap_4()
}
impl Render for BreadcrumbPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let mut settings = row();
        for (id, label, checked) in [
            ("collapse", "折叠中间层级", self.collapsed),
            ("icons", "前置图标", self.icons),
            ("disabled", "禁用设计规范", self.disabled),
            ("readonly", "只读路径", self.readonly),
            ("current", "允许点击当前页", self.current_clickable),
            ("ellipsis", "省略号显示为更多", self.word_ellipsis),
        ] {
            let owner = cx.entity();
            settings = settings.child(
                row().child(label).child(
                    YueSwitch::new(format!("breadcrumb-{id}"), theme, checked, label)
                        .size(SwitchSize::Small)
                        .on_change(move |value, _, _, cx| {
                            owner.update(cx, |page, cx| {
                                match id {
                                    "collapse" => page.collapsed = value,
                                    "icons" => page.icons = value,
                                    "disabled" => page.disabled = value,
                                    "readonly" => page.readonly = value,
                                    "current" => page.current_clickable = value,
                                    _ => page.word_ellipsis = value,
                                }
                                cx.notify();
                            })
                        }),
                ),
            );
        }
        settings = settings
            .child(
                YueSegmented::new(
                    "breadcrumb-separator",
                    theme,
                    if self.slash { "slash" } else { "chevron" },
                )
                .label("路径分隔符")
                .options([
                    YueSegmentedOption::new("chevron", "箭头分隔"),
                    YueSegmentedOption::new("slash", "斜杠分隔"),
                ])
                .on_change(cx.listener(|page, value: &SharedString, _, cx| {
                    page.slash = value == "slash";
                    cx.notify();
                })),
            )
            .child(
                YueSegmented::new(
                    "breadcrumb-retain",
                    theme,
                    if self.expanded_ends { "two" } else { "one" },
                )
                .label("折叠保留数量")
                .options([
                    YueSegmentedOption::new("one", "首尾各一项"),
                    YueSegmentedOption::new("two", "首尾各两项"),
                ])
                .on_change(cx.listener(|page, value: &SharedString, _, cx| {
                    page.expanded_ends = value == "two";
                    cx.notify();
                })),
            );
        let mut preview = YueBreadcrumb::new("breadcrumb-preview", theme)
            .width(gpui_kit::rems(42.))
            .label("示例路径")
            .overflow_label("显示隐藏的层级")
            .current_description("当前页")
            .items(items(self.depth, self.icons, self.disabled))
            .current_clickable(self.current_clickable);
        if self.collapsed {
            let retained = if self.expanded_ends { 2 } else { 1 };
            preview = preview.collapse(BreadcrumbCollapse::new(5).before(retained).after(retained));
        }
        if self.slash {
            preview = preview.separator(BreadcrumbSeparator::text("/"));
        }
        if self.word_ellipsis {
            preview = preview.ellipsis(BreadcrumbEllipsis::text("更多"));
        }
        if !self.readonly {
            preview =
                preview.on_navigate(cx.listener(|page, request: &BreadcrumbNavigate, _, cx| {
                    page.activations += 1;
                    page.last = format!("{} · {:?}", request.value(), request.source()).into();
                    if let Some(depth) = request
                        .value()
                        .strip_prefix("section-")
                        .and_then(|n| n.parse::<usize>().ok())
                    {
                        page.depth = depth + 1;
                    }
                    cx.notify();
                }));
        }
        let mut themes = column();
        for (id, mode, accent) in [
            ("light-azure", ThemeMode::Light, Accent::Azure),
            ("dark-azure", ThemeMode::Dark, Accent::Azure),
            ("light-neutral", ThemeMode::Light, Accent::Neutral),
            ("dark-neutral", ThemeMode::Dark, Accent::Neutral),
        ] {
            let theme = YueTheme::new(mode, accent);
            themes = themes.child(
                div()
                    .w_full()
                    .p_4()
                    .bg(gpui_kit::rgb(theme.surface()))
                    .child(
                        YueBreadcrumb::new(format!("breadcrumb-{id}"), theme)
                            .items(items(3, true, false)),
                    ),
            );
        }
        column()
            .debug_selector(|| "breadcrumb-content".into())
            .p_8()
            .gap_6()
            .child(div().text_2xl().child("Breadcrumb 面包屑"))
            .child(readout(
                theme,
                "文本按钮展示祖先，静态文字标识当前页；隐藏路径由下拉菜单承载。".into(),
            ))
            .child(section(
                theme,
                "配置与导航",
                "点击可用祖先或隐藏层级后，由此页面更新路径；组件只发导航请求。",
                column()
                    .child(settings)
                    .child(preview)
                    .child(
                        YueButton::new("breadcrumb-restore", "恢复完整路径")
                            .variant(ButtonVariant::Ghost)
                            .on_click(cx.listener(|page, _, _, cx| {
                                page.depth = 8;
                                cx.notify();
                            })),
                    )
                    .child(readout(
                        theme,
                        format!("导航次数：{} · 最近请求：{}", self.activations, self.last),
                    )),
            ))
            .child(section(
                theme,
                "基础与图标",
                "默认 ChevronRight 分隔；图标与文字组成同一个导航项。",
                column()
                    .child(
                        YueBreadcrumb::new("breadcrumb-basic", theme).items(items(3, false, false)),
                    )
                    .child(
                        YueBreadcrumb::new("breadcrumb-icons-example", theme)
                            .items(items(3, true, false)),
                    )
                    .child(
                        YueBreadcrumb::new("breadcrumb-slash-example", theme)
                            .items(items(3, false, false))
                            .separator(BreadcrumbSeparator::text("/")),
                    ),
            ))
            .child(section(
                theme,
                "隐藏路径与只读",
                "省略号支持键盘打开；只读路径显示完整层级名称，不伪造导航命令。",
                column()
                    .child(
                        YueBreadcrumb::new("breadcrumb-readonly", theme)
                            .items(items(8, true, true))
                            .collapse(BreadcrumbCollapse::new(5).before(2).after(2))
                            .overflow_label("查看只读层级"),
                    )
                    .child(
                        YueBreadcrumb::new("breadcrumb-dots", theme)
                            .items(items(8, false, false))
                            .collapse(BreadcrumbCollapse::new(5))
                            .ellipsis(BreadcrumbEllipsis::icon(outline::Dots))
                            .overflow_label("查看隐藏层级"),
                    ),
            ))
            .child(section(
                theme,
                "长文字与窄宽",
                "文字按真实宽度省略；溢出才显示完整提示。极窄时横向滚动，键盘焦点保持可见。",
                column().child(
                    YueBreadcrumb::new("breadcrumb-long", theme)
                        .width(gpui_kit::rems(25.))
                        .items([
                            YueBreadcrumbItem::new("root", "这是一个名称很长的项目目录"),
                            YueBreadcrumbItem::new("sub", "组件设计与交互规范"),
                            YueBreadcrumbItem::new("current", "当前页面的完整名称与设计说明"),
                        ]),
                ),
            ))
            .child(section(
                theme,
                "主题",
                "沿用 Yue 文本、图标、焦点与菜单 token。",
                themes,
            ))
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::base::{Root, test_support::snapshots};
    use gpui_kit::{Modifiers, Role, TestAppContext, VisualTestContext, px};

    fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|w, cx| {
            w.refresh();
            w.draw(cx).clear(cx);
        });
        cx.run_until_parked();
    }
    fn click(cx: &mut VisualTestContext, label: &str, role: Role) {
        let center = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.label() == Some(label) && n.role() == Some(role) && n.visible())
                .unwrap_or_else(|| panic!("missing {role:?}: {label}"))
                .bounds()
                .center()
        });
        cx.simulate_click(center, Modifiers::default());
        draw(cx);
    }

    #[gpui_kit::test]
    fn configuration_and_menu_navigation_update_the_preview(cx: &mut TestAppContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
            cx.set_reduce_motion(true);
        });
        let mut page = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            let view = cx.new(BreadcrumbPage::new);
            page = Some(view.clone());
            Root::new(view, w, cx)
        });
        cx.simulate_resize(gpui_kit::size(px(1250.), px(1600.)));
        cx.update(|w, _| w.activate_window());
        draw(cx);
        let page = page.unwrap();
        click(cx, "显示隐藏的层级", Role::Button);
        click(cx, "Yue Component", Role::MenuItem);
        assert_eq!(page.read_with(cx, |p, _| (p.depth, p.activations)), (3, 1));
        assert!(page.read_with(cx, |p, _| p.last.contains("section-2 · OverflowMenu")));
        click(cx, "恢复完整路径", Role::Button);
        assert_eq!(page.read_with(cx, |p, _| p.depth), 8);
        click(cx, "前置图标", Role::Switch);
        click(cx, "斜杠分隔", Role::RadioButton);
        click(cx, "省略号显示为更多", Role::Switch);
        click(cx, "首尾各一项", Role::RadioButton);
        assert!(page.read_with(cx, |p, _| p.icons
            && p.slash
            && p.word_ellipsis
            && !p.expanded_ends));
        click(cx, "只读路径", Role::Switch);
        click(cx, "显示隐藏的层级", Role::Button);
        cx.update(|w, _| {
            let nodes = snapshots(w);
            assert!(nodes.iter().any(|n| n.label() == Some("Yue Component")
                && n.role() == Some(Role::Caption)
                && n.visible()));
            assert!(!nodes.iter().any(|n| n.label() == Some("Yue Component")
                && n.role() == Some(Role::MenuItem)
                && n.visible()));
        });
        cx.simulate_keystrokes("escape");
        draw(cx);
        click(cx, "只读路径", Role::Switch);
        click(cx, "允许点击当前页", Role::Switch);
        click(cx, "当前示例", Role::Button);
        assert_eq!(page.read_with(cx, |p, _| (p.depth, p.activations)), (8, 2));
        assert!(page.read_with(cx, |p, _| p.last.contains("section-7 · Item")));
    }
}
