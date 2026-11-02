//! The Layout page: the navigation skeletons, the ways a skeleton fills its
//! content area, and one simulated window whose width decides the sidebar fold
//! and the card count.
//!
//! The wireframes are sketches at a fixed small scale. Every measured size —
//! the simulated window, the readings, the captions — comes from a `--layout-*`
//! token through `yue_component::layout`: rule 7 of `layout.md` says a
//! skeleton's widths are never written as a literal 232, 64 or 56, and a demo
//! that hardcoded them would contradict what it teaches.

use gpui_kit::base::slider::{Slider, SliderIndicator, SliderState, SliderThumb, SliderTrack};
use gpui_kit::base::transition;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    Context, IntoElement, ParentElement as _, Pixels, SharedString, Styled as _, Window, div, px,
    rgb,
};
use yue_component::layout::{
    Breakpoint, container_page, container_text, grid_columns, grid_fit, grid_gap, header_height,
    page_gutter, page_gutter_for, page_gutter_lg, page_gutter_md, pane_width, sidebar_collapse_at,
    sidebar_collapsed, sidebar_transition, sidebar_width, sidebar_width_collapsed,
};
use yue_component::motion::MotionDuration;
use yue_component::{Accent, Icon, IconSize, TextRole, ThemeMode, YueIcon, YueTheme};

use crate::Showcase;

/// Narrowest window the slider reaches: a phone-sized viewport, where Yue keeps
/// flowing rather than switching on a horizontal scrollbar.
pub(crate) const SIM_MIN: f32 = 360.;
/// A 1920px window, where the fixed page container has stopped growing.
pub(crate) const SIM_MAX: f32 = 1920.;
/// Where the slider starts: a roomy desktop window, sidebar open.
pub(crate) const SIM_DEFAULT: f32 = 1280.;

/// The narrowest card the demo grid declares. The count is derived from it, so
/// the number on screen is `grid_fit`'s answer, not a choice the page made.
const CARD_MIN: f32 = 240.;

/// The simulated window is drawn smaller than it measures: at full scale a 1920px
/// window would be wider than the showcase's own content area.
const DRAW: f32 = 0.42;

const RULES: [&str; 8] = [
    "页面内容使用最大宽度容器，长文本不要铺满宽屏。",
    "小屏优先使用流式布局和换行，不能依赖横向滚动承载主要内容。",
    "栅格用于页面编排，不应让组件内部出现与页面耦合的媒体查询。",
    "间距优先从 --space-* 和 --gap-* 选择；只有无法表达的特殊几何才增加新 Token。",
    "触控目标和按钮高度由组件 Token 保证，页面布局不能用负 margin 抵消它们。",
    "窗口宽度小于 --layout-sidebar-collapse-at 时侧栏自动收起；收起后保留图标栏与点击区，不做 0 宽。",
    "骨架的宽高只从 Token 取，页面里不出现写死的 232、64、56。",
    "Yue 不设「页面最小宽度」：窄下去继续走流式，桌面端真要限制宽度用 WindowOptions::min_size。",
];

/// The slider state the Layout page drags, seeded at the default width.
///
/// `max` goes first: each setter re-clamps the thumb against the current range,
/// and the default range ends at 100, so `min(360.)` before it panics.
pub(crate) fn simulated_width_state() -> SliderState {
    SliderState::new()
        .max(SIM_MAX)
        .min(SIM_MIN)
        .step(8.)
        .default_value(SIM_DEFAULT)
}

