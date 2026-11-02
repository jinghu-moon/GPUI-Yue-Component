//! The Color page: Yue's palette, roles and contrast pairs as GPUI renders them,
//! next to the places where the GPUI port cannot behave the way the web does.
//!
//! Every swatch reads the same token catalog the components read, so the page
//! shows what a component paints rather than a copy of the design sheet.

use crate::scroll::ScrollableElement as _;
use gpui_kit::base::{Theme as BaseTheme, Transition, transition};
use gpui_kit::{
    AnyElement, Context, InteractiveElement as _, IntoElement, ParentElement as _, Rgba,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
};
use yue_component::{
    Accent, ButtonVariant, NeutralTone, RadioAppearance, ThemeMode, YueButton, YueInput,
    YueRadioGroup, YueRadioOption, YueTheme,
};

use crate::Showcase;

/// Families in the order the design sheet lists them.
const FAMILIES: [&str; 6] = ["neutral", "azure", "blue", "red", "amber", "green"];

/// Hues for categories, tags and charts only; they never carry a status.
const EXTENDED: [&str; 8] = [
    "orange", "yellow", "lime", "teal", "cyan", "purple", "magenta", "rose",
];

/// Neutral ladders with a cast, each one a whole-set swap for `neutral`.
const TINTED: [&str; 4] = ["neutral", "slate", "stone", "haze"];

/// Text pairs need 4.5:1, borders and focus 3:1 — the bar Yue's audit holds.
const TEXT: f32 = 4.5;
const UI: f32 = 3.0;

const PAIRS: [(&str, &str, f32); 17] = [
    ("--text-primary", "--surface", TEXT),
    ("--text-secondary", "--surface", TEXT),
    ("--text-placeholder", "--surface", TEXT),
    ("--text-link", "--surface", TEXT),
    ("--text-inverse", "--surface-inverse", TEXT),
    ("--on-accent", "--accent-solid", TEXT),
    ("--accent-subtle-text", "--accent-subtle", TEXT),
    ("--selected-color", "--selected-background", TEXT),
    ("--action-on-danger", "--action-danger", TEXT),
    ("--info-foreground", "--info-bg", TEXT),
    ("--success-foreground", "--success-bg", TEXT),
    ("--warning-foreground", "--warning-bg", TEXT),
    ("--error-foreground", "--error-bg", TEXT),
    ("--border-control", "--surface", UI),
    ("--border-strong", "--surface", UI),
    ("--focus-ring-color", "--surface", UI),
    ("--accent-border", "--surface", UI),
];

/// How long the blending demo takes, slowed so the middle of the ramp can be seen.
const SLOW_RAMP: std::time::Duration = std::time::Duration::from_millis(2000);

