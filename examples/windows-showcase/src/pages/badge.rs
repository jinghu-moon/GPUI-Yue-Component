use super::super::*;
use gpui_kit::{point, rems};
use yue_component::{BadgeShape, BadgeSize, BadgeTone, YueAvatar, YueBadge, YueList, YueListItem};

pub(crate) struct BadgePage {
    count: u64,
    shown: bool,
    zero: bool,
    dot: bool,
    animated: bool,
    clicks: usize,
}

impl BadgePage {
    pub(crate) fn new() -> Self {
        Self {
            count: 8,
            shown: true,
            zero: false,
            dot: false,
            animated: false,
            clicks: 0,
        }
    }
}

fn row() -> gpui_kit::Div {
    div().flex().items_center().flex_wrap().gap_8().py_4()
}
fn column() -> gpui_kit::Div {
    div().flex().flex_col().items_start().gap_4()
}
fn avatar(id: &'static str, theme: YueTheme) -> YueAvatar {
    YueAvatar::new(id, theme).name("Yue")
}

impl Render for BadgePage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let mut interactive = YueBadge::new("badge-interactive")
            .count(self.count)
            .show_zero(self.zero)
            .visible(self.shown)
            .animated(self.animated)
            .semantics(false);
        if self.dot {
            interactive = interactive.dot();
        }
        let description = if !self.shown || (!self.dot && self.count == 0 && !self.zero) {
            "没有新提醒".to_owned()
        } else if self.dot {
            "有新内容".to_owned()
        } else {
            format!("{} 条未读消息", self.count)
        };
        div()
            .debug_selector(|| "badge-content".into())
            .w_full()
            .flex()
            .flex_col()
            .gap_8()
            .p_8()
            .child(div().text_2xl().child("Badge 徽标"))
            .child(section(
                theme,
                "基础角标",
                "徽标附着在头像、图标或按钮右上角，保持宿主布局尺寸",
                row()
                    .child(
                        YueBadge::new("badge-avatar-count")
                            .count(9)
                            .child(avatar("badge-avatar-a", theme)),
                    )
                    .child(
                        YueBadge::new("badge-avatar-dot")
                            .dot()
                            .child(avatar("badge-avatar-b", theme)),
                    )
                    .child(
                        YueBadge::new("badge-icon-dot")
                            .dot()
                            .accessibility_label("有未读通知")
                            .child(
                                yue_component::Icon::new(yue_component::outline::Bell)
                                    .size(rems(1.5)),
                            ),
                    )
                    .child(
                        YueBadge::new("badge-static-button")
                            .count(128)
                            .semantics(false)
                            .child(
                                YueButton::new("badge-notification-button", "通知")
                                    .accessibility_description("128 条未读消息")
                                    .on_click(cx.listener(|page, _, _, cx| {
                                        page.clicks += 1;
                                        cx.notify();
                                    })),
                            ),
                    ),
            ))
            .child(section(
                theme,
                "独立使用",
                "数量独立参与布局；零值可显式显示，超过上限保留真实数量语义",
                row().children([
                    YueBadge::new("badge-zero").count(0).show_zero(true),
                    YueBadge::new("badge-two").count(2),
                    YueBadge::new("badge-sixteen")
                        .count(16)
                        .tone(BadgeTone::Default),
                    YueBadge::new("badge-overflow")
                        .count(128)
                        .accessibility_label("128 条未读消息"),
                ]),
            ))
            .child(section(
                theme,
                "短文本与最大值",
                "短文本可显示 NEW、HOT 或中文；数量上限可设为 9、99、999",
                column()
                    .child(
                        row().children([
                            YueBadge::new("badge-new")
                                .text("NEW")
                                .child(avatar("badge-new-host", theme)),
                            YueBadge::new("badge-hot")
                                .text("HOT")
                                .tone(BadgeTone::Warning)
                                .child(avatar("badge-hot-host", theme)),
                            YueBadge::new("badge-cn")
                                .text("新")
                                .tone(BadgeTone::Primary)
                                .child(avatar("badge-cn-host", theme)),
                            YueBadge::new("badge-long").text("一段需要省略的完整提示文本"),
                        ]),
                    )
                    .child(row().children([
                        YueBadge::new("badge-max9").count(10).max_count(9),
                        YueBadge::new("badge-max99").count(100).max_count(99),
                        YueBadge::new("badge-max999").count(1000).max_count(999),
                    ])),
            ))
            .child(section(
                theme,
                "状态点与语义色",
                "状态点附带可读说明，主色随主题变化；真实进度请使用 Spinner 或 Progress",
                column()
                    .child(
                        row().children([
                            YueBadge::new("badge-status-default")
                                .dot()
                                .tone(BadgeTone::Default)
                                .label("未启动"),
                            YueBadge::new("badge-status-processing")
                                .dot()
                                .tone(BadgeTone::Primary)
                                .label("处理中"),
                            YueBadge::new("badge-status-success")
                                .dot()
                                .tone(BadgeTone::Success)
                                .label("已同步"),
                            YueBadge::new("badge-status-warning")
                                .dot()
                                .tone(BadgeTone::Warning)
                                .label("需要注意"),
                            YueBadge::new("badge-status-error")
                                .dot()
                                .tone(BadgeTone::Danger)
                                .label("同步失败"),
                        ]),
                    )
                    .child(
                        row().children([
                            YueBadge::new("badge-tone-default")
                                .count(8)
                                .tone(BadgeTone::Default),
                            YueBadge::new("badge-tone-primary")
                                .count(8)
                                .tone(BadgeTone::Primary),
                            YueBadge::new("badge-tone-success")
                                .count(8)
                                .tone(BadgeTone::Success),
                            YueBadge::new("badge-tone-warning")
                                .count(8)
                                .tone(BadgeTone::Warning),
                            YueBadge::new("badge-tone-danger")
                                .count(8)
                                .tone(BadgeTone::Danger),
                            YueBadge::new("badge-custom-colors").text("自定义").colors(
                                gpui_kit::rgb(theme.accent_solid()),
                                gpui_kit::rgb(theme.on_accent()),
                            ),
                        ]),
                    ),
            ))
            .child(section(
                theme,
                "尺寸、外形与偏移",
                "标准与小号；圆形/胶囊与圆角方形；偏移以 rem 表达",
                row().children([
                    YueBadge::new("badge-size-medium")
                        .count(8)
                        .child(avatar("badge-size-host-a", theme)),
                    YueBadge::new("badge-size-small")
                        .count(8)
                        .size(BadgeSize::Small)
                        .child(avatar("badge-size-host-b", theme)),
                    YueBadge::new("badge-shape-rounded")
                        .count(99)
                        .shape(BadgeShape::Rounded)
                        .child(avatar("badge-shape-host", theme)),
                    YueBadge::new("badge-offset")
                        .dot()
                        .offset(point(rems(-0.25), rems(0.25)))
                        .child(avatar("badge-offset-host", theme)),
                ]),
            ))
            .child(section(
                theme,
                "列表数量",
                "列表行尾复用独立徽标，业务数据由列表宿主管理",
                div().w_full().child(
                    YueList::new("badge-list")
                        .bordered(true)
                        .child(
                            YueListItem::new("badge-inbox-row").content("收件箱").extra(
                                YueBadge::new("badge-inbox")
                                    .count(128)
                                    .tone(BadgeTone::Default),
                            ),
                        )
                        .child(
                            YueListItem::new("badge-drafts-row").content("草稿").extra(
                                YueBadge::new("badge-drafts")
                                    .count(2)
                                    .tone(BadgeTone::Default),
                            ),
                        ),
                ),
            ))
            .child(section(
                theme,
                "交互与可选过渡",
                "增减数量、隐藏或切换圆点，通知按钮保持可点击；界面动效开关支持减少动态效果",
                column()
                    .child(
                        row()
                            .child(
                                interactive.child(
                                    YueButton::new("badge-interactive-host", "通知")
                                        .accessibility_description(description)
                                        .on_click(cx.listener(|page, _, _, cx| {
                                            page.clicks += 1;
                                            cx.notify();
                                        })),
                                ),
                            )
                            .child(format!("数量 {} · 点击次数 {}", self.count, self.clicks)),
                    )
                    .child(
                        row()
                            .child(YueButton::new("badge-minus", "减少").on_click(cx.listener(
                                |p, _, _, cx| {
                                    p.count = p.count.saturating_sub(1);
                                    cx.notify();
                                },
                            )))
                            .child(YueButton::new("badge-plus", "增加").on_click(cx.listener(
                                |p, _, _, cx| {
                                    p.count = p.count.saturating_add(1);
                                    cx.notify();
                                },
                            )))
                            .child(YueButton::new("badge-over-limit", "设为 128").on_click(
                                cx.listener(|p, _, _, cx| {
                                    p.count = 128;
                                    cx.notify();
                                }),
                            ))
                            .child(YueButton::new("badge-reset", "清零").on_click(cx.listener(
                                |p, _, _, cx| {
                                    p.count = 0;
                                    cx.notify();
                                },
                            ))),
                    )
                    .child(
                        row()
                            .child(
                                YueSwitch::new("badge-shown", theme, self.shown, "显示徽标")
                                    .on_change({
                                        let entity = cx.entity();
                                        move |value, _, _, cx| {
                                            entity.update(cx, |p, cx| {
                                                p.shown = value;
                                                cx.notify();
                                            });
                                        }
                                    }),
                            )
                            .child(
                                YueSwitch::new("badge-show-zero", theme, self.zero, "显示零值")
                                    .on_change({
                                        let entity = cx.entity();
                                        move |value, _, _, cx| {
                                            entity.update(cx, |p, cx| {
                                                p.zero = value;
                                                cx.notify();
                                            });
                                        }
                                    }),
                            )
                            .child(
                                YueSwitch::new("badge-dot-mode", theme, self.dot, "圆点模式")
                                    .on_change({
                                        let entity = cx.entity();
                                        move |value, _, _, cx| {
                                            entity.update(cx, |p, cx| {
                                                p.dot = value;
                                                cx.notify();
                                            });
                                        }
                                    }),
                            )
                            .child(
                                YueSwitch::new(
                                    "badge-animation",
                                    theme,
                                    self.animated,
                                    "显示隐藏过渡",
                                )
                                .on_change({
                                    let entity = cx.entity();
                                    move |value, _, _, cx| {
                                        entity.update(cx, |p, cx| {
                                            p.animated = value;
                                            cx.notify();
                                        });
                                    }
                                }),
                            ),
                    ),
            ))
    }
}
