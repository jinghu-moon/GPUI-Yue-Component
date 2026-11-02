use super::super::*;
use yue_component::{
    CollapseChangeRequest, CollapseExpandTrigger, CollapseIndicatorPosition, CollapseMode,
    CollapseMotion, CollapseSize, CollapseVariant, InputState, YueCollapse, YueCollapseItem,
    YueSegmented, YueSegmentedOption,
};

pub(crate) struct CollapsePage {
    values: Vec<Vec<SharedString>>,
    input: gpui_kit::Entity<InputState>,
    size: CollapseSize,
    borderless: bool,
    trailing: bool,
    icon_only: bool,
    indicator: bool,
    custom: bool,
    motion: bool,
    disabled: bool,
    refuse: bool,
    long: bool,
    disabled_items: [bool; 3],
    resets: [usize; 3],
    last: String,
}
impl CollapsePage {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut values = vec![vec!["one".into()]; 8];
        values[0].push("two".into());
        Self {
            values,
            input: cx.new(|cx| InputState::new(window, cx).placeholder("收起后再展开，输入仍保留")),
            size: CollapseSize::Medium,
            borderless: false,
            trailing: false,
            icon_only: false,
            indicator: true,
            custom: false,
            motion: true,
            disabled: false,
            refuse: false,
            long: false,
            disabled_items: [false; 3],
            resets: [0; 3],
            last: "尚未操作".into(),
        }
    }
    fn group(
        &self,
        id: &'static str,
        slot: usize,
        items: Vec<YueCollapseItem>,
        cx: &mut Context<Self>,
    ) -> YueCollapse {
        YueCollapse::new(id, self.values[slot].clone())
            .accessibility_label(format!("{id} 折叠面板"))
            .items(items)
            .motion(if self.motion {
                CollapseMotion::Height
            } else {
                CollapseMotion::None
            })
            .on_change(
                cx.listener(move |p, request: &CollapseChangeRequest, _, cx| {
                    p.last = format!(
                        "{} · {:?} → {:?}{}",
                        request.key(),
                        request.source(),
                        request.expanded_keys(),
                        if slot == 1 && p.refuse {
                            "（宿主拒绝）"
                        } else {
                            ""
                        }
                    );
                    if slot != 1 || !p.refuse {
                        p.values[slot] = request.expanded_keys().to_vec();
                    }
                    cx.notify();
                }),
            )
    }
    fn switch(
        &self,
        id: &'static str,
        label: &'static str,
        checked: bool,
        field: usize,
        theme: YueTheme,
        cx: &mut Context<Self>,
    ) -> YueSwitch {
        let owner = cx.entity();
        YueSwitch::new(id, theme, checked, label)
            .size(SwitchSize::Small)
            .on_change(move |checked, _, _, cx| {
                owner.update(cx, |p, cx| {
                    match field {
                        0 => p.trailing = checked,
                        1 => {
                            p.icon_only = checked;
                            if checked {
                                p.indicator = true;
                            }
                        }
                        2 => {
                            p.indicator = checked;
                            if !checked {
                                p.icon_only = false;
                            }
                        }
                        3 => p.custom = checked,
                        4 => p.disabled = checked,
                        5 => p.borderless = checked,
                        6 => p.motion = checked,
                        7 => p.refuse = checked,
                        _ => p.long = checked,
                    }
                    cx.notify();
                });
            })
    }
}
fn column() -> gpui_kit::Div {
    div().flex().flex_col().gap_4().w_full().min_w_0()
}
fn row() -> gpui_kit::Div {
    div().flex().items_center().flex_wrap().gap_4()
}
fn items() -> Vec<YueCollapseItem> {
    [("one", "常规设置"), ("two", "导出设置"), ("three", "这一项使用较长标题，在窄窗口中自然换行，右侧操作仍然保持独立"), ("four", "高级设置")].into_iter().map(|(key, title)| YueCollapseItem::new(key, title).content(move |_, _| div().child(format!("{title}的内容。内容在文档流中展开，后续标题随高度平滑移动。可以使用 ↑ / ↓ / Home / End 移动标题焦点，Enter / Space 切换。" )).into_any_element())).collect()
}
impl Render for CollapsePage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = yue_component::current_theme(cx);
        let owner = cx.entity();
        let nested_value = self.values[6].clone();
        let nested_owner = owner.clone();
        let mut basic = items();
        basic[1] = YueCollapseItem::new("two", "嵌套折叠面板").content(move |_, _| {
            let owner = nested_owner.clone();
            YueCollapse::new("nested-collapse", nested_value.clone()).heading_level(4).items([
                YueCollapseItem::new("one", "子分组一").content(|_, _| div().child("内层键盘导航只作用于这一组。父项收起后，子组的受控值仍由宿主保留。").into_any_element()),
                YueCollapseItem::new("two", "子分组二").content(|_, _| div().child("子分组的内容").into_any_element()),
            ]).on_change(move |r, _, cx| owner.update(cx, |p, cx| { p.values[6] = r.expanded_keys().to_vec(); cx.notify(); })).into_any_element()
        });
        basic[3] = YueCollapseItem::new("four", "自定义内容").content(move |_, _| {
            column()
                .child(
                    div()
                        .h_16()
                        .rounded_md()
                        .bg(gpui_kit::rgb(theme.surface_component()))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child("图片 / 图表占位"),
                )
                .child("内容由宿主构建，组件不承担加载或缓存。")
                .into_any_element()
        });
        let basic = self.group("collapse-basic", 0, basic, cx);
        let accordion = self
            .group("collapse-accordion", 1, items(), cx)
            .mode(CollapseMode::Accordion);
        let reject = self.switch(
            "collapse-refuse",
            "宿主拒绝展开请求",
            self.refuse,
            7,
            theme,
            cx,
        );
        let settings = row()
            .child(self.switch("collapse-trailing", "尾侧图标", self.trailing, 0, theme, cx))
            .child(self.switch(
                "collapse-icon-only",
                "仅图标触发",
                self.icon_only,
                1,
                theme,
                cx,
            ))
            .child(self.switch(
                "collapse-show-indicator",
                "显示图标",
                self.indicator,
                2,
                theme,
                cx,
            ))
            .child(self.switch(
                "collapse-custom",
                "自定义静态图标",
                self.custom,
                3,
                theme,
                cx,
            ))
            .child(self.switch(
                "collapse-disabled",
                "整组禁用展开",
                self.disabled,
                4,
                theme,
                cx,
            ))
            .child(self.switch("collapse-motion", "高度动效", self.motion, 6, theme, cx));
        let mut configurable = items();
        if self.custom {
            configurable = configurable
                .into_iter()
                .map(|i| i.indicator(outline::Star))
                .collect();
        }
        let configurable = self
            .group("collapse-configurable", 2, configurable, cx)
            .size(self.size)
            .disabled(self.disabled)
            .expand_trigger(if self.icon_only {
                CollapseExpandTrigger::Icon
            } else {
                CollapseExpandTrigger::Header
            })
            .indicator_position(if self.trailing {
                CollapseIndicatorPosition::Trailing
            } else {
                CollapseIndicatorPosition::Leading
            })
            .show_indicator(self.indicator);
        let action_items = (0..3).map(|ix| {
            let disabled = self.disabled_items[ix];
            let reset_count = self.resets[ix];
            let owner = owner.clone();
            YueCollapseItem::new(["one", "two", "three"][ix], format!("独立操作 {}", ix + 1)).disabled(disabled)
                .content(move |_, _| div().child(format!("已重置 {reset_count} 次。禁用只影响展开按钮，已展开内容和右侧操作仍可使用。")).into_any_element())
                .actions(move |_, _| {
                    let check_owner = owner.clone();
                    let reset_owner = owner.clone();
                    row().child(YueCheckbox::new(("collapse-disable", ix), theme, if disabled { CheckboxState::Checked } else { CheckboxState::Unchecked }).label("禁用")
                        .on_change(move |state, _, _, cx| check_owner.update(cx, |p, cx| { p.disabled_items[ix] = state == CheckboxState::Checked; cx.notify(); })))
                        .child(YueButton::new(("collapse-reset", ix), "重置").size(ButtonSize::Small).on_click(move |_, _, cx| reset_owner.update(cx, |p, cx| { p.resets[ix] += 1; cx.notify(); })))
                        .child(YueDropdownMenu::new(("collapse-menu", ix), theme, YueButton::new(("collapse-more", ix), "更多").size(ButtonSize::Small))
                            .items([YueMenuItem::new("独立菜单命令")])).into_any_element()
                })
        }).collect();
        let actions = self.group("collapse-actions", 3, action_items, cx);
        let sizes = YueSegmented::new(
            "collapse-size",
            theme,
            match self.size {
                CollapseSize::Small => "small",
                CollapseSize::Medium => "medium",
                CollapseSize::Large => "large",
            },
        )
        .label("尺寸")
        .options(
            [("small", "小"), ("medium", "中"), ("large", "大")]
                .map(|(key, label)| YueSegmentedOption::new(key, label)),
        )
        .on_change(cx.listener(|p, key: &SharedString, _, cx| {
            p.size = match key.as_ref() {
                "small" => CollapseSize::Small,
                "large" => CollapseSize::Large,
                _ => CollapseSize::Medium,
            };
            cx.notify();
        }));
        let borderless = self.switch(
            "collapse-borderless",
            "无外框",
            self.borderless,
            5,
            theme,
            cx,
        );
        let appearance = self
            .group("collapse-appearance", 4, items(), cx)
            .size(self.size)
            .variant(if self.borderless {
                CollapseVariant::Borderless
            } else {
                CollapseVariant::Bordered
            });
        let input = self.input.clone();
        let long = self.long;
        let keep = self.group(
            "collapse-retained",
            5,
            vec![
                YueCollapseItem::new("one", "宿主持有的输入状态")
                    .region(true)
                    .content(move |_, cx| {
                        column()
                            .child(YueInput::new(
                                "collapse-retained-input",
                                yue_component::current_theme(cx),
                                &input,
                            ))
                            .child(
                                YueDropdownMenu::new(
                                    "collapse-content-menu",
                                    theme,
                                    YueButton::new("collapse-content-more", "正文菜单"),
                                )
                                .items([YueMenuItem::new("关闭父项时，这个菜单随锚点一起清理")]),
                            )
                            .into_any_element()
                    }),
                YueCollapseItem::new("two", "动态自然高度").content(move |_, _| {
                    column()
                        .children((0..if long { 8 } else { 1 }).map(|ix| {
                            div().child(format!(
                                "第 {} 行：稳定展开时自然更新高度；动画中改变内容从当前像素继续。",
                                ix + 1
                            ))
                        }))
                        .into_any_element()
                }),
                YueCollapseItem::new("three", "无默认内容内边距")
                    .content_padding(false)
                    .content(|_, _| {
                        div()
                            .p_4()
                            .child("由宿主完全负责内边距。")
                            .into_any_element()
                    }),
            ],
            cx,
        );
        let dynamic = self.switch("collapse-dynamic", "增加动态内容", self.long, 8, theme, cx);
        column()
            .debug_selector(|| "collapse-content".into())
            .p_8()
            .gap_6()
            .child(div().text_2xl().child("Collapse 折叠面板"))
            .child(section(
                theme,
                "基础与嵌套",
                "前两项默认展开。每个可用标题都是正常 Tab 入口。",
                basic,
            ))
            .child(section(
                theme,
                "手风琴与受控请求",
                "一次请求包含完整展开集合，可全部收起。宿主拒绝时保持原状态。",
                column().child(reject).child(accordion),
            ))
            .child(section(
                theme,
                "图标与触发范围",
                "仅图标触发会保留可见图标；隐藏图标自动切回标题触发，避免无入口。",
                column().child(settings).child(configurable),
            ))
            .child(section(
                theme,
                "右侧独立操作",
                "Checkbox、按钮和菜单是展开 Trigger 的兄弟节点，不会误触展开。",
                actions,
            ))
            .child(section(
                theme,
                "尺寸与外观",
                "文字、行高、图标命中区与内边距成套变化。长标题自然换行。",
                column()
                    .child(row().child(sizes).child(borderless))
                    .child(appearance),
            ))
            .child(section(
                theme,
                "卸载、输入保留与动态内容",
                "收起立即卸载正文，只留下背景壳收拢。输入 Entity 和嵌套值由宿主保留。",
                column().child(dynamic).child(keep),
            ))
            .child(div().text_sm().child(format!(
                "展开值：{:?} · 最近请求：{} · 展开 / 收起 320ms · 正文裁切",
                self.values[0], self.last
            )))
    }
}
