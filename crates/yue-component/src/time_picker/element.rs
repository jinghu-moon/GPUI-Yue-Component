use super::*;
use crate::tokens::{rems, rgba};
use crate::{ButtonSize, ButtonVariant, Icon, YueButton, YueInput, outline};
use gpui_kit::base::{A11ySelectionExt as _, SelectionA11y, StyledExt as _, TestSupportExt as _};
use gpui_kit::{
    Context, InteractiveElement as _, IntoElement, ParentElement as _, Role,
    StatefulInteractiveElement as _, Styled as _, div, prelude::FluentBuilder as _,
};

pub(super) fn render(
    state: &mut YueTimePickerState,
    window: &mut Window,
    cx: &mut Context<YueTimePickerState>,
) -> AnyElement {
    let Some(p) = state.presentation.clone() else {
        return div().into_any_element();
    };
    let theme = p.theme.unwrap_or_else(|| crate::current_theme(cx));
    let id = state.options.id.clone();
    if p.panel_only {
        return panel::inline(state, &p, theme, window, cx);
    }
    let suffix = panel::suffix(p.size);
    let size = match p.size {
        InputSize::Small => ButtonSize::Small,
        InputSize::Medium => ButtonSize::Medium,
        InputSize::Large => ButtonSize::Large,
    };
    let mut affix = div().flex().items_center().flex_shrink_0();
    if p.clearable && !state.value().is_empty() && !p.readonly && !p.disabled {
        affix = affix.child(
            YueButton::icon((id.clone(), "clear"), theme, outline::X, "清空时间")
                .size(size)
                .variant(ButtonVariant::Ghost)
                .on_click(cx.listener(|s, _, w, cx| {
                    cx.stop_propagation();
                    s.clear(w, cx);
                })),
        );
    }
    affix = affix.child(
        YueButton::icon(
            (id.clone(), "toggle"),
            theme,
            outline::Clock,
            "打开时间选择",
        )
        .size(size)
        .variant(ButtonVariant::Ghost)
        .disabled(p.disabled || p.readonly)
        .expanded(state.open)
        .on_click(cx.listener(|s, _, w, cx| {
            cx.stop_propagation();
            if s.open {
                s.close(TimePickerCloseReason::Escape, true, w, cx);
            } else {
                s.open(w, cx);
                s.focus_panel(w, cx);
            }
        })),
    );
    let mut affix = Some(affix);
    let status = if state.error.is_some() {
        InputStatus::Error
    } else {
        p.status
    };
    let mut fields = div().flex().items_center().min_w_0().w_full();
    if state.range()
        && let Some(icon) = p.leading
    {
        fields = fields.child(
            icon.size(rems(theme, "--input-icon-size-md"))
                .color(theme.text_secondary()),
        );
    }
    {
        let range = state.range();
        for index in 0..if range { 2 } else { 1 } {
            let placeholder = if range {
                p.range_placeholders[index].clone()
            } else {
                p.placeholder.clone()
            };
            let mut description = state
                .error
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| {
                    format!("格式：{}；Alt+向下键打开面板", state.options.format_str())
                });
            if let Some(extra) = &p.description {
                description = format!("{extra}；{description}");
            }
            let mut input = YueInput::new(
                (id.clone(), format!("input-{index}")),
                theme,
                &state.inputs[index],
            )
            .size(p.size)
            .status(status)
            .accessibility_label(if range {
                format!("{}{}", p.name, if index == 0 { "开始" } else { "结束" })
            } else {
                p.name.to_string()
            })
            .accessibility_description(description)
            .placeholder(placeholder)
            .block()
            .bordered(!range && p.bordered)
            .readonly(p.readonly || !p.editable)
            .disabled(p.disabled);
            if !range {
                if let Some(icon) = p.leading {
                    input = input.leading(icon);
                }
                input = input.trailing(affix.take().unwrap());
                fields = fields.child(div().flex_1().min_w_0().child(input));
                break;
            }
            fields = fields.child(div().flex_1().min_w_0().child(input));
            if index == 0 {
                fields = fields.child(
                    Icon::new(outline::ArrowRight)
                        .size(rems(theme, "--input-icon-size-md"))
                        .color(theme.text_secondary()),
                );
            }
        }
        if range {
            fields = fields.children(affix.take());
        }
    }
    let focused = state.contains_focus(window, cx);
    let composite = state.range();
    let width = if state.range() { 24. } else { 14. };
    let mut frame = div()
        .id((id.clone(), "field"))
        .test_support()
        .aria_expanded(state.open)
        .key_context("YueTimePicker")
        .on_action(
            cx.listener(|s, _: &super::behavior::NextField, w, cx| s.traverse_tab(false, w, cx)),
        )
        .on_action(
            cx.listener(|s, _: &super::behavior::PreviousField, w, cx| s.traverse_tab(true, w, cx)),
        )
        .on_action(cx.listener(|s, _: &super::behavior::OpenPanel, w, cx| {
            s.open(w, cx);
            s.focus_panel(w, cx);
        }))
        .on_action(cx.listener(|s, _: &super::behavior::StepUp, w, cx| s.step_input(1, w, cx)))
        .on_action(cx.listener(|s, _: &super::behavior::StepDown, w, cx| s.step_input(-1, w, cx)))
        .relative()
        .min_w_0()
        .w_full()
        .track_focus(&state.focus)
        .when(composite, |s| {
            s.px_1()
                .min_h(rems(theme, format!("--input-height-{suffix}").as_str()))
                .refine_style(&crate::input::surface_style(
                    theme, status, p.disabled, p.readonly, p.bordered, focused,
                ))
        })
        .on_key_down(cx.listener(|s, event: &gpui_kit::KeyDownEvent, w, cx| {
            if event.keystroke.key == "escape" && !s.composing(w, cx) {
                if s.open {
                    s.close(TimePickerCloseReason::Escape, true, w, cx);
                } else {
                    s.cancel(w, cx);
                }
                cx.stop_propagation();
            }
        }))
        .child(fields)
        .child(state.anchor.capture());
    if !p.editable && !p.readonly && !p.disabled {
        frame = frame.on_click(cx.listener(|s, _, w, cx| {
            s.open(w, cx);
            s.focus_panel(w, cx);
        }));
    }
    let mut root = div()
        .id(id.clone())
        .flex()
        .flex_col()
        .min_w_0()
        .max_w_full()
        .w(gpui_kit::rems(width))
        .when(p.block, |s| s.w_full())
        .gap_1()
        .child(
            frame.a11y_selection(
                SelectionA11y::default()
                    .disabled(p.disabled)
                    .readonly(p.readonly)
                    .invalid(state.error.is_some())
                    .required(p.required),
            ),
        );
    if let Some(error) = &state.error {
        root = root.child(
            div()
                .id((id, "error"))
                .test_support()
                .role(Role::Label)
                .aria_label(error.to_string())
                .text_sm()
                .text_color(rgba(theme, "--input-border-color-invalid"))
                .child(error.to_string()),
        );
    }
    if let Some(popup) = panel::popup(state, &p, theme, window, cx) {
        root = root.child(popup);
    }
    root.into_any_element()
}
