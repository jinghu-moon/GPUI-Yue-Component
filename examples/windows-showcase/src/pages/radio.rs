use super::super::*;

pub(crate) fn radio_formats() -> Vec<YueRadioOption> {
    vec![
        YueRadioOption::new("png", "PNG"),
        YueRadioOption::new("svg", "SVG（当前不可选）").disabled(true),
        YueRadioOption::new("pdf", "PDF"),
        YueRadioOption::new("jpg", "JPG"),
    ]
}

pub(crate) fn radio_modes() -> Vec<YueRadioOption> {
    vec![
        YueRadioOption::new("light", "浅色"),
        YueRadioOption::new("dark", "深色"),
        YueRadioOption::new("auto", "跟随系统"),
    ]
}

pub(crate) fn radio_rhythm() -> Vec<YueRadioOption> {
    vec![
        YueRadioOption::new("regular", "常规"),
        YueRadioOption::new("occasional", "偶尔"),
        YueRadioOption::new("never", "从不"),
    ]
}

pub(crate) fn radio_frequencies() -> Vec<YueRadioOption> {
    vec![
        YueRadioOption::new("daily", "每天"),
        YueRadioOption::new("weekly", "每周"),
        YueRadioOption::new("monthly", "每月"),
    ]
}

pub(crate) fn radio_locks() -> Vec<YueRadioOption> {
    vec![
        YueRadioOption::new("locked", "锁定编辑"),
        YueRadioOption::new("unlocked", "允许编辑"),
        YueRadioOption::new("archive", "归档只读"),
    ]
}

pub(crate) fn radio_long_labels() -> Vec<YueRadioOption> {
    vec![
        YueRadioOption::new("long", "这一条标签写得很长，用来检查窄宽度下是否换行"),
        YueRadioOption::new("short", "短标签"),
        YueRadioOption::new("middle", "中等长度标签"),
    ]
}