pub(crate) fn color_content(
    showcase: &Showcase,
    window: &mut Window,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    // The page wears the picked profile whole, so the window's scrollbar and
    // Base's global theme agree with what is on screen; see `Showcase::render`.
    let theme = YueTheme::new(showcase.color_mode, showcase.color_accent)
        .with_neutral(showcase.color_neutral);
    let page_theme = theme;

    div()
        .w_full()
        .min_h_full()
        .bg(rgba(theme, "--page"))
        .text_color(rgba(theme, "--text-primary"))
        .flex()
        .flex_col()
        .gap_6()
        .p_8()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_2xl().child("Color"))
                .child(
                    div()
                        .text_color(gpui_kit::rgb(page_theme.text_secondary()))
                        .child("Yue 的色板、语义角色与对比度，以及 GPUI 端与 Web 端的差异"),
                ),
        )
        .child(profile_picker(showcase, page_theme, cx))
        .child(block(
            page_theme,
            "Palette",
            "原始色阶。切换浅色/深色可以看到深色模式把整条色阶压暗，而不是另一套设计。\
             数字是该色与白字（W）、黑字（B）的对比度。",
            palette(theme, &FAMILIES),
        ))
        .child(block(
            page_theme,
            "Extended hues",
            "扩展色共用一条明度阶梯，同一档在各色相间明度一致。只用于分类、标签与图表，不表达状态。",
            palette(theme, &EXTENDED),
        ))
        .child(block(
            page_theme,
            "Tinted neutrals",
            "带色彩倾向的中性色与 neutral 逐档同明度。用上方「中性色」切换时，整条 neutral 被替换，\
             所有引用 neutral 的语义 token 随之变色，accent 与状态色不变。",
            palette(theme, &TINTED),
        ))
        .child(block(
            page_theme,
            "Surfaces",
            "表面层级。浅色模式下 Level 2 和 Level 3 都是白色，层级只靠各自的边框角色区分。",
            surfaces(theme),
        ))
        .child(block(
            page_theme,
            "Contrast pairs",
            "语义配对的对比度在 GPUI 端现场计算：文字 ≥ 4.5:1，边框与焦点 ≥ 3:1，未达标标红。",
            contrast_pairs(theme),
        ))
        .child(block(
            page_theme,
            "Interaction states",
            "Hover 与 Pressed 由前景色按 --opacity-hover / --opacity-pressed 混合得到。\
             色块是组件读取的同一组 token；右侧是真实组件。",
            interaction_states(theme),
        ))
        .child(block(
            page_theme,
            "Differences from the web",
            "Yue 的 CSS 在浏览器里由级联、var() 和 color-mix() 运行时计算；GPUI 端是同步时求值后的常量。\
             下表是这一点带来的全部差异。",
            differences(page_theme),
        ))
        .child(block(
            page_theme,
            "Demo · transparent 过渡的插值",
            "差异 3，已修复。深色 Ghost hover：从 transparent 到 #f5f5f5 12%。上排是修复前的逐通道插值，\
             下排是组件现在的预乘插值，与浏览器一致。悬停右侧色块可看 2 秒慢速过渡。",
            blending_demo(showcase, window, cx),
        ))
        .child(block(
            page_theme,
            "Demo · 选区与滚动条",
            "差异 5，已修复。YueInput 的选区改由 Yue 自己给出；滚动条读 gpui-base 的全局 Theme，\
             现在由 yue_component::set_theme 同步。在输入框里拖选文字，或滚动右侧列表。",
            base_theme_demo(showcase, theme, cx),
        ))
}

fn profile_picker(
    showcase: &Showcase,
    theme: YueTheme,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let entity = cx.entity();
    let mode_entity = entity.clone();
    let accent_entity = entity.clone();
    div()
        .flex()
        .items_start()
        .gap_6()
        .child(
            YueRadioGroup::new(
                "color-mode",
                theme,
                Some(SharedString::from(match showcase.color_mode {
                    ThemeMode::Light => "light",
                    ThemeMode::Dark => "dark",
                })),
            )
            .label("模式")
            .appearance(RadioAppearance::Button)
            .options(vec![
                YueRadioOption::new("light", "浅色"),
                YueRadioOption::new("dark", "深色"),
            ])
            .on_change(move |value, _, cx| {
                mode_entity.update(cx, |showcase, cx| {
                    showcase.color_mode = if value == "dark" {
                        ThemeMode::Dark
                    } else {
                        ThemeMode::Light
                    };
                    showcase.sync_theme(cx);
                    cx.notify();
                });
            }),
        )
        .child(
            YueRadioGroup::new(
                "color-accent",
                theme,
                Some(SharedString::from(match showcase.color_accent {
                    Accent::Azure => "azure",
                    Accent::Neutral => "neutral",
                })),
            )
            .label("Accent")
            .appearance(RadioAppearance::Button)
            .options(vec![
                YueRadioOption::new("azure", "Azure"),
                YueRadioOption::new("neutral", "Neutral"),
            ])
            .on_change(move |value, _, cx| {
                accent_entity.update(cx, |showcase, cx| {
                    showcase.color_accent = if value == "neutral" {
                        Accent::Neutral
                    } else {
                        Accent::Azure
                    };
                    showcase.sync_theme(cx);
                    cx.notify();
                });
            }),
        )
        .child(
            YueRadioGroup::new(
                "color-neutral",
                theme,
                Some(SharedString::from(showcase.color_neutral.family())),
            )
            .label("中性色")
            .appearance(RadioAppearance::Button)
            .options(vec![
                YueRadioOption::new("neutral", "Neutral"),
                YueRadioOption::new("slate", "Slate"),
                YueRadioOption::new("stone", "Stone"),
                YueRadioOption::new("haze", "Haze"),
            ])
            .on_change(move |value, _, cx| {
                entity.update(cx, |showcase, cx| {
                    showcase.color_neutral = NeutralTone::ALL
                        .into_iter()
                        .find(|tone| tone.family() == value.as_ref())
                        .unwrap_or_default();
                    showcase.sync_theme(cx);
                    cx.notify();
                });
            }),
        )
}

