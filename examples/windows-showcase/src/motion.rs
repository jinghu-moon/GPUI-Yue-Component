//! The Motion page: the duration ladder, the semantic roles built on it, the
//! values each component actually resolves, springs for motion that is
//! interrupted, the macro patterns, and Yue's curves next to the curves other
//! design systems ship.
//!
//! Every demo reads the token a component reads, and runs through Base's
//! `transition` or `spring`, so the sidebar's motion switch settles it the way
//! it settles the components.

use std::time::Duration;

use gpui_kit::base::{Easing, Spring, Transition, spring, transition};
use gpui_kit::{
    Animation, AnimationExt as _, Context, Hsla, InteractiveElement as _, IntoElement,
    ParentElement as _, PathBuilder, Rgba, SharedString, StatefulInteractiveElement as _,
    Styled as _, Window, canvas, div, point, prelude::FluentBuilder as _, px,
};
use yue_component::motion::{self, LADDER, MotionDuration, MotionEase};
use yue_component::{
    ButtonSize, ButtonVariant, Icon, IconSize, SwitchSize, TextRole, YueButton, YueIcon, YueSwitch,
    YueTheme,
};

use crate::Showcase;

/// Which demo a `motion_play` slot drives.
pub(crate) const PLAY_LADDER: usize = 0;
pub(crate) const PLAY_POPOVER: usize = 1;
pub(crate) const PLAY_DIALOG: usize = 2;
pub(crate) const PLAY_CURVES: usize = 3;
pub(crate) const PLAY_REFERENCE: usize = 4;
pub(crate) const PLAY_STOW: usize = 5;
pub(crate) const PLAY_PATTERNS: usize = 6;
pub(crate) const PLAY_EXPRESSIVE: usize = 7;
pub(crate) const PLAY_PAGE: usize = 8;
pub(crate) const PLAY_SLOTS: usize = 9;

/// How much the "slow motion" switch stretches every demo.
const SLOW_FACTOR: u32 = 4;

const RULES: [&str; 10] = [
    "先选语义角色：交互反馈、小面板进出、大面板进出、页面进出、循环。绝大多数组件到此为止。",
    "组件的布局确实需要别的节奏时，直接用阶梯上的一档（--duration-300），并在 token 旁写明原因。",
    "阶梯上没有合适的值时，用 calc() 从某一档派生特殊值，单独命名并写明原因；不写裸数值。",
    "离开比进入短，约为进入的 0.5–0.75：用户已经做出决定，不需要再看一遍。",
    "时长随尺寸与移动距离增长：面板越大、走得越远越慢。除循环外不超过 400ms，400ms 只留给页面级转场。",
    "会被马上召回的离开（侧栏收进图标栏、折叠面板合上）用 stow，即 standard 曲线；只有真正离场才用加速的 exit。",
    "手势松开或运动中途改目标（拖动后松手、追随指针）用 spring：GPUI 保留速度掉头，Web 只能用 linear() 近似形状。",
    "expressive 曲线只配页面角色，留给少数值得注意的时刻；菜单、浮层等高频开合一律用 productive。UI 元素不用回弹。",
    "宏观动效先选模式：有空间关系用轴运动，有共享内容用容器转换，两者都没有就用淡入淡出。",
    "减少动效时整条阶梯归零，所以三种取值方式都会随之归零；spring 直接落位，循环停在第一帧。",
];

const SELF_CHECK: [&str; 9] = [
    "这个动效在说明什么？去掉它，用户会不会更难理解发生了什么？",
    "用的是语义角色吗？如果不是，token 旁是否写明了为什么要用阶梯档位或特殊值？",
    "进入与离开是否成对：离开更短，进入减速、离开加速？",
    "离开的东西会被马上召回吗？会的话用 stow，而不是 exit。",
    "运动会被中途打断，或由手势驱动吗？会的话用 spring。",
    "同时运动的多个元素是否共用同一时长与曲线，而不是各自为政？",
    "打开「减少动效」后，界面是否直接落到终态、状态仍然可读？",
    "慢放 4 倍看一遍：有没有跳变、闪烁或两段动效互相打架？",
    "首屏载入是否避免了逐个入场？直接呈现，或整体淡入即可。",
];

/// A curve another system ships, for comparison only.
struct Reference {
    source: &'static str,
    name: &'static str,
    points: (f32, f32, f32, f32),
    color: &'static str,
}

const fn reference(
    source: &'static str,
    name: &'static str,
    points: (f32, f32, f32, f32),
    color: &'static str,
) -> Reference {
    Reference {
        source,
        name,
        points,
        color,
    }
}

/// Moving within the screen. Yue's own curve is prepended from its token.
const STANDARD_REFERENCES: [Reference; 7] = [
    reference(
        "Carbon expressive",
        "standard",
        (0.4, 0.14, 0.3, 1.0),
        "--purple-500",
    ),
    reference(
        "Material 3",
        "standard",
        (0.2, 0.0, 0.0, 1.0),
        "--green-500",
    ),
    reference("Material 2", "standard", (0.4, 0.0, 0.2, 1.0), "--cyan-500"),
    reference(
        "Fluent 2",
        "easyEase",
        (0.33, 0.0, 0.67, 1.0),
        "--orange-500",
    ),
    reference(
        "TDesign",
        "standard",
        (0.38, 0.0, 0.24, 1.0),
        "--magenta-500",
    ),
    reference(
        "Ant Design",
        "ease-in-out",
        (0.645, 0.045, 0.355, 1.0),
        "--amber-500",
    ),
    reference("CSS", "ease", (0.25, 0.1, 0.25, 1.0), "--slate-500"),
];

/// Entering view: decelerating.
const ENTER_REFERENCES: [Reference; 7] = [
    reference(
        "Carbon expressive",
        "entrance",
        (0.0, 0.0, 0.3, 1.0),
        "--purple-500",
    ),
    reference(
        "Material 3",
        "emphasized decelerate",
        (0.05, 0.7, 0.1, 1.0),
        "--green-500",
    ),
    reference(
        "Material 2",
        "decelerate",
        (0.0, 0.0, 0.2, 1.0),
        "--cyan-500",
    ),
    reference(
        "Fluent 2",
        "decelerateMid",
        (0.0, 0.0, 0.0, 1.0),
        "--orange-500",
    ),
    reference(
        "TDesign",
        "ease-out",
        (0.0, 0.0, 0.15, 1.0),
        "--magenta-500",
    ),
    reference(
        "Ant Design",
        "ease-out",
        (0.215, 0.61, 0.355, 1.0),
        "--amber-500",
    ),
    reference("CSS", "ease-out", (0.0, 0.0, 0.58, 1.0), "--slate-500"),
];

/// Leaving view: accelerating.
const EXIT_REFERENCES: [Reference; 7] = [
    reference(
        "Carbon expressive",
        "exit",
        (0.4, 0.14, 1.0, 1.0),
        "--purple-500",
    ),
    reference(
        "Material 3",
        "emphasized accelerate",
        (0.3, 0.0, 0.8, 0.15),
        "--green-500",
    ),
    reference(
        "Material 2",
        "accelerate",
        (0.4, 0.0, 1.0, 1.0),
        "--cyan-500",
    ),
    reference(
        "Fluent 2",
        "accelerateMid",
        (1.0, 0.0, 1.0, 1.0),
        "--orange-500",
    ),
    reference("TDesign", "ease-in", (0.82, 0.0, 1.0, 0.9), "--magenta-500"),
    reference(
        "Ant Design",
        "ease-in",
        (0.55, 0.055, 0.675, 0.19),
        "--amber-500",
    ),
    reference("CSS", "ease-in", (0.42, 0.0, 1.0, 1.0), "--slate-500"),
];

