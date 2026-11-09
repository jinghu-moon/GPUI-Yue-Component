use super::state::modifier_names;
use super::*;
use crate::{
    KbdSize, KbdState, ShortcutSpec, YueKbd, YueShortcut,
    theme::keys,
    tokens::{rems, rgba},
};
use gpui_kit::base::{A11yLive, Button};
use gpui_kit::base::{Easing, Interpolate, Transition, transition};
use gpui_kit::{
    Animation, AnimationExt as _, App, InteractiveElement as _, IntoElement, ParentElement as _,
    RenderOnce, Role, StatefulInteractiveElement as _, Styled as _, Window, div,
    prelude::FluentBuilder as _, px,
};
use std::time::Duration;

#[derive(Clone, PartialEq)]
struct RecorderMotion {
    border: gpui_kit::Rgba,
    background: gpui_kit::Rgba,
    ring: gpui_kit::Rgba,
    spread: f32,
}
impl Interpolate for RecorderMotion {
    fn interpolate(&self, target: &Self, progress: f32) -> Self {
        let color = |a: gpui_kit::Rgba, b: gpui_kit::Rgba| gpui_kit::Rgba {
            r: a.r + (b.r - a.r) * progress,
            g: a.g + (b.g - a.g) * progress,
            b: a.b + (b.b - a.b) * progress,
            a: a.a + (b.a - a.a) * progress,
        };
        Self {
            border: color(self.border, target.border),
            background: color(self.background, target.background),
            ring: color(self.ring, target.ring),
            spread: self.spread + (target.spread - self.spread) * progress,
        }
    }
}