/// A titled block whose example sits on the page, not inside a framed card:
/// the examples carry their own profile surface.
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

/// The profile's own surface, so a dark profile reads as dark on the light page.
pub(crate) fn stage(theme: YueTheme) -> gpui_kit::Div {
    div()
        .p_4()
        .rounded_md()
        .border_1()
        .border_color(rgba(theme, "--border-default"))
        .bg(rgba(theme, "--surface"))
        .text_color(rgba(theme, "--text-primary"))
}

fn palette(theme: YueTheme, families: &[&'static str]) -> impl IntoElement {
    let families: Vec<(&str, Vec<(u32, Rgba)>)> = families
        .iter()
        .copied()
        .map(|family| {
            let prefix = format!("--{family}-");
            let steps = theme
                .all_tokens()
                .filter(|token| token.layer() == "primitives")
                .filter_map(|token| {
                    let step = token.name().strip_prefix(prefix.as_str())?.parse().ok()?;
                    Some((step, token.rgba()?))
                })
                .collect();
            (family, steps)
        })
        .collect();
    // Only neutral has 0 and 1000 (pure white and black); the chromatic rows
    // leave those columns empty so every step lines up.
    let mut columns: Vec<u32> = families
        .iter()
        .flat_map(|(_, steps)| steps.iter().map(|(step, _)| *step))
        .collect();
    columns.sort_unstable();
    columns.dedup();

    let mut rows: Vec<AnyElement> = Vec::new();
    for (family, steps) in families {
        rows.push(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(div().w_16().text_sm().child(family))
                .children(columns.iter().map(|&step| {
                    let Some(&(_, color)) = steps.iter().find(|(s, _)| *s == step) else {
                        return div().w_20().h_16().into_any_element();
                    };
                    let ink = readable_ink(color);
                    div()
                        .w_20()
                        .h_16()
                        .flex()
                        .flex_col()
                        .justify_between()
                        .p_1()
                        .rounded_sm()
                        .bg(color)
                        .text_color(ink)
                        .text_xs()
                        .child(format!("{step}"))
                        .child(div().flex().flex_col().child(hex(color)).child(format!(
                            "W{:.1} B{:.1}",
                            contrast(color, white()),
                            contrast(color, black())
                        )))
                        .into_any_element()
                }))
                .into_any_element(),
        );
    }
    stage(theme).flex().flex_col().gap_1().children(rows)
}

fn surfaces(theme: YueTheme) -> impl IntoElement {
    let layer = |name: &'static str, fill: &'static str, border: Option<&'static str>| {
        let note = if fill == "--surface-inverse" {
            "--text-inverse"
        } else {
            "--text-secondary"
        };
        div()
            .flex()
            .flex_col()
            .gap_1()
            .p_3()
            .rounded_md()
            .bg(rgba(theme, fill))
            .when_some(border, |layer, border| {
                layer.border_1().border_color(rgba(theme, border))
            })
            .child(div().text_sm().child(name))
            .child(
                div()
                    .text_xs()
                    .text_color(rgba(theme, note))
                    .child(format!("{fill} {}", hex(rgba(theme, fill)))),
            )
    };

    div()
        .flex()
        .flex_col()
        .gap_3()
        .p_4()
        .rounded_md()
        .bg(rgba(theme, "--page"))
        .text_color(rgba(theme, "--text-primary"))
        .child(layer("Level 0 · 页面画布", "--page", None))
        .child(
            layer("Level 1 · 普通容器", "--surface", Some("--border-default")).child(
                div()
                    .flex()
                    .gap_3()
                    .child(layer("Subtle", "--surface-subtle", None))
                    .child(layer("Component", "--surface-component", None))
                    .child(
                        layer(
                            "Level 2 · Popover / Menu",
                            "--surface-level-2",
                            Some("--surface-level-2-border"),
                        )
                        .shadow(shadow(theme, 2))
                        .child(
                            layer(
                                "Level 3 · Dialog",
                                "--surface-level-3",
                                Some("--surface-level-3-border"),
                            )
                            .shadow(shadow(theme, 3)),
                        ),
                    )
                    .child(
                        layer("Inverse · Tooltip / Toast", "--surface-inverse", None)
                            .text_color(rgba(theme, "--text-inverse")),
                    ),
            ),
        )
}

