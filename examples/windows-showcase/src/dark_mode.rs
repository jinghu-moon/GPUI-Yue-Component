//! The Dark Mode page: the same roles in the light and dark profiles side by
//! side, sorted into the three kinds the dark-mode guide names, with every
//! pair's contrast computed from the token catalog as it renders.

use gpui_kit::{
    AnyElement, Context, IntoElement, ParentElement as _, Rgba, SharedString, Styled as _, div,
};
use yue_component::{
    Accent, GroupDirection, RadioAppearance, ThemeMode, ThemePreference, YueRadioGroup,
    YueRadioOption, YueTheme,
};

use crate::Showcase;
use crate::color::{contrast, hex, over, rgba, stage, straight_mix};

const TEXT: f32 = 4.5;
const UI: f32 = 3.0;

/// Neutral text levels, measured on `--surface`. Disabled text is reported,
/// not gated, as WCAG 1.4.3 exempts it.
const TEXT_LEVELS: [(&str, &str, f32); 4] = [
    ("主文字", "--text-primary", TEXT),
    ("次要文字", "--text-secondary", TEXT),
    ("占位文字", "--text-placeholder", TEXT),
    ("禁用文字", "--text-disabled", 0.),
];

/// Colours that are read: a status colour on the surface, and the tint's own
/// text on the tint.
const FEEDBACK: [(&str, &str); 4] = [
    ("错误", "error"),
    ("警告", "warning"),
    ("成功", "success"),
    ("信息", "info"),
];

/// Fills that carry a label, each with the label paired to it.
const SOLIDS: [(&str, &str, &str); 4] = [
    ("主题色", "--accent-solid", "--on-accent"),
    ("错误", "--error-solid", "--on-error-solid"),
    ("警告", "--warning-solid", "--on-warning-solid"),
    ("成功", "--success-solid", "--on-success-solid"),
];

/// From the page forward; in dark each step is lighter than the one before.
const SURFACES: [(&str, &str); 5] = [
    ("页面", "--page"),
    ("内容表面", "--surface"),
    ("辅助表面", "--surface-subtle"),
    ("二级浮层", "--surface-level-2"),
    ("三级浮层", "--surface-level-3"),
];

pub(crate) fn dark_mode_content(
    showcase: &Showcase,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    // The page wears the resolved preference whole, as the Color page wears its
    // profile, so Base's global theme agrees with it; see `Showcase::render`.
    let theme = YueTheme::new(showcase.dark_mode_theme(), Accent::Azure);
    let light = YueTheme::new(ThemeMode::Light, Accent::Azure);
    let dark = YueTheme::new(ThemeMode::Dark, Accent::Azure);

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
                .child(div().text_2xl().child("Dark Mode"))
                .child(
                    div()
                        .text_color(gpui_kit::rgb(theme.text_secondary()))
                        .child("同一组角色在浅色和深色里各自取值；每一对的对比度都由 Token 现场计算"),
                ),
        )
        .child(preference_picker(showcase, theme, cx))
        .child(section(
            theme,
            "中性色",
            "明度反向，但不是对调：深色的文字是独立的一组值，按对比度重新取。比例相对 --surface。",
            |profile| text_levels(profile).into_any_element(),
            light,
            dark,
        ))
        .child(section(
            theme,
            "表面层级",
            "深色下表面越靠前越亮；阴影在近黑底上几乎看不见，层级要靠明度或描边。",
            |profile| surfaces(profile).into_any_element(),
            light,
            dark,
        ))
        .child(section(
            theme,
            "功能色（用来读的）",
            "深色换到同一色相更亮的台阶，淡底换到更暗的台阶。上行是状态色在表面上，下行是淡底上的文字。",
            |profile| feedback(profile).into_any_element(),
            light,
            dark,
        ))
        .child(section(
            theme,
            "实心填充（用来托字的）",
            "色相不变，台阶上移，字色翻转。字色在静止 / 悬停 / 按下三态都要 ≥ 4.5:1，填充相对页面要 ≥ 3:1。",
            |profile| solids(profile).into_any_element(),
            light,
            dark,
        ))
        .child(section_single(
            theme,
            "为什么深色不能沿用 600 + 白字",
            "同一条 azure，在深色页面上对比两种做法：600 档白字一叠加悬停、按下就掉到 4.5 以下，而且与页面只拉开 3.36；400 档配近黑字三态都稳。",
            why_flip(dark),
        ))
}