/// How a component motion token reaches its value.
#[derive(Clone, Copy)]
enum Route {
    Role,
    Rung,
    Special,
}

impl Route {
    fn label(self) -> &'static str {
        match self {
            Self::Role => "语义角色",
            Self::Rung => "阶梯档位",
            Self::Special => "特殊值",
        }
    }
}

/// One component token: what it resolves to and why it takes that route.
struct ComponentValue {
    component: &'static str,
    duration: &'static str,
    ease: &'static str,
    source: &'static str,
    route: Route,
    reason: &'static str,
}

const COMPONENT_VALUES: [ComponentValue; 8] = [
    ComponentValue {
        component: "Button / Input / Tag",
        duration: "--button-duration",
        ease: "--button-ease",
        source: "interaction · standard",
        route: Route::Role,
        reason: "组件内的颜色、边框反馈",
    },
    ComponentValue {
        component: "Checkbox / Radio / Switch",
        duration: "--switch-duration",
        ease: "--switch-ease",
        source: "enter / exit · standard",
        route: Route::Role,
        reason: "勾选与滑块是一次小幅进出；取消时用 exit 角色",
    },
    ComponentValue {
        component: "Popover · 进入",
        duration: "--popover-duration-enter",
        ease: "--popover-ease",
        source: "enter · standard",
        route: Route::Role,
        reason: "小面板从触发器一侧展开",
    },
    ComponentValue {
        component: "Popover · 淡入",
        duration: "--popover-duration-enter",
        ease: "--popover-ease-fade",
        source: "enter · fade",
        route: Route::Role,
        reason: "伴随展开的透明度保持匀速，不读成第二段动效",
    },
    ComponentValue {
        component: "Dialog",
        duration: "--dialog-duration-enter",
        ease: "--dialog-ease",
        source: "enter-large · standard",
        route: Route::Role,
        reason: "覆盖页面的大面板，比浮层更从容",
    },
    ComponentValue {
        component: "Spinner / Button loading",
        duration: "--spinner-duration",
        ease: "--motion-ease-standard",
        source: "loop = --duration-300 × 3",
        route: Route::Role,
        reason: "循环角色本身由阶梯派生，不受 400ms 上限约束",
    },
    ComponentValue {
        component: "Menu item hover",
        duration: "--menu-item-duration",
        ease: "--menu-item-ease",
        source: "--duration-300 · --ease-in-out",
        route: Route::Rung,
        reason: "菜单行紧密相邻，指针扫过时多行同时过渡；更长、两端都缓的过渡让高亮像在行间滑动，而不是逐行闪烁",
    },
    ComponentValue {
        component: "Dropdown 行入场",
        duration: "--menu-item-duration",
        ease: "--menu-item-ease",
        source: "每行延迟 20ms（GPUI 常量 ROW_STAGGER）",
        route: Route::Special,
        reason: "错峰只用于菜单打开时的行入场；20ms 远小于阶梯最小档，是为列表节奏单独设定的值",
    },
];

pub(crate) fn motion_content(
    showcase: &Showcase,
    window: &mut Window,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let theme = YueTheme::new(
        yue_component::ThemeMode::Light,
        yue_component::Accent::Azure,
    );
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
                .child(div().text_2xl().child("Motion"))
                .child(
                    div()
                        .text_color(gpui_kit::rgb(theme.text_secondary()))
                        .child("一条有规律的时长阶梯，几种语义角色，允许有理由的特殊值；曲线沿用 Carbon productive"),
                ),
        )
        .child(controls(showcase, theme, cx))
        .child(block(
            theme,
            "Value rules",
            "组件各有自己的布局与尺度，不可能共用一个数值。Yue 给出一组有规律的值，并允许三种取值方式。",
            bullets(theme, &RULES),
        ))
        .child(block(
            theme,
            "Duration ladder",
            "--duration-100 到 --duration-400，每档 50ms。左侧条长与时长成正比，标签是落在这一档上的语义角色与组件。\
             点「播放」让各档同时出发，曲线统一用 standard。",
            ladder(showcase, theme, window, cx),
        ))
        .child(block(
            theme,
            "Semantic roles",
            "组件优先使用的默认值。下方演示直接读取组件 token：Popover 从触发器一侧展开并淡入，\
             Dialog 遮罩淡入、卡片从 0.96 放大；进度条与加载图标共用循环节奏。",
            roles(showcase, theme, window, cx),
        ))
        .child(block(
            theme,
            "Recallable exit · stow",
            "同一个大面板离开，左边是真正离场（exit：缓起、加速走开），右边是收进图标栏（stow：standard 两端都缓）。\
             收起的面板随时会被召回，加速离场会让它显得一去不返。",
            stow_demo(showcase, theme, window, cx),
        ))
        .child(block(
            theme,
            "Spring · 手势与打断",
            "在上方格子间来回移动指针，两颗滑块追随同一个目标。transition 每次改目标都从新曲线的起点重来，\
             速度会跳变；spring 保留当前速度，减速掉头。两者都在 --motion-duration-settle 内落位。",
            spring_demo(showcase, theme, window, cx),
        ))
        .child(block(
            theme,
            "Macro patterns · 宏观模式",
            "参照 TDesign：宏观动效按元素间的关系选模式。有空间关系（相邻标签、上下层级）用轴运动；\
             前后共享同一容器用容器转换；两者都没有就用淡入淡出。",
            macro_patterns(showcase, theme, window, cx),
        ))
        .child(block(
            theme,
            "Component values",
            "各组件 token 实际解析出的值与取值方式。阶梯档位与特殊值必须写明原因。",
            component_values(theme),
        ))
        .child(block(
            theme,
            "Special value · 菜单行 hover",
            "左边按交互角色（100ms · standard），右边是菜单实际使用的 300ms · ease-in-out。\
             快速上下扫过两列，对比高亮的连贯程度。",
            row_comparison(showcase, theme, window, cx),
        ))
        .child(block(
            theme,
            "Curves",
            "Yue 的 productive 曲线。横轴是时间，纵轴是进度；虚线对角线是匀速。点「播放」以 1.2s 同时运行。",
            yue_curves(showcase, theme, window, cx),
        ))
        .child(block(
            theme,
            "Expressive · 表现层",
            "Carbon expressive 三条曲线，加减速更强烈，只配页面角色（进入 400ms、离开 250ms），\
             用于页面切换、空状态、引导等少数值得注意的时刻。细线是对应的 productive 曲线。",
            expressive(showcase, theme, window, cx),
        ))
        .child(block(
            theme,
            "主流曲线对照",
            "同一用途下各设计体系的曲线，统一以 1.2s 运行。粗线是 Yue。\
             Yue 选 Carbon productive：起步不拖沓、收尾干脆，适合高频操作的工具界面；\
             expressive 与 Material 3 emphasized 更戏剧化，适合品牌与大幅转场。",
            reference_curves(showcase, theme, window, cx),
        ))
        .child(block(
            theme,
            "Reduced motion",
            "Web 端在 prefers-reduced-motion 下把阶梯七档全部改为 0ms，角色、档位和派生值随之归零；\
             循环停在第一帧，旁边的文字或 aria-busy 继续说明忙碌状态。GPUI 端读取系统设置，\
             Base 的每个 transition 与 spring 直接落到终态。",
            reduced_motion(showcase, theme),
        ))
        .child(block(
            theme,
            "Self-check",
            "提交一个新动效之前问自己：",
            bullets(theme, &SELF_CHECK),
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
                // The page scrolls, so its width is not bounded and a long
                // line would never wrap without a measure of its own.
                .max_w(px(1100.))
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
        .rounded_sm()
        .when(strong, |chip| {
            chip.bg(gpui_kit::rgb(theme.accent_subtle()))
                .text_color(gpui_kit::rgb(theme.accent_subtle_text()))
        })
        .when(!strong, |chip| {
            chip.border_1()
                .border_color(gpui_kit::rgb(theme.border_default()))
                .text_color(gpui_kit::rgb(theme.text_secondary()))
        })
        .child(text.into())
}

fn token_rgba(theme: YueTheme, name: &'static str) -> Rgba {
    theme
        .token(name)
        .and_then(|token| token.rgba())
        .unwrap_or_else(|| panic!("Yue token {name} is not a supported color"))
}

/// `--scrim-modal`, written `rgb(0 0 0/.5)`, which the token catalog's colour
/// reader does not take.
fn scrim(theme: YueTheme) -> Rgba {
    let value = theme
        .token("--scrim-modal")
        .map(|token| token.value())
        .unwrap_or_else(|| panic!("missing Yue token --scrim-modal"));
    let channels: Vec<f32> = value
        .trim_start_matches("rgb(")
        .trim_end_matches(')')
        .split([' ', '/'])
        .filter(|part| !part.is_empty())
        .map(|part| part.parse().expect("--scrim-modal is rgb(r g b/a)"))
        .collect();
    Rgba {
        r: channels[0] / 255.0,
        g: channels[1] / 255.0,
        b: channels[2] / 255.0,
        a: channels[3],
    }
}

fn mix(from: Rgba, to: Rgba, progress: f32) -> Rgba {
    let lerp = |a: f32, b: f32| a + (b - a) * progress;
    Rgba {
        r: lerp(from.r, to.r),
        g: lerp(from.g, to.g),
        b: lerp(from.b, to.b),
        a: lerp(from.a, to.a),
    }
}

/// `top` composited over an opaque `bottom`.
fn over(top: Rgba, bottom: Rgba) -> Rgba {
    let blend = |a: f32, b: f32| a * top.a + b * (1.0 - top.a);
    Rgba {
        r: blend(top.r, bottom.r),
        g: blend(top.g, bottom.g),
        b: blend(top.b, bottom.b),
        a: 1.0,
    }
}

fn bezier((x1, y1, x2, y2): (f32, f32, f32, f32)) -> Easing {
    Easing::CubicBezier { x1, y1, x2, y2 }
}

fn points_text((x1, y1, x2, y2): (f32, f32, f32, f32)) -> String {
    if (x1, y1, x2, y2) == (0.0, 0.0, 1.0, 1.0) {
        return "linear".into();
    }
    format!("cubic-bezier({x1}, {y1}, {x2}, {y2})")
}

fn paced(showcase: &Showcase, length: Duration) -> Duration {
    if showcase.motion_slow {
        length * SLOW_FACTOR
    } else {
        length
    }
}

/// The time a comparison race runs, slowed so the curves' shapes can be read.
fn race_length(theme: YueTheme) -> Duration {
    motion::rung(theme, 400) * 3
}

fn play_button(
    id: &'static str,
    theme: YueTheme,
    showcase: &Showcase,
    slot: usize,
    labels: (&'static str, &'static str),
    cx: &mut Context<Showcase>,
) -> gpui_kit::Div {
    let entity = cx.entity();
    // Wrapped so a column layout does not stretch the button across its row.
    div().flex().child(
        YueButton::text(
            id,
            theme,
            if showcase.motion_play[slot] {
                labels.1
            } else {
                labels.0
            },
        )
        .size(ButtonSize::Small)
        .variant(ButtonVariant::Outline)
        .on_click(move |_, _, cx| {
            entity.update(cx, |showcase, cx| {
                showcase.motion_play[slot] = !showcase.motion_play[slot];
                cx.notify();
            });
        }),
    )
}

fn controls(showcase: &Showcase, theme: YueTheme, cx: &mut Context<Showcase>) -> impl IntoElement {
    let entity = cx.entity();
    stage(theme)
        .flex()
        .items_center()
        .gap_8()
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    YueSwitch::new("motion-slow", theme, showcase.motion_slow, "慢放 ×4")
                        .size(SwitchSize::Small)
                        .on_change(move |checked, _, _, cx| {
                            entity.update(cx, |showcase, cx| {
                                showcase.motion_slow = checked;
                                cx.notify();
                            });
                        }),
                )
                .child(TextRole::Label.apply(theme, div()).child("慢放 ×4")),
        )
        .child(caption(
            theme,
            "作用于时长阶梯、语义角色和菜单行对比；曲线比赛固定为 1.2s。",
        ))
        .child(caption(
            theme,
            if showcase.motion_off {
                "侧栏「界面动效」已关闭：所有演示直接落到终态。"
            } else {
                "侧栏「界面动效」开启中。"
            },
        ))
}