fn contrast_pairs(theme: YueTheme) -> impl IntoElement {
    stage(theme)
        .flex()
        .flex_wrap()
        .gap_2()
        .children(PAIRS.into_iter().map(|(fore, back, minimum)| {
            let background = over(rgba(theme, back), rgba(theme, "--surface"));
            let foreground = over(rgba(theme, fore), background);
            let ratio = contrast(foreground, background);
            let passed = ratio >= minimum;
            div()
                .w_64()
                .flex()
                .flex_col()
                .gap_1()
                .p_3()
                .rounded_md()
                .bg(background)
                .border_1()
                .border_color(if passed {
                    rgba(theme, "--border-divider")
                } else {
                    rgba(theme, "--error")
                })
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .child(div().text_color(foreground).child(if minimum == TEXT {
                            "示例文字 Aa"
                        } else {
                            "■ 边界 / 图形"
                        }))
                        .child(
                            div()
                                .text_sm()
                                .text_color(if passed {
                                    readable_ink(background)
                                } else {
                                    rgba(theme, "--error")
                                })
                                .child(format!("{ratio:.2}:1 {}", if passed { "✓" } else { "✗" })),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(readable_ink(background))
                        .child(format!("{fore}\n on {back} · 需 ≥ {minimum}:1")),
                )
        }))
}

fn interaction_states(theme: YueTheme) -> impl IntoElement {
    let surface = rgba(theme, "--surface");
    let swatch = |label: &'static str, token: &'static str, ink: &'static str| {
        let fill = over(rgba(theme, token), surface);
        div()
            .w_32()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .h_12()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .bg(fill)
                    .text_color(over(rgba(theme, ink), fill))
                    .child(label),
            )
            .child(div().text_xs().child(token))
            .child(div().text_xs().child(hex(fill)))
    };

    stage(theme)
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .flex()
                .gap_3()
                .child(swatch(
                    "Default",
                    "--button-default-background",
                    "--button-default-color",
                ))
                .child(swatch(
                    "Hover",
                    "--button-default-background-hover",
                    "--button-default-color",
                ))
                .child(swatch(
                    "Pressed",
                    "--button-default-background-pressed",
                    "--button-default-color",
                ))
                .child(swatch(
                    "Selected",
                    "--selected-background",
                    "--selected-color",
                ))
                .child(swatch(
                    "Disabled",
                    "--disabled-container",
                    "--disabled-content",
                )),
        )
        .child(
            div()
                .flex()
                .gap_3()
                .child(swatch(
                    "Ghost",
                    "--button-ghost-background",
                    "--button-ghost-color",
                ))
                .child(swatch(
                    "Ghost hover",
                    "--button-ghost-background-hover",
                    "--button-ghost-color",
                ))
                .child(swatch(
                    "Ghost pressed",
                    "--button-ghost-background-pressed",
                    "--button-ghost-color",
                ))
                .child(swatch(
                    "Primary",
                    "--button-primary-background",
                    "--button-primary-color",
                ))
                .child(swatch(
                    "Primary hover",
                    "--button-primary-background-hover",
                    "--button-primary-color",
                )),
        )
        .child(
            div()
                .flex()
                .gap_3()
                .child(YueButton::text("color-state-default", theme, "Default"))
                .child(
                    YueButton::text("color-state-ghost", theme, "Ghost")
                        .variant(ButtonVariant::Ghost),
                )
                .child(
                    YueButton::text("color-state-primary", theme, "Primary")
                        .variant(ButtonVariant::Primary),
                )
                .child(YueButton::text("color-state-disabled", theme, "Disabled").disabled(true)),
        )
}

