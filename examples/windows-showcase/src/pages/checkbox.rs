use super::super::*;

pub(crate) fn checkbox_tags_options() -> Vec<YueCheckboxOption> {
    vec![
        YueCheckboxOption::new("docs", "文档"),
        YueCheckboxOption::new("design", "设计稿"),
        YueCheckboxOption::new("release", "发布说明"),
    ]
}

pub(crate) fn checkbox_channels() -> Vec<YueCheckboxOption> {
    vec![
        YueCheckboxOption::new("mail", "邮件"),
        YueCheckboxOption::new("tasks", "任务"),
        YueCheckboxOption::new("calendar", "日历（当前不可选）").disabled(true),
        YueCheckboxOption::new("notes", "备忘"),
    ]
}

pub(crate) fn checkbox_scopes() -> Vec<YueCheckboxOption> {
    vec![
        YueCheckboxOption::new("web", "网页端"),
        YueCheckboxOption::new("design", "设计稿"),
        YueCheckboxOption::new("print", "印刷（当前不可选）").disabled(true),
    ]
}

pub(crate) fn checkbox_locks() -> Vec<YueCheckboxOption> {
    vec![
        YueCheckboxOption::new("readme", "更新 README"),
        YueCheckboxOption::new("draft", "同时更新草稿"),
    ]
}

pub(crate) fn checkbox_long_labels() -> Vec<YueCheckboxOption> {
    vec![
        YueCheckboxOption::new("long", "这一条标签写得很长，用来检查窄宽度下是否换行"),
        YueCheckboxOption::new("short", "短标签"),
        YueCheckboxOption::new("middle", "中等长度标签"),
    ]
}

