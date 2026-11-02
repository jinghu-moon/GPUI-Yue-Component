use super::super::*;

pub(crate) fn input_content(theme: YueTheme, showcase: &Showcase) -> impl IntoElement {
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
                .child(div().text_2xl().child("Input")),
        )
        .child(section(
            theme,
            "Basic",
            "The field names the control for assistive technology",
            YueField::new(theme)
                .label("项目名称")
                .description("名称会展示在工作区列表中")
                .required(true)
                .input(
                    YueInput::new("input-basic", theme, &showcase.input_basic)
                        .width(gpui_kit::rems(26.0)),
                ),
        ))
        .child(section(
            theme,
            "Sizes",
            "Small、Medium、Large",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueInput::new("input-small", theme, &showcase.input_sizes[0])
                        .size(InputSize::Small)
                        .width(gpui_kit::rems(26.0)),
                )
                .child(
                    YueInput::new("input-medium", theme, &showcase.input_sizes[1])
                        .size(InputSize::Medium)
                        .width(gpui_kit::rems(26.0)),
                )
                .child(
                    YueInput::new("input-large", theme, &showcase.input_sizes[2])
                        .size(InputSize::Large)
                        .width(gpui_kit::rems(26.0)),
                ),
        ))
        .child(section(
            theme,
            "Validation",
            "Normal、Success、Warning、Error，边框与说明同源",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueField::new(theme)
                        .message(InputStatus::Normal, "提示文字不改变边框")
                        .input(
                            YueInput::new("input-normal", theme, &showcase.input_statuses[0])
                                .width(gpui_kit::rems(26.0)),
                        ),
                )
                .child(
                    YueField::new(theme)
                        .message(InputStatus::Success, "名称可用")
                        .input(
                            YueInput::new("input-success", theme, &showcase.input_statuses[1])
                                .width(gpui_kit::rems(26.0)),
                        ),
                )
                .child(
                    YueField::new(theme)
                        .message(InputStatus::Warning, "包含多余空格")
                        .input(
                            YueInput::new("input-warning", theme, &showcase.input_statuses[2])
                                .width(gpui_kit::rems(26.0)),
                        ),
                )
                .child(
                    YueField::new(theme)
                        .message(InputStatus::Error, "名称不能为空")
                        .input(
                            YueInput::new("input-error", theme, &showcase.input_statuses[3])
                                .width(gpui_kit::rems(26.0)),
                        ),
                ),
        ))
        .child(section(
            theme,
            "Disabled and readonly",
            "Disabled cannot receive edits; readonly remains selectable",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueInput::new("input-disabled", theme, &showcase.input_disabled)
                        .disabled(true)
                        .width(gpui_kit::rems(26.0)),
                )
                .child(
                    YueInput::new("input-readonly", theme, &showcase.input_readonly)
                        .readonly(true)
                        .width(gpui_kit::rems(26.0)),
                ),
        ))
        .child(section(
            theme,
            "Password and clearable",
            "Password visibility and clear actions update Base state",
            YueInput::new("input-password", theme, &showcase.input_password)
                .password(true)
                .clearable(true)
                .width(gpui_kit::rems(26.0)),
        ))
        .child(section(
            theme,
            "Narrow slots",
            "窄宽度下前后缀与清除、显隐按钮同时保留",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueInput::new("input-narrow-search", theme, &showcase.input_narrow[0])
                        .leading_icon(outline::Search)
                        .trailing("kg")
                        .clearable(true)
                        .width(gpui_kit::rems(8.0)),
                )
                .child(
                    YueInput::new("input-narrow-password", theme, &showcase.input_narrow[1])
                        .leading_icon(outline::Lock)
                        .password(true)
                        .clearable(true)
                        .width(gpui_kit::rems(11.0)),
                )
                .child(
                    YueInput::new("input-narrow-value", theme, &showcase.input_narrow[2])
                        .trailing("kg")
                        .width(gpui_kit::rems(14.0)),
                ),
        ))
        .child(section(
            theme,
            "Length limit",
            "超出 12 字的粘贴与输入会被截断，计数显示在框内",
            YueInput::new("input-counter", theme, &showcase.input_counter)
                .max_length(12)
                .show_counter(true)
                .width(gpui_kit::rems(26.0)),
        ))
        .child(section(
            theme,
            "Semantic types",
            "Search、Email、URL 决定读屏播报的角色",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueField::new(theme).label("搜索项目").input(
                        YueInput::new("input-type-search", theme, &showcase.input_semantic[0])
                            .input_type(InputType::Search)
                            .leading_icon(outline::Search)
                            .width(gpui_kit::rems(26.0)),
                    ),
                )
                .child(
                    YueField::new(theme).label("邮箱").input(
                        YueInput::new("input-type-email", theme, &showcase.input_semantic[1])
                            .input_type(InputType::Email)
                            .leading_icon(outline::Mail)
                            .width(gpui_kit::rems(26.0)),
                    ),
                )
                .child(
                    YueField::new(theme).label("主页").input(
                        YueInput::new("input-type-url", theme, &showcase.input_semantic[2])
                            .input_type(InputType::Url)
                            .leading_icon(outline::Link)
                            .width(gpui_kit::rems(26.0)),
                    ),
                ),
        ))
        .child(section(
            theme,
            "Locked combinations",
            "禁用与只读叠加前后缀、清除、密码、计数",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueInput::new("input-locked-affix", theme, &showcase.input_locked[0])
                        .leading_icon(outline::Tag)
                        .trailing("kg")
                        .clearable(true)
                        .disabled(true)
                        .width(gpui_kit::rems(26.0)),
                )
                .child(
                    YueInput::new("input-locked-password", theme, &showcase.input_locked[1])
                        .password(true)
                        .clearable(true)
                        .readonly(true)
                        .width(gpui_kit::rems(26.0)),
                )
                .child(
                    YueInput::new("input-locked-counter", theme, &showcase.input_locked[2])
                        .readonly(true)
                        .max_length(12)
                        .show_counter(true)
                        .width(gpui_kit::rems(26.0)),
                ),
        ))
        .child(section(
            theme,
            "Affixes and icons",
            "Leading and trailing slots accept any GPUI element",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueInput::new("input-search", theme, &showcase.input_affix)
                        .leading_icon(outline::Search)
                        .width(gpui_kit::rems(26.0)),
                )
                .child(
                    YueInput::new("input-unit", theme, &showcase.input_unit)
                        .trailing("kg")
                        .width(gpui_kit::rems(26.0)),
                ),
        ))
        .child(section(
            theme,
            "Text alignment",
            "Left、Center、Right",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueInput::new("input-align-left", theme, &showcase.input_alignments[0])
                        .text_align(InputTextAlign::Left)
                        .width(gpui_kit::rems(26.0)),
                )
                .child(
                    YueInput::new("input-align-center", theme, &showcase.input_alignments[1])
                        .text_align(InputTextAlign::Center)
                        .width(gpui_kit::rems(26.0)),
                )
                .child(
                    YueInput::new("input-align-right", theme, &showcase.input_alignments[2])
                        .text_align(InputTextAlign::Right)
                        .width(gpui_kit::rems(26.0)),
                ),
        ))
        .child(section(
            theme,
            "Textarea",
            "Multi-line editing with content-driven height",
            YueField::new(theme)
                .label("Description")
                .description("The field grows between three and six lines")
                .textarea(
                    YueTextarea::new("textarea-description", theme, &showcase.textarea)
                        .max_length(120)
                        .show_counter(true)
                        .width(gpui_kit::rems(32.0)),
                ),
        ))
        .child(section(
            theme,
            "Number input",
            "Arrow keys and step buttons adjust a bounded value",
            YueField::new(theme)
                .label("Quantity")
                .message(InputStatus::Error, "Enter a value from 0 to 100")
                .number_input(
                    YueNumberInput::new("number-quantity", theme, &showcase.number_input)
                        .width(gpui_kit::rems(16.0)),
                ),
        ))
}
