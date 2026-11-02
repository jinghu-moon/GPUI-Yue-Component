//! Executable Icon API examples; visual conventions live on the design page.
use super::super::*;
use yue_component::{Icon, IconSize, TextRole, YueIcon, filled};

pub(crate) fn icon_content(theme: YueTheme) -> impl IntoElement {
    div()
        .debug_selector(|| "icon-component-content".into())
        .w_full()
        .flex()
        .flex_col()
        .gap_6()
        .p_8()
        .child(div().text_2xl().child("Icon"))
        .child(
            div()
                .text_color(gpui_kit::rgb(theme.text_secondary()))
                .child("图标组件 API 与组合示例；尺寸配对、线宽与绘制规范见「设计 / Icon」"),
        )
        .child(section(
            theme,
            "Basic",
            "use yue_component::{Icon, outline, filled};",
            div().flex().items_center().gap_6()
                .child(sample(theme, "Icon::new(outline::Search)", Icon::new(outline::Search).color(theme.text_primary())))
                .child(sample(theme, "Icon::new(filled::Star)", Icon::new(filled::Star).color(theme.accent_text())))
                .child(sample(theme, "Icon::new(YueIcon::Close)", Icon::new(YueIcon::Close).color(theme.text_primary()))),
        ))
        .child(section(
            theme,
            "Size",
            ".size(IconSize::Small.rems(theme))；支持 Small、Medium、Large，也可传入自定义 Rems",
            div().flex().items_end().gap_6().children(IconSize::ALL.into_iter().map(|size| {
                sample(theme, size.token_name(), Icon::new(outline::Search).size(size.rems(theme)).color(theme.text_primary()))
            })),
        ))
        .child(section(
            theme,
            "Color and rotation",
            ".color(theme.accent_text()) 设置颜色；.rotation(0.25) 绕中心旋转四分之一圈，不改变布局尺寸",
            div().flex().items_center().gap_6()
                .child(sample(theme, "Primary text", Icon::new(outline::ChevronRight).color(theme.text_primary())))
                .child(sample(theme, "Accent text", Icon::new(outline::ChevronRight).color(theme.accent_text())))
                .child(sample(theme, "Quarter turn", Icon::new(outline::ChevronRight).color(theme.text_primary()).rotation(0.25))),
        ))
        .child(section(
            theme,
            "Composition",
            "图标本身用于绘制；交互交给 YueButton。纯图标按钮必须提供可访问名称",
            div().flex().items_center().gap_3()
                .child(YueButton::icon("icon-api-search", theme, outline::Search, "Search").tooltip("搜索"))
                .child(YueButton::icon_text("icon-api-add", theme, outline::Plus, "新建"))
                .child(
                    div().flex().items_center().gap_2()
                        .child(Icon::new(YueIcon::Info).size(IconSize::Small.rems(theme)).color(theme.text_secondary()))
                        .child("图标与说明文字"),
                ),
        ))
}

fn sample(theme: YueTheme, label: &'static str, icon: Icon) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap_2()
        .child(icon)
        .child(TextRole::Caption.apply(theme, div()).child(label))
}