/// What differs, why, and how the page backs the claim up.
const DIFFERENCES: [(&str, &str, &str, &str, &str); 7] = [
    (
        "1",
        "颜色引用",
        "var() / color-mix() 运行时计算，改一个基础色，派生色全部跟着变",
        "同步时已求成最终值，引用关系不保留",
        "生成文件确认 · 改 accent 不会带动 hover / subtle",
    ),
    (
        "2",
        "currentColor",
        "自动取当前元素的文字色",
        "无此概念，组件把自己的前景色传进来再混合",
        "代码确认 · button.rs 中 --button-subtle-background-hover 的回退分支",
    ),
    (
        "3",
        "transparent 过渡",
        "预乘 alpha 插值：全程是前景色，只有透明度变化",
        "已对齐：组件改用预乘插值。此前逐通道插值把 transparent 当黑色，深色下中途偏暗",
        "下方 Demo · tokens.rs 测试",
    ),
    (
        "4",
        "主题作用域",
        "data-theme 挂在根节点，靠继承；局部换主题需完整作用域",
        "YueTheme 是值，逐个传给组件；同一窗口可并排多套主题",
        "设计差异 · GPUI 更灵活",
    ),
    (
        "5",
        "滚动条与选区",
        "滚动条随 color-scheme 变色；选区是实心 --selection-bg，文字换成 --selection-foreground",
        "滚动条读 Base 全局 Theme，整个应用只有一份，需在事件里调用 yue_component::set_theme 同步，不能逐组件换主题；选区画在文字下层、改不了字色，所以用 --selection-bg 的 30% 浅底",
        "下方 Demo · theme 测试",
    ),
    (
        "6",
        "forced-colors",
        "高对比度模式下回退到 CanvasText 等系统颜色",
        "未处理",
        "代码确认 · 无相关分支",
    ),
    (
        "7",
        "跟随系统",
        "prefers-color-scheme",
        "未读取窗口外观；本页只能手动切换",
        "见下方当前窗口外观",
    ),
];

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
        .child(cell("#", gpui_kit::rems(2.)))
        .child(cell("方面", gpui_kit::rems(8.)))
        .child(cell("Web", gpui_kit::rems(20.)))
        .child(cell("GPUI", gpui_kit::rems(22.)))
        .child(cell("依据", gpui_kit::rems(18.)));

    stage(theme)
        .p_0()
        .flex()
        .flex_col()
        .child(header)
        .children(
            DIFFERENCES
                .into_iter()
                .map(|(index, aspect, web, gpui, proof)| {
                    div()
                        .flex()
                        .border_t_1()
                        .border_color(gpui_kit::rgb(theme.border_default()))
                        .child(cell(index, gpui_kit::rems(2.)))
                        .child(cell(aspect, gpui_kit::rems(8.)))
                        .child(cell(web, gpui_kit::rems(20.)))
                        .child(cell(gpui, gpui_kit::rems(22.)))
                        .child(
                            cell(proof, gpui_kit::rems(18.))
                                .text_color(gpui_kit::rgb(theme.text_secondary())),
                        )
                }),
        )
}