pub(crate) fn checkbox_content(
    theme: YueTheme,
    showcase: &Showcase,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let mut alone: Vec<gpui_kit::AnyElement> = Vec::new();
    for (index, label) in [
        "未选中（Tab 到此查看焦点环）",
        "已选中",
        "半选（Indeterminate）",
        "禁用（已选中）",
    ]
    .into_iter()
    .enumerate()
    {
        let state = showcase.checkbox_single[index];
        let update = cx.listener(move |showcase, next: &CheckboxState, _, cx| {
            showcase.checkbox_single[index] = *next;
            cx.notify();
        });
        alone.push(
            YueCheckbox::new(format!("checkbox-alone-{index}"), theme, state)
                .label(label)
                .disabled(index == 3)
                .on_change(move |next, _, window, cx| update(&next, window, cx))
                .into_any_element(),
        );
    }
    let name_update = cx.listener(|showcase, next: &CheckboxState, _, cx| {
        showcase.checkbox_single[4] = *next;
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
            YueCheckbox::new("checkbox-alone-name", theme, showcase.checkbox_single[4])
                .accessibility_label("只有方框的选项")
                .on_change(move |next, _, window, cx| name_update(&next, window, cx)),
        )
        .child(
            div()
                .text_sm()
                .text_color(gpui_kit::rgb(theme.text_secondary()))
                .child("无可见文字，名称来自 accessibility_label"),
        );

    let mut sizes: Vec<gpui_kit::AnyElement> = Vec::new();
    for (index, size) in [
        CheckboxSize::Small,
        CheckboxSize::Medium,
        CheckboxSize::Large,
    ]
    .into_iter()
    .enumerate()
    {
        let state = showcase.checkbox_sizes[index];
        let update = cx.listener(move |showcase, next: &CheckboxState, _, cx| {
            showcase.checkbox_sizes[index] = *next;
            cx.notify();
        });
        sizes.push(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(div().text_sm().child(format!("{size:?}")))
                .child(
                    YueCheckbox::new(format!("checkbox-size-{index}"), theme, state)
                        .size(size)
                        .label("包含附件")
                        .on_change(move |next, _, window, cx| update(&next, window, cx)),
                )
                .into_any_element(),
        );
    }

    let tags_update = cx.listener(|showcase, values: &Vec<SharedString>, _, cx| {
        showcase.checkbox_tags = values.clone();
        cx.notify();
    });
    let tags_reset = cx.listener(|showcase, _: &gpui_kit::ClickEvent, _, cx| {
        showcase.checkbox_tags = vec!["docs".into()];
        cx.notify();
    });
    let built_in_update = cx.listener(|showcase, values: &Vec<SharedString>, _, cx| {
        showcase.checkbox_built_in = values.clone();
        cx.notify();
    });
    let mixed_update = cx.listener(|showcase, values: &Vec<SharedString>, _, cx| {
        showcase.checkbox_mixed = values.clone();
        cx.notify();
    });
    let narrow_update = cx.listener(|showcase, values: &Vec<SharedString>, _, cx| {
        showcase.checkbox_narrow = values.clone();
        cx.notify();
    });

    let manual_options: [(&'static str, &'static str, bool); 3] = [
        ("desktop", "桌面端", false),
        ("web", "网页端", false),
        ("mobile", "移动端（当前不可选）", true),
    ];
    let manual_values: Vec<SharedString> = manual_options
        .iter()
        .filter(|(_, _, disabled)| !*disabled)
        .map(|(value, _, _)| SharedString::from(*value))
        .collect();
    let manual_hits = manual_values
        .iter()
        .filter(|value| showcase.checkbox_manual.contains(value))
        .count();
    let manual_header = if manual_hits == 0 {
        CheckboxState::Unchecked
    } else if manual_hits == manual_values.len() {
        CheckboxState::Checked
    } else {
        CheckboxState::Indeterminate
    };
    let manual_all_update = cx.listener(move |showcase, next: &CheckboxState, _, cx| {
        showcase.checkbox_manual = if *next == CheckboxState::Unchecked {
            Vec::new()
        } else {
            manual_values.clone()
        };
        cx.notify();
    });
    let mut manual_rows: Vec<gpui_kit::AnyElement> = Vec::new();
    for (index, (value, label, disabled)) in manual_options.into_iter().enumerate() {
        let value = SharedString::from(value);
        let state = if showcase.checkbox_manual.contains(&value) {
            CheckboxState::Checked
        } else {
            CheckboxState::Unchecked
        };
        let update = cx.listener(move |showcase, next: &CheckboxState, _, cx| {
            if *next == CheckboxState::Unchecked {
                showcase.checkbox_manual.retain(|held| held != &value);
            } else if !showcase.checkbox_manual.contains(&value) {
                showcase.checkbox_manual.push(value.clone());
            }
            cx.notify();
        });
        manual_rows.push(
            YueCheckbox::new(format!("checkbox-manual-{index}"), theme, state)
                .label(label)
                .disabled(disabled)
                .on_change(move |next, _, window, cx| update(&next, window, cx))
                .into_any_element(),
        );
    }

    let mut tiles: Vec<gpui_kit::AnyElement> = Vec::new();
    for (index, (title, mode, accent)) in PROFILES.into_iter().enumerate() {
        let profile_theme = YueTheme::new(mode, accent);
        let update = cx.listener(move |showcase, values: &Vec<SharedString>, _, cx| {
            showcase.checkbox_profiles[index] = values.clone();
            cx.notify();
        });
        tiles.push(
            theme_tile(
                title,
                profile_theme,
                YueCheckboxGroup::new(
                    format!("checkbox-profile-{index}"),
                    profile_theme,
                    showcase.checkbox_profiles[index].clone(),
                )
                .label("导出内容")
                .select_all("全选")
                .options(checkbox_tags_options())
                .on_change(move |values, _, window, cx| update(&values, window, cx)),
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
                .child(div().text_2xl().child("Checkbox"))
                .child(
                    div()
                        .text_color(gpui_kit::rgb(theme.text_secondary()))
                        .child("多选控件：勾与半选短横由 GPUI 矢量绘制，三态为未选 / 已选 / 半选，组的选中集合由 Showcase 持有"),
                ),
        )
        .child(section(
            theme,
            "Single options",
            "未选中 / 已选中 / 半选 / 禁用；半选首次激活变成已选，而不是清空",
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
            "Size ladder",
            "Small / Medium / Large 的方框、勾与 dash 一同缩放",
            div().flex().flex_col().gap_3().children(sizes),
        ))
        .child(section(
            theme,
            "Controlled multi-select",
            "三个选项的组：回调报告新的完整集合，页面同时显示当前 value 并可重置",
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_4()
                .child(
                    YueCheckboxGroup::new(
                        "checkbox-tags",
                        theme,
                        showcase.checkbox_tags.clone(),
                    )
                    .label("导出内容")
                    .options(checkbox_tags_options())
                    .on_change(move |values, _, window, cx| tags_update(&values, window, cx)),
                )
                .child(
                    YueButton::text("checkbox-tags-reset", theme, "重置")
                        .variant(ButtonVariant::Outline)
                        .on_click(move |event, window, cx| tags_reset(event, window, cx)),
                )
                .child(readout(
                    theme,
                    format!("当前 value：{}", values_text(&showcase.checkbox_tags)),
                )),
        ))
        .child(section(
            theme,
            "Grouping A: app-drawn select-all",
            "页面自己组合独立 YueCheckbox：全选态由可用子项派生，组结构只有可见标题，容器不带 Group 语义",
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
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(gpui_kit::rgb(theme.text_secondary()))
                                        .child("导出平台"),
                                )
                                .child(
                                    YueCheckbox::new(
                                        "checkbox-manual-all",
                                        theme,
                                        manual_header,
                                    )
                                    .label("全选")
                                    .on_change(move |next, _, window, cx| {
                                        manual_all_update(&next, window, cx)
                                    }),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .pl_4()
                                .children(manual_rows),
                        )
                        .child(readout(
                            theme,
                            format!("当前 value：{}", values_text(&showcase.checkbox_manual)),
                        )),
                ),
        ))
        .child(section(
            theme,
            "Grouping B: built-in select-all",
            "YueCheckboxGroup 的全选头由选项定义派生：禁用项不参与三态判断，也不会被全选或清空",
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
                            YueCheckboxGroup::new(
                                "checkbox-built-in",
                                theme,
                                showcase.checkbox_built_in.clone(),
                            )
                            .label("通知渠道")
                            .select_all("全选")
                            .options(checkbox_channels())
                            .on_change(move |values, _, window, cx| {
                                built_in_update(&values, window, cx)
                            }),
                        )
                        .child(readout(
                            theme,
                            format!(
                                "当前 value：{}",
                                values_text(&showcase.checkbox_built_in)
                            ),
                        )),
                ),
        ))
        .child(section(
            theme,
            "Mixed, locked and empty",
            "全选头的三态来自可用子项的聚合：半选、个别禁用、整组禁用与空选项组",
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
                            YueCheckboxGroup::new(
                                "checkbox-mixed",
                                theme,
                                showcase.checkbox_mixed.clone(),
                            )
                            .label("范围（含禁用项）")
                            .select_all("全选")
                            .options(checkbox_scopes())
                            .on_change(move |values, _, window, cx| {
                                mixed_update(&values, window, cx)
                            }),
                        )
                        .child(readout(
                            theme,
                            format!("当前 value：{}", values_text(&showcase.checkbox_mixed)),
                        )),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            YueCheckboxGroup::new(
                                "checkbox-locked",
                                theme,
                                showcase.checkbox_locked.clone(),
                            )
                            .label("整组禁用")
                            .select_all("全选")
                            .disabled(true)
                            .options(checkbox_locks()),
                        )
                        .child(readout(
                            theme,
                            format!(
                                "当前 value：{}（不可操作，回调不会触发）",
                                values_text(&showcase.checkbox_locked)
                            ),
                        )),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            YueCheckboxGroup::new(
                                "checkbox-empty",
                                theme,
                                Vec::<SharedString>::new(),
                            )
                            .label("空选项组")
                            .select_all("全选")
                            .options(Vec::<YueCheckboxOption>::new()),
                        )
                        .child(readout(
                            theme,
                            "没有可操作的选项，全选头自动禁用".to_string(),
                        )),
                )
                .child(
                    div()
                        .w_72()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            YueCheckboxGroup::new(
                                "checkbox-narrow",
                                theme,
                                showcase.checkbox_narrow.clone(),
                            )
                            .label("窄宽度水平组")
                            .direction(GroupDirection::Horizontal)
                            .options(checkbox_long_labels())
                            .on_change(move |values, _, window, cx| {
                                narrow_update(&values, window, cx)
                            }),
                        )
                        .child(readout(
                            theme,
                            format!("当前 value：{}", values_text(&showcase.checkbox_narrow)),
                        )),
                ),
        ))
        .child(section(
            theme,
            "Keyboard",
            "上面的每个控件都能直接用键盘操作；这里给出多选与单选在行为上的差别",
            div()
                .flex()
                .flex_col()
                .gap_1()
                .text_sm()
                .child("Tab 逐个进入组内每一项与全选头，Space 或 Enter 切换聚焦的控件。")
                .child("与单选组不同：多选组没有漫游焦点，每一项都是独立的 Tab 停靠点，方向键不属于它。")
                .child("已选中的项再按一次会取消；单选的 Space 只会把选中留在当前项。")
                .child("半选首次激活变成已选，而不是清空；全选头在禁用项存在时只聚合可用项。")
                .child("没有只读开关：不可改变的状态由 disabled 表达，校验文案交给 YueField。"),
        ))
        .child(section(
            theme,
            "Theme profiles",
            "四种 Yue profile 下的方框、勾与 dash 对照",
            div().flex().flex_wrap().gap_4().children(tiles),
        ))
}

// A row handler that writes the row's own name into the section's readout, so a
// click on the page has a consequence the reader can see.
