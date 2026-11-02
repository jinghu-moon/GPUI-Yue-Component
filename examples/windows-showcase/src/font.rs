//! The Font page: Yue's three families, the `--type-*` text styles and the
//! figures they set, all drawn from the fonts the crate embeds.
//!
//! Every sample reads the same tokens the web stylesheet declares, so a family
//! or a step retuned there shows up here after a sync.

use std::sync::Arc;

use gpui_kit::{
    FontFeatures, FontWeight, IntoElement, ParentElement as _, SharedString, Styled as _, div,
    prelude::FluentBuilder as _, px,
};
use yue_component::typography::{mono_font, number_font, ui_font};
use yue_component::{TextRole, YueTheme, tabular_nums};

const WEIGHTS: [(f32, &str); 4] = [
    (400.0, "Regular 400"),
    (500.0, "Medium 500"),
    (600.0, "Semibold 600"),
    (700.0, "Bold 700"),
];

const PANGRAM: &str = "悦动界面 Yue Design 让每一个像素各得其所 0123456789";

/// Figures that change in place, picked so a proportional face visibly drifts:
/// a run of ones is the narrowest a line can be, a run of eights the widest.
const FIGURES: [&str; 4] = ["1,111.11", "8,888.88", "4,107.25", "-0.01%"];

const CODE: [&str; 3] = [
    "fn main() -> Result<(), Error> {",
    "    let ratio = width != 0 && height >= 1 => 0x1F;",
    "}",
];

const DIFFERENCES: [(&str, &str, &str, &str); 4] = [
    (
        "字体来源",
        "@font-face 按需下载 woff2，加载前先用回退字体（font-display: swap）",
        "TTF 编译进 yue-component，启动时 load_fonts 一次注册，首帧即为最终字体",
        "无闪烁，但可执行文件多约 33 MB",
    ),
    (
        "回退链",
        "--font-number 缺字时回退到 HarmonyOS Sans SC，与正文同字形",
        "DirectWrite 的自定义回退只在系统字体中查找，TCloudNumber 缺的字符落到系统字体（如微软雅黑）",
        "只影响 --type-figure；单位等非数字字符放在 Figure 样式之外",
    ),
    (
        "通用族名",
        "system-ui、sans-serif、monospace 由浏览器解析",
        "通用族名不是真实字体，解析时丢弃；只保留具名字体",
        "回退链实际为 PingFang SC、Microsoft YaHei UI 等",
    ),
    (
        "字号单位",
        "px，跟随浏览器缩放",
        "按 16px = 1rem 换算成 rems，跟随窗口的 rem 尺寸",
        "默认缩放下两端一致",
    ),
];

pub(crate) fn font_content(theme: YueTheme) -> impl IntoElement {
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
                .child(div().text_2xl().child("Font"))
                .child(
                    div()
                        .text_color(gpui_kit::rgb(theme.text_secondary()))
                        .child("字体族、文字样式与数字排版；GPUI 与 Web 使用同一套字体文件和 token"),
                ),
        )
        .child(block(
            theme,
            "Families",
            "三套字体由 --font-ui、--font-mono、--font-number 指定，族名与 TTF 内的名称一致，两端用同一个 token。",
            families(theme),
        ))
        .child(block(
            theme,
            "Weights",
            "HarmonyOS Sans SC 内嵌 400 / 500 / 600 / 700 四个字重。正文 400，强调与控件 500，标题 600，展示 700。",
            weights(theme),
        ))
        .child(block(
            theme,
            "Text styles",
            "--type-* 是 CSS font 简写，一个名字同时给出字重、字号、行高和字体族；GPUI 端用 TextRole::apply 一次套用。",
            text_styles(theme),
        ))
        .child(block(
            theme,
            "Numbers",
            "数字分两种。和文字混排的数字用 --type-number（HarmonyOS），默认数字已等宽，粗细与正文一致；\
             单独展示的大号数字用 --type-figure（TCloudNumber 24px）。TCloudNumber 笔画接近等粗，\
             横笔与 HarmonyOS Medium 一样粗，同为 400 时明显更重，所以只用在大字号上。",
            numbers(theme),
        ))
        .child(block(
            theme,
            "Code",
            "--type-code：Fira Code 12px，行高沿用正文的 20px，便于与正文混排。",
            code(theme),
        ))
        .child(block(
            theme,
            "Differences from the web",
            "字体文件和 token 两端相同；以下是平台机制带来的差异。",
            differences(theme),
        ))
}