fn blending_demo(
    showcase: &Showcase,
    window: &mut Window,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let theme =
        YueTheme::new(ThemeMode::Dark, showcase.color_accent).with_neutral(showcase.color_neutral);
    let surface = rgba(theme, "--surface");
    let rest = rgba(theme, "--button-ghost-background");
    let hover = rgba(theme, "--button-ghost-background-hover");

    let row = |label: &'static str,
               blend: fn(Rgba, Rgba, f32) -> Rgba,
               slot: usize,
               window: &mut Window,
               cx: &mut Context<Showcase>| {
        let progress = transition(
            if slot == 0 {
                "color-blend-straight"
            } else {
                "color-blend-premultiplied"
            },
            if showcase.color_hover[slot] {
                1.0_f32
            } else {
                0.0
            },
            Transition::new(SLOW_RAMP).ease(|progress| progress),
            window,
            cx,
        );
        let live = over(blend(rest, hover, progress), surface);
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(div().w_48().text_sm().child(label))
            .children([0.0, 0.25, 0.5, 0.75, 1.0].map(|step| {
                let fill = over(blend(rest, hover, step), surface);
                div()
                    .w_24()
                    .h_16()
                    .flex()
                    .flex_col()
                    .justify_end()
                    .p_1()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgba(theme, "--border-divider"))
                    .bg(fill)
                    .text_xs()
                    .child(format!("{:.0}%", step * 100.))
                    .child(hex(fill))
            }))
            .child(
                div()
                    .id(("color-blend-live", slot))
                    .ml_4()
                    .w_40()
                    .h_16()
                    .flex()
                    .flex_col()
                    .justify_end()
                    .p_1()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgba(theme, "--border-default"))
                    .bg(live)
                    .text_xs()
                    .on_hover(cx.listener(move |showcase, hovered: &bool, _, cx| {
                        showcase.color_hover[slot] = *hovered;
                        cx.notify();
                    }))
                    .child("悬停这里")
                    .child(hex(live)),
            )
    };

    stage(theme)
        .flex()
        .flex_col()
        .gap_3()
        .child(row("修复前（逐通道）", straight_mix, 0, window, cx))
        .child(row(
            "现在 = Web（预乘 alpha）",
            premultiplied_mix,
            1,
            window,
            cx,
        ))
}

fn base_theme_demo(
    showcase: &Showcase,
    theme: YueTheme,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let base = BaseTheme::global(cx);
    let surface = rgba(theme, "--surface");
    // Before the fix nothing wrote Base's theme, so both fell back to its own
    // light palette: `InputEditorStyle::resolved` took `accent` at 0.4 for the
    // selection and the scrollbar thumb took `foreground` at 0.55.
    let before = gpui_kit::base::ColorTokens::light();
    let before_selection = before.accent.alpha(0.4);
    let before_thumb = before.foreground.alpha(0.55);
    let now_thumb = base.tokens.colors.foreground.alpha(0.55);

    let ink = rgba(theme, "--input-color");
    // A selection is a wash under the glyphs, so what matters is the text it
    // sits under; a thumb sits on the surface itself.
    let pair = |title: &'static str, before: Rgba, now: Rgba, on_ink: bool| {
        let before = over(before, surface);
        let now = over(now, surface);
        let (measure, label) = if on_ink {
            (ink, "文字对选区")
        } else {
            (surface, "对 surface")
        };
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(div().w_24().text_sm().child(title))
            .child(chip("修复前", before))
            .child(chip("现在", now))
            .child(div().text_xs().child(format!(
                "{label} {:.2}:1 → {:.2}:1",
                contrast(before, measure),
                contrast(now, measure)
            )))
    };

    stage(theme)
        .flex()
        .gap_6()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    YueInput::new("color-selection", theme, &showcase.color_input)
                        .width(gpui_kit::rems(18.)),
                )
                .child(pair(
                    "文字选区",
                    before_selection.to_rgb(),
                    theme.selection().to_rgb(),
                    true,
                ))
                .child(pair(
                    "滚动条",
                    before_thumb.to_rgb(),
                    now_thumb.to_rgb(),
                    false,
                ))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgba(theme, "--text-secondary"))
                        .child(format!(
                            "Base 全局 Theme：appearance = {:?}，foreground = {}；窗口外观 = {:?}",
                            base.appearance,
                            hex(base.tokens.colors.foreground.to_rgb()),
                            cx.window_appearance(),
                        )),
                ),
        )
        .child(
            div()
                .id("color-scroll")
                .w_48()
                .h_40()
                .rounded_md()
                .border_1()
                .border_color(rgba(theme, "--border-default"))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .p_2()
                        .text_sm()
                        .children((1..=30).map(|line| format!("第 {line} 行"))),
                )
                .overflow_y_scrollbar(),
        )
}

