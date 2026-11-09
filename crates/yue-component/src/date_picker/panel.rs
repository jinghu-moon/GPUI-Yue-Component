use super::*;
use crate::popup::{
    PopupDismissReason, PopupDismissRoute, PopupHost, PopupId, PopupPlacement, PopupPointerMode,
    PopupSpec,
};
use crate::tokens::{pixels, rems, rgba};
use crate::{ButtonSize, ButtonVariant, Icon, YueButton, YueTheme, outline};
use gpui_kit::base::{
    Align, Calendar, CalendarItemKind, Placement, TestSupportExt as _, TimeField,
};
use gpui_kit::{
    Context, InteractiveElement as _, IntoElement, ParentElement as _, Role,
    StatefulInteractiveElement as _, Styled as _, div, prelude::FluentBuilder as _, px,
};

pub(super) fn suffix(size: InputSize) -> &'static str {
    match size {
        InputSize::Small => "sm",
        InputSize::Medium => "md",
        InputSize::Large => "lg",
    }
}
pub(super) fn calendar(
    state: &YueDatePickerState,
    p: &Presentation,
    theme: YueTheme,
    count: usize,
    interactive: bool,
    cx: &App,
) -> AnyElement {
    let suffix = suffix(p.size);
    let cell = pixels(theme, format!("--calendar-cell-size-{suffix}").as_str()) / 16.;
    let height = p.cell_height.map_or_else(
        || pixels(theme, format!("--calendar-cell-height-{suffix}").as_str()) / 16.,
        |height| height.0,
    );
    let font = rems(theme, format!("--calendar-font-size-{suffix}").as_str());
    let view = state.calendar.read(cx).view();
    let quarter = state.options.granularity == DatePickerGranularity::Quarter;
    let day = p.day.clone();
    let item_width = if view.is_day() {
        cell
    } else {
        cell * 7. / if quarter && view.is_month() { 2. } else { 3. }
    };
    let item_height = if view.is_day() {
        height
    } else {
        height * 7. / if quarter && view.is_month() { 2. } else { 4. }
    };
    Calendar::new((state.options.id.clone(), "calendar"), &state.calendar)
        .number_of_months(count)
        .first_day_of_week(state.options.week_start)
        .interactive(interactive)
        .gap(rems(theme, "--calendar-panel-gap"))
        .label(|kind, value| match kind {
            CalendarItemKind::Previous if value == -12 => "上一年".into(),
            CalendarItemKind::Next if value == 12 => "下一年".into(),
            CalendarItemKind::Previous => "上一个月或年份".into(),
            CalendarItemKind::Next => "下一个月或年份".into(),
            CalendarItemKind::MonthToggle | CalendarItemKind::Month => format!("{value}月").into(),
            CalendarItemKind::YearToggle | CalendarItemKind::Year => format!("{value}年").into(),
            CalendarItemKind::Weekday => {
                ["日", "一", "二", "三", "四", "五", "六"][value as usize].into()
            }
            _ => value.to_string().into(),
        })
        .item(move |item, st, window, cx| {
            let navigation = matches!(
                st.kind(),
                CalendarItemKind::Previous | CalendarItemKind::Next
            );
            let toggle = matches!(
                st.kind(),
                CalendarItemKind::YearToggle | CalendarItemKind::MonthToggle
            );
            let header = st.kind() == CalendarItemKind::Weekday;
            let mut item = item
                .flex()
                .items_center()
                .justify_center()
                .flex_shrink_0()
                .text_size(font)
                .text_color(rgba(
                    theme,
                    if st.is_disabled() && !header {
                        "--calendar-disabled-color"
                    } else if st.is_muted() {
                        "--calendar-muted-color"
                    } else {
                        "--calendar-color"
                    },
                ))
                .w(gpui_kit::rems(if navigation {
                    cell
                } else if toggle {
                    cell * 1.5
                } else {
                    item_width
                }))
                .h(gpui_kit::rems(if navigation || toggle || header {
                    height
                } else {
                    item_height
                }));
            if navigation {
                item = item.clear_children().child(
                    if st.navigation_months() == -12 {
                        Icon::new(outline::ChevronsLeft)
                    } else if st.navigation_months() == 12 {
                        Icon::new(outline::ChevronsRight)
                    } else if st.kind() == CalendarItemKind::Previous {
                        Icon::new(outline::ChevronLeft)
                    } else {
                        Icon::new(outline::ChevronRight)
                    }
                    .size(rems(theme, "--input-icon-size-md")),
                );
            }
            if !header && !navigation && !toggle {
                let label = st
                    .date()
                    .map(|date| {
                        use chrono::Datelike as _;
                        if !(1..=9999).contains(&date.year()) {
                            String::new()
                        } else {
                            match st.kind() {
                                CalendarItemKind::Year => format!("{}年", date.year()),
                                CalendarItemKind::Month => {
                                    if quarter {
                                        format!("Q{}", date.month0() / 3 + 1)
                                    } else {
                                        format!("{}月", date.month())
                                    }
                                }
                                _ => date.day().to_string(),
                            }
                        }
                    })
                    .unwrap_or_default();
                let inner = div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(rems(theme, "--calendar-cell-radius"))
                    .border_1()
                    .border_color(gpui_kit::rgba(0))
                    .when(st.is_today(), |s| {
                        s.border_color(rgba(theme, "--calendar-today-color"))
                    })
                    .when(st.is_active(), |s| {
                        s.bg(rgba(theme, "--calendar-selected-background"))
                            .text_color(rgba(theme, "--calendar-selected-color"))
                    })
                    .when(st.is_focused(), |s| {
                        s.border_color(rgba(theme, "--calendar-focus-color"))
                    })
                    .child(label);
                item = item
                    .clear_children()
                    .when(st.is_in_range() || st.is_preview(), |s| {
                        s.bg(rgba(theme, "--calendar-range-background"))
                    })
                    .p(rems(theme, "--calendar-content-inset"))
                    .child(inner);
            }
            if !header && !st.is_disabled() {
                item = item
                    .hover(|s| s.bg(rgba(theme, "--calendar-hover-background")))
                    .focus_visible(|s| {
                        s.border_1()
                            .border_color(rgba(theme, "--calendar-focus-color"))
                    });
            }
            if st.kind() == CalendarItemKind::Day
                && let Some(render) = &day
            {
                return render(item, st, window, cx);
            }
            item.into_any_element()
        })
        .into_any_element()
}

