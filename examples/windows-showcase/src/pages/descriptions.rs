use super::super::*;
use gpui_kit::{Entity, FocusHandle, Focusable, base::input::InputState, rems};
use yue_component::{
    AvatarSize, BadgeTone, DescriptionsAlign as Align, DescriptionsLayout as Layout,
    DescriptionsSize as Size, DescriptionsSpan as Span, DescriptionsTextOverflow as Overflow,
    DescriptionsVerticalAlign as VAlign, ProgressValue, SelectMode, SelectOption, SelectValue,
    YueAvatar, YueBadge, YueCard, YueCardHeader, YueDescriptionItem as Item,
    YueDescriptions as Details, YueLink, YueList, YueListItem, YueListItemMeta, YueProgress,
    YueResizable, YueResizablePanel, YueResizableState, YueSelect, YueSelectState, YueTag,
    YueTagGroup,
};

const NOTES: &str = "设备运行时出现间歇性振动，请检查连接位置与轴承温升。完成后保留测量记录，并与负责人确认后续安排。";
pub(crate) struct DescriptionsPage {
    columns: u16,
    layout: Layout,
    size: Size,
    bordered: bool,
    colon: bool,
    label_align: Align,
    value_align: Align,
    vertical: VAlign,
    wide_label: bool,
    lines: usize,
    narrow: bool,
    split: Entity<YueResizableState>,
    selected: usize,
    names: [SharedString; 3],
    statuses: [SharedString; 3],
    editor: Entity<InputState>,
    status: Entity<YueSelectState>,
    form: Entity<YueForm>,
    editing: bool,
    edit_focus: FocusHandle,
    message: SharedString,
}