fn chip(label: &'static str, color: Rgba) -> impl IntoElement {
    div()
        .w_32()
        .h_10()
        .flex()
        .flex_col()
        .justify_center()
        .px_2()
        .rounded_sm()
        .bg(color)
        .text_color(readable_ink(color))
        .text_xs()
        .child(label)
        .child(hex(color))
}

pub(crate) fn rgba(theme: YueTheme, name: &'static str) -> Rgba {
    theme
        .token(name)
        .and_then(|token| token.rgba())
        .unwrap_or_else(|| panic!("Yue token {name} is not a supported color"))
}

fn shadow(theme: YueTheme, level: u8) -> Vec<gpui_kit::BoxShadow> {
    let alpha = match (theme.token("--surface").map(|t| t.value()), level) {
        (Some("#fff"), 2) => 0.16,
        (Some("#fff"), _) => 0.22,
        (_, 2) => 0.38,
        _ => 0.52,
    };
    vec![gpui_kit::BoxShadow {
        color: gpui_kit::hsla(0., 0., 0., alpha),
        offset: gpui_kit::point(
            gpui_kit::px(0.),
            gpui_kit::px(if level == 2 { 12. } else { 18. }),
        ),
        blur_radius: gpui_kit::px(if level == 2 { 36. } else { 56. }),
        spread_radius: gpui_kit::px(0.),
        inset: false,
    }]
}

/// The blend components use today: every channel, alpha included, moves on its own.
pub(crate) fn straight_mix(from: Rgba, to: Rgba, progress: f32) -> Rgba {
    let channel = |from: f32, to: f32| from + (to - from) * progress;
    Rgba {
        r: channel(from.r, to.r),
        g: channel(from.g, to.g),
        b: channel(from.b, to.b),
        a: channel(from.a, to.a),
    }
}

/// The blend a browser uses for a CSS transition: colour weighted by its alpha,
/// so a transparent end contributes no colour of its own.
fn premultiplied_mix(from: Rgba, to: Rgba, progress: f32) -> Rgba {
    let a = from.a + (to.a - from.a) * progress;
    if a <= f32::EPSILON {
        return Rgba {
            r: 0.,
            g: 0.,
            b: 0.,
            a: 0.,
        };
    }
    let channel =
        |from_c: f32, to_c: f32| (from_c * from.a + (to_c * to.a - from_c * from.a) * progress) / a;
    Rgba {
        r: channel(from.r, to.r),
        g: channel(from.g, to.g),
        b: channel(from.b, to.b),
        a,
    }
}

/// Composite a colour over an opaque one, as the screen will show it.
pub(crate) fn over(top: Rgba, bottom: Rgba) -> Rgba {
    let channel = |top_c: f32, bottom_c: f32| top_c * top.a + bottom_c * (1. - top.a);
    Rgba {
        r: channel(top.r, bottom.r),
        g: channel(top.g, bottom.g),
        b: channel(top.b, bottom.b),
        a: 1.,
    }
}

fn luminance(color: Rgba) -> f32 {
    let linear = |c: f32| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}

pub(crate) fn contrast(a: Rgba, b: Rgba) -> f32 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

fn white() -> Rgba {
    gpui_kit::rgb(0xffffff)
}

fn black() -> Rgba {
    gpui_kit::rgb(0x000000)
}

fn readable_ink(background: Rgba) -> Rgba {
    if contrast(background, white()) >= contrast(background, black()) {
        white()
    } else {
        black()
    }
}

pub(crate) fn hex(color: Rgba) -> String {
    let byte = |c: f32| (c.clamp(0., 1.) * 255.).round() as u8;
    format!(
        "#{:02x}{:02x}{:02x}",
        byte(color.r),
        byte(color.g),
        byte(color.b)
    )
}

use gpui_kit::prelude::FluentBuilder as _;