impl RenderOnce for YueShortcutRecorder {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = self.theme.unwrap_or_else(|| crate::current_theme(cx));
        let s = self.state.read(cx);
        let disabled = s.disabled;
        let recording = s.recording;
        let conflict = s.pending.is_some();
        let invalid = s.invalid;
        let error_flash = s.error_flash;
        let feedback_revision = s.feedback_revision;
        let success = s.success;
        let hovered = s.hovered;
        let focus = s.focus.clone();
        let region = s.region.clone();
        let message = s.message.clone();
        let value = s.pending.as_ref().or(s.value()).cloned();
        let modifiers = s.modifiers;
        let (height, font, kbd_size) = match self.size {
            InputSize::Small => (
                keys::INPUT_HEIGHT_SM,
                keys::INPUT_FONT_SIZE_SM,
                KbdSize::Small,
            ),
            InputSize::Medium => (
                keys::INPUT_HEIGHT_MD,
                keys::INPUT_FONT_SIZE_MD,
                KbdSize::Small,
            ),
            InputSize::Large => (
                keys::INPUT_HEIGHT_LG,
                keys::INPUT_FONT_SIZE_LG,
                KbdSize::Medium,
            ),
        };
        let content = if recording {
            let modifiers = modifier_names(modifiers);
            if modifiers.is_empty() {
                div()
                    .text_color(rgba(theme, keys::TEXT_SECONDARY))
                    .child("按下快捷键…")
                    .into_any_element()
            } else {
                div()
                    .flex()
                    .gap_1()
                    .children(modifiers.into_iter().map(|key| {
                        YueKbd::new(key)
                            .theme(theme)
                            .size(kbd_size)
                            .state(KbdState::Pressed)
                    }))
                    .into_any_element()
            }
        } else if let Some(value) = value.clone() {
            YueShortcut::new((self.id.clone(), "value"), value)
                .theme(theme)
                .size(kbd_size)
                .state(if disabled {
                    KbdState::Disabled
                } else {
                    KbdState::Default
                })
                .semantics(false)
                .into_any_element()
        } else {
            div()
                .text_color(rgba(theme, keys::TEXT_SECONDARY))
                .child("未设置")
                .into_any_element()
        };
        let focused = focus.is_focused(window);
        let border = if disabled {
            keys::INPUT_BORDER_COLOR_DISABLED
        } else if error_flash {
            keys::INPUT_BORDER_COLOR_INVALID
        } else if success {
            keys::SUCCESS_BORDER
        } else if conflict {
            keys::WARNING_BORDER
        } else if recording || focused {
            keys::INPUT_BORDER_COLOR_FOCUS
        } else if hovered {
            keys::INPUT_BORDER_COLOR_HOVER
        } else {
            keys::INPUT_BORDER_COLOR
        };
        let background = if disabled {
            keys::INPUT_BACKGROUND_DISABLED
        } else if success {
            keys::SUCCESS_BG
        } else {
            keys::INPUT_BACKGROUND
        };
        let (ring, spread) = if !disabled && conflict {
            (rgba(theme, keys::WARNING).alpha(0.18), 3.)
        } else if !disabled && (recording || focused) {
            (rgba(theme, keys::INPUT_FOCUS_RING_COLOR).alpha(0.16), 3.)
        } else {
            (rgba(theme, keys::INPUT_FOCUS_RING_COLOR).alpha(0.), 0.)
        };
        let target = RecorderMotion {
            border: rgba(theme, border),
            background: rgba(theme, background),
            ring,
            spread: spread * f32::from(window.rem_size()) / 16.,
        };
        let reduce_motion = cx.reduce_motion();
        let motion = transition(
            (self.id.clone(), "recorder-motion"),
            target,
            Transition::new(if reduce_motion {
                Duration::ZERO
            } else {
                crate::tokens::duration(theme, keys::DURATION_NORMAL)
            })
            .easing(Easing::cubic_bezier(0.2, 0., 0.38, 0.9).expect("valid reference easing")),
            window,
            cx,
        );
        let description = if message.is_empty() {
            "点击或按 Enter 开始录制".into()
        } else {
            message.clone()
        };
        let owner = self.state.clone();
        let hover = self.state.clone();
        let release = self.state.clone();
        let changes = self.state.clone();
        let field = Button::new((self.id.clone(), "field"))
            .track_focus(&focus)
            .disabled(disabled)
            .accessibility_label(format!(
                "{}，{}",
                self.label,
                if recording {
                    let names = modifier_names(modifiers);
                    if names.is_empty() {
                        "正在录制".to_owned()
                    } else {
                        format!("正在录制，{}", names.join(" + "))
                    }
                } else {
                    value
                        .as_ref()
                        .map(ShortcutSpec::display_label)
                        .unwrap_or_else(|| "未设置".to_owned())
                }
            ))
            .aria_description(description)
            .h(rems(theme, height))
            .w_full()
            .min_w_0()
            .px_2()
            .gap_2()
            .justify_start()
            .border(rems(theme, keys::INPUT_BORDER_WIDTH))
            .border_color(motion.border)
            .rounded(rems(theme, keys::INPUT_BORDER_RADIUS))
            .bg(motion.background)
            .when(motion.spread > 0.01, |button| {
                button.shadow(vec![
                    gpui_kit::BoxShadow::new(px(0.), px(0.), motion.ring.into())
                        .spread_radius(px(motion.spread)),
                ])
            })
            .font_family(crate::typography::ui_font(theme).family)
            .text_size(rems(theme, font))
            .text_color(rgba(theme, keys::TEXT_PRIMARY))
            .on_hover(move |value, _, cx| {
                hover.update(cx, |s, cx| {
                    if s.hovered != *value {
                        s.hovered = *value;
                        cx.notify();
                    }
                });
            })
            .on_click(move |_, w, cx| {
                owner.update(cx, |s, cx| {
                    if s.pending.is_some() {
                        s.cancel(cx);
                    } else if !s.recording {
                        s.start(w, cx);
                    }
                })
            })
            .on_key_up(move |e, _, cx| {
                release.update(cx, |s, cx| {
                    s.release(&e.keystroke.key, e.keystroke.modifiers, cx)
                })
            })
            .on_modifiers_changed(move |e, _, cx| {
                changes.update(cx, |s, cx| s.modifiers_changed(e.modifiers, cx))
            })
            .children(recording.then(|| {
                let dot = div()
                    .size_2()
                    .flex_shrink_0()
                    .rounded_full()
                    .bg(rgba(theme, keys::ERROR));
                if reduce_motion {
                    dot.into_any_element()
                } else {
                    dot.with_animation(
                        (self.id.clone(), "recording-pulse"),
                        Animation::new(Duration::from_secs(1))
                            .repeat()
                            .with_easing(|t| (1. - (t * std::f32::consts::TAU).cos()) * 0.5),
                        |dot, phase| dot.opacity(1. - phase * 0.7),
                    )
                    .into_any_element()
                }
            }))
            .child(
                div()
                    .id((self.id.clone(), "content"))
                    .flex_1()
                    .min_w_0()
                    .overflow_x_scroll()
                    .child(content),
            );
        let field = if error_flash && !reduce_motion {
            field
                .with_animation(
                    (
                        gpui_kit::ElementId::from((self.id.clone(), "recording-shake")),
                        feedback_revision.to_string(),
                    ),
                    Animation::new(Duration::from_millis(300)),
                    |button, progress| button.relative().left(px(shake_offset(progress))),
                )
                .into_any_element()
        } else {
            field.into_any_element()
        };
        let mut root = div()
            .id(self.id.clone())
            .track_focus(&region)
            .key_context("YueShortcutRecorder")
            .flex()
            .flex_col()
            .items_start()
            .gap_1()
            .min_w_0()
            .w(self.width.unwrap_or_else(|| gpui_kit::rems(14.).into()))
            .child(field);
        if conflict {
            let apply = self.state.clone();
            let cancel = self.state.clone();
            root = root.child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        crate::YueButton::new((self.id.clone(), "overwrite"), "覆盖")
                            .theme(theme)
                            .size(crate::ButtonSize::Small)
                            .on_click(move |_, w, cx| apply.update(cx, |s, cx| s.overwrite(w, cx))),
                    )
                    .child(
                        crate::YueButton::new((self.id.clone(), "cancel"), "取消")
                            .theme(theme)
                            .size(crate::ButtonSize::Small)
                            .on_click(move |_, w, cx| {
                                cancel.update(cx, |s, cx| {
                                    s.cancel(cx);
                                    s.focus.focus(w, cx);
                                })
                            }),
                    ),
            )
        }
        root.child(
            A11yLive::new(
                div()
                    .id((self.id, "message"))
                    .role(Role::Status)
                    .aria_label(message.clone())
                    .text_size(rems(theme, keys::INPUT_FONT_SIZE_SM))
                    .line_height(rems(theme, keys::LINE_HEIGHT_SM))
                    .text_color(rgba(
                        theme,
                        if invalid {
                            keys::ERROR
                        } else if conflict {
                            keys::WARNING
                        } else {
                            keys::TEXT_SECONDARY
                        },
                    ))
                    .min_h(rems(theme, keys::LINE_HEIGHT_SM))
                    .child(message),
                gpui_kit::accesskit::Live::Polite,
            )
            .atomic(true),
        )
    }
}

fn shake_offset(progress: f32) -> f32 {
    if progress < 0.25 {
        -12. * progress
    } else if progress < 0.75 {
        -3. + (progress - 0.25) * 12.
    } else {
        3. - (progress - 0.75) * 12.
    }
}