fn ladder(
    showcase: &Showcase,
    theme: YueTheme,
    window: &mut Window,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let longest = *LADDER.last().expect("the ladder has rungs");
    // One linear clock over the longest rung; each rung reaches its end after
    // its own share of it, so they all start together.
    let clock = transition(
        "motion-ladder-clock",
        if showcase.motion_play[PLAY_LADDER] {
            1.0_f32
        } else {
            0.0
        },
        Transition::new(paced(showcase, motion::rung(theme, longest))).easing(Easing::Linear),
        window,
        cx,
    );
    let standard = bezier(MotionEase::Standard.control_points(theme));
    let track = 360.0;
    let dot = 12.0;
    let menu = motion::token_duration(theme, "--menu-item-duration").as_millis();

    let rows = LADDER.into_iter().map(|step| {
        let local = (clock * longest as f32 / step as f32).clamp(0.0, 1.0);
        let progress = standard.sample(local);
        let roles: Vec<_> = MotionDuration::ALL
            .into_iter()
            .filter(|role| role.duration(theme).as_millis() == u128::from(step))
            .map(|role| {
                chip(
                    theme,
                    role.token_name().trim_start_matches("--motion-duration-"),
                    true,
                )
            })
            .collect();
        div()
            .flex()
            .items_center()
            .gap_4()
            .child(
                TextRole::Code
                    .apply(theme, div())
                    .w_40()
                    .flex_shrink_0()
                    .child(format!("--duration-{step}")),
            )
            .child(
                div().w(px(longest as f32 * 0.4)).flex_shrink_0().child(
                    div()
                        .h_2()
                        .w(px(step as f32 * 0.4))
                        .rounded_sm()
                        .bg(gpui_kit::rgb(theme.accent_solid())),
                ),
            )
            .child(
                div()
                    .relative()
                    .flex_shrink_0()
                    .w(px(track))
                    .h(px(dot))
                    .rounded_full()
                    .bg(gpui_kit::rgb(theme.surface_component()))
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left(px(progress * (track - dot)))
                            .size(px(dot))
                            .rounded_full()
                            .bg(gpui_kit::rgb(theme.accent_solid())),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_1()
                    .children(roles)
                    .when(u128::from(step) == menu, |row| {
                        row.child(chip(theme, "menu-item", false))
                    }),
            )
    });

    stage(theme)
        .flex()
        .flex_col()
        .gap_3()
        .child(play_button(
            "motion-ladder-play",
            theme,
            showcase,
            PLAY_LADDER,
            ("播放", "返回"),
            cx,
        ))
        .children(rows)
        .child(caption(
            theme,
            format!(
                "循环：--motion-duration-loop = calc(--duration-300 × 3) = {}ms，进度条一趟 --motion-duration-loop-track = \
                 calc(--duration-300 × 6) = {}ms；都不在阶梯上，也不受 400ms 上限约束。",
                MotionDuration::Loop.duration(theme).as_millis(),
                MotionDuration::LoopTrack.duration(theme).as_millis()
            ),
        ))
}