#[derive(Clone, Copy)]
pub(super) struct PanelLayout {
    pub(super) count: usize,
    pub(super) interactive: bool,
}

pub(super) fn body(
    state: &YueDatePickerState,
    p: &Presentation,
    theme: YueTheme,
    layout: PanelLayout,
    entity: &Entity<YueDatePickerState>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let PanelLayout { count, interactive } = layout;
    let id = state.options.id.clone();
    let mut body = div()
        .flex()
        .flex_col()
        .min_w_0()
        .gap(rems(theme, "--calendar-panel-gap"))
        .child(calendar(state, p, theme, count, interactive, cx));
    if state.options.precision.is_some() {
        let mut row = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(rems(theme, "--calendar-panel-gap"));
        for index in 0..if state.options.selection == DatePickerSelection::Range {
            2
        } else {
            1
        } {
            let locked = state.options.locked[index];
            let label = if state.options.selection == DatePickerSelection::Range {
                if index == 0 {
                    "开始时间"
                } else {
                    "结束时间"
                }
            } else {
                "时间"
            };
            row = row.child(
                div().flex().items_center().gap_2().child(label).child(
                    TimeField::new((id.clone(), format!("time-{index}")), &state.times[index])
                        .disabled(!interactive || locked)
                        .p_1()
                        .gap_1()
                        .rounded(rems(theme, "--input-border-radius"))
                        .border_1()
                        .border_color(rgba(theme, "--input-border-color"))
                        .render_segment(move |segment, st, _, _| {
                            segment
                                .p_1()
                                .rounded(rems(theme, "--calendar-cell-radius"))
                                .when(st.is_selected(), |s| {
                                    s.bg(rgba(theme, "--calendar-selected-background"))
                                        .text_color(rgba(theme, "--calendar-selected-color"))
                                })
                                .into_any_element()
                        }),
                ),
            );
        }
        body = body.child(row);
    }
    if !p.presets.is_empty() {
        let mut row = div()
            .flex()
            .flex_wrap()
            .gap(rems(theme, "--date-picker-footer-gap"));
        for preset in &p.presets {
            let preset = preset.clone();
            row = row.child(
                YueButton::text(
                    (id.clone(), format!("preset-{}", preset.id)),
                    theme,
                    preset.label.clone(),
                )
                .size(ButtonSize::Small)
                .variant(ButtonVariant::Ghost)
                .disabled(!interactive)
                .on_click(window.listener_for(entity, move |s, _, w, cx| {
                    let value = (preset.resolve)((s.options.clock)());
                    if let Err(error) = s.choose(value, DatePickerChangeSource::Preset, w, cx) {
                        s.error = Some(error);
                        cx.notify();
                    }
                })),
            );
        }
        body = body.child(row);
    }
    if let Some(error) = &state.error {
        body = body.child(
            div()
                .id((id.clone(), "panel-error"))
                .role(Role::Label)
                .aria_label(error.to_string())
                .text_sm()
                .text_color(rgba(theme, "--input-border-color-invalid"))
                .child(error.to_string()),
        );
    }
    if state.options.selection == DatePickerSelection::Multiple {
        let values = state.calendar.read(cx).selection().dates();
        let mut tags = div()
            .flex()
            .flex_wrap()
            .gap(rems(theme, "--date-picker-tags-gap"));
        for date in values {
            tags = tags.child(
                crate::YueTag::new((id.clone(), format!("panel-tag-{date}")), date.to_string())
                    .theme(theme)
                    .disabled(!interactive)
                    .on_close(
                        window.listener_for(entity, move |s, _, w, cx| s.remove_date(date, w, cx)),
                    ),
            );
        }
        body = body.child(tags);
    }
    if p.show_today || state.options.requires_confirmation() {
        let mut footer = div()
            .flex()
            .items_center()
            .justify_between()
            .gap(rems(theme, "--date-picker-footer-gap"));
        if p.show_today {
            footer = footer.child(
                YueButton::text(
                    (id.clone(), "today"),
                    theme,
                    if state.options.precision.is_some() {
                        "此刻"
                    } else {
                        "今天"
                    },
                )
                .size(ButtonSize::Small)
                .variant(ButtonVariant::Ghost)
                .disabled(!interactive)
                .on_click(window.listener_for(entity, |s, _, w, cx| s.today(w, cx))),
            );
        }
        if state.options.requires_confirmation() {
            footer = footer.child(
                YueButton::text(
                    (id.clone(), "confirm"),
                    theme,
                    if state.options.selection == DatePickerSelection::Multiple {
                        "完成"
                    } else {
                        "确定"
                    },
                )
                .size(ButtonSize::Small)
                .variant(ButtonVariant::Primary)
                .disabled(!interactive || state.candidate(cx).is_err() || state.error.is_some())
                .on_click(window.listener_for(entity, |s, _, w, cx| {
                    s.confirm(w, cx);
                })),
            );
        }
        body = body.child(footer);
    }
    if let Some(footer) = &p.footer {
        body = body.child(footer(interactive, window, cx));
    }
    body.into_any_element()
}