pub(crate) fn layout_content(
    showcase: &Showcase,
    window: &mut Window,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
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
                .child(div().text_2xl().child("Layout"))
                .child(
                    div()
                        .text_color(rgb(theme.text_secondary()))
                        .child(
                            "骨架决定页面留出哪些区域，断点决定这些区域怎么变；\
                             线框只是示意，模拟窗口与读数里的每个尺寸都来自 --layout-* Token。",
                        ),
                ),
        )
        .child(block(
            theme,
            "导航骨架",
            "导航决定用户在页面间移动的效率。前三张面向网页，第四张是 Yue 为 GPUI 桌面端补充的工作台：\
             每个区域可折叠、可拖宽，键盘直达。",
            skeletons(theme),
        ))
        .child(block(
            theme,
            "内容排布",
            "骨架定下内容区的边界，剩下的问题是怎么用它。每张卡片下方写明它靠哪个 Token 定宽。",
            arrangements(theme),
        ))
        .child(block(
            theme,
            "断点与自动收起",
            "拖动滑块模拟窗口宽度：断点条实时切换，页边距随档位走 16 → 24 → 32；宽度落到 \
             --layout-sidebar-collapse-at 以下时侧栏用 stow 收进图标栏，回到阈值以上时按 enter 展开；\
             内容区里不小于 240px 的卡片按 grid_fit 重新数列。",
            responsive_demo(showcase, theme, window, cx),
        ))
        .child(block(
            theme,
            "Token 读数",
            "本页每一格实际读到的值。改 packages/tokens 里的一个数，网页端与这里的骨架一起动。",
            token_readings(theme),
        ))
        .child(block(
            theme,
            "规则",
            "layout.md 里那七条，加上「不设页面最小宽度」：",
            bullets(theme, &RULES),
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
                .max_w(px(1100.))
                .text_sm()
                .text_color(rgb(theme.text_secondary()))
                .child(description),
        )
        .child(example)
}

fn stage(theme: YueTheme) -> gpui_kit::Div {
    div()
        .p_4()
        .rounded_md()
        .border_1()
        .border_color(rgb(theme.border_default()))
        .bg(rgb(theme.surface()))
}

fn caption(theme: YueTheme, text: impl Into<SharedString>) -> gpui_kit::Div {
    TextRole::Caption
        .apply(theme, div())
        .text_color(rgb(theme.text_secondary()))
        .child(text.into())
}

fn bullets(theme: YueTheme, items: &[&'static str]) -> gpui_kit::Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .children(items.iter().map(|item| {
            div()
                .flex()
                .gap_2()
                .child(div().text_color(rgb(theme.text_secondary())).child("•"))
                .child(
                    TextRole::Body
                        .apply(theme, div())
                        .max_w(px(1100.))
                        .child(*item),
                )
        }))
}

fn chip(theme: YueTheme, text: impl Into<SharedString>, strong: bool) -> gpui_kit::Div {
    TextRole::Caption
        .apply(theme, div())
        .px_2()
        .py_1()
        .rounded_sm()
        .when(strong, |chip| {
            chip.bg(rgb(theme.accent_subtle()))
                .text_color(rgb(theme.accent_subtle_text()))
        })
        .when(!strong, |chip| {
            chip.border_1()
                .border_color(rgb(theme.border_default()))
                .text_color(rgb(theme.text_secondary()))
        })
        .child(text.into())
}

/// One region of a wireframe: a bordered box with its name inside.
fn cell(text: impl Into<SharedString>, theme: YueTheme) -> gpui_kit::Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .overflow_hidden()
        .border_1()
        .border_color(rgb(theme.border_default()))
        .bg(rgb(theme.surface()))
        .text_size(px(9.))
        .line_height(px(12.))
        .text_color(rgb(theme.text_secondary()))
        .child(text.into())
}

/// The window outline a wireframe is drawn inside.
fn outline(theme: YueTheme, width: f32, height: f32) -> gpui_kit::Div {
    div()
        .w(px(width))
        .h(px(height))
        .flex()
        .flex_col()
        .overflow_hidden()
        .rounded(px(4.))
        .border_1()
        .border_color(rgb(theme.text_secondary()))
        .bg(rgb(theme.page()))
}

/// A labelled skeleton card: the wireframe, then what it is and how it trades off.
fn skeleton(
    theme: YueTheme,
    name: &'static str,
    structure: &'static str,
    body: impl IntoElement,
) -> gpui_kit::Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(TextRole::Label.apply(theme, div()).child(name))
        .child(body)
        .child(caption(theme, structure))
}

