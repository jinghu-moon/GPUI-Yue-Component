use super::super::*;

pub(crate) fn content(theme: YueTheme) -> impl IntoElement {
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
                .child(div().text_2xl().child("Button"))
                .child(
                    div()
                        .text_color(gpui_kit::rgb(theme.text_secondary()))
                        .child("按钮的三种内容形态、变体与交互状态"),
                ),
        )
        .child(section(
            theme,
            "Content forms",
            "文本、图标、图标加文本",
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    YueButton::text("button-text", theme, "Save").variant(ButtonVariant::Primary),
                )
                .child(
                    YueButton::icon("button-icon", theme, outline::Search, "Search")
                        .variant(ButtonVariant::Outline),
                )
                .child(YueButton::icon_text(
                    "button-icon-text",
                    theme,
                    outline::Plus,
                    "Add",
                )),
        ))
        .child(section(
            theme,
            "Variants",
            "Default、Primary、Outline、Ghost、Text、Danger",
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(YueButton::text("default", theme, "Default"))
                .child(YueButton::text("primary", theme, "Primary").variant(ButtonVariant::Primary))
                .child(YueButton::text("outline", theme, "Outline").variant(ButtonVariant::Outline))
                .child(YueButton::text("ghost", theme, "Ghost").variant(ButtonVariant::Ghost))
                .child(YueButton::text("text-variant", theme, "Text").variant(ButtonVariant::Text))
                .child(YueButton::text("danger", theme, "Delete").variant(ButtonVariant::Danger)),
        ))
        .child(section(
            theme,
            "Sizes and shapes",
            "Small、Medium、Large 与圆角形状",
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(YueButton::text("small", theme, "Small").size(ButtonSize::Small))
                .child(YueButton::text("medium", theme, "Medium"))
                .child(YueButton::text("large", theme, "Large").size(ButtonSize::Large))
                .child(
                    YueButton::icon("circle", theme, outline::Plus, "Add")
                        .shape(ButtonShape::Circle),
                )
                .child(
                    YueButton::icon("square", theme, outline::Settings, "Settings")
                        .shape(ButtonShape::RoundedSquare),
                )
                .child(YueButton::text("pill", theme, "Pill").shape(ButtonShape::Pill)),
        ))
        .child(section(
            theme,
            "Icon position",
            "图标可以位于文本前或文本后",
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(YueButton::icon_text(
                    "leading",
                    theme,
                    outline::ArrowRight,
                    "Continue",
                ))
                .child(
                    YueButton::icon_text("trailing", theme, outline::ArrowRight, "Continue")
                        .icon_position(IconPosition::Trailing),
                ),
        ))
        .child(section(
            theme,
            "Loading and tooltip",
            "Loading 会阻止重复激活并保留按钮尺寸",
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    YueButton::icon_text("loading", theme, outline::DeviceFloppy, "Saving")
                        .loading(true),
                )
                .child(
                    YueButton::text("tooltip", theme, "Hover me")
                        .tooltip("Saves the current document"),
                ),
        ))
        .child(section(
            theme,
            "Block",
            "占满父容器宽度",
            YueButton::text("block", theme, "Full width").block(),
        ))
        .child(section(
            theme,
            "Button group",
            "组合布局不改变成员按钮的交互与可访问性",
            ButtonGroup::new(theme)
                .child(YueButton::text("group-left", theme, "Day"))
                .child(YueButton::text("group-mid", theme, "Week"))
                .child(YueButton::text("group-right", theme, "Month")),
        ))
        .child(section(
            theme,
            "Disabled",
            "禁用态保留可读性，并阻止交互",
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(YueButton::text("disabled-text", theme, "Disabled").disabled(true))
                .child(
                    YueButton::icon_text("disabled-icon-text", theme, outline::Plus, "Disabled")
                        .disabled(true),
                ),
        ))
        .child(section(
            theme,
            "Theme profiles",
            "Yue Design System profiles",
            div()
                .flex()
                .gap_3()
                .child(profile_card(
                    "Light / Azure",
                    YueTheme::new(ThemeMode::Light, Accent::Azure),
                ))
                .child(profile_card(
                    "Dark / Azure",
                    YueTheme::new(ThemeMode::Dark, Accent::Azure),
                ))
                .child(profile_card(
                    "Light / Neutral",
                    YueTheme::new(ThemeMode::Light, Accent::Neutral),
                ))
                .child(profile_card(
                    "Dark / Neutral",
                    YueTheme::new(ThemeMode::Dark, Accent::Neutral),
                )),
        ))
}

pub(crate) fn profile_card(title: &'static str, theme: YueTheme) -> impl IntoElement {
    div()
        .w_40()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(gpui_kit::rgb(theme.border_default()))
        .bg(gpui_kit::rgb(theme.surface()))
        .text_color(gpui_kit::rgb(theme.text_primary()))
        .child(title)
        .child(
            div()
                .p_2()
                .rounded_sm()
                .bg(gpui_kit::rgb(theme.accent_solid()))
                .text_color(gpui_kit::rgb(theme.on_accent()))
                .child("Accent"),
        )
}
