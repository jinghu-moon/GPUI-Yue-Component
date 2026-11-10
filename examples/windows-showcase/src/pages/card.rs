use super::super::*;
use gpui_kit::base::{SelectableText, TestSupportExt as _, input::InputState};
use gpui_kit::{ElementId, Entity, ObjectFit, StyledImage as _, Task, img, px, rems};
use std::{rc::Rc, time::Duration};
use yue_component::{
    CardCoverPlacement, CardHoverEffect, CardSize, CardVariant, DialogFooter, ListGrid,
    ListGridSizing, ListGridState, SkeletonShape, VirtualizerSource, YueAvatar, YueAvatarGroup,
    YueCard, YueCardCover, YueCardFooter, YueCardHeader, YueCardMeta, YueCollapse, YueCollapseItem,
    YueDialog, YueDialogState, YueEmpty, YueList, YueListItem, YueListItemMeta, YueLoadingRegion,
    YueSkeleton, YueTabItem, YueTabs, YueTag, YueVirtualListGrid,
};

pub(crate) struct CardPage {
    size: CardSize,
    hover: CardHoverEffect,
    narrow: bool,
    truncate: bool,
    selected: bool,
    disabled: bool,
    plan: Option<SharedString>,
    tab: Option<SharedString>,
    expanded: Vec<SharedString>,
    phase: u8,
    refreshing: bool,
    task: Option<Task<()>>,
    message: SharedString,
    editor: Entity<InputState>,
    embedded_editor: Entity<InputState>,
    dialog: Entity<YueDialogState>,
    grid: Entity<ListGridState>,
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::base::{
        Root,
        test_support::{ElementSnapshot, snapshots},
    };
    use gpui_kit::{Modifiers, Role, TestAppContext, VisualTestContext, point};
    fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|w, cx| {
            w.refresh();
            w.draw(cx).clear(cx);
        });
    }
    fn node(cx: &mut VisualTestContext, id: &str) -> ElementSnapshot {
        cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.path().last() == Some(&ElementId::from(id.to_owned())))
                .unwrap_or_else(|| panic!("missing {id}"))
        })
    }
    fn reveal(cx: &mut VisualTestContext, id: &str) {
        let p = node(cx, id).bounds().center();
        let height = cx.update(|w, _| w.viewport_size().height);
        if p.y < px(60.) || p.y > height - px(60.) {
            cx.simulate_event(gpui_kit::ScrollWheelEvent {
                position: point(p.x, height / 2.),
                delta: gpui_kit::ScrollDelta::Pixels(point(px(0.), height / 2. - p.y)),
                touch_phase: gpui_kit::TouchPhase::Moved,
                ..Default::default()
            });
            draw(cx);
        }
    }
    fn click(cx: &mut VisualTestContext, id: &str) {
        reveal(cx, id);
        let p = node(cx, id).bounds().center();
        cx.simulate_click(p, Modifiers::default());
        draw(cx);
    }
    fn open(cx: &mut TestAppContext) -> (Entity<CardPage>, &mut VisualTestContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap()
        });
        let mut saved = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            w.activate_window();
            let view = cx.new(|cx| CardPage::new(w, cx));
            saved = Some(view.clone());
            Root::new(view, w, cx)
        });
        cx.simulate_resize(gpui_kit::size(px(1280.), px(900.)));
        draw(cx);
        (saved.unwrap(), cx)
    }
    #[gpui_kit::test]
    fn page_controls_selection_and_dialog_use_real_components(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);
        click(cx, "card-size");
        assert_eq!(view.read_with(cx, |p, _| p.size), CardSize::Large);
        click(cx, "card-width");
        assert!(view.read_with(cx, |p, _| p.narrow));
        click(cx, "card-width");
        click(cx, "card-truncate");
        assert!(view.read_with(cx, |p, _| p.truncate));
        click(cx, "card-check");
        assert!(view.read_with(cx, |p, _| p.selected));
        assert_eq!(node(cx, "card-check").role(), Some(Role::CheckBox));
        click(cx, "card-independent");
        assert!(view.read_with(cx, |p, _| p.message.contains("独立收藏")));
        click(cx, "card-open");
        cx.executor().advance_clock(Duration::from_millis(500));
        draw(cx);
        assert!(node(cx, "card-details-close").visible());
        click(cx, "card-details-close");
        cx.executor().advance_clock(Duration::from_millis(500));
        draw(cx);
        assert_eq!(node(cx, "card-open").focused(), Some(true));
    }
    #[gpui_kit::test]
    fn first_load_refresh_and_cancel_keep_host_state(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);
        let editor = view.read_with(cx, |p, _| p.editor.clone());
        click(cx, "card-load-note");
        cx.simulate_keystrokes("ctrl-a");
        cx.simulate_input("Custom note 42");
        draw(cx);
        assert_eq!(
            editor.read_with(cx, |s, _| s.value()).as_ref(),
            "Custom note 42"
        );
        click(cx, "card-load");
        assert_eq!(view.read_with(cx, |p, _| p.phase), 1);
        click(cx, "card-empty-button");
        cx.executor().advance_clock(Duration::from_millis(1600));
        draw(cx);
        assert_eq!(
            view.read_with(cx, |p, _| p.phase),
            2,
            "cancelled request must not replace the manual empty state"
        );
        click(cx, "card-error-button");
        click(cx, "card-retry");
        cx.executor().advance_clock(Duration::from_millis(1600));
        draw(cx);
        assert_eq!(view.read_with(cx, |p, _| p.phase), 0);
        assert_eq!(
            view.read_with(cx, |p, _| p.editor.entity_id()),
            editor.entity_id()
        );
        click(cx, "card-refresh");
        assert!(view.read_with(cx, |p, _| p.refreshing));
        assert_eq!(node(cx, "card-refresh").disabled(), Some(true));
        cx.executor().advance_clock(Duration::from_millis(1600));
        draw(cx);
        assert!(!view.read_with(cx, |p, _| p.refreshing));
        assert_eq!(
            editor.read_with(cx, |s, _| s.value()).as_ref(),
            "Custom note 42"
        );
    }
    #[gpui_kit::test]
    fn virtual_collection_builds_a_bounded_window_and_resizes(cx: &mut TestAppContext) {
        let (view, cx) = open(cx);
        reveal(cx, "card-virtual-viewport");
        draw(cx);
        draw(cx);
        let mounted = cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .filter(|n| n.path().last() == Some(&ElementId::from("list-grid-row")))
                .count()
        });
        assert!(
            mounted > 0 && mounted < 100,
            "mounted {mounted} virtual rows for 10,000 cards"
        );
        let before = view.read_with(cx, |p, cx| p.grid.read(cx).geometry().columns());
        click(cx, "card-width");
        reveal(cx, "card-virtual-viewport");
        draw(cx);
        draw(cx);
        let after = view.read_with(cx, |p, cx| p.grid.read(cx).geometry().columns());
        assert!(after <= before);
        assert!(after > 0);
    }
}
struct Orders;
impl VirtualizerSource for Orders {
    fn len(&self) -> usize {
        10_000
    }
    fn key_at(&self, index: usize) -> ElementId {
        format!("order-{index}").into()
    }
    fn index_of(&self, key: &ElementId) -> Option<usize> {
        if let ElementId::Name(key) = key {
            key.strip_prefix("order-")?
                .parse::<usize>()
                .ok()
                .filter(|x| *x < self.len())
        } else {
            None
        }
    }
}
impl CardPage {
    pub(crate) fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            size: CardSize::Medium,
            hover: CardHoverEffect::None,
            narrow: false,
            truncate: false,
            selected: false,
            disabled: false,
            plan: Some("standard".into()),
            tab: Some("one".into()),
            expanded: Vec::new(),
            phase: 0,
            refreshing: false,
            task: None,
            message: "操作结果显示在这里".into(),
            editor: cx.new(|cx| InputState::new(w, cx).default_value("这份备注保留在宿主中")),
            embedded_editor: cx.new(|cx| InputState::new(w, cx).default_value("独立的工单说明")),
            dialog: cx.new(YueDialogState::new),
            grid: cx.new(|cx| {
                ListGridState::new(
                    Rc::new(Orders),
                    ListGrid::min_width(rems(14.), rems(1.)).unwrap(),
                    ListGridSizing::measured(px(190.)),
                    cx,
                )
                .unwrap()
            }),
        }
    }
    fn load(&mut self, refresh: bool, cx: &mut Context<Self>) {
        self.task = None;
        if refresh {
            self.refreshing = true;
        } else {
            self.phase = 1;
        }
        self.task = Some(cx.spawn(async move |weak, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(1200))
                .await;
            let _ = weak.update(cx, |p, cx| {
                p.phase = 0;
                p.refreshing = false;
                p.message = "工单已更新，备注与选择保持不变".into();
                p.task = None;
                cx.notify();
            });
        }));
        cx.notify();
    }
    fn action(
        &self,
        id: impl Into<ElementId>,
        label: &'static str,
        cx: &Context<Self>,
    ) -> YueButton {
        YueButton::new(id, label).on_click(cx.listener(move |p, _, _, cx| {
            p.message = format!("已执行：{label}").into();
            cx.notify();
        }))
    }
    fn more(&self, id: &'static str, theme: YueTheme, cx: &Context<Self>) -> YueDropdownMenu {
        YueDropdownMenu::new(
            id,
            theme,
            YueButton::new((ElementId::from(id), "trigger"), "更多").variant(ButtonVariant::Ghost),
        )
        .items([
            YueMenuItem::new("归档工单").on_click(cx.listener(|p, _, _, cx| {
                p.message = "工单已归档".into();
                cx.notify();
            })),
            YueMenuItem::new("复制工单编号").on_click(cx.listener(|p, _, _, cx| {
                p.message = "已复制 WO-24100".into();
                cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("WO-24100".into()));
                cx.notify();
            })),
        ])
    }
}
fn stack() -> gpui_kit::Div {
    div().min_w_0().w_full().flex().flex_col().gap_4()
}
fn row() -> gpui_kit::Div {
    div().min_w_0().flex().flex_wrap().items_start().gap_4()
}
fn section(
    title: &str,
    description: &str,
    content: impl IntoElement,
    theme: YueTheme,
) -> gpui_kit::Div {
    stack()
        .gap_3()
        .child(
            div()
                .text_lg()
                .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                .child(title.to_owned()),
        )
        .child(
            div()
                .text_sm()
                .text_color(gpui_kit::rgb(theme.text_secondary()))
                .child(description.to_owned()),
        )
        .child(content)
}
fn artwork(theme: YueTheme) -> impl IntoElement + gpui_kit::Styled {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(gpui_kit::rgb(theme.surface_component()))
        .child(
            yue_component::Icon::new(outline::Settings)
                .size(rems(4.))
                .color(theme.text_secondary()),
        )
}
fn work_order(id: impl Into<ElementId>, title: String, theme: YueTheme, size: CardSize) -> YueCard {
    YueCard::new(id)
        .theme(theme)
        .size(size)
        .header(YueCardHeader::new(title).description("大圈线 · 今日排产"))
        .child("内径 25 · 外径 52 · 宽 15 mm，防尘盖密封，适用于中小型电机。")
        .footer(YueCardFooter::new(
            YueTag::new("state", "待排产").theme(theme),
        ))
}
impl Render for CardPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let size = self.size;
        let controls = row()
            .child(
                YueButton::new("card-size", format!("尺寸：{size:?}")).on_click(cx.listener(
                    |p, _, _, cx| {
                        p.size = match p.size {
                            CardSize::Small => CardSize::Medium,
                            CardSize::Medium => CardSize::Large,
                            CardSize::Large => CardSize::Small,
                        };
                        cx.notify();
                    },
                )),
            )
            .child(
                YueButton::new("card-hover", format!("悬停：{:?}", self.hover)).on_click(
                    cx.listener(|p, _, _, cx| {
                        p.hover = match p.hover {
                            CardHoverEffect::None => CardHoverEffect::Outline,
                            CardHoverEffect::Outline => CardHoverEffect::Lift,
                            CardHoverEffect::Lift => CardHoverEffect::None,
                        };
                        cx.notify();
                    }),
                ),
            )
            .child(
                YueButton::new(
                    "card-width",
                    if self.narrow {
                        "展开容器"
                    } else {
                        "收窄容器"
                    },
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.narrow = !p.narrow;
                    cx.notify();
                })),
            )
            .child(
                YueButton::new(
                    "card-truncate",
                    if self.truncate {
                        "标题自然换行"
                    } else {
                        "标题单行省略"
                    },
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.truncate = !p.truncate;
                    cx.notify();
                })),
            );
        let basic = row()
            .child(
                div().w(rems(21.)).max_w_full().child(
                    YueCard::new("card-basic")
                        .size(size)
                        .header(
                            YueCardHeader::new("6205-ZZ 深沟球轴承")
                                .description("今日生产工单")
                                .extra(self.more("card-basic-menu", theme, cx)),
                        )
                        .child(
                            "相关信息组织为一个单元；正文可以是文字、列表、表单或其他已有组件。",
                        ),
                ),
            )
            .child(
                div().w(rems(18.)).max_w_full().child(
                    YueCard::new("card-body-only")
                        .size(size)
                        .child("只有正文时保留完整内边距，没有头部或页脚空位。"),
                ),
            )
            .child(
                div().w(rems(18.)).max_w_full().child(
                    YueCard::new("card-header-only").size(size).header(
                        YueCardHeader::new("仅头部")
                            .leading(YueAvatar::new("card-leading", theme).name("王"))
                            .extra(self.action("card-follow", "关注", cx)),
                    ),
                ),
            );
        let variants = row().children(
            [
                CardVariant::Outlined,
                CardVariant::Elevated,
                CardVariant::Filled,
                CardVariant::Borderless,
            ]
            .into_iter()
            .enumerate()
            .map(|(i, variant)| {
                div().w(rems(16.)).max_w_full().child(
                    YueCard::new(format!("variant-{i}"))
                        .variant(variant)
                        .size(size)
                        .hover_effect(self.hover)
                        .header(YueCardHeader::new(format!("{variant:?}")))
                        .child("悬停与点击是独立配置。此处只预览表面和密度。"),
                )
            }),
        );
        let covers = row().children(
            [
                CardCoverPlacement::BeforeHeader,
                CardCoverPlacement::AfterHeader,
                CardCoverPlacement::Left,
                CardCoverPlacement::Right,
            ]
            .into_iter()
            .enumerate()
            .map(|(i, position)| {
                div()
                    .w(rems(if i < 2 { 19. } else { 26. }))
                    .max_w_full()
                    .child(
                        YueCard::new(format!("cover-{i}"))
                            .size(size)
                            .header(
                                YueCardHeader::new(format!("{position:?}"))
                                    .description("封面独立裁剪"),
                            )
                            .cover(YueCardCover::new(artwork(theme)).placement(position))
                            .child("图像、SVG 或其他媒体元素都可以作为封面。")
                            .footer(YueCardFooter::new(self.action(
                                format!("cover-details-{i}"),
                                "查看参数",
                                cx,
                            ))),
                    )
            }),
        );
        // GPUI Img fallbacks draw their own root; give it the same inset corners.
        let cover_radius = rems(
            (theme.token("--card-radius").unwrap().pixels().unwrap()
                - theme
                    .token("--card-border-width")
                    .unwrap()
                    .pixels()
                    .unwrap())
            .max(0.)
                / 16.,
        );
        let media_failure = YueCard::new("card-media-failure")
            .size(size)
            .width(rems(19.))
            .cover(YueCardCover::new(
                img("assets/card-missing.png")
                    .size_full()
                    .object_fit(ObjectFit::Cover)
                    .with_fallback(move || {
                        div()
                            .size_full()
                            .rounded_t(cover_radius)
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(gpui_kit::rgb(theme.surface_component()))
                            .child("封面暂不可用")
                            .into_any_element()
                    }),
            ))
            .header(YueCardHeader::new("图片失败回退"))
            .child("媒体框比例保持不变。");
        let meta = YueCard::new("card-meta-example")
            .size(size)
            .width(rems(24.))
            .header(
                YueCardHeader::new("生产任务与质量检测的完整协作记录，需要在窄宽度下保持可读")
                    .description("截断只作用于标题，描述按内容自然换行。")
                    .truncate_title(self.truncate)
                    .leading(YueAvatar::new("card-meta-avatar", theme).name("林")),
            )
            .child(
                YueCardMeta::new("card-meta-person", "林静")
                    .size(size)
                    .description("质量检验组长 · 负责异常工单复核")
                    .leading(YueAvatar::new("card-meta-body-avatar", theme).name("林")),
            )
            .footer(YueCardFooter::new(
                YueAvatarGroup::new("card-team", theme).children([
                    YueAvatar::new("card-team-a", theme).name("王"),
                    YueAvatar::new("card-team-b", theme).name("李"),
                    YueAvatar::new("card-team-c", theme).name("林"),
                ]),
            ));
        let footers = row()
            .child(
                div().w(rems(22.)).max_w_full().child(
                    YueCard::new("card-footer-actions")
                        .size(size)
                        .header(YueCardHeader::new("等分操作区"))
                        .child("按钮独立进入 Tab 顺序，各自执行自己的命令。")
                        .footer(YueCardFooter::actions([
                            self.action("card-star", "收藏", cx),
                            self.action("card-share", "分享", cx),
                            self.action("card-archive", "归档", cx),
                        ])),
                ),
            )
            .child(div().w(rems(22.)).max_w_full().child(meta));
        let primary = YueCard::new("card-primary")
            .size(size)
            .width(rems(24.))
            .header(YueCardHeader::new("整卡主操作").extra(self.more(
                "card-primary-menu",
                theme,
                cx,
            )))
            .child(SelectableText::new(
                "card-selectable",
                "可拖选这段工单说明；单击静态区域打开详情，拖选不会打开。",
            ))
            .primary_action(
                YueButton::new("card-open", "工单详情…")
                    .disabled(self.disabled)
                    .on_click(cx.listener(|p, _, w, cx| {
                        p.message = "已打开工单详情".into();
                        p.dialog.update(cx, |s, cx| {
                            let _ = s.open(w, cx);
                        });
                        cx.notify();
                    })),
            )
            .footer(YueCardFooter::new(self.action(
                "card-independent",
                "独立收藏",
                cx,
            )));
        let check_update = cx.listener(|p, value: &CheckboxState, _, cx| {
            p.selected = *value == CheckboxState::Checked;
            cx.notify();
        });
        let plan_update = cx.listener(|p, value: &SharedString, _, cx| {
            p.plan = Some(value.clone());
            cx.notify();
        });
        let choices = row()
            .child(primary)
            .child(
                div().w(rems(21.)).max_w_full().child(
                    YueCard::new("card-choice")
                        .size(size)
                        .selected(self.selected)
                        .header(
                            YueCardHeader::new("重点工单").leading(
                                YueCheckbox::new(
                                    "card-check",
                                    theme,
                                    if self.selected {
                                        CheckboxState::Checked
                                    } else {
                                        CheckboxState::Unchecked
                                    },
                                )
                                .label("选择工单")
                                .disabled(self.disabled)
                                .on_change(move |value, _, w, cx| check_update(&value, w, cx)),
                            ),
                        )
                        .child("Checkbox 与卡片边界读取同一份宿主值。"),
                ),
            )
            .child(
                div().w(rems(21.)).max_w_full().child(
                    YueCard::new("card-plan")
                        .size(size)
                        .header(YueCardHeader::new("生产方案"))
                        .child(
                            YueRadioGroup::new("card-plans", theme, self.plan.clone())
                                .label("生产方案")
                                .options([
                                    YueRadioOption::new("standard", "标准生产"),
                                    YueRadioOption::new("priority", "优先排产"),
                                ])
                                .on_change(move |value, w, cx| plan_update(&value, w, cx)),
                        ),
                ),
            );
        let tabs = YueTabs::new("card-tabs", self.tab.clone())
            .items([
                YueTabItem::new("one", "大圈线").content(|_, _| {
                    div()
                        .p_4()
                        .child("6205-ZZ · 产量 1,440 套")
                        .into_any_element()
                }),
                YueTabItem::new("two", "小圈线")
                    .content(|_, _| div().p_4().child("6310 · 产量 1,160 套").into_any_element()),
            ])
            .on_change(
                cx.listener(|p, request: &yue_component::TabsChangeRequest, _, cx| {
                    p.tab = Some(request.key().clone());
                    cx.notify();
                }),
            );
        let collapse = YueCollapse::new("card-collapse", self.expanded.clone())
            .items([
                YueCollapseItem::new("inspection", "质检记录").content(|_, _| {
                    div()
                        .child("最近一次检验合格，抽样 32 件。")
                        .into_any_element()
                }),
            ])
            .on_change(
                cx.listener(|p, request: &yue_component::CollapseChangeRequest, _, cx| {
                    p.expanded = request.expanded_keys().to_vec();
                    cx.notify();
                }),
            );
        let composition = row()
            .child(
                div().w(rems(26.)).max_w_full().child(
                    YueCard::new("card-tabbed")
                        .size(size)
                        .header(YueCardHeader::new("产线产量"))
                        .body_inset(false)
                        .child(tabs),
                ),
            )
            .child(
                div().w(rems(26.)).max_w_full().child(
                    YueCard::new("card-composed")
                        .size(size)
                        .header(YueCardHeader::new("工单备注与质检"))
                        .child(YueInput::new("card-note", theme, &self.embedded_editor))
                        .child(collapse),
                ),
            );
        let mut loaded = work_order("card-loaded", "6205-ZZ".into(), theme, size);
        loaded = loaded
            .child(YueInput::new("card-load-note", theme, &self.editor))
            .footer(YueCardFooter::new(
                YueButton::new("card-refresh", "刷新工单")
                    .loading(self.refreshing)
                    .on_click(cx.listener(|p, _, _, cx| p.load(true, cx))),
            ));
        let load_view = match self.phase {
            2 => YueCard::new("card-empty")
                .size(size)
                .header(YueCardHeader::new("工单"))
                .child(YueEmpty::new("card-no-orders").description("当前没有待处理工单"))
                .into_any_element(),
            3 => YueCard::new("card-error")
                .size(size)
                .header(YueCardHeader::new("工单暂时无法加载"))
                .child("请重试以重新取得工单。")
                .footer(YueCardFooter::new(
                    YueButton::new("card-retry", "重新加载")
                        .on_click(cx.listener(|p, _, _, cx| p.load(false, cx))),
                ))
                .into_any_element(),
            1 => YueLoadingRegion::new("card-loading", true)
                .placeholder(move |_, _| {
                    YueCard::new("card-placeholder")
                        .size(size)
                        .header(YueCardHeader::custom(YueSkeleton::new("card-sk-title")))
                        .child(
                            YueSkeleton::new("card-sk-body")
                                .shape(SkeletonShape::Rectangle)
                                .height(rems(5.)),
                        )
                        .footer(YueCardFooter::new(YueSkeleton::new("card-sk-footer")))
                        .into_any_element()
                })
                .into_any_element(),
            _ => loaded.into_any_element(),
        };
        // Retained business entities live in CardPage. The visible branch is replaced only on first-load demonstrations.
        let loading =
            stack()
                .child(
                    row()
                        .child(
                            YueButton::new("card-load", "模拟首次加载")
                                .on_click(cx.listener(|p, _, _, cx| p.load(false, cx))),
                        )
                        .child(
                            YueButton::new("card-empty-button", "空数据").on_click(cx.listener(
                                |p, _, _, cx| {
                                    p.task = None;
                                    p.phase = 2;
                                    p.refreshing = false;
                                    cx.notify();
                                },
                            )),
                        )
                        .child(YueButton::new("card-error-button", "请求失败").on_click(
                            cx.listener(|p, _, _, cx| {
                                p.task = None;
                                p.phase = 3;
                                p.refreshing = false;
                                cx.notify();
                            }),
                        )),
                )
                .child(div().w(rems(28.)).max_w_full().child(load_view));
        let grid = YueList::new("card-grid")
            .grid(ListGrid::min_width(rems(14.), rems(1.)).unwrap())
            .children((0..5).map(|i| {
                YueListItem::new(format!("card-grid-item-{i}"))
                    .inset(false)
                    .content(
                        work_order(
                            format!("card-grid-{i}"),
                            format!("工单 WO-{}", 24100 + i),
                            theme,
                            size,
                        )
                        .child(if i == 1 {
                            "这张工单具有更长的说明，用于检查同一行的表面高度与底部操作是否对齐。"
                        } else {
                            "按期交付。"
                        }),
                    )
            }));
        let virtual_cards = div()
            .id("card-virtual-viewport")
            .test_support()
            .h(rems(20.))
            .w_full()
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .child(
                YueVirtualListGrid::new("card-virtual-grid", &self.grid, move |i, key, _, cx| {
                    YueListItem::new(key.clone())
                        .inset(false)
                        .content(work_order(
                            (key.clone(), "card"),
                            format!("工单 {}", i + 1),
                            yue_component::current_theme(cx),
                            size,
                        ))
                })
                .accessibility_label("一万张工单卡片"),
            );
        let board = row()
            .child(
                div().w(rems(15.)).max_w_full().child(
                    YueCard::new("card-board-stat")
                        .size(size)
                        .header(YueCardHeader::new("今日产量"))
                        .child(div().text_3xl().child("12,480"))
                        .child("较昨日增长 5.2%"),
                ),
            )
            .child(
                div().w(rems(24.)).max_w_full().child(
                    YueCard::new("card-board-exceptions")
                        .size(size)
                        .header(YueCardHeader::new("待处理异常"))
                        .body_inset(false)
                        .child(
                            YueList::new("card-exceptions")
                                .child(
                                    YueListItem::new("machine").content(
                                        YueListItemMeta::new("3 号机台温度偏高")
                                            .description("设备组 · 10 分钟前"),
                                    ),
                                )
                                .child(
                                    YueListItem::new("material").content(
                                        YueListItemMeta::new("6310 原料待补充")
                                            .description("仓储组 · 20 分钟前"),
                                    ),
                                ),
                        ),
                ),
            );
        let dialog =
            YueDialog::new("card-details", theme, &self.dialog)
                .title("工单 WO-24100")
                .description("6205-ZZ 深沟球轴承")
                .child("计划数量 1,440 套 · 大圈线 · 今日排产")
                .footer(DialogFooter::None)
                .footer_content(YueButton::new("card-details-close", "关闭").on_click(
                    cx.listener(|p, _, w, cx| {
                        p.dialog.update(cx, |s, cx| {
                            s.close(w, cx);
                        });
                    }),
                ));
        div()
            .id("card-content")
            .debug_selector(|| "card-content".into())
            .h_full()
            .w_full()
            .min_w_0()
            .flex_1()
            .min_h_0()
            .child(
                stack()
                    .p_6()
                    .child(div().text_2xl().child("Card 卡片"))
                    .child("把相关内容组织成一个单元，复用已有控件完成命令、选择、加载与布局。")
                    .child(controls)
                    .child(div().text_sm().child(self.message.clone()))
                    .child(
                        div()
                            .w(if self.narrow { rems(26.) } else { rems(68.) })
                            .max_w_full()
                            .child(
                                stack()
                                    .child(section(
                                        "基础与简洁",
                                        "区域按需出现；头部、正文和页脚都可独立省略。",
                                        basic,
                                        theme,
                                    ))
                                    .child(section(
                                        "四种表面",
                                        "默认描边；轻量阴影、填充和无边框按场景选择。",
                                        div().p_4().bg(gpui_kit::rgb(theme.page())).child(variants),
                                        theme,
                                    ))
                                    .child(section(
                                        "封面与布局",
                                        "封面位置独立配置；窄侧封面请使用上方顶部布局。",
                                        stack().child(covers).child(media_failure),
                                        theme,
                                    ))
                                    .child(section(
                                        "信息与操作区",
                                        "头像、描述、头像组和按钮均复用已有组件。",
                                        footers,
                                        theme,
                                    ))
                                    .child(section(
                                        "命令与选择",
                                        "详情可从静态区域打开；选择使用真实控件，互不串联。",
                                        stack()
                                            .child(
                                                YueButton::new(
                                                    "card-disable",
                                                    if self.disabled {
                                                        "启用工单操作"
                                                    } else {
                                                        "禁用工单操作"
                                                    },
                                                )
                                                .on_click(cx.listener(|p, _, _, cx| {
                                                    p.disabled = !p.disabled;
                                                    cx.notify();
                                                })),
                                            )
                                            .child(choices),
                                        theme,
                                    ))
                                    .child(section(
                                        "内容组合",
                                        "Tabs、Input 与 Collapse 继续拥有自己的输入、焦点和状态。",
                                        composition,
                                        theme,
                                    ))
                                    .child(section(
                                        "加载与数据状态",
                                        "首次加载替换分支；刷新保留内容，备注由宿主保留。",
                                        loading,
                                        theme,
                                    ))
                                    .child(section(
                                        "响应式网格",
                                        "调整容器宽度，检查列数、同行等高和页脚贴底。",
                                        grid,
                                        theme,
                                    ))
                                    .child(section(
                                        "虚拟卡片集合",
                                        "一万张工单复用已有虚拟网格，只构建可见条目。",
                                        virtual_cards,
                                        theme,
                                    ))
                                    .child(section(
                                        "生产看板组合",
                                        "统计与异常列表通过同一套 Card 区域组合。",
                                        board,
                                        theme,
                                    )),
                            ),
                    )
                    .child(dialog),
            )
            .overflow_y_scrollbar()
    }
}