fn block(
    theme: YueTheme,
    title: &'static str,
    description: &'static str,
    example: impl IntoElement,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(div().text_lg().child(title))
        .child(
            div()
                .text_sm()
                .text_color(gpui_kit::rgb(theme.text_secondary()))
                .child(description),
        )
        .child(example)
}

fn stage(theme: YueTheme) -> gpui_kit::Div {
    div()
        .p_4()
        .rounded_md()
        .border_1()
        .border_color(gpui_kit::rgb(theme.border_default()))
        .bg(gpui_kit::rgb(theme.surface()))
}

fn caption(theme: YueTheme, text: impl Into<SharedString>) -> gpui_kit::Div {
    TextRole::Caption
        .apply(theme, div())
        .text_color(gpui_kit::rgb(theme.text_secondary()))
        .child(text.into())
}

fn families(theme: YueTheme) -> impl IntoElement {
    let row = |token: &'static str, family: SharedString, sample: &'static str| {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(caption(theme, format!("{token} · {family}")))
            .child(div().font_family(family).text_xl().child(sample))
    };
    stage(theme)
        .flex()
        .flex_col()
        .gap_4()
        .child(row("--font-ui", ui_font(theme).family, PANGRAM))
        .child(row(
            "--font-mono",
            mono_font(theme).family,
            "const yue = (a) => a !== 0 && a <= 0x1F; // -> != >=",
        ))
        .child(row(
            "--font-number",
            number_font(theme).family,
            "0123456789 +-.,%",
        ))
}

fn weights(theme: YueTheme) -> impl IntoElement {
    let family = ui_font(theme).family;
    stage(theme)
        .flex()
        .flex_col()
        .gap_3()
        .children(WEIGHTS.into_iter().map(|(weight, label)| {
            div()
                .flex()
                .items_baseline()
                .gap_4()
                .child(caption(theme, label).w_24().flex_shrink_0())
                .child(
                    div()
                        .font_family(family.clone())
                        .font_weight(FontWeight(weight))
                        .text_lg()
                        .child(PANGRAM),
                )
        }))
}

fn text_styles(theme: YueTheme) -> impl IntoElement {
    let sample = |role: TextRole| match role {
        TextRole::Display => "悦动界面 Yue",
        TextRole::Heading1 | TextRole::Heading2 | TextRole::Heading3 => "页面与区块标题",
        TextRole::Title => "卡片与对话框标题",
        TextRole::Label => "控件标签、按钮文字",
        TextRole::Code => "let total = items.len();",
        TextRole::Number => "共 1,234 项，已选 56 项",
        TextRole::Figure => "1,234,567.89",
        _ => "正文用于段落与说明文字，Body text for paragraphs.",
    };
    stage(theme)
        .p_0()
        .flex()
        .flex_col()
        .children(TextRole::ALL.into_iter().enumerate().map(|(index, role)| {
            let style = role.style(theme);
            let spec = format!(
                "{} · {}/{} · {}",
                style.weight, style.size, style.line_height, style.font.family
            );
            div()
                .flex()
                .items_center()
                .gap_4()
                .px_4()
                .py_3()
                .when(index > 0, |row| {
                    row.border_t_1()
                        .border_color(gpui_kit::rgb(theme.border_default()))
                })
                .child(
                    div()
                        .w_56()
                        .flex_shrink_0()
                        .flex()
                        .flex_col()
                        .child(TextRole::Label.apply(theme, div()).child(role.token_name()))
                        .child(caption(theme, spec)),
                )
                .child(
                    role.apply(theme, div())
                        .min_w_0()
                        .when(
                            matches!(role, TextRole::Number | TextRole::Figure),
                            |text| text.font_features(tabular_nums()),
                        )
                        .child(sample(role)),
                )
        }))
}