fn skeletons(theme: YueTheme) -> impl IntoElement {
    let frame = || outline(theme, 208., 124.);
    div()
        .flex()
        .flex_wrap()
        .gap_6()
        .child(skeleton(
            theme,
            "侧边导航",
            "顶部区 + 侧边区 + 内容区 · 切换效率高，压缩横向空间",
            frame()
                .child(cell("顶部区", theme).w_full().h(px(18.)))
                .child(
                    div()
                        .flex()
                        .flex_1()
                        .min_h_0()
                        .child(cell("侧栏", theme).w(px(56.)))
                        .child(cell("内容区", theme).flex_1()),
                ),
        ))
        .child(skeleton(
            theme,
            "顶部导航",
            "顶部区 + 内容区 · 横向利用率高，内容常设为定宽",
            frame()
                .child(cell("顶部区 · 一级导航", theme).w_full().h(px(18.)))
                .child(
                    div()
                        .flex()
                        .flex_1()
                        .min_h_0()
                        .justify_center()
                        .child(cell("内容区 · 定宽", theme).w(px(124.))),
                ),
        ))
        .child(skeleton(
            theme,
            "混合导航",
            "顶部区放一级，侧边区放二级 · 各司其职，多占一块空间",
            frame()
                .child(
                    div()
                        .flex()
                        .w_full()
                        .h(px(18.))
                        .child(cell("Logo", theme).w(px(40.)))
                        .child(cell("一级导航", theme).flex_1()),
                )
                .child(
                    div()
                        .flex()
                        .flex_1()
                        .min_h_0()
                        .child(cell("二级", theme).w(px(46.)))
                        .child(cell("内容区", theme).flex_1()),
                ),
        ))
        .child(skeleton(
            theme,
            "桌面工作台",
            "标题栏 + 图标栏 + 侧栏 + 主区 + 底部面板 / 检查器",
            frame()
                .child(cell("标题栏", theme).w_full().h(px(15.)))
                .child(
                    div()
                        .flex()
                        .flex_1()
                        .min_h_0()
                        .child(cell("图标栏", theme).w(px(20.)))
                        .child(cell("侧栏", theme).w(px(48.)))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .child(cell("主区", theme).flex_1())
                                .child(cell("底部面板", theme).h(px(24.))),
                        )
                        .child(cell("检查器", theme).w(px(40.))),
                ),
        ))
}

/// One content-arrangement card: the wireframe over its name and its measure.
fn arrangement(
    theme: YueTheme,
    name: &'static str,
    measure: impl Into<SharedString>,
    body: impl IntoElement,
) -> gpui_kit::Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(body)
        .child(TextRole::Caption.apply(theme, div()).child(name))
        .child(caption(theme, measure))
}

fn arrangements(theme: YueTheme) -> impl IntoElement {
    let frame = || outline(theme, 152., 84.);
    div()
        .flex()
        .flex_wrap()
        .gap_6()
        .child(arrangement(
            theme,
            "单栏阅读",
            format!("正文 --container-text {}", px_text(container_text(theme))),
            frame().child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .justify_center()
                    .child(cell("正文", theme).w(px(62.))),
            ),
        ))
        .child(arrangement(
            theme,
            "定宽居中",
            format!(
                "--container-page {} + {} 边距",
                px_text(container_page(theme)),
                px_text(page_gutter_md(theme))
            ),
            frame().child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .justify_center()
                    .items_center()
                    .child(cell("内容", theme).w(px(104.)).h(px(52.))),
            ),
        ))
        .child(arrangement(
            theme,
            "栅格与卡片网格",
            format!("{} 列，槽宽流动", grid_columns(theme)),
            frame().child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .gap(px(3.))
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .min_h_0()
                            .gap(px(3.))
                            .child(cell("", theme).flex_1())
                            .child(cell("", theme).flex_1())
                            .child(cell("", theme).flex_1()),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .min_h_0()
                            .gap(px(3.))
                            .child(cell("", theme).flex_1())
                            .child(cell("", theme).flex_1())
                            .child(cell("", theme).flex_1()),
                    ),
            ),
        ))
        .child(arrangement(
            theme,
            "列表–详情",
            format!("列表 --layout-pane-width {}", px_text(pane_width(theme))),
            frame().child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(cell("列表", theme).w(px(40.)))
                    .child(cell("详情", theme).flex_1()),
            ),
        ))
        .child(arrangement(
            theme,
            "三栏",
            "导航｜列表｜详情 · 邮件与即时通讯",
            frame().child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(cell("导航", theme).w(px(28.)))
                    .child(cell("列表", theme).w(px(38.)))
                    .child(cell("详情", theme).flex_1()),
            ),
        ))
        .child(arrangement(
            theme,
            "主区 + 检查器",
            format!(
                "检查器同样落在 --layout-pane-width {}",
                px_text(pane_width(theme))
            ),
            frame().child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(cell("主区", theme).flex_1())
                    .child(cell("检查器", theme).w(px(40.))),
            ),
        ))
        .child(arrangement(
            theme,
            "可拖拽分栏",
            "分割线可拖动，松手后 spring 落位",
            frame().child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(cell("编辑器", theme).flex_1())
                    .child(
                        div()
                            .w(px(3.))
                            .h_full()
                            .bg(rgb(theme.text_secondary()))
                            .flex_shrink_0(),
                    )
                    .child(cell("预览", theme).flex_1()),
            ),
        ))
        .child(arrangement(
            theme,
            "表单与设置页",
            "表单区不超过 --container-text",
            frame().child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(cell("分类", theme).w(px(36.)))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .gap(px(4.))
                            .p_2()
                            .children((0..4).map(|_| {
                                div()
                                    .h(px(7.))
                                    .w_full()
                                    .rounded(px(2.))
                                    .bg(rgb(theme.surface()))
                            })),
                    ),
            ),
        ))
        .child(arrangement(
            theme,
            "居中卡片",
            "登录、向导、空状态：400–480px",
            frame().child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .items_center()
                    .justify_center()
                    .child(cell("卡片", theme).w(px(66.)).h(px(50.))),
            ),
        ))
}

