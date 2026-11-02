use super::super::*;
use yue_component::{LinkHover, LinkSize, LinkVariant, YueLink, YueSegmented, YueSegmentedOption};

const TARGET: &str = "https://gpui-kit.com";
const VARIANTS: [(&str, LinkVariant); 5] = [
    ("中性", LinkVariant::Default),
    ("主色", LinkVariant::Primary),
    ("危险", LinkVariant::Danger),
    ("警告", LinkVariant::Warning),
    ("成功", LinkVariant::Success),
];

pub(crate) struct LinkPage {
    disabled: bool,
    underline: bool,
    leading: bool,
    trailing: bool,
    variant: LinkVariant,
    hover: LinkHover,
    size: LinkSize,
    activations: usize,
    handled: usize,
    last_target: SharedString,
}
impl LinkPage {
    pub(crate) fn new(_: &mut Context<Self>) -> Self {
        Self {
            disabled: false,
            underline: false,
            leading: false,
            trailing: false,
            variant: LinkVariant::Default,
            hover: LinkHover::Underline,
            size: LinkSize::Medium,
            activations: 0,
            handled: 0,
            last_target: "尚未请求".into(),
        }
    }
}
fn column() -> gpui_kit::Div {
    div().flex().flex_col().items_start().gap_4()
}
fn row() -> gpui_kit::Div {
    div().flex().items_center().flex_wrap().gap_6()
}
fn matrix(prefix: &str, underline: bool, disabled: bool, hover: LinkHover) -> impl IntoElement {
    row().children(VARIANTS.map(|(name, variant)| {
        YueLink::new(
            format!("{prefix}-{variant:?}"),
            format!("{name}链接"),
            TARGET,
        )
        .variant(variant)
        .underline(underline)
        .disabled(disabled)
        .hover(hover)
    }))
}
impl Render for LinkPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let preview = YueLink::new("link-preview", "跳转链接", TARGET)
            .variant(self.variant)
            .size(self.size)
            .hover(self.hover)
            .underline(self.underline)
            .disabled(self.disabled)
            .when(self.leading, |link| link.leading_icon(outline::Link))
            .when(self.trailing, |link| {
                link.trailing_icon(outline::ExternalLink)
            })
            .accessibility_description("使用系统默认应用打开 GPUI Kit 网站")
            .on_activate(cx.listener(|page, _, _, cx| {
                page.activations += 1;
                cx.notify();
            }));
        let toggles = column()
            .w_full()
            .child(
                div()
                    .w_full()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child("禁用")
                    .child(
                        YueSwitch::new("link-disabled", theme, self.disabled, "禁用链接")
                            .size(SwitchSize::Small)
                            .on_change({
                                let page = cx.entity();
                                move |value, _, _, cx| {
                                    page.update(cx, |page, cx| {
                                        page.disabled = value;
                                        cx.notify();
                                    })
                                }
                            }),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child("常显下划线")
                    .child(
                        YueSwitch::new("link-underline", theme, self.underline, "常显下划线")
                            .size(SwitchSize::Small)
                            .on_change({
                                let page = cx.entity();
                                move |value, _, _, cx| {
                                    page.update(cx, |page, cx| {
                                        page.underline = value;
                                        cx.notify();
                                    })
                                }
                            }),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child("前置图标")
                    .child(
                        YueSwitch::new("link-leading", theme, self.leading, "前置图标")
                            .size(SwitchSize::Small)
                            .on_change({
                                let page = cx.entity();
                                move |value, _, _, cx| {
                                    page.update(cx, |page, cx| {
                                        page.leading = value;
                                        cx.notify();
                                    })
                                }
                            }),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child("后置图标")
                    .child(
                        YueSwitch::new("link-trailing", theme, self.trailing, "后置图标")
                            .size(SwitchSize::Small)
                            .on_change({
                                let page = cx.entity();
                                move |value, _, _, cx| {
                                    page.update(cx, |page, cx| {
                                        page.trailing = value;
                                        cx.notify();
                                    })
                                }
                            }),
                    ),
            );
        let config =
            column()
                .w(gpui_kit::rems(19.))
                .flex_shrink_0()
                .child(div().font_weight(gpui_kit::FontWeight::BOLD).child("配置"))
                .child(toggles)
                .child("配色")
                .child(
                    YueSegmented::new("link-variant", theme, format!("{:?}", self.variant))
                        .label("链接配色")
                        .options(VARIANTS.map(|(label, value)| {
                            YueSegmentedOption::new(format!("{value:?}"), label)
                        }))
                        .on_change(cx.listener(|page, value: &SharedString, _, cx| {
                            page.variant = VARIANTS
                                .iter()
                                .find(|(_, v)| format!("{v:?}") == value.as_ref())
                                .unwrap()
                                .1;
                            cx.notify();
                        })),
                )
                .child("悬停效果")
                .child(
                    YueSegmented::new("link-hover", theme, format!("{:?}", self.hover))
                        .label("链接悬停效果")
                        .options([
                            YueSegmentedOption::new("Underline", "下划线"),
                            YueSegmentedOption::new("Color", "变色"),
                            YueSegmentedOption::new("None", "无"),
                        ])
                        .on_change(cx.listener(|page, value: &SharedString, _, cx| {
                            page.hover = match value.as_ref() {
                                "Color" => LinkHover::Color,
                                "None" => LinkHover::None,
                                _ => LinkHover::Underline,
                            };
                            cx.notify();
                        })),
                )
                .child("尺寸")
                .child(
                    YueSegmented::new("link-size", theme, format!("{:?}", self.size))
                        .label("链接尺寸")
                        .options([
                            YueSegmentedOption::new("Small", "小"),
                            YueSegmentedOption::new("Medium", "中"),
                            YueSegmentedOption::new("Large", "大"),
                        ])
                        .on_change(cx.listener(|page, value: &SharedString, _, cx| {
                            page.size = match value.as_ref() {
                                "Small" => LinkSize::Small,
                                "Large" => LinkSize::Large,
                                _ => LinkSize::Medium,
                            };
                            cx.notify();
                        })),
                );
        column()
            .debug_selector(|| "link-content".into())
            .w_full()
            .p_8()
            .gap_6()
            .child(div().text_2xl().child("Link 链接"))
            .child(readout(
                theme,
                "打开网页或邮件地址；应用命令使用 Button。Tab 聚焦，Enter / Space 激活。".into(),
            ))
            .child(section(
                theme,
                "交互预览",
                "点击使用系统默认应用打开 GPUI Kit 网站；右侧配置立即生效。",
                div()
                    .flex()
                    .w_full()
                    .gap_8()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .min_h(gpui_kit::rems(12.))
                            .child(preview),
                    )
                    .child(config),
            ))
            .child(readout(
                theme,
                format!("系统打开请求：{} 次", self.activations),
            ))
            .child(section(
                theme,
                "基础文字链接",
                "五种语义配色；默认构造器使用主色。",
                matrix("basic", false, false, LinkHover::Underline),
            ))
            .child(section(
                theme,
                "下划线文字链接",
                "下划线只跟随文字，不穿过图标。",
                matrix("underlined", true, false, LinkHover::Underline),
            ))
            .child(section(
                theme,
                "悬停变色",
                "与常显下划线独立；颜色使用主题 interaction 时长连续过渡。",
                column()
                    .child(matrix("color", false, false, LinkHover::Color))
                    .child(matrix("color-line", true, false, LinkHover::Color)),
            ))
            .child(section(
                theme,
                "禁用链接",
                "不打开、不通知、跳过 Tab，保留减弱后的配色。",
                column()
                    .child(matrix("disabled", false, true, LinkHover::Underline))
                    .child(matrix("disabled-line", true, true, LinkHover::Color)),
            ))
            .child(section(
                theme,
                "尺寸与图标",
                "小 / 中 / 大；图标随文字档位变化，文本不缩放。",
                column().children(
                    [
                        ("small", LinkSize::Small),
                        ("medium", LinkSize::Medium),
                        ("large", LinkSize::Large),
                    ]
                    .map(|(id, size)| {
                        row()
                            .child(
                                YueLink::new(
                                    format!("size-{id}"),
                                    format!("{id} 文字链接"),
                                    TARGET,
                                )
                                .size(size),
                            )
                            .child(
                                YueLink::new(format!("leading-{id}"), "前置图标", TARGET)
                                    .size(size)
                                    .leading_icon(outline::Link),
                            )
                            .child(
                                YueLink::new(format!("trailing-{id}"), "外部网站", TARGET)
                                    .size(size)
                                    .trailing_icon(outline::ExternalLink)
                                    .underline(true),
                            )
                    }),
                ),
            ))
            .child(section(
                theme,
                "邮件与宿主接管",
                "第一项交给默认邮件应用；第二项由宿主记录目标，不请求系统打开。",
                column()
                    .child(
                        row()
                            .child(
                                YueLink::new("link-mail", "发送邮件", "mailto:hello@example.com")
                                    .leading_icon(outline::Mail),
                            )
                            .child(
                                YueLink::new("link-custom", "由宿主处理 GPUI Kit 网站", TARGET)
                                    .open_with({
                                        let page = cx.entity();
                                        move |href, _, _, cx| {
                                            page.update(cx, |page, cx| {
                                                page.handled += 1;
                                                page.last_target = href.to_owned().into();
                                                cx.notify();
                                            })
                                        }
                                    }),
                            ),
                    )
                    .child(readout(
                        theme,
                        format!("宿主处理 {} 次 · {}", self.handled, self.last_target),
                    )),
            ))
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::base::test_support::{ElementSnapshot, snapshots};
    use gpui_kit::{Entity, Modifiers, TestAppContext, VisualTestContext, px};

    fn draw(cx: &mut VisualTestContext) {
        cx.update(|w, cx| {
            w.refresh();
            w.draw(cx).clear(cx);
        });
    }
    fn open(cx: &mut TestAppContext) -> (Entity<LinkPage>, &mut VisualTestContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
            cx.set_reduce_motion(true);
        });
        let mut page = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            let view = cx.new(LinkPage::new);
            page = Some(view.clone());
            gpui_kit::base::Root::new(view, w, cx)
        });
        cx.simulate_resize(gpui_kit::size(px(1200.), px(2000.)));
        draw(cx);
        (page.unwrap(), cx)
    }
    fn node(cx: &mut VisualTestContext, id: &str) -> ElementSnapshot {
        cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.path().last() == Some(&gpui_kit::ElementId::from(id.to_owned())))
                .unwrap_or_else(|| panic!("Link showcase node {id}"))
        })
    }
    fn click(cx: &mut VisualTestContext, id: &str) {
        let p = node(cx, id).bounds().center();
        cx.simulate_click(p, Modifiers::default());
        draw(cx);
    }
    fn choose(cx: &mut VisualTestContext, group: &str, label: &str) {
        let p = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| {
                    n.label() == Some(label)
                        && n.path()
                            .contains(&gpui_kit::ElementId::from(group.to_owned()))
                })
                .expect("config option")
                .bounds()
                .center()
        });
        cx.simulate_click(p, Modifiers::default());
        draw(cx);
    }
    #[gpui_kit::test]
    fn configuration_changes_the_real_preview(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        let original = node(cx, "link-preview").bounds();
        click(cx, "link-leading");
        click(cx, "link-trailing");
        assert!(node(cx, "link-preview").bounds().size.width > original.size.width);
        choose(cx, "link-size", "大");
        assert_eq!(node(cx, "link-preview").bounds().size.height, px(24.));
        choose(cx, "link-hover", "变色");
        choose(cx, "link-variant", "危险");
        click(cx, "link-underline");
        click(cx, "link-disabled");
        assert_eq!(node(cx, "link-preview").disabled(), Some(true));
        click(cx, "link-preview");
        assert_eq!(cx.opened_url(), None);
        assert_eq!(
            page.read_with(cx, |p, _| (p.activations, p.hover, p.variant, p.underline)),
            (0, LinkHover::Color, LinkVariant::Danger, true)
        );
        click(cx, "link-disabled");
        click(cx, "link-preview");
        assert_eq!(cx.opened_url().as_deref(), Some(TARGET));
        assert_eq!(page.read_with(cx, |p, _| p.activations), 1);
    }
    #[gpui_kit::test]
    fn host_strategy_records_a_target_without_system_open(cx: &mut TestAppContext) {
        let (page, cx) = open(cx);
        click(cx, "link-custom");
        assert_eq!(
            page.read_with(cx, |p, _| (p.handled, p.last_target.clone())),
            (1, TARGET.into())
        );
        assert_eq!(cx.opened_url(), None);
    }
}