fn role_card(theme: YueTheme, title: &'static str, lines: Vec<String>) -> gpui_kit::Div {
    stage(theme)
        .w(px(300.))
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap_3()
        .child(TextRole::Label.apply(theme, div()).child(title))
        .child(
            div()
                .flex()
                .flex_col()
                .children(lines.into_iter().map(|line| caption(theme, line))),
        )
}

fn ms(length: Duration) -> String {
    format!("{}ms", length.as_millis())
}

fn roles(
    showcase: &Showcase,
    theme: YueTheme,
    window: &mut Window,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_wrap()
        .gap_4()
        .child(interaction_card(showcase, theme, window, cx))
        .child(popover_card(showcase, theme, window, cx))
        .child(dialog_card(showcase, theme, window, cx))
        .child(loop_card(showcase, theme, cx))
        .child(progress_card(showcase, theme, cx))
}

fn interaction_card(
    showcase: &Showcase,
    theme: YueTheme,
    window: &mut Window,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let length = MotionDuration::Interaction.duration(theme);
    let tint = transition(
        "motion-interaction",
        if showcase.motion_hover { 1.0_f32 } else { 0.0 },
        Transition::new(paced(showcase, length)).ease(MotionEase::Standard.easing(theme)),
        window,
        cx,
    );
    let surface = token_rgba(theme, "--surface");
    let hover = over(
        token_rgba(theme, "--button-ghost-background-hover"),
        surface,
    );
    let border = mix(
        token_rgba(theme, "--border-default"),
        token_rgba(theme, "--accent-border"),
        tint,
    );
    role_card(
        theme,
        "Interaction · 交互反馈",
        vec![
            format!("--motion-duration-interaction · {}", ms(length)),
            "--motion-ease-standard".into(),
            "hover、按下、颜色与边框变化".into(),
        ],
    )
    .child(
        div()
            .id("motion-interaction-target")
            .h(px(120.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .border_1()
            .border_color(border)
            .bg(mix(surface, hover, tint))
            .on_hover(cx.listener(|showcase, hovered: &bool, _, cx| {
                showcase.motion_hover = *hovered;
                cx.notify();
            }))
            .child(TextRole::Body.apply(theme, div()).child("悬停这里")),
    )
}

fn popover_card(
    showcase: &Showcase,
    theme: YueTheme,
    window: &mut Window,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let open = showcase.motion_play[PLAY_POPOVER];
    let enter = motion::token_duration(theme, "--popover-duration-enter");
    let exit = motion::token_duration(theme, "--popover-duration-exit");
    let length = paced(showcase, if open { enter } else { exit });
    let target = if open { 1.0_f32 } else { 0.0 };
    let reveal = transition(
        "motion-popover-reveal",
        target,
        Transition::new(length).easing(bezier(motion::token_curve(theme, "--popover-ease"))),
        window,
        cx,
    );
    // Entering fades at a constant rate beside the reveal; leaving fades along
    // the reveal's own curve, as the web's keyframes do.
    let fade_curve = if open {
        "--popover-ease-fade"
    } else {
        "--popover-ease"
    };
    let fade = transition(
        "motion-popover-fade",
        target,
        Transition::new(length).easing(bezier(motion::token_curve(theme, fade_curve))),
        window,
        cx,
    );
    let panel = 72.0;
    role_card(
        theme,
        "Enter / Exit · 小面板",
        vec![
            format!("--popover-duration-enter · {}", ms(enter)),
            format!("--popover-duration-exit · {}", ms(exit)),
            "展开 standard，淡入 fade（匀速）".into(),
        ],
    )
    .child(
        div()
            .h(px(120.))
            .flex()
            .flex_col()
            .gap_1()
            .child(play_button(
                "motion-popover-play",
                theme,
                showcase,
                PLAY_POPOVER,
                ("打开", "关闭"),
                cx,
            ))
            .child(
                // The web reveals the surface with a clip-path collapsed onto
                // the edge facing its trigger; a clipped height does the same.
                div()
                    .w(px(200.))
                    .h(px(panel * reveal))
                    .overflow_hidden()
                    .opacity(fade)
                    .child(
                        div()
                            .h(px(panel))
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(gpui_kit::rgb(theme.border_default()))
                            .bg(gpui_kit::rgb(theme.surface()))
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(TextRole::Body.apply(theme, div()).child("浮层内容"))
                            .child(caption(theme, "从触发器一侧展开")),
                    ),
            ),
    )
}

fn dialog_card(
    showcase: &Showcase,
    theme: YueTheme,
    window: &mut Window,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let open = showcase.motion_play[PLAY_DIALOG];
    let enter = motion::token_duration(theme, "--dialog-duration-enter");
    let exit = motion::token_duration(theme, "--dialog-duration-exit");
    let length = paced(showcase, if open { enter } else { exit });
    let progress = transition(
        "motion-dialog",
        if open { 1.0_f32 } else { 0.0 },
        Transition::new(length).easing(bezier(motion::token_curve(theme, "--dialog-ease"))),
        window,
        cx,
    );
    // GPUI divs cannot scale; the card's size runs .96 → 1 around its centre.
    let scale = 0.96 + 0.04 * progress;
    role_card(
        theme,
        "Enter / Exit large · 大面板",
        vec![
            format!("--dialog-duration-enter · {}", ms(enter)),
            format!("--dialog-duration-exit · {}", ms(exit)),
            "遮罩淡入；卡片淡入并从 0.96 放大".into(),
        ],
    )
    .child(
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(play_button(
                "motion-dialog-play",
                theme,
                showcase,
                PLAY_DIALOG,
                ("打开", "关闭"),
                cx,
            ))
            .child(
                div()
                    .relative()
                    .h(px(120.))
                    .rounded_md()
                    .overflow_hidden()
                    .bg(gpui_kit::rgb(theme.page()))
                    .border_1()
                    .border_color(gpui_kit::rgb(theme.border_default()))
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .bg(scrim(theme))
                            .opacity(progress),
                    )
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .w(px(180. * scale))
                                    .h(px(80. * scale))
                                    .opacity(progress)
                                    .p_2()
                                    .rounded_md()
                                    .bg(gpui_kit::rgb(theme.surface()))
                                    .child(TextRole::Body.apply(theme, div()).child("对话框")),
                            ),
                    ),
            ),
    )
}

fn loop_card(showcase: &Showcase, theme: YueTheme, cx: &mut Context<Showcase>) -> impl IntoElement {
    let length = MotionDuration::Loop.duration(theme);
    let icon = Icon::new(YueIcon::Loading)
        .size(IconSize::Large.rems(theme))
        .color(theme.accent_solid());
    // Reduced motion stops the turn on its first frame; the text beside the
    // indicator keeps saying it is busy.
    let indicator = if cx.reduce_motion() {
        icon.into_any_element()
    } else {
        icon.with_animation(
            "motion-loop",
            Animation::new(paced(showcase, length))
                .repeat()
                .with_easing(MotionEase::Standard.easing(theme)),
            |icon, delta| icon.rotation(delta),
        )
        .into_any_element()
    };
    role_card(
        theme,
        "Loop · 循环",
        vec![
            format!("--motion-duration-loop · {}", ms(length)),
            "calc(--duration-300 × 3)".into(),
            "不确定进度的忙碌指示".into(),
        ],
    )
    .child(
        div()
            .h(px(120.))
            .flex()
            .items_center()
            .justify_center()
            .gap_2()
            .child(indicator)
            .child(TextRole::Body.apply(theme, div()).child("加载中…")),
    )
}