fn responsive_demo(
    showcase: &Showcase,
    theme: YueTheme,
    window: &mut Window,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let sim = showcase.layout_sim.round();
    let width = px(sim);
    let band = Breakpoint::of(theme, width);
    let collapsed = sidebar_collapsed(theme, width);
    // Folding is a stow — the panel stays one click away, so it leaves on the
    // standard curve instead of accelerating off — and unfolding is an entrance.
    // Base reads the policy each frame, so the target's direction picks the pair.
    let fold = transition(
        "layout-sidebar-fold",
        if collapsed { 1.0 } else { 0.0 },
        sidebar_transition(theme, collapsed),
        window,
        cx,
    );
    let open = sidebar_width(theme).as_f32();
    let rail = sidebar_width_collapsed(theme).as_f32();
    let sidebar = open - (open - rail) * fold;
    let gutter = page_gutter_for(theme, width).as_f32();
    let gap = grid_gap(theme).as_f32();
    let available = (sim - sidebar - 2. * gutter).max(0.);
    let columns = grid_fit(theme, px(available), px(CARD_MIN));

    stage(theme)
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .flex()
                .items_center()
                .gap_4()
                .child(TextRole::Label.apply(theme, div()).child("模拟窗口宽度"))
                .child(
                    div()
                        .w(px(380.))
                        .child(width_slider(&showcase.layout_width, theme, cx)),
                )
                .child(caption(theme, format!("{sim:.0}px"))),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .children(Breakpoint::ALL.iter().map(|b| {
                    let start = b.min_width(theme).as_f32();
                    chip(
                        theme,
                        if start == 0. {
                            format!(
                                "{} · 窄于 {}",
                                b.label(),
                                px_text(Breakpoint::Sm.min_width(theme))
                            )
                        } else {
                            format!("{} · {}", b.label(), px_text(px(start)))
                        },
                        *b == band,
                    )
                }))
                .child(chip(
                    theme,
                    format!("侧栏收起于 {}", px_text(sidebar_collapse_at(theme))),
                    sidebar_collapsed(theme, width),
                ))
                .child(chip(
                    theme,
                    format!("页边距 {}", px_text(page_gutter_for(theme, width))),
                    true,
                )),
        )
        .child(
            outline(theme, sim * DRAW, 150.)
                .child(
                    cell(format!("顶部区 {}", px_text(header_height(theme))), theme)
                        .w_full()
                        .h(px(header_height(theme).as_f32() * DRAW)),
                )
                .child(
                    div()
                        .flex()
                        .flex_1()
                        .min_h_0()
                        .child(
                            div()
                                .w(px(sidebar * DRAW))
                                .h_full()
                                .flex()
                                .items_center()
                                .gap_1()
                                .overflow_hidden()
                                .px(px(3.))
                                .border_r_1()
                                .border_color(rgb(theme.border_default()))
                                .bg(rgb(theme.surface()))
                                .child(
                                    div().flex_shrink_0().child(
                                        Icon::new(YueIcon::ChevronRight)
                                            .size(IconSize::Small.rems(theme))
                                            .color(theme.text_secondary()),
                                    ),
                                )
                                .child(
                                    div()
                                        .text_size(px(9.))
                                        .line_height(px(12.))
                                        .text_color(rgb(theme.text_secondary()))
                                        .opacity(1.0 - fold)
                                        .child(format!("{:.0}", sidebar)),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_1()
                                .min_w_0()
                                .items_center()
                                .px(px(gutter * DRAW))
                                .child(div().flex().w_full().gap(px(gap * DRAW)).children(
                                    (0..columns).map(|_| cell("", theme).flex_1().h(px(64.))),
                                )),
                        ),
                ),
        )
        .child(caption(
            theme,
            format!(
                "内容区 {:.0}px = 窗口 {sim:.0} − 侧栏 {sidebar:.0} − 2 × {gutter:.0}；\
                 每列不小于 {CARD_MIN:.0}px 时放得下 {columns} 列（栅格共 {} 列）",
                available,
                grid_columns(theme)
            ),
        ))
        .child(band_changes(theme, width))
}