pub(super) fn popup(
    state: &mut YueDatePickerState,
    p: &Presentation,
    theme: YueTheme,
    window: &mut Window,
    cx: &mut Context<YueDatePickerState>,
) -> Option<AnyElement> {
    let scale = f32::from(window.rem_size()) / 16.;
    let margin = px(pixels(theme, "--date-picker-window-margin") * scale);
    let shadows = crate::tokens::shadow(theme, "--popover-shadow");
    let mut outsets: gpui_kit::Edges<gpui_kit::Pixels> = Default::default();
    for s in &shadows {
        let radius = (s.blur_radius + s.spread_radius).max(px(0.));
        outsets.left = outsets.left.max(radius - s.offset.x);
        outsets.right = outsets.right.max(radius + s.offset.x);
        outsets.top = outsets.top.max(radius - s.offset.y);
        outsets.bottom = outsets.bottom.max(radius + s.offset.y);
    }
    let available = PopupHost::available_size(None, outsets, margin, window);
    let cell = pixels(
        theme,
        format!("--calendar-cell-size-{}", suffix(p.size)).as_str(),
    ) * scale;
    let padding = px(pixels(theme, "--popover-padding") * scale);
    let double_width = px(cell * 14.) + padding * 2.;
    let count = if state.options.selection == DatePickerSelection::Range
        && available.width >= double_width
    {
        2
    } else {
        1
    };
    let max_height = available.height;
    let anchor = state.anchor.handle();
    let entity = cx.entity();
    let content_entity = entity.clone();
    let p = p.clone();
    let spec = PopupSpec::new(
        PopupId::new(cx.entity_id(), (state.options.id.clone(), "date-popup")),
        anchor.clone(),
        state.open,
        PopupPlacement::Side(Placement::Bottom, Align::Start),
    )
    .surface_gap(px(pixels(theme, "--date-picker-popup-gap") * scale))
    .margin(margin)
    .visual_outsets(outsets)
    .pointer_mode(PopupPointerMode::Block)
    .dismiss_route(PopupDismissRoute::Runtime)
    .dismiss_on(false, true)
    .exclude(anchor)
    .dismiss_unplaceable()
    .motion(crate::motion::dropdown_popup(theme))
    .on_dismiss(move |request, w, cx| {
        entity.update(cx, |s, cx| {
            let escape = request.reason() == PopupDismissReason::Escape;
            if request.reason() == PopupDismissReason::OutsidePress {
                s.dismiss_outside(w, cx);
            } else {
                s.close(
                    if escape {
                        DatePickerCloseReason::Escape
                    } else {
                        DatePickerCloseReason::Lifecycle
                    },
                    escape,
                    w,
                    cx,
                );
            }
        });
    });
    PopupHost::new(spec).mount(
        move |presentation, _, w, cx| {
            let interactive = presentation.is_interactive() && state.can_modify();
            let content = body(
                state,
                &p,
                theme,
                PanelLayout { count, interactive },
                &content_entity,
                w,
                cx,
            );
            div()
                .id((state.options.id.clone(), "popup-surface"))
                .test_support()
                .role(Role::Dialog)
                .aria_label(format!("{}选择", p.name))
                .track_focus(&state.panel_focus)
                .key_context("YueDatePickerPanel")
                .on_action(w.listener_for(
                    &content_entity,
                    |s, _: &super::behavior::NextField, w, cx| s.traverse_tab(false, w, cx),
                ))
                .on_action(w.listener_for(
                    &content_entity,
                    |s, _: &super::behavior::PreviousField, w, cx| s.traverse_tab(true, w, cx),
                ))
                .on_action(w.listener_for(
                    &content_entity,
                    |s, _: &gpui_kit::base::actions::SelectNextColumn, w, cx| {
                        s.traverse_tab(false, w, cx)
                    },
                ))
                .on_action(w.listener_for(
                    &content_entity,
                    |s, _: &gpui_kit::base::actions::SelectPrevColumn, w, cx| {
                        s.traverse_tab(true, w, cx)
                    },
                ))
                .on_key_down(w.listener_for(
                    &content_entity,
                    |s, event: &gpui_kit::KeyDownEvent, w, cx| {
                        if event.keystroke.key == "escape" && !s.composing(w, cx) {
                            s.close(DatePickerCloseReason::Escape, true, w, cx);
                            cx.stop_propagation();
                        }
                    },
                ))
                .max_w(available.width)
                .max_h(max_height)
                .overflow_y_scroll()
                .p(rems(theme, "--popover-padding"))
                .bg(rgba(theme, "--popover-background"))
                .text_color(rgba(theme, "--popover-color"))
                .border(rems(theme, "--popover-border-width"))
                .border_color(rgba(theme, "--popover-border-color"))
                .rounded(rems(theme, "--popover-border-radius"))
                .shadow(shadows)
                .child(content)
                .into_any_element()
        },
        window,
        cx,
    )
}
