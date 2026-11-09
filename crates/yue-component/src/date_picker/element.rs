use super::*;
use crate::tokens::{pixels, rems, rgba};
use crate::{ButtonSize, ButtonVariant, Icon, YueButton, YueInput, outline};
use gpui_kit::base::{A11ySelectionExt as _, SelectionA11y, StyledExt as _, TestSupportExt as _};
use gpui_kit::{
    Context, InteractiveElement as _, IntoElement, ParentElement as _, Role,
    StatefulInteractiveElement as _, Styled as _, div, prelude::FluentBuilder as _,
};

pub(super) fn render(
    state: &mut YueDatePickerState,
    window: &mut Window,
    cx: &mut Context<YueDatePickerState>,
) -> AnyElement {
    let Some(p) = state.presentation.clone() else {
        return div().into_any_element();
    };
    let theme = p.theme.unwrap_or_else(|| crate::current_theme(cx));
    let id = state.options.id.clone();
    if p.calendar_only {
        let scale = f32::from(window.rem_size()) / 16.;
        let cell = pixels(
            theme,
            format!("--calendar-cell-size-{}", panel::suffix(p.size)).as_str(),
        );
        let count = if state.options.selection == DatePickerSelection::Range
            && f32::from(window.viewport_size().width) >= cell * 14. * scale
        {
            2
        } else {
            1
        };
        return div()
            .id((id, "inline"))
            .p(rems(theme, "--popover-padding"))
            .bg(rgba(theme, "--popover-background"))
            .text_color(rgba(theme, "--calendar-color"))
            .border_1()
            .border_color(rgba(theme, "--popover-border-color"))
            .rounded(rems(theme, "--popover-border-radius"))
            .child(panel::body(
                state,
                &p,
                theme,
                panel::PanelLayout {
                    count,
                    interactive: state.can_modify(),
                },
                &cx.entity(),
                window,
                cx,
            ))
            .into_any_element();
    }
    let suffix = panel::suffix(p.size);
    let cell = pixels(theme, format!("--calendar-cell-size-{suffix}").as_str()) / 16.;
    let size = match p.size {
        InputSize::Small => ButtonSize::Small,
        InputSize::Medium => ButtonSize::Medium,
        InputSize::Large => ButtonSize::Large,
    };
    let mut affix = div().flex().items_center().flex_shrink_0();
    if p.clearable
        && !state.value().is_empty()
        && !p.readonly
        && !p.disabled
        && !state.options.locked.iter().any(|v| *v)
    {
        affix = affix.child(
            YueButton::icon((id.clone(), "clear"), theme, outline::X, "清空日期")
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
            outline::Calendar,
            "打开日期选择",
        )
        .size(size)
        .variant(ButtonVariant::Ghost)
        .disabled(p.disabled || p.readonly)
        .expanded(state.open)
        .on_click(cx.listener(|s, _, w, cx| {
            cx.stop_propagation();
            if s.open {
                s.close(DatePickerCloseReason::Escape, true, w, cx);
            } else {
                s.open(w, cx);
                s.focus_calendar(w, cx);
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
    if state.options.selection != DatePickerSelection::Single
        && let Some(icon) = p.leading
    {
        fields = fields.child(
            icon.size(rems(theme, format!("--input-icon-size-{suffix}").as_str()))
                .color(theme.text_secondary()),
        );
    }
    if state.options.selection == DatePickerSelection::Multiple {
        let dates = if state.open {
            state.calendar.read(cx).selection().dates()
        } else {
            state.value().dates()
        };
        let count = dates.len();
        let mut tags = div()
            .flex()
            .flex_1()
            .min_w_0()
            .flex_wrap()
            .items_center()
            .gap(rems(theme, "--date-picker-tags-gap"));
        if dates.is_empty() {
            tags = tags.child(
                div()
                    .text_color(rgba(theme, "--input-placeholder-color"))
                    .child(p.placeholder.clone()),
            );
        }
        for date in dates
            .into_iter()
            .take(if state.expanded_tags { usize::MAX } else { 2 })
        {
            let mut tag = crate::YueTag::new((id.clone(), format!("tag-{date}")), date.to_string())
                .theme(theme);
            if state.can_modify() {
                tag = tag.on_close(cx.listener(move |s, _, w, cx| {
                    cx.stop_propagation();
                    s.remove_date(date, w, cx);
                }));
            }
            tags = tags.child(tag);
        }
        if count > 2 {
            tags = tags.child(
                YueButton::text(
                    (id.clone(), "expand-tags"),
                    theme,
                    if state.expanded_tags {
                        "收起".to_string()
                    } else {
                        format!("+{}", count - 2)
                    },
                )
                .size(ButtonSize::Small)
                .variant(ButtonVariant::Ghost)
                .disabled(p.disabled)
                .on_click(cx.listener(|s, _, _, cx| {
                    cx.stop_propagation();
                    s.expanded_tags = !s.expanded_tags;
                    cx.notify();
                })),
            );
        }
        fields = fields.child(tags).children(affix.take());
    } else {
        let range = state.options.selection == DatePickerSelection::Range;
        for index in 0..if range { 2 } else { 1 } {
            let locked = state.options.locked[index];
            let placeholder = if range {
                if index == 0 {
                    "开始日期".into()
                } else {
                    "结束日期".into()
                }
            } else {
                p.placeholder.clone()
            };
            let mut description = state
                .error
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| format!("格式：{}；向下键打开日历", state.options.format_str()));
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
            .readonly(p.readonly || !p.editable || locked)
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
    let composite = state.options.selection != DatePickerSelection::Single;
    let width = cell
        * 7.
        * if state.options.selection == DatePickerSelection::Range {
            2.
        } else {
            1.
        };
    let mut frame = div()
        .id((id.clone(), "field"))
        .test_support()
        .key_context("YueDatePicker")
        .on_action(
            cx.listener(|s, _: &super::behavior::NextField, w, cx| s.traverse_tab(false, w, cx)),
        )
        .on_action(
            cx.listener(|s, _: &super::behavior::PreviousField, w, cx| s.traverse_tab(true, w, cx)),
        )
        .relative()
        .min_w_0()
        .w_full()
        .track_focus(
            &state
                .focus
                .clone()
                .tab_stop(state.options.selection == DatePickerSelection::Multiple && !p.disabled),
        )
        .when(composite, |s| {
            s.px_1()
                .min_h(rems(theme, format!("--input-height-{suffix}").as_str()))
                .refine_style(&crate::input::surface_style(
                    theme, status, p.disabled, p.readonly, p.bordered, focused,
                ))
        })
        .when(
            state.options.selection == DatePickerSelection::Multiple,
            |s| {
                s.role(Role::ComboBox)
                    .aria_label(p.name.clone())
                    .aria_expanded(state.open)
            },
        )
        .on_key_down(cx.listener(|s, event: &gpui_kit::KeyDownEvent, w, cx| {
            if event.keystroke.key == "down" && !s.composing(w, cx) {
                s.open(w, cx);
                s.focus_calendar(w, cx);
                cx.stop_propagation();
            } else if event.keystroke.key == "escape" && !s.composing(w, cx) {
                if s.open {
                    s.close(DatePickerCloseReason::Escape, true, w, cx);
                } else {
                    s.cancel_text(w, cx);
                }
                cx.stop_propagation();
            }
        }))
        .child(fields)
        .child(state.anchor.capture());
    if !p.editable && !p.readonly && !p.disabled {
        frame = frame.on_click(cx.listener(|s, _, w, cx| {
            s.open(w, cx);
            s.focus_calendar(w, cx);
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
                    .required(p.required)
                    .multiple(state.options.selection == DatePickerSelection::Multiple),
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