/// The 布局变化 column of `layout.md`, with the thresholds the simulated window
/// has already passed lit up: a min-width query keeps everything below it too,
/// so each row says what changes from that width up.
fn band_changes(theme: YueTheme, width: Pixels) -> impl IntoElement {
    const CHANGES: [(Breakpoint, &str, &str); 4] = [
        (
            Breakpoint::Sm,
            "列表–详情恢复并排；窄于这一档时单栏到底，列表栏与检查器改为覆盖层",
            "TDesign —",
        ),
        (
            Breakpoint::Md,
            "卡片与内容块开始多列并排；页边距 16 → 24；侧栏仍收在图标栏",
            "TDesign sm 768px",
        ),
        (
            Breakpoint::Lg,
            "侧栏展开；--layout-sidebar-collapse-at 指向这一档",
            "TDesign md 992px",
        ),
        (
            Breakpoint::Xl,
            "页边距 24 → 32；顶部导航下 --container-page 不再被窗口挤压，居中留白",
            "TDesign lg 1200px",
        ),
    ];
    div()
        .flex()
        .flex_col()
        .gap_1()
        .children(CHANGES.into_iter().map(|(band, change, tdesign)| {
            let reached = width >= band.min_width(theme);
            div()
                .flex()
                .gap_2()
                .items_center()
                .child(chip(
                    theme,
                    format!("{} {}", band.label(), px_text(band.min_width(theme))),
                    reached,
                ))
                .child(
                    TextRole::Body
                        .apply(theme, div())
                        .max_w(px(900.))
                        .when(!reached, |row| row.opacity(0.45))
                        .child(change),
                )
                .child(caption(theme, tdesign).when(!reached, |row| row.opacity(0.45)))
        }))
}

fn token_readings(theme: YueTheme) -> impl IntoElement {
    let readings: [(&str, &str, String); 12] = [
        (
            "顶部区高度",
            "--layout-header-height",
            px_text(header_height(theme)),
        ),
        (
            "侧栏展开",
            "--layout-sidebar-width",
            px_text(sidebar_width(theme)),
        ),
        (
            "侧栏收起",
            "--layout-sidebar-width-collapsed",
            px_text(sidebar_width_collapsed(theme)),
        ),
        (
            "自动收起阈值",
            "--layout-sidebar-collapse-at",
            format!("{} = --breakpoint-lg", px_text(sidebar_collapse_at(theme))),
        ),
        (
            "窗格宽度",
            "--layout-pane-width",
            px_text(pane_width(theme)),
        ),
        (
            "页面内边距",
            "--layout-page-gutter / -md / -lg",
            format!(
                "{} / {} / {}（< md / md–xl / ≥ xl）",
                px_text(page_gutter(theme)),
                px_text(page_gutter_md(theme)),
                px_text(page_gutter_lg(theme))
            ),
        ),
        (
            "栅格",
            "--layout-grid-columns / --layout-grid-gap",
            format!("{} 列 · {}", grid_columns(theme), px_text(grid_gap(theme))),
        ),
        (
            "文本容器",
            "--container-text",
            px_text(container_text(theme)),
        ),
        (
            "页面容器",
            "--container-page",
            px_text(container_page(theme)),
        ),
        (
            "断点",
            "--breakpoint-sm / -md / -lg / -xl",
            Breakpoint::ALL
                .iter()
                .filter(|band| **band != Breakpoint::Xs)
                .map(|band| px_text(band.min_width(theme)))
                .collect::<Vec<_>>()
                .join(" / "),
        ),
        (
            "侧栏收起",
            "--motion-ease-stow",
            format!(
                "配 --motion-duration-exit-large：{}ms，收起的东西随时召回",
                MotionDuration::ExitLarge.duration(theme).as_millis()
            ),
        ),
        (
            "侧栏展开",
            "--motion-ease-enter",
            format!(
                "配 --motion-duration-enter-large：{}ms，减速落位",
                MotionDuration::EnterLarge.duration(theme).as_millis()
            ),
        ),
    ];
    div()
        .flex()
        .flex_col()
        .gap_1()
        .children(readings.into_iter().map(|(use_for, token, value)| {
            div()
                .flex()
                .gap_3()
                .items_center()
                .child(div().w(px(110.)).text_sm().child(use_for))
                .child(
                    div()
                        .w(px(290.))
                        .text_sm()
                        .text_color(rgb(theme.text_secondary()))
                        .child(token),
                )
                .child(div().text_sm().child(value))
        }))
}

