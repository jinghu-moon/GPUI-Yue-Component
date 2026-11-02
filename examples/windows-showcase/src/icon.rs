//! The Icon page: which icons Yue draws itself, how big they sit next to text,
//! what their strokes render at, and the rules a hand-drawn icon must follow to
//! sit among them unnoticed.

use gpui_kit::{
    InteractiveElement as _, IntoElement, ParentElement as _, SharedString, Styled as _, div,
    prelude::FluentBuilder as _, px, rems,
};
use yue_component::{
    Icon, IconDefinition as _, IconSize, TextRole, YueIcon, YueTheme, filled,
    icon::rendered_stroke, outline,
};

/// The Tabler release `yue-component-icons` embeds and `@yue-ui/vue` pins.
const TABLER_VERSION: &str = "3.49.0";

const BOUNDARY: [&str; 4] = [
    "组件内置图标统一取自 Tabler outline，GPUI 与 Web 各自锁定同一版本，升级时分别进行。",
    "组件按语义取图标（YueIcon::Close），不直接写 outline::X；两端共享的是语义表本身，不是源文件。",
    "应用内容里的图标由应用自选，leading_icon 等接口接受任何 IconDefinition。",
    "Yue 不提供图标字体，也不在运行时加载图标；SVG 在编译期嵌入。",
];

const CANVAS_RULES: [&str; 3] = [
    "24×24 画布，墨迹（线条外缘）落在居中的 20×20 区域内，四周各留 2。",
    "线宽 2 时墨迹比路径中心线向外多 1，所以路径坐标落在 3–21 之间。",
    "按视觉重量补偿：圆形、三角形最多再外扩 1（墨迹不越过 1–23），方形略小于 20×20。",
];

const STROKE_RULES: [&str; 3] = [
    "stroke-width=\"2\"，stroke-linecap 与 stroke-linejoin 均为 round。",
    "fill=\"none\"，stroke=\"currentColor\"，不写任何具体颜色。",
    "不用 transform，不在 <g> 上挂样式，导出前展开为纯路径。",
];

const GEOMETRY_RULES: [&str; 5] = [
    "坐标落在 0.5 或 1 的网格上。",
    "角度取 45° 的倍数，必要时取 15° 的倍数。",
    "弧线从圆上截取，不画自由曲线。",
    "矩形圆角 rx=\"2\"。",
    "两条线最小间距 2；断口不大于 2，且为 0.5 的倍数。",
];

const NAMING_RULES: [&str; 3] = [
    "kebab-case，描述图形而非用途：cloud-upload，而不是 upload-button。",
    "独立命名空间（如 yue- 前缀），不与 Tabler 已有名称重名。",
    "先画 outline；只有状态确实需要区分时才补 filled。",
];

const TEMPLATE: [&str; 6] = [
    "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\"",
    "     fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\"",
    "     stroke-linecap=\"round\" stroke-linejoin=\"round\">",
    "  <path d=\"M3 10a7 7 0 1 0 14 0a7 7 0 1 0 -14 0\" />",
    "  <path d=\"M21 21l-6 -6\" />",
    "</svg>",
];

pub(crate) fn icon_content(theme: YueTheme) -> impl IntoElement {
    div()
        .debug_selector(|| "icon-design-content".into())
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
                .child(div().text_2xl().child("Icon"))
                .child(
                    div()
                        .text_color(gpui_kit::rgb(theme.text_secondary()))
                        .child(format!(
                            "组件内置图标统一使用 Tabler Icons {TABLER_VERSION}（MIT）；应用自己的图标自由选择"
                        )),
                ),
        )
        .child(block(
            theme,
            "Boundary",
            "Yue 只管自己画的图标；GPUI 与 Web 是两个独立发布的项目，靠文档化的约定对齐，没有构建期依赖。",
            bullets(theme, &BOUNDARY),
        ))
        .child(block(
            theme,
            "Semantic icons",
            "组件用到的全部图标。上行是语义名（两端一致），下行是对应的 Tabler 文件名。",
            semantic_icons(theme),
        ))
        .child(block(
            theme,
            "Size pairing",
            "图标尺寸跟随相邻文字：12/14px 文字配 16px，16px 配 20px，20px 及以上配 24px。\
             唯一例外是 Tag 的关闭图标，12px（--tag-close-icon-size）。",
            size_pairing(theme),
        ))
        .child(block(
            theme,
            "Stroke",
            "Tabler 在 24 网格上以线宽 2 绘制，SVG 编译期固定，线宽随尺寸等比缩放。\
             150% 缩放下 16px 图标的线宽正好是 2 个物理像素。目前不设线宽 token。",
            strokes(theme),
        ))
        .child(block(
            theme,
            "Outline and filled",
            "默认用线性图标；面性只表示选中、激活等状态。同一组图标不混用两种风格。",
            outline_and_filled(theme),
        ))
        .child(block(
            theme,
            "Drawing a custom icon",
            "先在 Tabler 中查找，确实没有再自制。自制图标放在 Tabler 旁边时应当看不出差别。\
             网格放大 8 倍：浅色区域是墨迹应落入的 20×20，细框是路径中心线的 3–21。\
             search 的墨迹正好贴住浅色区域；cloud-upload 是 Tabler 早期图标，墨迹横向 1.3–23.5，是反例。",
            custom_icon_rules(theme),
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

fn bullets(theme: YueTheme, items: &'static [&'static str]) -> gpui_kit::Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .children(items.iter().map(|item| {
            div()
                .flex()
                .gap_2()
                .child(
                    div()
                        .text_color(gpui_kit::rgb(theme.text_secondary()))
                        .child("•"),
                )
                .child(TextRole::Body.apply(theme, div()).child(*item))
        }))
}