fn numbers(theme: YueTheme) -> impl IntoElement {
    let ui = ui_font(theme).family;
    let column = |title: &'static str, styled: &dyn Fn(gpui_kit::Div) -> gpui_kit::Div| {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(caption(theme, title).pb_1())
            .children(
                FIGURES
                    .into_iter()
                    .map(|figure| styled(div()).child(figure)),
            )
    };
    // Each row sets one size and line height across its columns, so only the
    // faces differ.
    let row = |title: &'static str, columns: Vec<gpui_kit::Div>| {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(TextRole::Label.apply(theme, div()).child(title))
            .child(div().flex().gap_12().children(columns))
    };
    let figure = |el: gpui_kit::Div| {
        TextRole::Figure
            .apply(theme, el)
            .font_features(tabular_nums())
    };
    stage(theme)
        .flex()
        .flex_col()
        .gap_6()
        .child(row(
            "混排数字 · 14px",
            vec![
                column("--type-number（HarmonyOS 默认）", &|el| {
                    TextRole::Number
                        .apply(theme, el)
                        .font_features(tabular_nums())
                }),
                column("HarmonyOS + pnum（比例数字，对照）", &|el| {
                    TextRole::Number
                        .apply(theme, el)
                        .font_features(FontFeatures(Arc::new(vec![("pnum".into(), 1)])))
                }),
                column("TCloudNumber 14px（不推荐）", &|el| {
                    figure(el)
                        .text_size(gpui_kit::rems(14. / 16.))
                        .line_height(px(20.))
                }),
            ],
        ))
        .child(row(
            "展示数字 · 24px",
            vec![
                column("--type-figure（TCloudNumber）", &figure),
                column("HarmonyOS 400", &|el| figure(el).font_family(ui.clone())),
                column("HarmonyOS 500", &|el| {
                    figure(el)
                        .font_family(ui.clone())
                        .font_weight(FontWeight(500.))
                }),
            ],
        ))
}

fn code(theme: YueTheme) -> impl IntoElement {
    stage(theme)
        .bg(gpui_kit::rgb(theme.surface_component()))
        .flex()
        .flex_col()
        .children(
            CODE.into_iter()
                .map(|line| TextRole::Code.apply(theme, div()).child(line)),
        )
}

fn differences(theme: YueTheme) -> impl IntoElement {
    let cell = |text: &'static str, width: gpui_kit::Rems| {
        div()
            .w(width)
            .flex_shrink_0()
            .px_2()
            .py_2()
            .text_sm()
            .child(text)
    };
    let header = div()
        .flex()
        .bg(gpui_kit::rgb(theme.surface_component()))
        .child(cell("方面", gpui_kit::rems(6.)))
        .child(cell("Web", gpui_kit::rems(20.)))
        .child(cell("GPUI", gpui_kit::rems(22.)))
        .child(cell("影响", gpui_kit::rems(18.)));
    stage(theme)
        .p_0()
        .flex()
        .flex_col()
        .child(header)
        .children(DIFFERENCES.into_iter().map(|(aspect, web, gpui, effect)| {
            div()
                .flex()
                .border_t_1()
                .border_color(gpui_kit::rgb(theme.border_default()))
                .child(cell(aspect, gpui_kit::rems(6.)))
                .child(cell(web, gpui_kit::rems(20.)))
                .child(cell(gpui, gpui_kit::rems(22.)))
                .child(
                    cell(effect, gpui_kit::rems(18.))
                        .text_color(gpui_kit::rgb(theme.text_secondary())),
                )
        }))
}