fn px_text(value: Pixels) -> String {
    format!("{:.0}px", value.as_f32())
}

/// Base owns hit testing and dragging; this recipe owns only track/thumb styling.
fn width_slider(
    state: &gpui_kit::Entity<SliderState>,
    theme: YueTheme,
    cx: &gpui_kit::App,
) -> impl IntoElement {
    let percentage = state.read(cx).percentage();
    Slider::new(state)
        .accessibility_label("模拟窗口宽度")
        .h_6()
        .w_full()
        .flex()
        .items_center()
        .child(
            SliderTrack::new(state)
                .w_full()
                .h_6()
                .flex()
                .items_center()
                .child(
                    SliderIndicator::new(state)
                        .relative()
                        .w_full()
                        .h_1()
                        .rounded_full()
                        .bg(gpui_kit::rgb(theme.border_default()))
                        .child(
                            div()
                                .absolute()
                                .left_0()
                                .top_0()
                                .h_full()
                                .w(gpui_kit::relative(percentage.end))
                                .rounded_full()
                                .bg(gpui_kit::rgb(theme.accent_solid())),
                        )
                        .child(
                            SliderThumb::new(state)
                                .absolute()
                                .left(gpui_kit::relative(percentage.end))
                                .top(gpui_kit::rems(-0.375))
                                .ml(gpui_kit::rems(-0.5))
                                .size_4()
                                .rounded_full()
                                .bg(gpui_kit::rgb(theme.accent_solid())),
                        ),
                ),
        )
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests {
    use super::*;
    use gpui_kit::{AppContext as _, Entity, Modifiers, Render, TestAppContext, point};

    struct SliderExample(Entity<SliderState>);
    impl Render for SliderExample {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div().w(px(400.)).child(width_slider(
                &self.0,
                YueTheme::new(ThemeMode::Light, Accent::Azure),
                cx,
            ))
        }
    }

    #[gpui_kit::test]
    fn base_slider_track_clicks_update_the_simulated_width(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (view, cx) =
            cx.add_window_view(|_, cx| SliderExample(cx.new(|_| simulated_width_state())));
        let draw = |cx: &mut gpui_kit::VisualTestContext| {
            cx.update(|window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
            })
        };
        draw(cx);
        let state = view.read_with(cx, |view, _| view.0.clone());
        cx.simulate_click(point(px(100.), px(12.)), Modifiers::default());
        draw(cx);
        let low = state.read_with(cx, |state, _| state.value().end());
        assert!((low - (SIM_MIN + (SIM_MAX - SIM_MIN) * 0.25)).abs() <= 8.);
        cx.simulate_click(point(px(300.), px(12.)), Modifiers::default());
        draw(cx);
        let high = state.read_with(cx, |state, _| state.value().end());
        assert!((high - (SIM_MIN + (SIM_MAX - SIM_MIN) * 0.75)).abs() <= 8.);
        assert!(high > low);
    }
}
