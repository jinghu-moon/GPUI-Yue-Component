use super::state::modifier_names;
use super::*;
use crate::{
    KbdSize, KbdState, ShortcutSpec, YueKbd, YueShortcut,
    theme::keys,
    tokens::{pixels, rems, rgba},
};
use gpui_kit::base::{A11yLive, Button};
use gpui_kit::{
    App, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce, Role,
    StatefulInteractiveElement as _, Styled as _, Window, div, prelude::FluentBuilder as _,
};

impl RenderOnce for YueShortcutRecorder {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = self.theme.unwrap_or_else(|| crate::current_theme(cx));
        let s = self.state.read(cx);
        let disabled = s.disabled;
        let recording = s.recording;
        let conflict = s.pending.is_some();
        let invalid = s.invalid;
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
        let border = if disabled {
            keys::INPUT_BORDER_COLOR_DISABLED
        } else if invalid {
            keys::INPUT_BORDER_COLOR_INVALID
        } else if conflict {
            keys::WARNING
        } else if recording || focus.is_focused(window) {
            keys::INPUT_BORDER_COLOR_FOCUS
        } else {
            keys::INPUT_BORDER_COLOR
        };
        let description = if message.is_empty() {
            "点击或按 Enter 开始录制".into()
        } else {
            message.clone()
        };
        let owner = self.state.clone();
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
            .border_color(rgba(theme, border))
            .rounded(rems(theme, keys::INPUT_BORDER_RADIUS))
            .bg(rgba(
                theme,
                if disabled {
                    keys::INPUT_BACKGROUND_DISABLED
                } else {
                    keys::INPUT_BACKGROUND
                },
            ))
            .when(!disabled && (recording || conflict), |b| {
                b.shadow(vec![
                    gpui_kit::BoxShadow::new(
                        gpui_kit::px(0.),
                        gpui_kit::px(0.),
                        rgba(
                            theme,
                            if conflict {
                                keys::WARNING
                            } else {
                                keys::INPUT_FOCUS_RING_COLOR
                            },
                        )
                        .alpha(if conflict { 0.18 } else { 0.16 })
                        .into(),
                    )
                    .spread_radius(gpui_kit::px(
                        (pixels(theme, keys::INPUT_FOCUS_RING_WIDTH)
                            + pixels(theme, keys::INPUT_BORDER_WIDTH))
                            * f32::from(window.rem_size())
                            / 16.,
                    )),
                ])
            })
            .font_family(crate::typography::ui_font(theme).family)
            .text_size(rems(theme, font))
            .text_color(rgba(theme, keys::TEXT_PRIMARY))
            .when(!disabled && !recording && !conflict && !invalid, |b| {
                b.hover(|s| s.border_color(rgba(theme, keys::INPUT_BORDER_COLOR_HOVER)))
            })
            .when(!disabled && !invalid && !conflict, |b| {
                b.focus_visible(|s| s.border_color(rgba(theme, keys::INPUT_BORDER_COLOR_FOCUS)))
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
                div()
                    .size_2()
                    .flex_shrink_0()
                    .rounded_full()
                    .bg(rgba(theme, keys::ERROR))
            }))
            .child(
                div()
                    .id((self.id.clone(), "content"))
                    .flex_1()
                    .min_w_0()
                    .overflow_x_scroll()
                    .child(content),
            );
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