fn preference_picker(
    showcase: &Showcase,
    theme: YueTheme,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let entity = cx.entity();
    let system = match ThemePreference::System.resolve(showcase.appearance) {
        ThemeMode::Light => "浅色",
        ThemeMode::Dark => "深色",
    };
    div()
        .flex()
        .items_end()
        .gap_4()
        .child(
            YueRadioGroup::new(
                "theme-preference",
                theme,
                Some(SharedString::from(match showcase.theme_preference {
                    ThemePreference::System => "system",
                    ThemePreference::Light => "light",
                    ThemePreference::Dark => "dark",
                })),
            )
            .label("本页主题")
            .appearance(RadioAppearance::Button)
            .direction(GroupDirection::Horizontal)
            .options(vec![
                YueRadioOption::new("system", "跟随系统"),
                YueRadioOption::new("light", "浅色"),
                YueRadioOption::new("dark", "深色"),
            ])
            .on_change(move |value, _, cx| {
                entity.update(cx, |showcase, cx| {
                    showcase.theme_preference = match value.as_ref() {
                        "light" => ThemePreference::Light,
                        "dark" => ThemePreference::Dark,
                        _ => ThemePreference::System,
                    };
                    showcase.sync_theme(cx);
                    cx.notify();
                });
            }),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .pb_2()
                .text_sm()
                .text_color(gpui_kit::rgb(theme.text_secondary()))
                .child(format!(
                    "系统当前是{system}。「跟随系统」经 ThemePreference::resolve(window.appearance()) 解析，\
                     并在 observe_window_appearance 回调里随系统切换。"
                )),
        )
}

/// A titled row holding the light and the dark profile next to each other.
fn section(
    theme: YueTheme,
    title: &'static str,
    description: &'static str,
    example: impl Fn(YueTheme) -> AnyElement,
    light: YueTheme,
    dark: YueTheme,
) -> impl IntoElement {
    let column = |label: &'static str, profile: YueTheme| {
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .text_color(gpui_kit::rgb(theme.text_secondary()))
                    .child(label),
            )
            .child(
                stage(profile)
                    .bg(rgba(profile, "--page"))
                    .child(example(profile)),
            )
    };
    section_single(
        theme,
        title,
        description,
        div()
            .flex()
            .gap_4()
            .child(column("浅色", light))
            .child(column("深色", dark)),
    )
}

fn section_single(
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

/// The ratio of `foreground` on `background`, marked against `minimum`; a zero
/// minimum is only reported.
fn ratio_label(profile: YueTheme, foreground: Rgba, background: Rgba, minimum: f32) -> AnyElement {
    let ratio = contrast(foreground, background);
    let (mark, color) = if minimum == 0. {
        ("仅报告", rgba(profile, "--text-secondary"))
    } else if ratio >= minimum {
        ("✓", rgba(profile, "--text-secondary"))
    } else {
        ("✗", rgba(profile, "--error"))
    };
    div()
        .text_xs()
        .text_color(color)
        .child(format!("{ratio:.2}:1 {mark}"))
        .into_any_element()
}

fn text_levels(profile: YueTheme) -> impl IntoElement {
    let surface = rgba(profile, "--surface");
    div()
        .p_3()
        .rounded_md()
        .bg(surface)
        .flex()
        .flex_col()
        .gap_2()
        .children(TEXT_LEVELS.into_iter().map(|(label, token, minimum)| {
            let ink = over(rgba(profile, token), surface);
            div()
                .flex()
                .justify_between()
                .gap_2()
                .child(div().text_color(ink).child(format!("{label} {}", hex(ink))))
                .child(ratio_label(profile, ink, surface, minimum))
        }))
}

fn surfaces(profile: YueTheme) -> impl IntoElement {
    div()
        .flex()
        .gap_2()
        .children(SURFACES.into_iter().map(|(label, token)| {
            let fill = rgba(profile, token);
            div()
                .flex_1()
                .min_w_0()
                .h_20()
                .p_2()
                .rounded_md()
                .border_1()
                .border_color(rgba(profile, "--border-default"))
                .bg(fill)
                .flex()
                .flex_col()
                .justify_end()
                .text_xs()
                .text_color(rgba(profile, "--text-secondary"))
                .child(label)
                .child(hex(fill))
        }))
}

fn feedback(profile: YueTheme) -> impl IntoElement {
    let surface = rgba(profile, "--surface");
    let token = |kind: &str, suffix: &str| -> Rgba {
        let name = format!("--{kind}{suffix}");
        profile
            .token(&name)
            .and_then(|token| token.rgba())
            .unwrap_or_else(|| panic!("Yue token {name} is not a supported color"))
    };
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .p_3()
                .rounded_md()
                .bg(surface)
                .flex()
                .gap_3()
                .children(FEEDBACK.into_iter().map(|(label, kind)| {
                    let ink = token(kind, "");
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(div().text_color(ink).child(label))
                        .child(ratio_label(profile, ink, surface, TEXT))
                })),
        )
        .child(
            div()
                .flex()
                .gap_2()
                .children(FEEDBACK.into_iter().map(|(label, kind)| {
                    let tint = token(kind, "-bg");
                    let ink = token(kind, "-foreground");
                    div()
                        .flex_1()
                        .min_w_0()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(token(kind, "-border"))
                        .bg(tint)
                        .flex()
                        .flex_col()
                        .child(div().text_color(ink).child(label))
                        .child(
                            div()
                                .text_xs()
                                .text_color(ink)
                                .child(format!("{:.2}:1", contrast(ink, tint))),
                        )
                })),
        )
}