fn progress_card(
    showcase: &Showcase,
    theme: YueTheme,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let length = MotionDuration::LoopTrack.duration(theme);
    let track = 240.0;
    let bar = 72.0;
    let segment = div()
        .absolute()
        .top_0()
        .h_full()
        .w(px(bar))
        .rounded_full()
        .bg(gpui_kit::rgb(theme.accent_solid()));
    // Reduced motion parks the segment mid-track; the text keeps saying busy.
    let segment = if cx.reduce_motion() {
        segment.left(px((track - bar) / 2.0)).into_any_element()
    } else {
        segment
            .with_animation(
                "motion-loop-track",
                Animation::new(paced(showcase, length))
                    .repeat()
                    .with_easing(MotionEase::Standard.easing(theme)),
                move |segment, delta| segment.left(px(-bar + delta * (track + bar))),
            )
            .into_any_element()
    };
    role_card(
        theme,
        "Loop track · 进度条",
        vec![
            format!("--motion-duration-loop-track · {}", ms(length)),
            "calc(--duration-300 × 6)，曲线 standard".into(),
            "不确定进度的进度条，一趟横穿轨道".into(),
        ],
    )
    .child(
        div()
            .h(px(120.))
            .flex()
            .flex_col()
            .justify_center()
            .gap_2()
            .child(
                div()
                    .relative()
                    .w(px(track))
                    .h(px(4.))
                    .rounded_full()
                    .overflow_hidden()
                    .bg(gpui_kit::rgb(theme.surface_component()))
                    .child(segment),
            )
            .child(TextRole::Body.apply(theme, div()).child("正在导入…")),
    )
}

/// A drawer pinned to the right edge of a small page.
fn drawer_frame(theme: YueTheme, drawer: gpui_kit::Div) -> gpui_kit::Div {
    div()
        .relative()
        .w(px(320.))
        .h(px(140.))
        .rounded_md()
        .overflow_hidden()
        .border_1()
        .border_color(gpui_kit::rgb(theme.border_default()))
        .bg(gpui_kit::rgb(theme.page()))
        .child(div().p_3().child(caption(theme, "页面内容")))
        .child(drawer)
}

fn drawer_panel(theme: YueTheme) -> gpui_kit::Div {
    div()
        .absolute()
        .top_0()
        .h_full()
        .overflow_hidden()
        .border_l_1()
        .border_color(gpui_kit::rgb(theme.border_default()))
        .bg(gpui_kit::rgb(theme.surface()))
}

fn stow_demo(
    showcase: &Showcase,
    theme: YueTheme,
    window: &mut Window,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let away = showcase.motion_play[PLAY_STOW];
    let enter = MotionDuration::EnterLarge.duration(theme);
    let exit = MotionDuration::ExitLarge.duration(theme);
    let length = paced(showcase, if away { exit } else { enter });
    let target = if away { 1.0_f32 } else { 0.0 };
    // Removed for good: accelerates away, and decelerates back in.
    let removed = transition(
        "motion-stow-removed",
        target,
        Transition::new(length).easing(bezier(
            if away {
                MotionEase::Exit
            } else {
                MotionEase::Enter
            }
            .control_points(theme),
        )),
        window,
        cx,
    );
    // Stowed to a rail: standard both ways, since it is coming back.
    let stowed = transition(
        "motion-stow-stowed",
        target,
        Transition::new(length).easing(bezier(MotionEase::Stow.control_points(theme))),
        window,
        cx,
    );
    let width = 160.0;
    let rail = 40.0;
    let body = |opacity: f32| {
        div()
            .w(px(width))
            .p_3()
            .flex()
            .flex_col()
            .gap_1()
            .opacity(opacity)
            .child(TextRole::Label.apply(theme, div()).child("属性面板"))
            .child(caption(theme, "尺寸 · 颜色 · 图层"))
    };
    let left = drawer_frame(
        theme,
        drawer_panel(theme)
            .left(px(320. - width + removed * width))
            .w(px(width))
            .child(body(1.0)),
    );
    let current = width - (width - rail) * stowed;
    let right = drawer_frame(
        theme,
        drawer_panel(theme)
            .right_0()
            .w(px(current))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .w(px(rail))
                    .pt_3()
                    .flex()
                    .justify_center()
                    .opacity(stowed)
                    .child(
                        Icon::new(YueIcon::Info)
                            .size(IconSize::Small.rems(theme))
                            .color(theme.text_secondary()),
                    ),
            )
            .child(body(1.0 - stowed)),
    );
    let column = |title: String, frame: gpui_kit::Div| {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(caption(theme, title))
            .child(frame)
    };
    stage(theme)
        .flex()
        .flex_col()
        .gap_3()
        .child(play_button(
            "motion-stow-play",
            theme,
            showcase,
            PLAY_STOW,
            ("离开", "回来"),
            cx,
        ))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_8()
                .child(column(
                    format!("移除 · exit-large {} · exit，回来时 enter", ms(exit)),
                    left,
                ))
                .child(column(
                    format!(
                        "收起到图标栏 · exit-large {} · stow（= standard）",
                        ms(exit)
                    ),
                    right,
                )),
        )
}

fn spring_demo(
    showcase: &Showcase,
    theme: YueTheme,
    window: &mut Window,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    const CELLS: usize = 5;
    let cell = 88.0;
    let knob = 20.0;
    let track = cell * CELLS as f32;
    let target = showcase.motion_chase as f32 / (CELLS - 1) as f32;
    let settle = MotionDuration::Settle.duration(theme);
    let eased = transition(
        "motion-chase-transition",
        target,
        Transition::new(paced(showcase, settle))
            .easing(bezier(MotionEase::Standard.control_points(theme))),
        window,
        cx,
    );
    // `motion::settle_spring` with its period stretched by the slow-motion
    // switch; a spring has no duration to pace.
    let sprung = spring(
        "motion-chase-spring",
        target,
        Spring::new(paced(showcase, motion::settle_response(theme))),
        window,
        cx,
    );
    let cells = (0..CELLS).map(|index| {
        let active = showcase.motion_chase == index;
        div()
            .id(SharedString::from(format!("motion-chase-{index}")))
            .w(px(cell))
            .h(px(48.))
            .flex()
            .items_center()
            .justify_center()
            .border_1()
            .border_color(gpui_kit::rgb(theme.border_default()))
            .when(active, |cell| cell.bg(gpui_kit::rgb(theme.accent_subtle())))
            .on_hover(cx.listener(move |showcase, hovered: &bool, _, cx| {
                if *hovered {
                    showcase.motion_chase = index;
                    cx.notify();
                }
            }))
            .child(caption(theme, format!("{}", index + 1)))
    });
    let lane = |label: String, value: f32, color: Hsla| {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(caption(theme, label))
            .child(
                div()
                    .relative()
                    .w(px(track))
                    .h(px(knob))
                    .child(
                        div()
                            .absolute()
                            .top(px(knob / 2.0 - 1.0))
                            .left(px(cell / 2.0))
                            .w(px(track - cell))
                            .h(px(2.))
                            .bg(gpui_kit::rgb(theme.surface_component())),
                    )
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .left(px(cell / 2.0 - knob / 2.0 + value * (track - cell)))
                            .size(px(knob))
                            .rounded_full()
                            .bg(color),
                    ),
            )
    };
    let transition_color: Hsla = token_rgba(theme, "--orange-500").into();
    let spring_color: Hsla = token_rgba(theme, "--azure-500").into();
    stage(theme)
        .flex()
        .flex_wrap()
        .gap_8()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(div().flex().children(cells))
                .child(lane(
                    format!("transition · {} · standard", ms(settle)),
                    eased,
                    transition_color,
                ))
                .child(lane(
                    format!(
                        "spring · 临界阻尼，{} 落位（Spring::new({}ms)）",
                        ms(settle),
                        motion::settle_response(theme).as_millis()
                    ),
                    sprung,
                    spring_color,
                )),
        )
        .child(
            div()
                .flex()
                .gap_4()
                .child(plot(
                    theme,
                    160.,
                    vec![
                        (
                            bezier(MotionEase::Enter.control_points(theme)),
                            transition_color,
                            1.5,
                        ),
                        (motion::spring_curve(theme), spring_color, 2.5),
                    ],
                    None,
                ))
                .child(
                    div()
                        .w(px(300.))
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(caption(
                            theme,
                            "蓝：--motion-ease-spring，Web 用 linear() 采样的同一条 spring 曲线",
                        ))
                        .child(caption(theme, "橙：enter 曲线，作对照"))
                        .child(caption(
                            theme,
                            "CSS 无法把手势速度带进曲线，所以 Web 只用到 spring 的形状；\
                             需要中途掉头的值，在 GPUI 用 settle_spring。",
                        )),
                ),
        )
}