impl DescriptionsPage {
    pub(crate) fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| InputState::new(w, cx).default_value("检查主轴温升"));
        let status = cx.new(|cx| {
            let mut s = YueSelectState::new(SelectMode::Single, w, cx);
            s.set_options(
                [
                    SelectOption::new("进行中", "进行中"),
                    SelectOption::new("待确认", "待确认"),
                    SelectOption::new("已完成", "已完成"),
                ],
                cx,
            )
            .unwrap();
            s.set_value(SelectValue::Single(Some("进行中".into())), w, cx)
                .unwrap();
            s
        });
        let form = cx.new(|cx| {
            let mut f = YueForm::new();
            f.require("description-name", &editor, "请填写工单名称");
            f.attach(&editor, w, cx);
            f
        });
        Self {
            columns: 3,
            layout: Layout::Horizontal,
            size: Size::Medium,
            bordered: true,
            colon: false,
            label_align: Align::Left,
            value_align: Align::Left,
            vertical: VAlign::Center,
            wide_label: false,
            lines: 3,
            narrow: false,
            split: cx.new(|_| YueResizableState::default()),
            selected: 0,
            names: [
                "检查主轴温升".into(),
                "校准传感器".into(),
                "更换冷却管".into(),
            ],
            statuses: ["进行中".into(), "待确认".into(), "已完成".into()],
            editor,
            status,
            form,
            editing: false,
            edit_focus: cx.focus_handle(),
            message: "选择工单查看详情".into(),
        }
    }
    fn start_edit(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.form.update(cx, |f, cx| f.reset(cx));
        let name = self.names[self.selected].clone();
        self.editor.update(cx, |s, cx| s.set_value(name, w, cx));
        let status = self.statuses[self.selected].clone();
        self.status.update(cx, |s, cx| {
            s.set_value(SelectValue::Single(Some(status)), w, cx)
                .unwrap();
        });
        self.editing = true;
        self.message = "修改后保存，取消会保留原记录".into();
        let focus = self.editor.read(cx).focus_handle(cx);
        w.defer(cx, move |w, cx| focus.focus(w, cx));
        cx.notify();
    }
    fn save(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if !self.form.update(cx, |f, cx| f.submit(w, cx)) {
            self.message = "请先修正必填项".into();
            cx.notify();
            return;
        }
        self.names[self.selected] = self.editor.read(cx).value();
        if let SelectValue::Single(Some(value)) = self.status.read(cx).value() {
            self.statuses[self.selected] = value.clone();
        }
        self.finish("已保存", w, cx);
    }
    fn finish(&mut self, message: &str, w: &mut Window, cx: &mut Context<Self>) {
        self.form.update(cx, |f, cx| f.reset(cx));
        self.editing = false;
        self.message = message.to_owned().into();
        let focus = self.edit_focus.clone();
        w.defer(cx, move |w, cx| focus.focus(w, cx));
        cx.notify();
    }
    fn details(&self, id: &'static str) -> Details {
        Details::new(id)
            .columns(self.columns)
            .layout(self.layout)
            .size(self.size)
            .bordered(self.bordered)
            .colon(self.colon)
            .label_align(self.label_align)
            .value_align(self.value_align)
            .vertical_align(self.vertical)
            .label_width(rems(if self.wide_label { 9. } else { 7. }))
    }
}
fn row() -> gpui_kit::Div {
    div().min_w_0().flex().flex_wrap().items_center().gap_3()
}
fn section(id: &'static str, title: &str, body: impl IntoElement) -> YueCard {
    YueCard::new(id)
        .header(YueCardHeader::new(title.to_owned()))
        .child(body)
}
fn basic_items(theme: YueTheme) -> Vec<Item> {
    vec![
        Item::new("number", "工单编号")
            .text("WO-20261110-0042")
            .copyable(true),
        Item::new("status", "状态").content(
            YueBadge::new("status")
                .text("进行中")
                .tone(BadgeTone::Primary),
        ),
        Item::new("owner", "负责人").content(
            row()
                .child(
                    YueAvatar::new("owner-avatar", theme)
                        .name("林悦")
                        .size(AvatarSize::Small),
                )
                .child("林悦"),
        ),
        Item::new("location", "设备位置").text("二号车间 · A-06"),
        Item::new("created", "创建时间").text("2026-11-10 15:40"),
        Item::new("note", "处理说明").text(NOTES).span(Span::Filled),
    ]
}
fn span_items() -> [Item; 6] {
    [
        Item::new("a", "A · 一列").text("普通属性"),
        Item::new("b", "B · 一列").text("有边框时补齐行尾"),
        Item::new("c", "C · 两列")
            .text("当前行不足时换行")
            .span(Span::Columns(2)),
        Item::new("d", "D · Filled")
            .text("填满当前行剩余位置")
            .span(Span::Filled),
        Item::new("e", "E · 99 列")
            .text("按当前列数收缩，配置保留")
            .span(Span::Columns(99)),
        Item::new("f", "F").text("最后一项"),
    ]
}
fn cycle(a: Align) -> Align {
    match a {
        Align::Left => Align::Center,
        Align::Center => Align::Right,
        Align::Right => Align::Left,
    }
}
impl Render for DescriptionsPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let controls = row()
            .child(
                YueButton::new("descriptions-columns", format!("最多 {} 列", self.columns))
                    .on_click(cx.listener(|p, _, _, cx| {
                        p.columns = p.columns % 4 + 1;
                        cx.notify();
                    })),
            )
            .child(
                YueButton::new(
                    "descriptions-layout",
                    if self.layout == Layout::Horizontal {
                        "标签在左"
                    } else {
                        "标签在上"
                    },
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.layout = if p.layout == Layout::Horizontal {
                        Layout::Vertical
                    } else {
                        Layout::Horizontal
                    };
                    cx.notify();
                })),
            )
            .child(
                YueButton::new("descriptions-size", format!("密度：{:?}", self.size)).on_click(
                    cx.listener(|p, _, _, cx| {
                        p.size = match p.size {
                            Size::Small => Size::Medium,
                            Size::Medium => Size::Large,
                            Size::Large => Size::Small,
                        };
                        cx.notify();
                    }),
                ),
            )
            .child(
                YueButton::new(
                    "descriptions-border",
                    if self.bordered {
                        "有边框"
                    } else {
                        "无边框"
                    },
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.bordered = !p.bordered;
                    cx.notify();
                })),
            )
            .child(
                YueButton::new("descriptions-colon", "切换冒号").on_click(cx.listener(
                    |p, _, _, cx| {
                        p.colon = !p.colon;
                        cx.notify();
                    },
                )),
            )
            .child(
                YueButton::new("descriptions-label-width", "切换标签宽度").on_click(cx.listener(
                    |p, _, _, cx| {
                        p.wide_label = !p.wide_label;
                        cx.notify();
                    },
                )),
            )
            .child(
                YueButton::new(
                    "descriptions-label-align",
                    format!("标签：{:?}", self.label_align),
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.label_align = cycle(p.label_align);
                    cx.notify();
                })),
            )
            .child(
                YueButton::new(
                    "descriptions-value-align",
                    format!("内容：{:?}", self.value_align),
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.value_align = cycle(p.value_align);
                    cx.notify();
                })),
            )
            .child(
                YueButton::new(
                    "descriptions-vertical",
                    format!("垂直：{:?}", self.vertical),
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.vertical = match p.vertical {
                        VAlign::Top => VAlign::Center,
                        VAlign::Center => VAlign::Bottom,
                        VAlign::Bottom => VAlign::Top,
                    };
                    cx.notify();
                })),
            )
            .child(
                YueButton::new(
                    "descriptions-width",
                    if self.narrow {
                        "恢复宽布局"
                    } else {
                        "窄面板"
                    },
                )
                .on_click(cx.listener(|p, _, _, cx| {
                    p.narrow = !p.narrow;
                    cx.notify();
                })),
            );
        let anatomy = self
            .details("descriptions-anatomy")
            .columns(2)
            .title("工单信息")
            .extra(
                YueButton::new("descriptions-copy-number", "复制编号").on_click(|_, _, cx| {
                    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                        "WO-20261110-0042".into(),
                    ))
                }),
            )
            .items(basic_items(theme));
        let resizable = YueResizable::horizontal("descriptions-resize", &self.split)
            .h(rems(28.))
            .w_full()
            .child(
                YueResizablePanel::new("details-panel")
                    .label("详情面板")
                    .initial_size(rems(43.))
                    .min_size(rems(11.))
                    .child(
                        div()
                            .size_full()
                            .p_3()
                            .min_w_0()
                            .child(
                                self.details("descriptions-resize-details")
                                    .items(basic_items(theme)),
                            )
                            .overflow_y_scrollbar(),
                    ),
            )
            .child(
                YueResizablePanel::new("help-panel")
                    .label("旁栏")
                    .min_size(rems(6.))
                    .child(
                        div()
                            .size_full()
                            .p_3()
                            .min_w_0()
                            .text_sm()
                            .child("同一窗口的窄详情面板独立降列。")
                            .child(
                                self.details("descriptions-side-details").columns(3).items([
                                    Item::new("site", "站点").text("二号车间"),
                                    Item::new("owner", "负责人").text("林悦"),
                                    Item::new("last", "最近更新")
                                        .text("2026-11-10 16:40")
                                        .span(Span::Filled),
                                ]),
                            )
                            .overflow_y_scrollbar(),
                    ),
            );
        let alignment = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                YueButton::new("descriptions-lines", format!("长编号：{} 行", self.lines))
                    .on_click(cx.listener(|p, _, _, cx| {
                        p.lines = p.lines % 3 + 1;
                        cx.notify();
                    })),
            )
            .child(
                self.details("descriptions-alignment").columns(3).items([
                    Item::new("number", "工单编号")
                        .text(
                            (0..self.lines)
                                .map(|i| format!("WO-20261110-{:04}", i + 42))
                                .collect::<Vec<_>>()
                                .join("\n"),
                        )
                        .copyable(true),
                    Item::new("state", "状态").content(
                        YueBadge::new("state-badge")
                            .text("进行中")
                            .tone(BadgeTone::Success),
                    ),
                    Item::new("owner", "负责人").content(
                        row()
                            .child(
                                YueAvatar::new("avatar", theme)
                                    .name("林悦")
                                    .size(AvatarSize::Small),
                            )
                            .child("林悦"),
                    ),
                ]),
            );
        let long=self.details("descriptions-overflow").items([
            Item::new("wrap","自然换行").text("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".repeat(4)),
            Item::new("ellipsis","单行截断").text(NOTES).text_overflow(Overflow::Ellipsis).copyable(true),
            Item::new("lines","两行截断").text(NOTES.repeat(3)).text_overflow(Overflow::Lines(2)),
            Item::new("short","完整短文").text("无额外提示").text_overflow(Overflow::Ellipsis),
            Item::new("url","资源地址").text("https://example.com/work-orders/2026/11/10/ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789").span(Span::Filled),
        ]);
        let slots = self.details("descriptions-slots").items([
            Item::new("link", "项目文档").content(YueLink::new(
                "docs-link",
                "GPUI Kit 文档",
                "https://gpui-kit.com/docs",
            )),
            Item::new("tags", "标签").content(YueTagGroup::new("tags").children([
                YueTag::new("high", "高优先级"),
                YueTag::new("repair", "检修"),
            ])),
            Item::new("progress", "处理进度").content(
                div()
                    .w(rems(10.))
                    .max_w_full()
                    .child(YueProgress::new("progress", ProgressValue::new(68.)).animated(false)),
            ),
            Item::new("custom", "自定义标签")
                .label_content(
                    row()
                        .child(yue_component::Icon::new(outline::Settings))
                        .child("设备"),
                )
                .content(YueTag::new("device", "A-06"))
                .copy_text("A-06"),
        ]);
        let mut list = YueList::new("descriptions-orders").bordered(true);
        for i in 0..3 {
            list = list.child(
                YueListItem::new(format!("order-{i}"))
                    .selected(i == self.selected)
                    .content(
                        YueListItemMeta::new(self.names[i].clone()).description(format!(
                            "WO-{:04} · {}",
                            i + 42,
                            self.statuses[i]
                        )),
                    )
                    .actions(
                        YueButton::new(format!("descriptions-select-{i}"), "查看")
                            .disabled(self.editing)
                            .on_click(cx.listener(move |p, _, _, cx| {
                                p.selected = i;
                                p.message = "已切换工单".into();
                                cx.notify();
                            })),
                    ),
            );
        }
        let extra = if self.editing {
            row()
                .child(
                    YueButton::new("descriptions-save", "保存")
                        .on_click(cx.listener(|p, _, w, cx| p.save(w, cx))),
                )
                .child(
                    YueButton::new("descriptions-cancel", "取消")
                        .on_click(cx.listener(|p, _, w, cx| p.finish("已取消修改", w, cx))),
                )
        } else {
            row().child(
                YueButton::new("descriptions-edit", "编辑")
                    .track_focus(&self.edit_focus)
                    .on_click(cx.listener(|p, _, w, cx| p.start_edit(w, cx))),
            )
        };
        let name = if self.editing {
            Item::new("name", "工单名称").content(
                YueField::new(theme)
                    .bind(&self.form, "description-name")
                    .input(
                        YueInput::new("descriptions-name", theme, &self.editor)
                            .accessibility_label("工单名称"),
                    ),
            )
        } else {
            Item::new("name", "工单名称").text(self.names[self.selected].clone())
        };
        let status = if self.editing {
            Item::new("status", "状态").content(
                YueSelect::new("descriptions-status", &self.status).accessibility_label("工单状态"),
            )
        } else {
            Item::new("status", "状态").content(
                YueBadge::new("order-status")
                    .text(self.statuses[self.selected].clone())
                    .tone(BadgeTone::Primary),
            )
        };
        let detail = YueCard::new("descriptions-record")
            .header(YueCardHeader::new(format!("工单 WO-{:04}", self.selected + 42)).extra(extra))
            .child(
                self.details("descriptions-record-details")
                    .columns(2)
                    .items([
                        name,
                        status,
                        Item::new("note", "处理说明").text(NOTES).span(Span::Filled),
                    ]),
            );
        let master_detail = div()
            .min_w_0()
            .flex()
            .flex_wrap()
            .items_start()
            .gap_4()
            .child(div().w(rems(19.)).max_w_full().child(list))
            .child(div().min_w_0().flex_1().child(detail));
        div()
            .id("descriptions-content")
            .debug_selector(|| "descriptions-content".into())
            .size_full()
            .min_w_0()
            .min_h_0()
            .child(
                div()
                    .p_6()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_5()
                    .child(div().text_2xl().child("Descriptions 描述列表"))
                    .child("一条记录的属性配对；多行内容撑高整排，短内容保持格内对齐。")
                    .child(controls)
                    .child(
                        div()
                            .w(if self.narrow { rems(24.) } else { rems(76.) })
                            .max_w_full()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_5()
                            .child(section("descriptions-basic-section", "结构与复制", anatomy))
                            .child(section(
                                "descriptions-layout-section",
                                "拖动容器，独立降列",
                                resizable,
                            ))
                            .child(section(
                                "descriptions-span-section",
                                "跨度与行尾补齐",
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_5()
                                    .child(
                                        self.details("descriptions-span-border")
                                            .bordered(true)
                                            .items(span_items()),
                                    )
                                    .child(
                                        self.details("descriptions-span-plain")
                                            .bordered(false)
                                            .items(span_items()),
                                    ),
                            ))
                            .child(section(
                                "descriptions-alignment-section",
                                "多行与同排对齐",
                                alignment,
                            ))
                            .child(section(
                                "descriptions-long-section",
                                "完整内容与截断提示",
                                long,
                            ))
                            .child(section("descriptions-slot-section", "组合已有组件", slots))
                            .child(section(
                                "descriptions-workflow-section",
                                "选择、编辑、保存与取消",
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_3()
                                    .child(
                                        div()
                                            .id("descriptions-message")
                                            .child(self.message.clone()),
                                    )
                                    .child(master_detail),
                            )),
                    ),
            )
            .overflow_y_scrollbar()
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::base::{
        Root,
        test_support::{ElementSnapshot, snapshots},
    };
    use gpui_kit::{Modifiers, TestAppContext, VisualTestContext, px, size};
    fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|w, cx| {
            w.refresh();
            w.draw(cx).clear(cx);
        });
        cx.run_until_parked();
    }
    fn node(cx: &mut VisualTestContext, id: &str) -> ElementSnapshot {
        cx.update(|w, _| {
            snapshots(w)
                .into_iter()
                .find(|n| n.path().last() == Some(&gpui_kit::ElementId::from(id.to_owned())))
                .unwrap_or_else(|| panic!("Missing {id}"))
        })
    }
    fn click(cx: &mut VisualTestContext, id: &str) {
        // The entire page shares the showcase scrollbar owner.
        for _ in 0..24 {
            let b = node(cx, id).bounds();
            if b.top() >= px(0.) && b.bottom() <= px(1000.) {
                break;
            }
            cx.simulate_event(gpui_kit::ScrollWheelEvent {
                position: gpui_kit::point(px(900.), px(500.)),
                delta: gpui_kit::ScrollDelta::Pixels(gpui_kit::point(
                    px(0.),
                    if b.top() < px(0.) {
                        px(400.)
                    } else {
                        px(-400.)
                    },
                )),
                touch_phase: gpui_kit::TouchPhase::Moved,
                ..Default::default()
            });
            draw(cx);
        }
        let position = node(cx, id).bounds().center();
        cx.simulate_click(position, Modifiers::default());
        draw(cx);
    }
    fn open(cx: &mut TestAppContext) -> (Entity<DescriptionsPage>, &mut VisualTestContext) {
        cx.update(|cx| {
            yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
            cx.set_reduce_motion(true);
        });
        let mut saved = None;
        let (_, cx) = cx.add_window_view(|w, cx| {
            let p = cx.new(|cx| DescriptionsPage::new(w, cx));
            saved = Some(p.clone());
            Root::new(p, w, cx)
        });
        cx.simulate_resize(size(px(1440.), px(1000.)));
        cx.update(|w, _| w.activate_window());
        draw(cx);
        (saved.unwrap(), cx)
    }
    #[gpui_kit::test]
    fn span_examples_stay_inside_card(cx: &mut TestAppContext) {
        let (p, cx) = open(cx);
        for columns in 1..=4 {
            for narrow in [false, true] {
                for layout in [Layout::Horizontal, Layout::Vertical] {
                    p.update(cx, |p, cx| {
                        p.columns = columns;
                        p.narrow = narrow;
                        p.layout = layout;
                        cx.notify();
                    });
                    draw(cx);
                    let card = node(cx, "descriptions-span-section").bounds();
                    let bordered = node(cx, "descriptions-span-border").bounds();
                    let plain = node(cx, "descriptions-span-plain").bounds();
                    assert!(bordered.bottom() <= plain.top(), "examples overlap");
                    for id in ["descriptions-span-border", "descriptions-span-plain"] {
                        cx.update(|w, _| {
                            for n in snapshots(w).into_iter().filter(|n| {
                                n.path().contains(&gpui_kit::ElementId::from(id.to_owned()))
                            }) {
                                let b = n.bounds();
                                assert!(
                                    b.top() >= card.top() && b.bottom() <= card.bottom(),
                                    "{id}: {:?} outside card {card:?} ({:?})",
                                    b,
                                    n.path()
                                );
                                assert!(b.left() >= card.left() && b.right() <= card.right());
                            }
                        });
                    }
                }
            }
        }
    }
    #[gpui_kit::test]
    fn detail_edit_validation_save_cancel_selection_and_focus(cx: &mut TestAppContext) {
        let (p, cx) = open(cx);
        click(cx, "descriptions-edit");
        assert!(p.read_with(cx, |p, _| p.editing));
        let input = p.read_with(cx, |p, _| p.editor.clone());
        assert!(cx.update(|w, cx| input.read(cx).focus_handle(cx).is_focused(w)));
        cx.update(|w, cx| input.update(cx, |s, cx| s.set_value("", w, cx)));
        draw(cx);
        click(cx, "descriptions-save");
        assert!(p.read_with(cx, |p, _| p.editing));
        assert!(p.read_with(cx, |p, cx| {
            p.form.read(cx).error("description-name", cx).is_some()
        }));
        cx.update(|w, cx| input.update(cx, |s, cx| s.set_value("Updated work order", w, cx)));
        draw(cx);
        click(cx, "descriptions-save");
        assert!(!p.read_with(cx, |p, _| p.editing));
        assert_eq!(
            p.read_with(cx, |p, _| p.names[0].clone()).as_ref(),
            "Updated work order"
        );
        assert_eq!(node(cx, "descriptions-edit").focused(), Some(true));
        click(cx, "descriptions-edit");
        cx.update(|w, cx| input.update(cx, |s, cx| s.set_value("Discard", w, cx)));
        draw(cx);
        click(cx, "descriptions-cancel");
        assert_eq!(
            p.read_with(cx, |p, _| p.names[0].clone()).as_ref(),
            "Updated work order"
        );
        assert_eq!(node(cx, "descriptions-edit").focused(), Some(true));
        click(cx, "descriptions-select-1");
        assert_eq!(p.read_with(cx, |p, _| p.selected), 1);
        click(cx, "descriptions-edit");
        assert_eq!(input.read_with(cx, |s, _| s.value()).as_ref(), "校准传感器");
    }
    #[gpui_kit::test]
    fn playground_controls_keep_editor_entity_and_change_layout(cx: &mut TestAppContext) {
        let (p, cx) = open(cx);
        let id = p.read_with(cx, |p, _| p.editor.entity_id());
        click(cx, "descriptions-columns");
        click(cx, "descriptions-layout");
        click(cx, "descriptions-size");
        click(cx, "descriptions-border");
        click(cx, "descriptions-value-align");
        click(cx, "descriptions-vertical");
        p.read_with(cx, |p, _| {
            assert_eq!(p.columns, 4);
            assert_eq!(p.layout, Layout::Vertical);
            assert_eq!(p.size, Size::Large);
            assert!(!p.bordered);
            assert_eq!(p.value_align, Align::Center);
            assert_eq!(p.vertical, VAlign::Bottom);
            assert_eq!(p.editor.entity_id(), id);
        });
        click(cx, "descriptions-width");
        assert!(p.read_with(cx, |p, _| p.narrow));
        cx.update(|w, cx| {
            w.simulate_next_frame(cx);
        });
        draw(cx);
        assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
    }
}