fn opacity(profile: YueTheme, name: &str) -> f32 {
    profile
        .token(name)
        .and_then(|token| token.value().parse().ok())
        .unwrap_or_else(|| panic!("Yue token {name} is not a number"))
}

/// One filled control in its three states, the label's ratio under each and
/// the fill's against the page.
fn solid_row(
    profile: YueTheme,
    label: &str,
    fill: Rgba,
    ink: Rgba,
    hover: f32,
    pressed: f32,
) -> impl IntoElement {
    let page = rgba(profile, "--page");
    let states = [
        ("静止", fill),
        ("悬停", straight_mix(fill, ink, hover)),
        ("按下", straight_mix(fill, ink, pressed)),
    ];
    div()
        .flex()
        .items_center()
        .gap_2()
        .children(states.into_iter().map(|(state, background)| {
            div()
                .flex_1()
                .min_w_0()
                .px_2()
                .py_1()
                .rounded_md()
                .bg(background)
                .text_color(ink)
                .flex()
                .flex_col()
                .child(div().text_sm().child(format!("{label} · {state}")))
                .child(div().text_xs().child({
                    let ratio = contrast(ink, background);
                    format!("{ratio:.2}:1 {}", if ratio >= TEXT { "✓" } else { "✗" })
                }))
        }))
        .child(
            div()
                .w_20()
                .flex()
                .flex_col()
                .text_xs()
                .text_color(rgba(profile, "--text-secondary"))
                .child("对页面")
                .child(ratio_label(profile, fill, page, UI)),
        )
}

fn solids(profile: YueTheme) -> impl IntoElement {
    let hover = opacity(profile, "--opacity-hover");
    let pressed = opacity(profile, "--opacity-pressed");
    div()
        .flex()
        .flex_col()
        .gap_2()
        .children(SOLIDS.into_iter().map(|(label, fill, ink)| {
            solid_row(
                profile,
                label,
                rgba(profile, fill),
                rgba(profile, ink),
                hover,
                pressed,
            )
        }))
        .child(
            div()
                .text_xs()
                .text_color(rgba(profile, "--text-secondary"))
                .child(format!(
                    "叠加比例：悬停 {hover}，按下 {pressed}（字色混进填充）"
                )),
        )
}

fn why_flip(dark: YueTheme) -> impl IntoElement {
    let hover = opacity(dark, "--opacity-hover");
    let pressed = opacity(dark, "--opacity-pressed");
    let option = |title: &'static str, fill: Rgba, ink: Rgba| {
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .child(div().text_sm().child(title))
            .child(solid_row(dark, "azure", fill, ink, hover, pressed))
    };
    stage(dark).bg(rgba(dark, "--page")).child(
        div()
            .flex()
            .gap_4()
            .child(option(
                "600 + 白字（浅色的做法照搬）",
                rgba(dark, "--azure-600"),
                rgba(dark, "--neutral-0"),
            ))
            .child(option(
                "400 + 近黑字（Yue 的深色取值）",
                rgba(dark, "--accent-solid"),
                rgba(dark, "--on-accent"),
            )),
    )
}