/// One macro-pattern panel: a title, a caption and a 120px stage.
fn pattern_panel(
    theme: YueTheme,
    title: &'static str,
    note: &'static str,
    scene: gpui_kit::Div,
) -> gpui_kit::Div {
    div()
        .w(px(260.))
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap_2()
        .child(TextRole::Label.apply(theme, div()).child(title))
        .child(
            scene
                .relative()
                .h(px(120.))
                .rounded_md()
                .overflow_hidden()
                .border_1()
                .border_color(gpui_kit::rgb(theme.border_default()))
                .bg(gpui_kit::rgb(theme.page())),
        )
        .child(caption(theme, note))
}

fn sheet(theme: YueTheme, title: &'static str) -> gpui_kit::Div {
    div()
        .absolute()
        .top(px(12.))
        .left(px(12.))
        .w(px(234.))
        .h(px(94.))
        .p_2()
        .rounded_md()
        .bg(gpui_kit::rgb(theme.surface()))
        .border_1()
        .border_color(gpui_kit::rgb(theme.border_default()))
        .child(TextRole::Body.apply(theme, div()).child(title))
}

fn macro_patterns(
    showcase: &Showcase,
    theme: YueTheme,
    window: &mut Window,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let on = showcase.motion_play[PLAY_PATTERNS];
    let target = if on { 1.0_f32 } else { 0.0 };
    let enter = MotionDuration::EnterLarge.duration(theme);
    let exit = MotionDuration::ExitLarge.duration(theme);
    let standard = bezier(MotionEase::Standard.control_points(theme));
    // The outgoing view clears the first half, the incoming one the second, so
    // the two never sit on top of each other at full strength.
    let first = |p: f32| (1.0 - 2.0 * p).clamp(0.0, 1.0);
    let second = |p: f32| (2.0 * p - 1.0).clamp(0.0, 1.0);

    let axis = transition(
        "motion-pattern-axis",
        target,
        Transition::new(paced(showcase, enter)).easing(standard.clone()),
        window,
        cx,
    );
    let shift = 32.0;
    let axis_scene = div()
        .child(
            sheet(theme, "标签 A")
                .left(px(12. - shift * axis))
                .opacity(first(axis)),
        )
        .child(
            sheet(theme, "标签 B")
                .left(px(12. + shift * (1.0 - axis)))
                .opacity(second(axis)),
        );

    let morph = transition(
        "motion-pattern-container",
        target,
        Transition::new(paced(showcase, enter)).easing(standard),
        window,
        cx,
    );
    let lerp = |from: f32, to: f32| from + (to - from) * morph;
    let container_scene = div().child(
        div()
            .absolute()
            .top(px(12.))
            .left(px(12.))
            .w(px(lerp(96., 234.)))
            .h(px(lerp(32., 94.)))
            .rounded_md()
            .bg(gpui_kit::rgb(theme.surface()))
            .border_1()
            .border_color(token_rgba(theme, "--accent-border"))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .h(px(30.))
                    .px_2()
                    .flex()
                    .items_center()
                    .opacity(first(morph))
                    .child(TextRole::Body.apply(theme, div()).child("一张卡片")),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .p_2()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .opacity(second(morph))
                    .child(TextRole::Body.apply(theme, div()).child("卡片详情"))
                    .child(caption(theme, "由同一个容器形变而来")),
            ),
    );

    // Fade through: the old view fades out over the exit role, then the new
    // one fades in over the enter role. One linear clock spans both.
    let total = exit + enter;
    let clock = transition(
        "motion-pattern-fade",
        target,
        Transition::new(paced(showcase, total)).easing(Easing::Linear),
        window,
        cx,
    );
    let split = exit.as_secs_f32() / total.as_secs_f32();
    let out = (clock / split).clamp(0.0, 1.0);
    let incoming = bezier(MotionEase::Enter.control_points(theme))
        .sample(((clock - split) / (1.0 - split)).clamp(0.0, 1.0));
    let fade_scene = div()
        .child(sheet(theme, "通知列表").opacity(1.0 - out))
        .child(sheet(theme, "设置").opacity(incoming));

    stage(theme)
        .flex()
        .flex_col()
        .gap_3()
        .child(play_button(
            "motion-patterns-play",
            theme,
            showcase,
            PLAY_PATTERNS,
            ("切换", "返回"),
            cx,
        ))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_8()
                .child(pattern_panel(
                    theme,
                    "轴运动 · 有空间关系",
                    "相邻标签沿 X 轴互换；纵向关系走 Y 轴，层级关系（弹窗、通知）走 Z 轴，即缩放加淡入。",
                    axis_scene,
                ))
                .child(pattern_panel(
                    theme,
                    "容器转换 · 有共享内容",
                    "卡片展开成详情：容器连续形变，旧内容先淡出，新内容后淡入。",
                    container_scene,
                ))
                .child(pattern_panel(
                    theme,
                    "淡入淡出 · 无关联",
                    "前后没有空间或容器关系：旧视图按 exit 时长淡出，新视图接着按 enter 淡入。",
                    fade_scene,
                )),
        )
        .child(caption(
            theme,
            "编排：同一序列只保留一个主方向；双轴位移走曲线路径而非直线；网格重排沿直线补位，不出画再入画；\
             首屏载入不做逐个入场，直接呈现或整体淡入。",
        )
        .max_w(px(1100.)))
}

