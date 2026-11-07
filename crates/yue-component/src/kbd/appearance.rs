//! Visual specification transcribed from refer/Shortcut 快捷键 · GPUI 组件 Demo.html.
//! Neutral colors are this reference's light/dark palette, not generic dividers.
use gpui_kit::{Rgba, rgb};

use super::{KbdSize, KbdState, KbdVariant};
use crate::{ThemeMode, YueTheme};

pub(super) struct Metrics {
    pub height: f32,
    pub font: f32,
    pub padding: f32,
    pub key_gap: f32,
}

impl KbdSize {
    pub(super) fn metrics(self) -> Metrics {
        match self {
            Self::Small => Metrics {
                height: 20.,
                font: 11.,
                padding: 4.,
                key_gap: 2.,
            },
            Self::Medium => Metrics {
                height: 24.,
                font: 12.,
                padding: 6.,
                key_gap: 4.,
            },
            Self::Large => Metrics {
                height: 28.,
                font: 14.,
                padding: 8.,
                key_gap: 4.,
            },
        }
    }
}

pub(super) struct Appearance {
    pub background: Rgba,
    pub foreground: Rgba,
    pub border: Rgba,
    pub secondary: Rgba,
    pub bottom: f32,
    pub translation: f32,
    pub opacity: f32,
}

impl Appearance {
    pub fn animate(
        &mut self,
        id: gpui_kit::ElementId,
        window: &mut gpui_kit::Window,
        cx: &mut gpui_kit::App,
    ) {
        use gpui_kit::base::{Easing, Transition, transition};
        let target = Motion {
            background: self.background,
            border: self.border,
            translation: self.translation,
        };
        let value = transition(
            (id, "kbd-appearance"),
            target,
            Transition::new(std::time::Duration::from_millis(100)).easing(Easing::Ease),
            window,
            cx,
        );
        self.background = value.background;
        self.border = value.border;
        self.translation = value.translation;
    }

    pub fn new(theme: YueTheme, variant: KbdVariant, state: KbdState, inherited: Rgba) -> Self {
        let dark = theme.mode() == ThemeMode::Dark;
        let surface = rgb(if dark { 0x1f1f1f } else { 0xffffff });
        let text = rgb(if dark { 0xf5f5f5 } else { 0x1f1f1f });
        let secondary = rgb(if dark { 0xa3a3a3 } else { 0x737373 });
        let control = rgb(if dark { 0x737373 } else { 0xa3a3a3 });
        let neutral_bg = rgb(if dark { 0x333333 } else { 0xeeeeee });
        let neutral_fg = rgb(if dark { 0xe0e0e0 } else { 0x525252 });
        let transparent = Rgba { a: 0., ..surface };
        // CSS `currentColor`: inverse keys inherit the surrounding foreground.
        let inverse_fg = inherited;
        let (background, foreground, border) = match variant {
            KbdVariant::Raised => (surface, text, control),
            KbdVariant::Flat => (neutral_bg, neutral_fg, transparent),
            KbdVariant::Ghost => (transparent, secondary, transparent),
            KbdVariant::Inverse => (
                transparent,
                inverse_fg,
                Rgba {
                    a: 0.4,
                    ..inverse_fg
                },
            ),
        };
        let mut result = Self {
            background,
            foreground,
            border,
            secondary: if variant == KbdVariant::Inverse {
                Rgba {
                    a: 0.7,
                    ..inverse_fg
                }
            } else {
                secondary
            },
            bottom: 2.,
            translation: 0.,
            opacity: 1.,
        };
        match state {
            KbdState::Pressed => {
                result.background = rgb(if dark { 0x0f2644 } else { 0xe6f0ff });
                result.foreground = rgb(if dark { 0x93c1ff } else { 0x125fb8 });
                result.border = rgb(if dark { 0x3581e2 } else { 0x1f75db });
                result.bottom = 1.;
                result.translation = 1.;
            }
            KbdState::Active => {
                result.background = text;
                result.foreground = surface;
                result.border = text;
            }
            KbdState::Disabled => result.opacity = 0.4,
            KbdState::Default => {}
        }
        result
    }
}

#[derive(Clone, PartialEq)]
struct Motion {
    background: Rgba,
    border: Rgba,
    translation: f32,
}
impl gpui_kit::base::Interpolate for Motion {
    fn interpolate(&self, target: &Self, progress: f32) -> Self {
        let color = |from: Rgba, to: Rgba| Rgba {
            r: from.r + (to.r - from.r) * progress,
            g: from.g + (to.g - from.g) * progress,
            b: from.b + (to.b - from.b) * progress,
            a: from.a + (to.a - from.a) * progress,
        };
        Self {
            background: color(self.background, target.background),
            border: color(self.border, target.border),
            translation: self.translation + (target.translation - self.translation) * progress,
        }
    }
}