fn semantic_icons(theme: YueTheme) -> impl IntoElement {
    // The page scrolls, so its width is not bounded and a wrapping row would
    // never wrap; the fourteen tiles are laid out as two fixed rows instead.
    let tile = |icon: YueIcon| {
        div()
            .w_32()
            .flex()
            .flex_col()
            .items_center()
            .gap_1()
            .p_3()
            .rounded_sm()
            .border_1()
            .border_color(gpui_kit::rgb(theme.border_default()))
            .child(
                Icon::new(icon)
                    .size(IconSize::Large.rems(theme))
                    .color(theme.text_primary()),
            )
            .child(
                TextRole::Label
                    .apply(theme, div())
                    .pt_1()
                    .child(icon.semantic_name()),
            )
            .child(caption(theme, icon.name()))
    };
    stage(theme).flex().flex_col().gap_3().children(
        YueIcon::ALL
            .chunks(7)
            .map(|row| div().flex().gap_3().children(row.iter().copied().map(tile))),
    )
}

fn size_pairing(theme: YueTheme) -> impl IntoElement {
    let rows = [
        (TextRole::Caption, "辅助说明"),
        (TextRole::Body, "正文与控件文字"),
        (TextRole::Title, "卡片与对话框标题"),
        (TextRole::Heading3, "区块标题"),
        (TextRole::Heading2, "页面标题"),
    ];
    stage(theme)
        .flex()
        .flex_col()
        .gap_4()
        .children(rows.into_iter().map(|(role, sample)| {
            let text = role.style(theme).size;
            let step = IconSize::for_text(text);
            div()
                .flex()
                .items_center()
                .gap_6()
                .child(
                    caption(
                        theme,
                        format!(
                            "{text}px 文字 → {}px 图标 · {}",
                            step.pixels(theme),
                            step.token_name()
                        ),
                    )
                    .w_64()
                    .flex_shrink_0(),
                )
                .child(
                    role.apply(theme, div())
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            Icon::new(YueIcon::Search)
                                .size(step.rems(theme))
                                .color(theme.text_primary()),
                        )
                        .child(sample)
                        .child(
                            Icon::new(YueIcon::ChevronDown)
                                .size(step.rems(theme))
                                .color(theme.text_secondary()),
                        ),
                )
        }))
}

fn strokes(theme: YueTheme) -> impl IntoElement {
    stage(theme)
        .flex()
        .items_end()
        .gap_12()
        .children(IconSize::ALL.into_iter().map(|step| {
            let size = step.pixels(theme);
            let stroke = rendered_stroke(size);
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            Icon::new(YueIcon::Close)
                                .size(step.rems(theme))
                                .color(theme.text_primary()),
                        )
                        .child(
                            Icon::new(YueIcon::Check)
                                .size(step.rems(theme))
                                .color(theme.text_primary()),
                        )
                        .child(
                            Icon::new(YueIcon::Info)
                                .size(step.rems(theme))
                                .color(theme.text_primary()),
                        ),
                )
                .child(
                    TextRole::Label
                        .apply(theme, div())
                        .child(format!("{size}px")),
                )
                .child(caption(
                    theme,
                    format!("线宽 {stroke:.2}px · 150% 下 {:.1} 物理像素", stroke * 1.5),
                ))
        }))
}