fn expressive(
    showcase: &Showcase,
    theme: YueTheme,
    window: &mut Window,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let clock = race_clock(
        "motion-expressive-clock",
        showcase,
        PLAY_EXPRESSIVE,
        theme,
        window,
        cx,
    );
    let pairs = [
        (
            MotionEase::ExpressiveStandard,
            MotionEase::Standard,
            "--azure-500",
        ),
        (
            MotionEase::ExpressiveEnter,
            MotionEase::Enter,
            "--green-500",
        ),
        (MotionEase::ExpressiveExit, MotionEase::Exit, "--orange-500"),
    ];
    let mut entries: Vec<(String, Easing, Hsla)> = Vec::new();
    let mut curves = Vec::new();
    for (expressive, productive, color) in pairs {
        let color: Hsla = token_rgba(theme, color).into();
        let points = expressive.control_points(theme);
        entries.push((
            format!("{} · {}", expressive.token_name(), points_text(points)),
            bezier(points),
            color,
        ));
        curves.push((
            bezier(productive.control_points(theme)),
            color.opacity(0.5),
            1.0,
        ));
        curves.push((bezier(points), color, 2.5));
    }

    // A page arriving: rises a little and fades in along expressive-enter;
    // leaving fades along expressive-exit, in the shorter page-exit time.
    let open = showcase.motion_play[PLAY_PAGE];
    let enter = MotionDuration::EnterPage.duration(theme);
    let exit = MotionDuration::ExitPage.duration(theme);
    let page = transition(
        "motion-expressive-page",
        if open { 1.0_f32 } else { 0.0 },
        Transition::new(paced(showcase, if open { enter } else { exit })).easing(bezier(
            if open {
                MotionEase::ExpressiveEnter
            } else {
                MotionEase::ExpressiveExit
            }
            .control_points(theme),
        )),
        window,
        cx,
    );
    let page_demo = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(play_button(
            "motion-page-play",
            theme,
            showcase,
            PLAY_PAGE,
            ("进入页面", "离开页面"),
            cx,
        ))
        .child(
            div()
                .relative()
                .w(px(280.))
                .h(px(160.))
                .rounded_md()
                .overflow_hidden()
                .border_1()
                .border_color(gpui_kit::rgb(theme.border_default()))
                .bg(gpui_kit::rgb(theme.page()))
                .child(
                    div()
                        .absolute()
                        .left(px(12.))
                        .top(px(12. + 24. * (1.0 - page)))
                        .w(px(254.))
                        .h(px(134.))
                        .p_3()
                        .rounded_md()
                        .bg(gpui_kit::rgb(theme.surface()))
                        .opacity(page)
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(TextRole::Title.apply(theme, div()).child("欢迎使用"))
                        .child(caption(theme, "空状态、引导页、首次进入的页面")),
                ),
        )
        .child(caption(
            theme,
            format!(
                "enter-page {} · expressive-enter；exit-page {} · expressive-exit",
                ms(enter),
                ms(exit)
            ),
        ));

    stage(theme)
        .flex()
        .flex_col()
        .gap_3()
        .child(play_button(
            "motion-expressive-play",
            theme,
            showcase,
            PLAY_EXPRESSIVE,
            ("播放", "返回"),
            cx,
        ))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_8()
                .child(plot(theme, 240., curves, Some(clock)))
                .child(lanes(theme, &entries, clock, 360.))
                .child(page_demo),
        )
}

fn component_values(theme: YueTheme) -> impl IntoElement {
    let cell = |width: f32| div().w(px(width)).flex_shrink_0();
    let header = div()
        .flex()
        .gap_4()
        .pb_2()
        .border_b_1()
        .border_color(token_rgba(theme, "--border-divider"))
        .child(cell(180.).child(caption(theme, "组件")))
        .child(cell(220.).child(caption(theme, "token")))
        .child(cell(230.).child(caption(theme, "解析值")))
        .child(cell(200.).child(caption(theme, "来源")))
        .child(cell(80.).child(caption(theme, "方式")))
        .child(div().w(px(300.)).child(caption(theme, "原因")));
    let rows = COMPONENT_VALUES.iter().map(|row| {
        let length = motion::token_duration(theme, row.duration);
        let curve = motion::token_curve(theme, row.ease);
        div()
            .flex()
            .gap_4()
            .py_2()
            .border_b_1()
            .border_color(token_rgba(theme, "--border-divider"))
            .child(cell(180.).child(TextRole::Body.apply(theme, div()).child(row.component)))
            .child(
                cell(220.)
                    .flex()
                    .flex_col()
                    .child(TextRole::Code.apply(theme, div()).child(row.duration))
                    .child(TextRole::Code.apply(theme, div()).child(row.ease)),
            )
            .child(
                cell(230.)
                    .flex()
                    .flex_col()
                    .child(TextRole::Body.apply(theme, div()).child(ms(length)))
                    .child(caption(theme, points_text(curve))),
            )
            .child(cell(200.).child(caption(theme, row.source)))
            .child(cell(80.).child(chip(
                theme,
                row.route.label(),
                matches!(row.route, Route::Role),
            )))
            .child(div().w(px(300.)).child(caption(theme, row.reason)))
    });
    stage(theme).flex().flex_col().child(header).children(rows)
}

fn row_comparison(
    showcase: &Showcase,
    theme: YueTheme,
    window: &mut Window,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let surface = token_rgba(theme, "--surface");
    let hover = over(token_rgba(theme, "--menu-item-background-hover"), surface);
    let column = |column: usize,
                  title: String,
                  policy: &dyn Fn() -> Transition,
                  window: &mut Window,
                  cx: &mut Context<Showcase>| {
        let rows: Vec<_> = (0..5)
            .map(|row| {
                let tint = transition(
                    SharedString::from(format!("motion-row-{column}-{row}")),
                    if showcase.motion_rows[column] == Some(row) {
                        1.0_f32
                    } else {
                        0.0
                    },
                    policy(),
                    window,
                    cx,
                );
                div()
                    .id(SharedString::from(format!(
                        "motion-row-target-{column}-{row}"
                    )))
                    .h(px(32.))
                    .px_3()
                    .flex()
                    .items_center()
                    .rounded_sm()
                    .bg(mix(surface, hover, tint))
                    .on_hover(cx.listener(move |showcase, hovered: &bool, _, cx| {
                        if *hovered {
                            showcase.motion_rows[column] = Some(row);
                        } else if showcase.motion_rows[column] == Some(row) {
                            showcase.motion_rows[column] = None;
                        }
                        cx.notify();
                    }))
                    .child(
                        TextRole::Body
                            .apply(theme, div())
                            .child(format!("菜单项 {}", row + 1)),
                    )
            })
            .collect();
        div()
            .w(px(240.))
            .flex()
            .flex_col()
            .gap_2()
            .child(caption(theme, title))
            .child(
                div()
                    .p_1()
                    .rounded_md()
                    .border_1()
                    .border_color(gpui_kit::rgb(theme.border_default()))
                    .bg(gpui_kit::rgb(theme.surface()))
                    .flex()
                    .flex_col()
                    .children(rows),
            )
    };
    let interaction = MotionDuration::Interaction.duration(theme);
    let menu = motion::token_duration(theme, "--menu-item-duration");
    let menu_curve = motion::token_curve(theme, "--menu-item-ease");
    let left = column(
        0,
        format!("交互角色 · {} · standard", ms(interaction)),
        &|| {
            Transition::new(paced(showcase, interaction))
                .easing(bezier(MotionEase::Standard.control_points(theme)))
        },
        window,
        cx,
    );
    let right = column(
        1,
        format!("--menu-item-* · {} · {}", ms(menu), points_text(menu_curve)),
        &|| Transition::new(paced(showcase, menu)).easing(bezier(menu_curve)),
        window,
        cx,
    );
    stage(theme).flex().gap_8().child(left).child(right)
}