pub(crate) fn radio_content(
    theme: YueTheme,
    showcase: &Showcase,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let mut alone: Vec<gpui_kit::AnyElement> = Vec::new();
    for (index, label) in ["未选中（Tab 到此查看焦点环）", "已选中", "禁用（已选中）"]
        .into_iter()
        .enumerate()
    {
        let checked = showcase.radio_single[index];
        let update = cx.listener(move |showcase, checked: &bool, _, cx| {
            showcase.radio_single[index] = *checked;
            cx.notify();
        });
        alone.push(
            YueRadio::new(format!("radio-alone-{index}"), theme, checked)
                .label(label)
                .disabled(index == 2)
                .on_change(move |checked, _, window, cx| update(&checked, window, cx))
                .into_any_element(),
        );
    }
    let name_update = cx.listener(|showcase, checked: &bool, _, cx| {
        showcase.radio_single[3] = *checked;
        cx.notify();
    });
    let name_only = div()
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(gpui_kit::rgb(theme.border_default()))
        .child(
            YueRadio::new("radio-alone-name", theme, showcase.radio_single[3])
                .accessibility_label("只有圆点的选项")
                .on_change(move |checked, _, window, cx| name_update(&checked, window, cx)),
        )
        .child(
            div()
                .text_sm()
                .text_color(gpui_kit::rgb(theme.text_secondary()))
                .child("无可见文字，名称来自 accessibility_label"),
        );

    let vertical_update = cx.listener(|showcase, value: &SharedString, _, cx| {
        showcase.radio_vertical = Some(value.clone());
        cx.notify();
    });
    let horizontal_update = cx.listener(|showcase, value: &SharedString, _, cx| {
        showcase.radio_horizontal = Some(value.clone());
        cx.notify();
    });
    let segment_update = cx.listener(|showcase, value: &SharedString, _, cx| {
        showcase.radio_segment = Some(value.clone());
        cx.notify();
    });
    let column_update = cx.listener(|showcase, value: &SharedString, _, cx| {
        showcase.radio_segments[0] = Some(value.clone());
        cx.notify();
    });
    let empty_update = cx.listener(|showcase, value: &SharedString, _, cx| {
        showcase.radio_empty = Some(value.clone());
        cx.notify();
    });
    let keyboard_update = cx.listener(|showcase, value: &SharedString, _, cx| {
        showcase.radio_keyboard = Some(value.clone());
        cx.notify();
    });
    let narrow_update = cx.listener(|showcase, value: &SharedString, _, cx| {
        showcase.radio_narrow = Some(value.clone());
        cx.notify();
    });

    let mut sizes: Vec<gpui_kit::AnyElement> = Vec::new();
    for (index, size) in [RadioSize::Small, RadioSize::Medium, RadioSize::Large]
        .into_iter()
        .enumerate()
    {
        let indicator_update = cx.listener(move |showcase, value: &SharedString, _, cx| {
            showcase.radio_sizes[index] = Some(value.clone());
            cx.notify();
        });
        let segmented_update = cx.listener(move |showcase, value: &SharedString, _, cx| {
            showcase.radio_segments[index + 1] = Some(value.clone());
            cx.notify();
        });
        sizes.push(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_sm().child(format!("{size:?}")))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_start()
                        .gap_6()
                        .child(
                            YueRadioGroup::new(
                                format!("radio-size-{index}"),
                                theme,
                                showcase.radio_sizes[index].clone(),
                            )
                            .label("圆点外观")
                            .size(size)
                            .options(radio_rhythm())
                            .on_change(move |value, window, cx| {
                                indicator_update(&value, window, cx)
                            }),
                        )
                        .child(
                            YueRadioGroup::new(
                                format!("radio-size-segment-{index}"),
                                theme,
                                showcase.radio_segments[index + 1].clone(),
                            )
                            .label("分段外观")
                            .size(size)
                            .appearance(RadioAppearance::Button)
                            .options(radio_rhythm())
                            .on_change(move |value, window, cx| {
                                segmented_update(&value, window, cx)
                            }),
                        ),
                )
                .into_any_element(),
        );
    }

    let mut tiles: Vec<gpui_kit::AnyElement> = Vec::new();
    for (index, (title, mode, accent)) in PROFILES.into_iter().enumerate() {
        let profile_theme = YueTheme::new(mode, accent);
        let indicator_update = cx.listener(move |showcase, value: &SharedString, _, cx| {
            showcase.radio_profiles[index] = Some(value.clone());
            cx.notify();
        });
        let segment_update = cx.listener(move |showcase, value: &SharedString, _, cx| {
            showcase.radio_profiles[index] = Some(value.clone());
            cx.notify();
        });
        tiles.push(
            theme_tile(
                title,
                profile_theme,
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        YueRadioGroup::new(
                            format!("radio-profile-{index}"),
                            profile_theme,
                            showcase.radio_profiles[index].clone(),
                        )
                        .label("输出格式")
                        .options(radio_formats())
                        .on_change(move |value, window, cx| indicator_update(&value, window, cx)),
                    )
                    .child(
                        YueRadioGroup::new(
                            format!("radio-profile-segment-{index}"),
                            profile_theme,
                            showcase.radio_profiles[index].clone(),
                        )
                        .label("提醒频率")
                        .appearance(RadioAppearance::Button)
                        .options(radio_rhythm())
                        .on_change(move |value, window, cx| segment_update(&value, window, cx)),
                    ),
            )
            .into_any_element(),
        );
    }

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_6()
        .p_8()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_2xl().child("Radio"))
                .child(
                    div()
                        .text_color(gpui_kit::rgb(theme.text_secondary()))
                        .child("互斥单选：组持有 value，选项定义 label 与禁用；圆点与分段两种外观共用同一套行为"),
                ),
        )
        .child(section(
            theme,
            "Single options",
            "未选中 / 已选中 / 禁用三态与仅有可访问名称的控件；Tab 聚焦可见焦点环，Space 或 Enter 激活",
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_6()
                .children(alone)
                .child(name_only),
        ))
        .child(section(
            theme,
            "Controlled groups",
            "垂直列与水平行；组只持有一个 value，点选或方向键换档后回调写回 Showcase，再传回组件",
            div()
                .flex()
                .flex_wrap()
                .items_start()
                .gap_8()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            YueRadioGroup::new(
                                "radio-vertical",
                                theme,
                                showcase.radio_vertical.clone(),
                            )
                            .label("输出格式")
                            .options(radio_formats())
                            .on_change(move |value, window, cx| {
                                vertical_update(&value, window, cx)
                            }),
                        )
                        .child(readout(
                            theme,
                            format!(
                                "当前 value：{}",
                                value_text(showcase.radio_vertical.as_ref())
                            ),
                        )),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            YueRadioGroup::new(
                                "radio-horizontal",
                                theme,
                                showcase.radio_horizontal.clone(),
                            )
                            .label("界面模式")
                            .options(radio_modes())
                            .direction(GroupDirection::Horizontal)
                            .on_change(move |value, window, cx| {
                                horizontal_update(&value, window, cx)
                            }),
                        )
                        .child(readout(
                            theme,
                            format!(
                                "当前 value：{}",
                                value_text(showcase.radio_horizontal.as_ref())
                            ),
                        )),
                ),
        ))
        .child(section(
            theme,
            "Button appearance",
            "整组一个连续轮廓：首尾圆角、相邻段共用一条分界线；选中段持续填充，禁用段留在行内",
            div()
                .flex()
                .flex_wrap()
                .items_start()
                .gap_8()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            YueRadioGroup::new(
                                "radio-segment-row",
                                theme,
                                showcase.radio_segment.clone(),
                            )
                            .label("提醒频率")
                            .appearance(RadioAppearance::Button)
                            .options(radio_rhythm())
                            .on_change(move |value, window, cx| {
                                segment_update(&value, window, cx)
                            }),
                        )
                        .child(readout(
                            theme,
                            format!(
                                "当前 value：{}",
                                value_text(showcase.radio_segment.as_ref())
                            ),
                        )),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            YueRadioGroup::new(
                                "radio-segment-column",
                                theme,
                                showcase.radio_segments[0].clone(),
                            )
                            .label("未选中时的分段列")
                            .appearance(RadioAppearance::Button)
                            .direction(GroupDirection::Vertical)
                            .options(radio_frequencies())
                            .on_change(move |value, window, cx| {
                                column_update(&value, window, cx)
                            }),
                        )
                        .child(readout(
                            theme,
                            format!(
                                "当前 value：{}",
                                value_text(showcase.radio_segments[0].as_ref())
                            ),
                        )),
                ),
        ))
        .child(section(
            theme,
            "Size ladder",
            "Small / Medium / Large 两种外观对照",
            div().flex().flex_col().gap_4().children(sizes),
        ))
        .child(section(
            theme,
            "Keyboard",
            "这里每个组都能直接用键盘操作：Tab / Shift+Tab 进出组，方向键在可用项之间移动并立即选中，Space 或 Enter 激活聚焦项",
            div()
                .flex()
                .flex_wrap()
                .items_start()
                .gap_8()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .text_sm()
                        .child("整组只有一个 Tab 停靠点，落在当前选中项；无选中时落在第一个可用项。")
                        .child("垂直组读 ↑ ↓，水平组读 ← →；越过首尾循环，禁用项跳过。")
                        .child("Home / End 跳到首 / 末可用项并选中。")
                        .child("已选项上再按 Space 不会取消——这是单选与多选的差别。"),
                )
                .child(
                    YueRadioGroup::new(
                        "radio-keyboard",
                        theme,
                        showcase.radio_keyboard.clone(),
                    )
                    .label("提醒周期")
                    .options(radio_frequencies())
                    .on_change(move |value, window, cx| keyboard_update(&value, window, cx)),
                ),
        ))
        .child(section(
            theme,
            "Empty, locked and narrow",
            "空 value 的组、整组禁用与窄宽度下的长标签",
            div()
                .flex()
                .flex_wrap()
                .items_start()
                .gap_8()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            YueRadioGroup::new("radio-empty", theme, showcase.radio_empty.clone())
                                .label("尚未选择的组")
                                .options(radio_formats())
                                .on_change(move |value, window, cx| {
                                    empty_update(&value, window, cx)
                                }),
                        )
                        .child(readout(
                            theme,
                            format!(
                                "当前 value：{}",
                                value_text(showcase.radio_empty.as_ref())
                            ),
                        )),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            YueRadioGroup::new("radio-locked", theme, showcase.radio_locked.clone())
                                .label("整组禁用")
                                .disabled(true)
                                .options(radio_locks()),
                        )
                        .child(readout(
                            theme,
                            format!(
                                "当前 value：{}（不可操作，回调不会触发）",
                                value_text(showcase.radio_locked.as_ref())
                            ),
                        )),
                )
                .child(
                    div()
                        .w_72()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            YueRadioGroup::new(
                                "radio-narrow",
                                theme,
                                showcase.radio_narrow.clone(),
                            )
                            .label("窄宽度水平组")
                            .options(radio_long_labels())
                            .direction(GroupDirection::Horizontal)
                            .on_change(move |value, window, cx| {
                                narrow_update(&value, window, cx)
                            }),
                        )
                        .child(readout(
                            theme,
                            format!(
                                "当前 value：{}",
                                value_text(showcase.radio_narrow.as_ref())
                            ),
                        )),
                ),
        ))
        .child(section(
            theme,
            "Theme profiles",
            "四种 Yue profile 下的两种外观对照",
            div().flex().flex_wrap().gap_4().children(tiles),
        ))
}