fn outline_and_filled(theme: YueTheme) -> impl IntoElement {
    let tile = |icon: Icon, active: bool| {
        div()
            .size_10()
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .when(active, |tile| tile.bg(gpui_kit::rgb(theme.accent_subtle())))
            // GPUI's svg does not inherit the text colour, so the icon is
            // coloured directly.
            .child(icon.size(IconSize::Small.rems(theme)).color(if active {
                theme.accent_subtle_text()
            } else {
                theme.text_primary()
            }))
    };
    let group = |title: &'static str, note: &'static str, ok: bool, tiles: Vec<gpui_kit::Div>| {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Icon::new(if ok { YueIcon::Check } else { YueIcon::Close })
                            .size(IconSize::Small.rems(theme))
                            .color(if ok {
                                theme.accent_text()
                            } else {
                                theme.button_danger_background()
                            }),
                    )
                    .child(TextRole::Label.apply(theme, div()).child(title)),
            )
            .child(div().flex().gap_1().children(tiles))
            .child(caption(theme, note))
    };
    stage(theme)
        .flex()
        .gap_12()
        .child(group(
            "线性为主，选中项用面性",
            "工具栏全为 outline，只有当前选中的“收藏”换成 filled。",
            true,
            vec![
                tile(Icon::new(outline::Home), false),
                tile(Icon::new(filled::Star), true),
                tile(Icon::new(outline::Bookmark), false),
                tile(Icon::new(outline::Bell), false),
            ],
        ))
        .child(group(
            "同组混用",
            "无状态差异却混用 outline 与 filled，读起来像是有两个被选中。",
            false,
            vec![
                tile(Icon::new(outline::Home), false),
                tile(Icon::new(filled::Star), false),
                tile(Icon::new(outline::Bookmark), false),
                tile(Icon::new(filled::Bell), false),
            ],
        ))
}

/// One grid unit, drawn at eight times its size.
const SCALE: f32 = 8.0;

fn grid(
    theme: YueTheme,
    icon: Icon,
    fits: bool,
    title: &'static str,
    note: &'static str,
) -> impl IntoElement {
    let side = 24.0 * SCALE;
    let line = |vertical: bool, at: usize| {
        let offset = px(at as f32 * SCALE);
        let major = at.is_multiple_of(4);
        let base = div()
            .absolute()
            .bg(gpui_kit::rgb(theme.border_default()))
            .opacity(if major { 1.0 } else { 0.45 });
        if vertical {
            base.left(offset).top_0().w(px(1.)).h(px(side))
        } else {
            base.top(offset).left_0().h(px(1.)).w(px(side))
        }
    };
    // The area at `inset` grid units from every edge.
    let area = |inset: f32| {
        div()
            .absolute()
            .left(px(inset * SCALE))
            .top(px(inset * SCALE))
            .size(px((24. - 2. * inset) * SCALE))
    };
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .gap_2()
                .items_center()
                .child(
                    Icon::new(if fits { YueIcon::Check } else { YueIcon::Close })
                        .size(IconSize::Small.rems(theme))
                        .color(if fits {
                            theme.accent_text()
                        } else {
                            theme.button_danger_background()
                        }),
                )
                .child(TextRole::Label.apply(theme, div()).child(title)),
        )
        .child(
            div()
                .relative()
                .size(px(side + 1.))
                .bg(gpui_kit::rgb(theme.surface()))
                .child(area(2.).bg(gpui_kit::rgb(theme.accent_subtle())))
                .children((0..=24).map(|at| line(true, at)))
                .children((0..=24).map(|at| line(false, at)))
                .child(
                    area(3.)
                        .border_1()
                        .border_color(gpui_kit::rgb(theme.accent_solid())),
                )
                .child(
                    div()
                        .absolute()
                        .left_0()
                        .top_0()
                        .child(icon.size(rems(side / 16.)).color(theme.text_primary())),
                ),
        )
        .child(caption(theme, note))
        .child(
            div().flex().items_end().gap_3().pt_1().children(
                IconSize::ALL
                    .into_iter()
                    .map(|step| icon.size(step.rems(theme)).color(theme.text_primary())),
            ),
        )
}

fn custom_icon_rules(theme: YueTheme) -> impl IntoElement {
    let group = |title: &'static str, items: &'static [&'static str]| {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(TextRole::Label.apply(theme, div()).child(title))
            .child(bullets(theme, items))
    };
    stage(theme)
        .flex()
        .flex_col()
        .gap_6()
        .child(
            div()
                .flex()
                .gap_8()
                .child(
                    div()
                        .flex_shrink_0()
                        .flex()
                        .gap_6()
                        .child(grid(
                            theme,
                            Icon::new(outline::Search),
                            true,
                            "search",
                            "墨迹 2–22，正好填满",
                        ))
                        .child(grid(
                            theme,
                            Icon::new(outline::CloudUpload),
                            false,
                            "cloud-upload",
                            "墨迹 1.3–23.5，左右溢出",
                        )),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_4()
                        .child(group("画布", &CANVAS_RULES))
                        .child(group("线条", &STROKE_RULES))
                        .child(group("几何", &GEOMETRY_RULES))
                        .child(group("命名与组织", &NAMING_RULES)),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(TextRole::Label.apply(theme, div()).child("SVG 模板"))
                .child(
                    div()
                        .p_3()
                        .rounded_md()
                        .bg(gpui_kit::rgb(theme.surface_component()))
                        .flex()
                        .flex_col()
                        .children(
                            TEMPLATE
                                .into_iter()
                                .map(|line| TextRole::Code.apply(theme, div()).child(line)),
                        ),
                ),
        )
}