/// A unit-square plot of `curves`, with an optional time marker at `time`.
fn plot(
    theme: YueTheme,
    side: f32,
    curves: Vec<(Easing, Hsla, f32)>,
    time: Option<f32>,
) -> impl IntoElement {
    let guide: Hsla = gpui_kit::rgb(theme.border_default()).into();
    let marker: Hsla = gpui_kit::rgb(theme.text_secondary()).into();
    let inset = 10.0;
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let span = side - 2.0 * inset;
            let at = |x: f32, y: f32| {
                point(
                    bounds.origin.x + px(inset + x * span),
                    bounds.origin.y + px(inset + (1.0 - y) * span),
                )
            };
            let mut frame = PathBuilder::stroke(px(1.));
            frame.move_to(at(0., 0.));
            frame.line_to(at(1., 0.));
            frame.line_to(at(1., 1.));
            frame.line_to(at(0., 1.));
            frame.line_to(at(0., 0.));
            if let Ok(path) = frame.build() {
                window.paint_path(path, guide);
            }
            // The linear diagonal, dashed so it reads as a reference.
            let mut diagonal = PathBuilder::stroke(px(1.));
            for dash in 0..20 {
                let start = dash as f32 / 20.0;
                diagonal.move_to(at(start, start));
                diagonal.line_to(at(start + 0.025, start + 0.025));
            }
            if let Ok(path) = diagonal.build() {
                window.paint_path(path, guide);
            }
            if let Some(time) = time {
                let mut line = PathBuilder::stroke(px(1.));
                line.move_to(at(time, 0.));
                line.line_to(at(time, 1.));
                if let Ok(path) = line.build() {
                    window.paint_path(path, marker);
                }
            }
            for (easing, color, width) in &curves {
                let mut stroke = PathBuilder::stroke(px(*width));
                stroke.move_to(at(0., easing.sample(0.)));
                for step in 1..=96 {
                    let x = step as f32 / 96.0;
                    stroke.line_to(at(x, easing.sample(x)));
                }
                if let Ok(path) = stroke.build() {
                    window.paint_path(path, *color);
                }
                if let Some(time) = time {
                    let centre = at(time, easing.sample(time));
                    let radius = 3.5;
                    let mut dot = PathBuilder::fill();
                    for corner in 0..8 {
                        let angle = corner as f32 * std::f32::consts::FRAC_PI_4;
                        let vertex = point(
                            centre.x + px(radius * angle.cos()),
                            centre.y + px(radius * angle.sin()),
                        );
                        if corner == 0 {
                            dot.move_to(vertex);
                        } else {
                            dot.line_to(vertex);
                        }
                    }
                    dot.close();
                    if let Ok(path) = dot.build() {
                        window.paint_path(path, *color);
                    }
                }
            }
        },
    )
    .size(px(side))
    .flex_shrink_0()
}

/// One race lane per curve, driven by a shared linear clock.
fn lanes(
    theme: YueTheme,
    entries: &[(String, Easing, Hsla)],
    clock: f32,
    track: f32,
) -> gpui_kit::Div {
    let dot = 10.0;
    div()
        .flex()
        .flex_col()
        .gap_2()
        .children(entries.iter().map(|(label, easing, color)| {
            let progress = easing.sample(clock);
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().flex_shrink_0().size_2().rounded_full().bg(*color))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(caption(theme, label.clone())),
                        ),
                )
                .child(
                    div()
                        .relative()
                        .w(px(track))
                        .h(px(dot))
                        .rounded_full()
                        .bg(gpui_kit::rgb(theme.surface_component()))
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .left(px(progress * (track - dot)))
                                .size(px(dot))
                                .rounded_full()
                                .bg(*color),
                        ),
                )
        }))
}

fn race_clock(
    id: &'static str,
    showcase: &Showcase,
    slot: usize,
    theme: YueTheme,
    window: &mut Window,
    cx: &mut Context<Showcase>,
) -> f32 {
    transition(
        id,
        if showcase.motion_play[slot] {
            1.0_f32
        } else {
            0.0
        },
        Transition::new(race_length(theme)).easing(Easing::Linear),
        window,
        cx,
    )
}

fn yue_curves(
    showcase: &Showcase,
    theme: YueTheme,
    window: &mut Window,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let clock = race_clock(
        "motion-curves-clock",
        showcase,
        PLAY_CURVES,
        theme,
        window,
        cx,
    );
    let colors = [
        "--azure-500",
        "--green-500",
        "--orange-500",
        "--purple-500",
        "--slate-500",
    ];
    let entries: Vec<(String, Easing, Hsla)> = MotionEase::PRODUCTIVE
        .into_iter()
        .zip(colors)
        .map(|(ease, color)| {
            let points = ease.control_points(theme);
            (
                format!("{} · {}", ease.token_name(), points_text(points)),
                bezier(points),
                token_rgba(theme, color).into(),
            )
        })
        .collect();
    // Stow is standard by design; drawn thinner on top so both stay visible.
    let curves = entries
        .iter()
        .zip(MotionEase::PRODUCTIVE)
        .map(|((_, easing, color), ease)| {
            (
                easing.clone(),
                *color,
                if ease == MotionEase::Stow { 1.0 } else { 2.0 },
            )
        })
        .collect();
    stage(theme)
        .flex()
        .flex_col()
        .gap_3()
        .child(play_button(
            "motion-curves-play",
            theme,
            showcase,
            PLAY_CURVES,
            ("播放", "返回"),
            cx,
        ))
        .child(
            div()
                .flex()
                .gap_8()
                .child(plot(theme, 240., curves, Some(clock)))
                .child(lanes(theme, &entries, clock, 360.)),
        )
        .child(caption(
            theme,
            "standard 两端都缓，用于屏幕内的移动与形变；enter 起步即达峰速再减速落位；\
             exit 缓慢起步后加速离开，在终点前达到最快；stow 就是 standard，用于会被召回的离开；fade 匀速。",
        )
        .max_w(px(1100.)))
}

fn reference_curves(
    showcase: &Showcase,
    theme: YueTheme,
    window: &mut Window,
    cx: &mut Context<Showcase>,
) -> impl IntoElement {
    let clock = race_clock(
        "motion-reference-clock",
        showcase,
        PLAY_REFERENCE,
        theme,
        window,
        cx,
    );
    let yue: Hsla = gpui_kit::rgb(theme.text_primary()).into();
    let panel = |title: &'static str, ease: MotionEase, references: &[Reference]| {
        let mut entries: Vec<(String, Easing, Hsla)> = vec![(
            format!(
                "Yue / Carbon productive · {}",
                points_text(ease.control_points(theme))
            ),
            bezier(ease.control_points(theme)),
            yue,
        )];
        entries.extend(references.iter().map(|reference| {
            (
                format!(
                    "{} {} · {}",
                    reference.source,
                    reference.name,
                    points_text(reference.points)
                ),
                bezier(reference.points),
                token_rgba(theme, reference.color).into(),
            )
        }));
        let curves = entries
            .iter()
            .enumerate()
            .map(|(index, (_, easing, color))| {
                (easing.clone(), *color, if index == 0 { 3.0 } else { 1.5 })
            })
            .rev()
            .collect();
        div()
            .w(px(420.))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_3()
            .child(TextRole::Label.apply(theme, div()).child(title))
            .child(plot(theme, 240., curves, Some(clock)))
            .child(lanes(theme, &entries, clock, 400.))
    };
    stage(theme)
        .flex()
        .flex_col()
        .gap_4()
        .child(play_button(
            "motion-reference-play",
            theme,
            showcase,
            PLAY_REFERENCE,
            ("播放", "返回"),
            cx,
        ))
        .child(
            // Three panels outgrow a narrow window; wrap rather than spill
            // past the stage.
            div()
                .flex()
                .flex_wrap()
                .gap_8()
                .child(panel(
                    "Standard · 屏幕内移动",
                    MotionEase::Standard,
                    &STANDARD_REFERENCES,
                ))
                .child(panel(
                    "Enter · 减速进入",
                    MotionEase::Enter,
                    &ENTER_REFERENCES,
                ))
                .child(panel("Exit · 加速离开", MotionEase::Exit, &EXIT_REFERENCES)),
        )
}

fn reduced_motion(showcase: &Showcase, theme: YueTheme) -> impl IntoElement {
    let state = if showcase.motion_off {
        "当前：已关闭界面动效（等同减少动效）。点上方任意「播放」「打开」，演示会直接跳到终态，加载图标停止旋转。"
    } else {
        "当前：界面动效开启。在侧栏关闭「界面动效」即可模拟系统的减少动效设置。"
    };
    stage(theme)
        .flex()
        .items_center()
        .gap_3()
        .child(
            Icon::new(if showcase.motion_off {
                YueIcon::Check
            } else {
                YueIcon::Info
            })
            .size(IconSize::Small.rems(theme))
            .color(theme.accent_text()),
        )
        .child(TextRole::Body.apply(theme, div()).child(state))
}
