use super::state::value;
use super::*;
use crate::popup::{
    PopupDismissReason, PopupDismissRoute, PopupHost, PopupId, PopupPlacement, PopupPointerMode,
    PopupSpec,
};
use crate::tokens::{pixels, rems, rgba};
use crate::{ButtonSize, ButtonVariant, YueButton, YueSegmented, YueSegmentedOption};
use gpui_kit::base::{Align, Placement, TestSupportExt as _, TimeOption, TimeSegment};
use gpui_kit::{
    Context, InteractiveElement as _, ParentElement as _, Role, StatefulInteractiveElement as _,
    Styled as _, div, prelude::FluentBuilder as _, px,
};
pub(super) fn suffix(size: InputSize) -> &'static str {
    match size {
        InputSize::Small => "sm",
        InputSize::Medium => "md",
        InputSize::Large => "lg",
    }
}
fn unit_name(s: TimeSegment) -> &'static str {
    match s {
        TimeSegment::Hour => "时",
        TimeSegment::Minute => "分",
        TimeSegment::Second => "秒",
        TimeSegment::Period => "上午／下午",
    }
}
fn column_width(theme: YueTheme) -> f32 {
    pixels(theme, "--time-picker-column-width")
}
#[derive(Clone, Copy)]
struct PanelLayout {
    theme: YueTheme,
    narrow: bool,
    interactive: bool,
    available: gpui_kit::Size<gpui_kit::Pixels>,
}
fn body(
    s: &YueTimePickerState,
    p: &Presentation,
    layout: PanelLayout,
    entity: &Entity<YueTimePickerState>,
    w: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let PanelLayout {
        theme,
        narrow,
        interactive,
        available,
    } = layout;
    let id = s.options.id.clone();
    let segments = s.segments();
    let height = rems(
        theme,
        &format!("--time-picker-item-height-{}", suffix(p.size)),
    );
    let scale = f32::from(w.rem_size()) / 16.;
    let row_pixels = pixels(
        theme,
        &format!("--time-picker-item-height-{}", suffix(p.size)),
    ) * scale;
    let rows = ((f32::from(available.height) - 180. * scale) / row_pixels)
        .floor()
        .clamp(2., 6.);
    let groups = if s.range() && !narrow { 2. } else { 1. };
    let insets = pixels(theme, "--popover-padding") * 2. * scale
        + if groups == 2. {
            pixels(theme, "--time-picker-panel-gap") * scale
        } else {
            0.
        };
    let actual_column_width = (column_width(theme) * scale).min(
        ((f32::from(available.width) - insets) / groups - (segments.len() - 1) as f32)
            / segments.len() as f32,
    );
    let mut body = div()
        .flex()
        .flex_col()
        .gap(rems(theme, "--time-picker-panel-gap"));
    if s.range() && narrow {
        body = body.child(
            YueSegmented::new(
                (id.clone(), "endpoint"),
                theme,
                if s.active_endpoint == 0 {
                    "start"
                } else {
                    "end"
                },
            )
            .label("编辑时间端点")
            .options([
                YueSegmentedOption::new("start", "开始时间"),
                YueSegmentedOption::new("end", "结束时间"),
            ])
            .disabled(!interactive)
            .on_change(w.listener_for(
                entity,
                |s, v: &gpui_kit::SharedString, w, cx| {
                    s.active_endpoint = usize::from(v.as_ref() == "end");
                    s.focus_panel(w, cx);
                    cx.notify();
                },
            )),
        );
    }
    let mut panels = div().flex().gap(rems(theme, "--time-picker-panel-gap"));
    for i in 0..if s.range() { 2 } else { 1 } {
        if s.range() && narrow && i != s.active_endpoint {
            continue;
        }
        let mut panel = div().flex().flex_col().gap_1();
        if s.range() {
            panel = panel.child(
                div()
                    .text_sm()
                    .text_color(rgba(
                        theme,
                        if s.active_endpoint == i {
                            "--time-picker-focus-color"
                        } else {
                            "--text-secondary"
                        },
                    ))
                    .child(if i == 0 {
                        "开始时间"
                    } else {
                        "结束时间"
                    }),
            );
        }
        let mut columns = div().flex();
        let selected = s.candidate_parts(cx)[i];
        for (j, segment) in segments.iter().copied().enumerate() {
            let column = &s.columns[i][j];
            column.positioning.position(
                (id.clone(), format!("scroll-{i}-{j}")).into(),
                &column.scroll,
                (row_pixels, rows, column.values.len()),
                theme,
                w,
                cx,
            );
            let positioning = column.positioning.clone();
            let focus = column.focus.clone();
            let mut list = div()
                .id((id.clone(), format!("column-{i}-{j}")))
                .test_support()
                .role(Role::ListBox)
                .aria_label(format!(
                    "{}{}",
                    if s.range() {
                        if i == 0 { "开始" } else { "结束" }
                    } else {
                        ""
                    },
                    unit_name(segment)
                ))
                .track_focus(&focus.clone().tab_stop(interactive))
                .key_context("YueTimeColumn")
                .h(gpui_kit::rems(height.0 * rows))
                .w(px(actual_column_width))
                .overflow_y_scroll()
                .track_scroll(&column.scroll)
                .on_scroll_wheel(move |_, _, _| positioning.cancel())
                .on_key_down(w.listener_for(
                    entity,
                    move |s, e: &gpui_kit::KeyDownEvent, w, cx| {
                        if !e.keystroke.modifiers.modified()
                            && (e.keystroke.key.chars().count() == 1
                                || matches!(
                                    e.keystroke.key.as_str(),
                                    "up" | "down"
                                        | "left"
                                        | "right"
                                        | "home"
                                        | "end"
                                        | "space"
                                        | "enter"
                                        | "escape"
                                ))
                        {
                            s.column_key(i, j, &e.keystroke.key, w, cx);
                            cx.stop_propagation();
                            w.prevent_default();
                        }
                    },
                ));
            for v in column.values.iter().copied() {
                let enabled = interactive && s.choices[i].candidate(segment, v).is_some();
                let is_selected = selected.is_some_and(|t| value(t, segment) == v);
                let label = match segment {
                    TimeSegment::Hour if s.options.cycle == HourCycle::H12 => {
                        format!("{:02}", v % 12 + if v % 12 == 0 { 12 } else { 0 })
                    }
                    TimeSegment::Period => {
                        if v == 0 {
                            "AM".into()
                        } else {
                            "PM".into()
                        }
                    }
                    _ => format!("{v:02}"),
                };
                let focus = focus.clone();
                let owner = entity.clone();
                let item = TimeOption::new((id.clone(), format!("item-{i}-{j}-{v}")), label)
                    .hover_style({
                        let mut style = div().bg(rgba(
                            theme,
                            if is_selected {
                                "--time-picker-selected-background"
                            } else {
                                "--time-picker-hover-background"
                            },
                        ));
                        style.style().clone()
                    })
                    .disabled(!enabled)
                    .selected(is_selected)
                    .active(column.active == Some(v) && column.focus.is_focused(w))
                    .on_change(move |w, cx| {
                        focus.focus(w, cx);
                        owner.update(cx, |s, cx| s.select(i, segment, v, w, cx));
                    })
                    .font_family(crate::typography::number_font(theme).family)
                    .font_features(crate::typography::tabular_nums())
                    .w_full()
                    .h(height)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(rems(
                        theme,
                        &format!("--input-font-size-{}", suffix(p.size)),
                    ))
                    .text_color(rgba(
                        theme,
                        if !enabled {
                            "--time-picker-disabled-color"
                        } else if is_selected {
                            "--time-picker-selected-color"
                        } else {
                            "--text-primary"
                        },
                    ))
                    .rounded(rems(theme, "--time-picker-item-radius"))
                    .when(is_selected, |s| {
                        s.bg(rgba(theme, "--time-picker-selected-background"))
                    })
                    .when(
                        column.active == Some(v) && column.focus.is_focused(w),
                        |s| {
                            s.border_1()
                                .border_color(rgba(theme, "--time-picker-focus-color"))
                        },
                    );
                list = list.child(item);
            }
            let mut wrapper = div()
                .flex()
                .flex_col()
                .flex_shrink_0()
                .child(
                    div()
                        .h(height)
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .text_color(rgba(theme, "--text-secondary"))
                        .child(if segment == TimeSegment::Period {
                            "日段"
                        } else {
                            unit_name(segment)
                        }),
                )
                .child({
                    let positioning = column.positioning.clone();
                    div()
                        .relative()
                        .h(gpui_kit::rems(height.0 * rows))
                        .w(px(actual_column_width))
                        .on_mouse_down(gpui_kit::MouseButton::Left, move |_, _, _| {
                            positioning.cancel()
                        })
                        .child(list)
                        .child(
                            crate::YueScrollbar::vertical(
                                (id.clone(), format!("scrollbar:{i}:{segment:?}")),
                                &column.scroll,
                            )
                            .theme(theme),
                        )
                });
            if j > 0 {
                wrapper = wrapper
                    .border_l_1()
                    .border_color(rgba(theme, "--popover-border-color"));
            }
            columns = columns.child(wrapper);
        }
        panel = panel.child(columns);
        panels = panels.child(panel);
    }
    body = body.child(panels);
    if s.no_available() {
        body = body.child(div().text_sm().child("没有可选时间"));
    }
    if let Some(error) = &s.error {
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
    if let Some(note) = &s.now_note {
        body = body.child(div().text_sm().child(note.clone()));
    }
    let mut footer = div().flex().items_center().justify_between().gap_2();
    if p.show_now {
        footer = footer.child(
            YueButton::text((id.clone(), "now"), theme, "此刻")
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Small)
                .disabled(!interactive || s.no_available())
                .on_click(w.listener_for(entity, |s, _, w, cx| s.now(w, cx))),
        );
    }
    if s.options.commit == TimePickerCommitMode::Confirm {
        footer = footer.child(
            div()
                .flex()
                .gap_2()
                .child(
                    YueButton::text((id.clone(), "cancel"), theme, "取消")
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::Small)
                        .disabled(!interactive)
                        .on_click(w.listener_for(entity, |s, _, w, cx| s.cancel(w, cx))),
                )
                .child(
                    YueButton::text((id.clone(), "confirm"), theme, "确定")
                        .variant(ButtonVariant::Primary)
                        .size(ButtonSize::Small)
                        .disabled(!interactive || s.error.is_some() || !s.has_candidate(cx))
                        .on_click(w.listener_for(entity, |s, _, w, cx| {
                            s.confirm(w, cx);
                        })),
                ),
        );
    }
    body = body.child(footer);
    if let Some(extra) = &p.footer {
        body = body.child(extra(interactive, w, cx));
    }
    body.into_any_element()
}
fn shell(
    s: &YueTimePickerState,
    p: &Presentation,
    layout: PanelLayout,
    entity: &Entity<YueTimePickerState>,
    w: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let theme = layout.theme;
    let content = body(s, p, layout, entity, w, cx);
    div()
        .id((s.options.id.clone(), "panel"))
        .test_support()
        .role(if p.panel_only {
            Role::Group
        } else {
            Role::Dialog
        })
        .aria_label(format!("{}选择", p.name))
        .track_focus(&s.panel_focus)
        .key_context("YueTimePickerPanel")
        .on_action(
            w.listener_for(entity, |s, _: &super::behavior::NextField, w, cx| {
                s.traverse_tab(false, w, cx)
            }),
        )
        .on_action(
            w.listener_for(entity, |s, _: &super::behavior::PreviousField, w, cx| {
                s.traverse_tab(true, w, cx)
            }),
        )
        .on_key_down(
            w.listener_for(entity, |s, e: &gpui_kit::KeyDownEvent, w, cx| {
                if e.keystroke.key == "escape" {
                    s.cancel(w, cx);
                    cx.stop_propagation();
                }
            }),
        )
        .p(rems(theme, "--popover-padding"))
        .bg(rgba(theme, "--popover-background"))
        .text_color(rgba(theme, "--popover-color"))
        .border(rems(theme, "--popover-border-width"))
        .border_color(rgba(theme, "--popover-border-color"))
        .rounded(rems(theme, "--popover-border-radius"))
        .child(content)
        .into_any_element()
}
pub(super) fn inline(
    s: &YueTimePickerState,
    p: &Presentation,
    theme: YueTheme,
    w: &mut Window,
    cx: &mut Context<YueTimePickerState>,
) -> AnyElement {
    let width = f32::from(w.viewport_size().width);
    let scale = f32::from(w.rem_size()) / 16.;
    let double = (column_width(theme) * s.segments().len() as f32 * 2. + 64.) * scale;
    shell(
        s,
        p,
        PanelLayout {
            theme,
            narrow: width < double,
            interactive: s.can_modify(),
            available: w.viewport_size(),
        },
        &cx.entity(),
        w,
        cx,
    )
}
pub(super) fn popup(
    s: &mut YueTimePickerState,
    p: &Presentation,
    theme: YueTheme,
    w: &mut Window,
    cx: &mut Context<YueTimePickerState>,
) -> Option<AnyElement> {
    let scale = f32::from(w.rem_size()) / 16.;
    let margin = px(pixels(theme, "--time-picker-window-margin") * scale);
    let shadows = crate::tokens::shadow(theme, "--popover-shadow");
    let mut outsets: gpui_kit::Edges<gpui_kit::Pixels> = Default::default();
    for shadow in &shadows {
        let r = (shadow.blur_radius + shadow.spread_radius).max(px(0.));
        outsets.left = outsets.left.max(r - shadow.offset.x);
        outsets.right = outsets.right.max(r + shadow.offset.x);
        outsets.top = outsets.top.max(r - shadow.offset.y);
        outsets.bottom = outsets.bottom.max(r + shadow.offset.y);
    }
    let available = PopupHost::available_size(None, outsets, margin, w);
    let narrow = f32::from(available.width)
        < (column_width(theme) * s.segments().len() as f32 * 2. + 64.) * scale;
    let anchor = s.anchor.handle();
    let entity = cx.entity();
    let dismiss = entity.clone();
    let p = p.clone();
    let spec = PopupSpec::new(
        PopupId::new(cx.entity_id(), (s.options.id.clone(), "time-popup")),
        anchor.clone(),
        s.open,
        PopupPlacement::Side(Placement::Bottom, Align::Start),
    )
    .surface_gap(px(pixels(theme, "--time-picker-popup-gap") * scale))
    .margin(margin)
    .visual_outsets(outsets)
    .pointer_mode(PopupPointerMode::Block)
    .dismiss_route(PopupDismissRoute::Runtime)
    .dismiss_on(false, true)
    .exclude(anchor)
    .dismiss_unplaceable()
    .motion(crate::motion::dropdown_popup(theme))
    .on_dismiss(move |request, w, cx| {
        dismiss.update(cx, |s, cx| {
            let escape = request.reason() == PopupDismissReason::Escape;
            s.close(
                if escape {
                    TimePickerCloseReason::Escape
                } else {
                    TimePickerCloseReason::OutsidePress
                },
                escape,
                w,
                cx,
            );
        });
    });
    PopupHost::new(spec).mount(
        move |presentation, _, w, cx| {
            div()
                .max_w(available.width)
                .max_h(available.height)
                .shadow(shadows)
                .child(shell(
                    s,
                    &p,
                    PanelLayout {
                        theme,
                        narrow,
                        interactive: presentation.is_interactive() && s.can_modify(),
                        available,
                    },
                    &entity,
                    w,
                    cx,
                ))
                .into_any_element()
        },
        w,
        cx,
    )
}
