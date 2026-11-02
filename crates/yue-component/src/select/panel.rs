use super::{behavior::*, model::*, state::SelectResultState, *};
use crate::popup::{
    PopupDismissReason, PopupDismissRoute, PopupHost, PopupId, PopupPlacement, PopupPointerMode,
    PopupSpec,
};
use crate::tokens::{duration, easing, pixels, rems, rgba, shadow};
use crate::{Icon, InputSize, InputType, TagSize, YueInput, YueOverflowTooltip, outline};
use gpui_kit::base::{
    A11yDisabledExt as _, A11ySelectionExt as _, Align, Button, InteractiveElementExt as _,
    Placement, SelectionA11y, TestSupportExt as _, Transition, transition, v_virtual_list,
};
use gpui_kit::{
    AccessibleAction, Animation, AnimationExt as _, AnyElement, Context, InteractiveElement as _,
    ParentElement as _, Pixels, Role, StatefulInteractiveElement as _, Styled as _, TextRun,
    canvas, div, prelude::FluentBuilder as _, px,
};
use std::{rc::Rc, time::Duration};

impl SelectSize {
    fn suffix(self) -> &'static str {
        match self {
            Self::Small => "sm",
            Self::Medium => "md",
            Self::Large => "lg",
        }
    }
    fn input(self) -> InputSize {
        match self {
            Self::Small => InputSize::Small,
            Self::Medium => InputSize::Medium,
            Self::Large => InputSize::Large,
        }
    }
    fn tag(self) -> TagSize {
        match self {
            Self::Small => TagSize::Small,
            Self::Medium => TagSize::Medium,
            Self::Large => TagSize::Large,
        }
    }
}
fn metric(theme: YueTheme, name: &str, window: &Window) -> Pixels {
    px(pixels(theme, name) * f32::from(window.rem_size()) / 16.)
}
fn measure(text: &SharedString, font: Pixels, window: &Window) -> Pixels {
    let style = window.text_style();
    window
        .text_system()
        .shape_line(
            text.clone(),
            font,
            &[TextRun {
                len: text.len(),
                font: style.font(),
                color: style.color,
                ..Default::default()
            }],
            None,
        )
        .width()
}
fn option_height(p: &Presentation, theme: YueTheme, window: &Window) -> Pixels {
    let name = if p.row_layout == SelectRowLayout::Description {
        format!("--select-option-description-height-{}", p.size.suffix())
    } else {
        format!("--select-option-height-{}", p.size.suffix())
    };
    metric(theme, &name, window)
}
fn row_height(row: &Row, p: &Presentation, theme: YueTheme, window: &Window) -> Pixels {
    if matches!(row, Row::Group(..)) {
        metric(theme, "--input-height-sm", window)
    } else {
        option_height(p, theme, window)
    }
}

pub(super) fn render(
    state: &mut YueSelectState,
    window: &mut Window,
    cx: &mut Context<YueSelectState>,
) -> AnyElement {
    let Some(p) = state.presentation.clone() else {
        return div().into_any_element();
    };
    assert!(
        !p.name.trim().is_empty(),
        "YueSelect requires accessibility_label or YueField label"
    );
    let entity = cx.entity();
    let theme = p.theme.unwrap_or_else(|| crate::current_theme(cx));
    let id = p.id.clone();
    let height = metric(
        theme,
        &format!("--input-height-{}", p.size.suffix()),
        window,
    );
    let padding = metric(
        theme,
        &format!("--input-padding-inline-{}", p.size.suffix()),
        window,
    );
    let font = metric(
        theme,
        &format!("--input-font-size-{}", p.size.suffix()),
        window,
    );
    let icon = metric(
        theme,
        &format!("--input-icon-size-{}", p.size.suffix()),
        window,
    );
    let gap = metric(theme, "--input-gap", window);
    let tag_gap = metric(theme, "--select-tag-gap", window);
    let selected = state.selected_items();
    let label: SharedString = selected
        .iter()
        .map(|option| option.label.as_str())
        .collect::<Vec<_>>()
        .join("、")
        .into();
    let default_width = metric(theme, "--select-width", window);
    let root_width = match p.width {
        Width::Fill => None,
        Width::Content => Some(
            (measure(
                if label.is_empty() {
                    &p.placeholder
                } else {
                    &label
                },
                font,
                window,
            ) + 2. * padding
                + icon
                + gap)
                .max(height * 2.),
        ),
        Width::Default => Some(p.layout.width.map_or(default_width, |width| {
            width.to_pixels(window.viewport_size().width.into(), window.rem_size())
        })),
    };
    let available = if state.measured_width > px(0.) {
        state.measured_width
    } else {
        root_width.unwrap_or(default_width)
    };
    let minimum = 2. * padding + 2. * icon + gap;
    let editable = state.editable();
    let focused = state.focus.contains_focused(window, cx);
    let border_key = crate::input::border_token(p.status, state.disabled, focused || state.open);
    let border_target = rgba(
        theme,
        if p.status == InputStatus::Normal
            && !focused
            && !state.open
            && state.field_hover
            && !state.disabled
        {
            "--input-border-color-hover"
        } else {
            border_key.name()
        },
    );
    let background = transition(
        (id.clone(), "select-background"),
        gpui_kit::Hsla::from(if state.disabled {
            rgba(theme, "--input-background-disabled")
        } else if state.readonly {
            rgba(theme, "--input-background-readonly")
        } else if p.variant == SelectVariant::Text {
            if state.field_hover || state.open {
                rgba(theme, "--menu-item-background-hover")
            } else {
                gpui_kit::rgba(0)
            }
        } else {
            rgba(theme, "--input-background")
        }),
        Transition::new(duration(theme, "--input-duration")).ease(easing(theme, "--input-ease")),
        window,
        cx,
    );
    let border = transition(
        (id.clone(), "select-border"),
        gpui_kit::Hsla::from(border_target),
        Transition::new(if state.disabled {
            Duration::ZERO
        } else {
            duration(theme, "--input-duration")
        })
        .ease(easing(theme, "--input-ease")),
        window,
        cx,
    );
    let foreground = rgba(
        theme,
        if state.disabled {
            "--input-color-disabled"
        } else {
            "--input-color"
        },
    );
    let mut content = div()
        .id((id.clone(), "selection"))
        .flex_1()
        .min_w_0()
        .flex()
        .items_center()
        .gap(tag_gap);
    if p.tag_layout == SelectTagLayout::Wrap {
        content = content.flex_wrap();
    }
    state.hidden_keys.clear();
    if let Some(renderer) = &p.value_renderer {
        content = content.child(renderer(&selected, window, cx));
    } else if selected.is_empty() {
        content = content.child(
            div()
                .min_w_0()
                .truncate()
                .text_color(rgba(theme, "--input-placeholder-color"))
                .child(p.placeholder.clone()),
        );
    } else if state.mode == SelectMode::Single {
        content = content.child(
            YueOverflowTooltip::new((id.clone(), "selected-label"), theme, label.clone())
                .width(gpui_kit::relative(1.))
                .track_focus(&state.focus),
        );
    } else {
        let budget = (available
            - 2. * padding
            - icon
            - gap
            - if p.clearable { icon + gap } else { px(0.) }
            - if p.icon.is_some() { icon + gap } else { px(0.) }
            - p.prefix
                .as_ref()
                .map_or(px(0.), |prefix| measure(prefix, font, window) + gap))
        .max(px(0.));
        let tag_font = metric(
            theme,
            &format!("--tag-font-size-{}", p.size.suffix()),
            window,
        );
        let tag_padding = metric(
            theme,
            &format!("--tag-padding-inline-{}", p.size.suffix()),
            window,
        );
        let widths = selected
            .iter()
            .map(|option| {
                measure(&option.label, tag_font, window)
                    + 2. * tag_padding
                    + if state.removable(&option.key) {
                        metric(theme, "--tag-close-icon-size", window) + tag_gap
                    } else {
                        px(0.)
                    }
                    + 2. * metric(theme, "--tag-border-width", window)
            })
            .collect::<Vec<_>>();
        let mut show = if p.tag_layout == SelectTagLayout::Wrap {
            selected.len()
        } else if p.tag_layout == SelectTagLayout::Collapse {
            p.collapse_count.min(selected.len())
        } else {
            selected.len()
        };
        if p.tag_layout != SelectTagLayout::Wrap {
            while show > 0 {
                let used = widths[..show].iter().copied().fold(px(0.), |a, b| a + b)
                    + tag_gap * (show.saturating_sub(1) as f32);
                let count = if show < selected.len() {
                    measure(
                        &format!("+{}", selected.len() - show).into(),
                        tag_font,
                        window,
                    ) + 2. * tag_padding
                        + tag_gap
                } else {
                    px(0.)
                };
                if used + count <= budget {
                    break;
                }
                show -= 1;
            }
        }
        for (ix, option) in selected.iter().enumerate() {
            if ix >= show {
                state.hidden_keys.push(option.key.clone());
                continue;
            }
            let mut tag = YueTag::new(
                (id.clone(), format!("tag:{}", option.key)),
                option.label.clone(),
            )
            .theme(theme)
            .size(p.size.tag())
            .overflow_tooltip(true)
            .max_width(budget.max(px(1.)))
            .disabled(state.disabled);
            if let Some(renderer) = &p.tag_renderer {
                tag = renderer(tag, option, window, cx);
            }
            tag = tag.reset_select_behavior().disabled(state.disabled);
            if state.removable(&option.key) {
                let key = option.key.clone();
                let owner = entity.clone();
                tag = tag
                    .close_label(format!("移除 {}", option.label))
                    .close_tab_stop(p.tab_stop)
                    .track_close_focus(
                        state
                            .tag_focus
                            .get(&option.key)
                            .expect("selected tag focus"),
                    )
                    .on_close(move |_, window, cx| {
                        cx.stop_propagation();
                        owner.update(cx, |state, cx| {
                            state.remove(&key, SelectChangeSource::TagRemove, window, cx)
                        })
                    });
            }
            content = content.child(tag);
        }
        if show < selected.len() {
            let owner = entity.clone();
            let count = selected.len() - show;
            let mut count_button = Button::new((id.clone(), "collapsed-count"))
                .accessibility_label(format!("查看其余 {count} 个选择"))
                .track_focus(&state.collapsed_focus)
                .tab_stop(p.tab_stop && !state.disabled)
                .disabled(state.disabled || state.readonly)
                .cursor_pointer()
                .h(metric(
                    theme,
                    &format!("--tag-height-{}", p.size.suffix()),
                    window,
                ))
                .px(tag_padding)
                .rounded(rems(theme, "--tag-border-radius"))
                .bg(rgba(theme, "--tag-background"))
                .text_color(foreground)
                .relative()
                .child(state.collapse_anchor.capture())
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    owner.update(cx, |state, cx| {
                        let open = !state.collapsed_open;
                        if state.open {
                            state.close(SelectCloseReason::Trigger, false, window, cx);
                        }
                        state.collapsed_open = open;
                        if open {
                            state.collapsed_focus.focus(window, cx);
                        }
                        cx.notify();
                    });
                });
            count_button = if let Some(renderer) = &p.collapse_renderer {
                count_button.child(renderer(count, window, cx))
            } else {
                count_button.child(format!("+{count}"))
            };
            content = content.child(count_button);
        }
    }
    if state.collapsed_open && state.hidden_keys.is_empty() {
        state.collapsed_open = false;
        state.focus.focus(window, cx);
    }
    let trigger_owner = entity.clone();
    // Only the value/empty-space activation zone toggles the popup. Tag closes,
    // clear and +N are sibling controls; no whole-frame click handler can steal
    // their activation or nest one native Button inside another.
    let mut activation = div()
        .id((id.clone(), "trigger-value"))
        .test_support()
        .flex_1()
        .min_w_0()
        .flex()
        .items_center()
        .gap(gap)
        .when(editable, |this| {
            this.cursor_pointer().on_click(move |_, window, cx| {
                trigger_owner.update(cx, |state, cx| {
                    state.keyboard_navigation = false;
                    if state.open {
                        state.close(SelectCloseReason::Trigger, true, window, cx);
                    } else {
                        state.focus.focus(window, cx);
                        state.change_open(true, SelectCloseReason::Trigger, true, window, cx);
                        state.content_focus(cx).focus(window, cx);
                    }
                });
            })
        });
    if let Some(icon_image) = p.icon {
        activation = activation.child(
            icon_image
                .size(gpui_kit::rems(
                    f32::from(icon) / f32::from(window.rem_size()),
                ))
                .color_value(foreground),
        );
    }
    if let Some(prefix) = &p.prefix {
        activation = activation.child(div().flex_shrink_0().child(prefix.clone()));
    }
    activation = activation.child(content);
    let hover_owner = entity.clone();
    let mut row = div()
        .id((id.clone(), "frame"))
        .test_support()
        .key_context("YueSelectTrigger")
        .relative()
        .w_full()
        .min_w(minimum)
        .min_h(height)
        .px(padding)
        .flex()
        .items_center()
        .gap(gap)
        .text_size(font)
        .text_color(foreground)
        .border(rems(theme, "--input-border-width"))
        .border_color(
            if p.variant == SelectVariant::Text
                && !focused
                && !state.open
                && p.status == InputStatus::Normal
            {
                gpui_kit::Hsla::from(gpui_kit::rgba(0))
            } else {
                border
            },
        )
        .rounded(rems(theme, "--input-border-radius"))
        .bg(background)
        .on_hover(move |hover, _, cx| {
            hover_owner.update(cx, |state, cx| {
                if state.field_hover != *hover {
                    state.field_hover = *hover;
                    cx.notify();
                }
            })
        })
        .child(state.anchor.capture())
        .child(activation);
    row = if p.tag_layout == SelectTagLayout::Wrap && state.mode == SelectMode::Multiple {
        row.py(
            ((height - metric(theme, &format!("--tag-height-{}", p.size.suffix()), window)) / 2.
                - metric(theme, "--input-border-width", window))
            .max(px(0.)),
        )
    } else {
        row.h(height)
    };
    if p.clearable {
        let can_clear = state.value.keys().iter().any(|key| state.removable(key));
        let owner = entity.clone();
        row = row.child(
            Button::new((id.clone(), "clear"))
                .accessibility_label(
                    if state.value.keys().iter().any(|key| !state.removable(key)) {
                        "清除可移除的选择"
                    } else {
                        "清空选择"
                    },
                )
                .disabled(!can_clear)
                .tab_stop(p.tab_stop && can_clear)
                .size(icon)
                .flex_shrink_0()
                .cursor_pointer()
                .opacity(if can_clear && (state.field_hover || focused) {
                    1.
                } else {
                    0.
                })
                .on_click(move |_, window, cx| {
                    owner.update(cx, |state, cx| state.clear(window, cx))
                })
                .child(
                    Icon::new(outline::X)
                        .size(gpui_kit::rems(
                            f32::from(icon) / f32::from(window.rem_size()),
                        ))
                        .color_value(rgba(theme, "--input-clear-color")),
                ),
        );
    }
    let arrow_owner = entity.clone();
    let open = state.open;
    let arrow_rotation = transition(
        (id.clone(), "arrow-rotation"),
        if open { 0.5_f32 } else { 0_f32 },
        Transition::new(duration(theme, "--input-duration")).ease(easing(theme, "--input-ease")),
        window,
        cx,
    );
    row = row.child(
        Button::new((id.clone(), "arrow"))
            .accessibility_label(if open { "收起选项" } else { "展开选项" })
            .tab_stop(false)
            .disabled(!editable)
            .size(icon)
            .cursor_pointer()
            .on_click(move |_, window, cx| {
                arrow_owner.update(cx, |state, cx| {
                    if state.open {
                        state.close(SelectCloseReason::Trigger, true, window, cx);
                    } else {
                        state.change_open(true, SelectCloseReason::Trigger, true, window, cx);
                        state.content_focus(cx).focus(window, cx);
                    }
                })
            })
            .child({
                let loading = state.loading || matches!(state.result, SelectResultState::Loading);
                let image = Icon::new(if loading {
                    crate::YueIcon::Loading
                } else {
                    crate::YueIcon::ChevronDown
                })
                .size(gpui_kit::rems(
                    f32::from(icon) / f32::from(window.rem_size()),
                ))
                .rotation(if loading { 0. } else { arrow_rotation })
                .color_value(foreground);
                if loading
                    && !cx.reduce_motion()
                    && window.is_window_active()
                    && state.anchor.handle().bounds().is_some()
                {
                    image
                        .with_animation(
                            (id.clone(), "spinner"),
                            Animation::new(duration(theme, "--button-spinner-duration")).repeat(),
                            |icon, delta| icon.rotation(delta),
                        )
                        .into_any_element()
                } else {
                    image.into_any_element()
                }
            }),
    );
    if focused && !state.disabled {
        let outside = metric(theme, "--input-focus-ring-width", window)
            + metric(theme, "--input-focus-ring-offset", window);
        // Absolute insets start inside the frame's border; the ring's offset
        // is measured from its outside edge.
        let inset = outside + metric(theme, "--input-border-width", window);
        row = row.child(
            div()
                .absolute()
                .top(-inset)
                .bottom(-inset)
                .left(-inset)
                .right(-inset)
                .rounded(metric(theme, "--input-border-radius", window) + outside)
                .border(rems(theme, "--input-focus-ring-width"))
                .border_color(rgba(theme, "--input-focus-ring-color")),
        );
    }
    let measured = entity.downgrade();
    row = row.child(
        canvas(
            move |bounds, window, cx| {
                let mounted_owner = measured.clone();
                let handle = window.window_handle();
                let _mount = window.use_keyed_state("select-mount-session", cx, |_, cx| {
                    cx.on_release(move |_, cx| {
                        let _ = handle.update(cx, |_, window, cx| {
                            let _ = mounted_owner.update(cx, |state, cx| {
                                state.close(SelectCloseReason::Lifecycle, false, window, cx);
                            });
                        });
                    })
                    .detach();
                });
                let width = bounds.size.width;
                let _ = measured.update(cx, |state, _| {
                    if state.measured_width != width {
                        state.measured_width = width;
                        let refresh = measured.clone();
                        window.on_next_frame(move |_, cx| {
                            let _ = refresh.update(cx, |_, cx| cx.notify());
                        });
                    }
                });
            },
            |_, _, _, _| {},
        )
        .absolute()
        .inset_0(),
    );
    let focus = state.focus.clone();
    let content_focus = state.content_focus(cx);
    let owner = entity.clone();
    let confirm = entity.clone();
    let mut semantic = gpui_kit::base::Select::new(id.clone())
        .open(state.open)
        .disabled(state.disabled)
        .readonly(state.readonly)
        .required(p.required)
        .invalid(p.status == InputStatus::Error)
        .tab_stop(p.tab_stop)
        .tab_index(p.tab_index)
        .focus_handle(&focus)
        .content_focus_handle(&content_focus)
        .accessibility_label(p.name.clone())
        .accessibility_value(label)
        .on_open_change(move |open, window, cx| {
            owner.update(cx, |state, cx| {
                state.keyboard_navigation = true;
                state.change_open(open, SelectCloseReason::Trigger, true, window, cx)
            })
        })
        .on_confirm(move |window, cx| {
            confirm.update(cx, |state, cx| {
                if let Some(key) = state.active.clone() {
                    state.activate(key, SelectChangeSource::Keyboard, window, cx);
                }
            })
        })
        .child(row)
        .w_full();
    if let Some(description) = &p.description {
        semantic = semantic.accessibility_description(description.clone());
    }
    if let Some(popup) = popup(state, false, &p, theme, window, cx) {
        semantic = semantic.child(popup);
    }
    if let Some(popup) = popup(state, true, &p, theme, window, cx) {
        semantic = semantic.child(popup);
    }
    let mut outer = div()
        .id((id.clone(), "select-owner"))
        .relative()
        .key_context("YueSelect")
        .flex()
        .flex_col()
        .min_w(minimum)
        .when_some(p.key_context.clone(), |this, context| {
            this.key_context(context.as_str())
        });
    outer = match p.width {
        Width::Fill => outer.w_full(),
        _ => outer.w(root_width.unwrap_or(default_width)),
    };
    let layout = (*p.layout).clone();
    outer = layout.apply(outer);
    outer = actions(outer, cx).child(semantic);
    outer.into_any_element()
}

fn actions(
    root: gpui_kit::Stateful<gpui_kit::Div>,
    cx: &mut Context<YueSelectState>,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    root.on_action(cx.listener(|state, _: &Next, w, cx| {
        state.keyboard_navigation = true;
        state.navigate(Move::Next, w, cx);
    }))
    .on_action(cx.listener(|state, _: &Previous, w, cx| {
        state.keyboard_navigation = true;
        state.navigate(Move::Previous, w, cx);
    }))
    .on_action(cx.listener(|state, _: &First, w, cx| state.navigate(Move::First, w, cx)))
    .on_action(cx.listener(|state, _: &Last, w, cx| state.navigate(Move::Last, w, cx)))
    .on_action(cx.listener(|state, _: &PageNext, w, cx| {
        state.navigate(Move::PageNext(state.page_size), w, cx)
    }))
    .on_action(cx.listener(|state, _: &PagePrevious, w, cx| {
        state.navigate(Move::PagePrevious(state.page_size), w, cx)
    }))
    .on_action(cx.listener(|state, _: &Accept, w, cx| state.accept(w, cx)))
    .on_action(cx.listener(|state, _: &Leave, w, cx| state.leave(false, w, cx)))
    .on_action(cx.listener(|state, _: &LeaveBack, w, cx| state.leave(true, w, cx)))
    .on_action(cx.listener(|state, _: &Find, w, cx| state.find(w, cx)))
    .on_action(cx.listener(|state, _: &TagBack, w, cx| state.tag_back(w, cx)))
    .on_action(cx.listener(|state, _: &Dismiss, w, cx| state.dismiss(w, cx)))
    .on_key_down(cx.listener(YueSelectState::type_key))
}

fn popup(
    state: &mut YueSelectState,
    collapsed: bool,
    p: &Presentation,
    theme: YueTheme,
    window: &mut Window,
    cx: &mut Context<YueSelectState>,
) -> Option<AnyElement> {
    let present = if collapsed {
        state.collapsed_open
    } else {
        state.open
    };
    let entity = cx.entity();
    let id = p.id.clone();
    let anchor = if collapsed {
        state.collapse_anchor.handle()
    } else {
        state.anchor.handle()
    };
    let margin = metric(theme, "--space-8", window);
    let shadows = shadow(theme, "--popover-shadow");
    let mut outsets: gpui_kit::Edges<Pixels> = Default::default();
    for shadow in &shadows {
        let reserve = (shadow.blur_radius + shadow.spread_radius).max(px(0.));
        outsets.left = outsets.left.max(reserve - shadow.offset.x);
        outsets.right = outsets.right.max(reserve + shadow.offset.x);
        outsets.top = outsets.top.max(reserve - shadow.offset.y);
        outsets.bottom = outsets.bottom.max(reserve + shadow.offset.y);
    }
    let available = PopupHost::available_size(None, outsets, margin, window);
    let trigger_width = if state.measured_width > px(0.) {
        state.measured_width
    } else {
        p.layout
            .width
            .map_or(metric(theme, "--select-width", window), |width| {
                width.to_pixels(window.viewport_size().width.into(), window.rem_size())
            })
    };
    let font = metric(
        theme,
        &format!("--input-font-size-{}", p.size.suffix()),
        window,
    );
    let width = match p.panel_width {
        SelectPanelWidth::MatchTrigger => trigger_width,
        SelectPanelWidth::Fixed(width) => width,
        SelectPanelWidth::Content => {
            let face = format!("{:?}:{:?}", window.text_style().font(), p.row_layout);
            let cached = state
                .width_cache
                .as_ref()
                .filter(|(rows, old_font, old_face, _)| {
                    Rc::ptr_eq(rows, &state.rows) && *old_font == font && *old_face == face
                });
            let measured = if let Some((_, _, _, measured)) = cached {
                *measured
            } else {
                let measured = state
                    .rows
                    .iter()
                    .map(|row| match row {
                        Row::Option(option) => {
                            measure(&option.label, font, window).max(
                                option
                                    .description
                                    .as_ref()
                                    .filter(|_| p.row_layout == SelectRowLayout::Description)
                                    .map_or(px(0.), |text| measure(text, font, window)),
                            ) + option
                                .meta
                                .as_ref()
                                .map_or(px(0.), |text| measure(text, font, window))
                                + metric(theme, "--input-height-md", window) * 2.
                        }
                        Row::Group(_, label) | Row::Create(label) => {
                            measure(label, font, window)
                                + metric(theme, "--input-height-md", window) * 2.
                        }
                    })
                    .fold(px(0.), Pixels::max);
                state.width_cache = Some((state.rows.clone(), font, face, measured));
                measured
            };
            measured.max(trigger_width)
        }
    }
    .min(available.width)
    .max(px(1.));
    let anchor_bounds = anchor.bounds();
    let gap = metric(theme, "--select-panel-gap", window);
    let side_available = anchor_bounds.map_or(available.height, |bounds| {
        (window.viewport_size().height - bounds.bottom() - gap - margin - outsets.bottom)
            .max(bounds.top() - gap - margin - outsets.top)
    });
    let maximum = available.height.min(side_available).max(px(1.));
    let owner = entity.clone();
    let mut spec = PopupSpec::new(
        PopupId::new(
            cx.entity_id(),
            (
                id.clone(),
                if collapsed {
                    "select-collapsed-popup"
                } else {
                    "select-popup"
                },
            ),
        ),
        anchor,
        present,
        PopupPlacement::Side(Placement::Bottom, Align::Start),
    )
    .surface_gap(gap)
    .margin(margin)
    .pointer_mode(PopupPointerMode::Block)
    .dismiss_route(PopupDismissRoute::Runtime)
    // Outside presses use the shared router; query Escape stays with the
    // owner so IME composition can consume it before closing the field.
    .dismiss_on(collapsed, true)
    .dismiss_unplaceable()
    .exclude(state.anchor.handle())
    .exclude(state.collapse_anchor.handle())
    .visual_outsets(outsets)
    .motion(crate::motion::dropdown_popup(theme))
    .on_dismiss(move |request, window, cx| {
        owner.update(cx, |state, cx| {
            let escape = request.reason() == PopupDismissReason::Escape;
            if collapsed {
                state.collapsed_open = false;
                if escape {
                    state.collapsed_focus.focus(window, cx);
                }
                cx.notify();
            } else {
                state.close(
                    if escape {
                        SelectCloseReason::Escape
                    } else if request.reason() == PopupDismissReason::OutsidePress {
                        SelectCloseReason::OutsidePress
                    } else {
                        SelectCloseReason::Lifecycle
                    },
                    escape,
                    window,
                    cx,
                );
            }
        });
    });
    if let Some(parent) = &state.parent {
        spec = spec.parent(parent.clone());
    }
    let use_virtual = p.virtualization == SelectVirtualization::Always
        || (p.virtualization == SelectVirtualization::Auto && state.rows.len() > 100);
    state.virtualized = use_virtual;
    let inline_rows = if !use_virtual && !collapsed {
        let rows = state.rows.clone();
        Some(
            rows.iter()
                .map(|row| render_row(row, state, p, theme, present, window, cx))
                .collect::<Vec<_>>(),
        )
    } else {
        None
    };
    let p = p.clone();
    let scroll = state.scroll.clone();
    let input = if state.search == SelectSearch::None {
        None
    } else {
        state.input.clone()
    };
    let multiple = state.mode == SelectMode::Multiple;
    let rows = state.rows.clone();
    let option_h = option_height(&p, theme, window);
    let group_h = metric(theme, "--input-height-sm", window);
    let geometry = if let Some(cached) = state.list_cache.as_ref().filter(|cached| {
        Rc::ptr_eq(&cached.rows, &rows)
            && cached.width == width
            && cached.option_height == option_h
            && cached.group_height == group_h
    }) {
        cached.clone()
    } else {
        let sizes = rows
            .iter()
            .map(|row| {
                gpui_kit::size(
                    width,
                    if matches!(row, Row::Group(..)) {
                        group_h
                    } else {
                        option_h
                    },
                )
            })
            .collect::<Vec<_>>();
        let total = sizes.iter().fold(px(0.), |sum, size| sum + size.height);
        let cached = Rc::new(super::state::ListGeometry {
            rows: rows.clone(),
            width,
            option_height: option_h,
            group_height: group_h,
            sizes: Rc::new(sizes),
            total,
            option_count: rows
                .iter()
                .filter(|row| matches!(row, Row::Option(_)))
                .count(),
        });
        state.list_cache = Some(cached.clone());
        cached
    };
    let hidden = state
        .selected_items()
        .into_iter()
        .filter(|option| state.hidden_keys.contains(&option.key))
        .collect::<Vec<_>>();
    let focus = if collapsed {
        state.collapsed_focus.clone()
    } else {
        state.list_focus.clone()
    };
    let result = state.result.clone();
    let more = state.more_result.clone();
    let create = state.create_result.clone();
    let has_more = state.has_more();
    let at_limit = state.at_limit();
    let limit = state.max_selected;
    let slot_heights = state.slot_heights;
    PopupHost::new(spec).mount(
        move |presentation, _, window, cx| {
            let interactive = presentation.is_interactive();
            let padding = metric(theme, "--popover-padding", window);
            let row_h = option_height(&p, theme, window);
            let mut panel = div()
                .id((
                    id.clone(),
                    if collapsed {
                        "collapsed-panel"
                    } else {
                        "select-panel"
                    },
                ))
                .test_support()
                .key_context("YueSelectPanel")
                .w(width)
                .max_h(maximum)
                .flex()
                .flex_col()
                .overflow_hidden()
                .p(padding)
                .gap(metric(theme, "--space-4", window))
                .bg(rgba(theme, "--popover-background"))
                .border(rems(theme, "--popover-border-width"))
                .border_color(rgba(theme, "--popover-border-color"))
                .rounded(rems(theme, "--popover-border-radius"))
                .shadow(shadows)
                .when(!interactive, |this| this.occlude());
            if collapsed {
                let mut list = div()
                    .id((id.clone(), "hidden-selections"))
                    .flex()
                    .flex_col()
                    .gap(metric(theme, "--select-tag-gap", window))
                    .max_h(maximum - 2. * padding)
                    .overflow_y_scroll();
                for option in hidden {
                    let owner = entity.clone();
                    let key = option.key.clone();
                    let mut tag = YueTag::new(
                        (id.clone(), format!("hidden:{}", option.key)),
                        option.label.clone(),
                    )
                    .theme(theme)
                    .size(p.size.tag());
                    if !option.disabled && interactive {
                        tag = tag.close_label(format!("移除 {}", option.label)).on_close(
                            move |_, window, cx| {
                                owner.update(cx, |state, cx| {
                                    state.remove(&key, SelectChangeSource::TagRemove, window, cx)
                                })
                            },
                        );
                    }
                    list = list.child(tag);
                }
                return panel
                    .role(Role::Group)
                    .aria_label("其余已选项")
                    .child(list)
                    .into_any_element();
            }
            let mut reserved = 2. * padding;
            if let Some(input) = input
                && interactive
            {
                // An existing YueInput, not a Select-specific text editor. The
                // context only overrides keys required by this composite.
                let search = YueInput::new((id.clone(), "query"), theme, &input)
                    .size(p.size.input())
                    .input_type(InputType::Search)
                    .accessibility_label("搜索选项")
                    .accessibility_description("方向键或回车进入结果，回车选择，Ctrl+F 返回搜索")
                    .placeholder("搜索选项")
                    .leading_icon(outline::Search)
                    .clearable(true)
                    .block();
                panel = panel.child(div().key_context("YueSelectQuery").child(search));
                reserved += metric(
                    theme,
                    &format!("--input-height-{}", p.size.suffix()),
                    window,
                ) + metric(theme, "--space-4", window);
            }
            if let Some(header) = &p.header {
                panel = panel.child(panel_slot(
                    header(interactive, window, cx),
                    0,
                    &entity,
                    &id,
                    maximum / 3.,
                ));
                reserved += slot_heights[0].max(row_h) + metric(theme, "--space-4", window);
            }
            if p.footer.is_some() {
                reserved += slot_heights[1].max(row_h) + metric(theme, "--space-4", window);
            }
            if has_more
                || !matches!(more, SelectResultState::Ready)
                || !matches!(create, SelectResultState::Ready)
                || at_limit
            {
                reserved += row_h;
            }
            let list_max = p
                .max_height
                .unwrap_or_else(|| metric(theme, "--select-list-max-height", window))
                .min((maximum - reserved).max(px(1.)));
            let total = geometry.total;
            let list_height = total.min(list_max).max(row_h.min(list_max));
            let list_id: ElementId = (id.clone(), "list").into();
            let mut list = div()
                .id(list_id.clone())
                .test_support()
                .key_context("YueSelectList")
                .h(list_height)
                .w_full()
                .min_w_0()
                .flex_shrink_0()
                .when(interactive, |this| {
                    this.role(Role::ListBox)
                        .aria_label(format!("{}选项", p.name))
                        .aria_description(match &result {
                            SelectResultState::Loading => p.loading_text.to_string(),
                            SelectResultState::Failed(message) => message.to_string(),
                            SelectResultState::Ready if has_more => {
                                format!("已加载 {} 个选项", geometry.option_count)
                            }
                            SelectResultState::Ready => {
                                format!("共 {} 个选项", geometry.option_count)
                            }
                        })
                        .track_focus(&focus)
                });
            let content = if matches!(result, SelectResultState::Loading) {
                div()
                    .p(metric(theme, "--space-8", window))
                    .child(p.loading_text.clone())
                    .into_any_element()
            } else if let SelectResultState::Failed(message) = &result {
                status_button(
                    (id.clone(), "retry"),
                    message.clone(),
                    &entity,
                    theme,
                    interactive,
                )
            } else if rows.is_empty() {
                div()
                    .p(metric(theme, "--space-8", window))
                    .child(p.empty_text.clone())
                    .into_any_element()
            } else if use_virtual {
                let sizes = geometry.sizes.clone();
                let config = p.clone();
                v_virtual_list(
                    entity.clone(),
                    (id.clone(), "virtual-list"),
                    sizes,
                    move |state, range, window, cx| {
                        range
                            .map(|ix| {
                                render_row(
                                    &state.rows[ix].clone(),
                                    state,
                                    &config,
                                    theme,
                                    interactive,
                                    window,
                                    cx,
                                )
                            })
                            .collect::<Vec<_>>()
                    },
                )
                .track_scroll(&scroll)
                .into_any_element()
            } else {
                div()
                    .id((id.clone(), "option-list"))
                    .size_full()
                    .overflow_y_scroll()
                    .lock_scroll_axis()
                    .track_scroll(scroll.base_handle())
                    .children(inline_rows.unwrap_or_default())
                    .into_any_element()
            };
            let page_owner = entity.downgrade();
            list = list.child(
                canvas(
                    move |bounds, _, cx| {
                        let _ = page_owner.update(cx, |state, _| {
                            state.page_size = (f32::from(bounds.size.height) / f32::from(row_h))
                                .floor()
                                .max(1.) as usize
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            );
            if interactive
                && has_more
                && matches!(more, SelectResultState::Ready)
                && -scroll.base_handle().offset().y + list_height >= total - row_h
            {
                let owner = entity.downgrade();
                window.on_next_frame(move |_, cx| {
                    let _ = owner.update(cx, |state, cx| state.request_more(cx));
                });
            }
            let more_owner = entity.clone();
            let observed_scroll = scroll.clone();
            list = list.child(content).on_scroll_wheel(move |_, window, _| {
                let owner = more_owner.clone();
                let handle = observed_scroll.clone();
                window.on_next_frame(move |_, cx| {
                    if f32::from(-handle.base_handle().offset().y) + f32::from(list_height)
                        >= f32::from(total - row_h)
                    {
                        owner.update(cx, |state, cx| state.request_more(cx));
                    }
                });
            });
            panel = panel.child(list.a11y_selection(SelectionA11y::default().multiple(multiple)));
            if at_limit {
                panel = panel.child(
                    div()
                        .text_sm()
                        .text_color(rgba(theme, "--text-secondary"))
                        .child(format!("最多选择 {} 项", limit.unwrap_or(0))),
                );
            }
            if matches!(create, SelectResultState::Loading) {
                panel = panel.child("正在创建选项");
            }
            if let SelectResultState::Failed(message) = &create {
                panel = panel.child(status_button(
                    (id.clone(), "create-retry"),
                    message.clone(),
                    &entity,
                    theme,
                    interactive,
                ));
            }
            if let SelectResultState::Failed(message) = &more {
                panel = panel.child(status_button(
                    (id.clone(), "more-retry"),
                    message.clone(),
                    &entity,
                    theme,
                    interactive,
                ));
            } else if matches!(more, SelectResultState::Loading) {
                panel = panel.child("正在加载更多");
            } else if has_more {
                let owner = entity.clone();
                panel = panel.child(
                    Button::new((id.clone(), "load-more"))
                        .disabled(!interactive)
                        .accessibility_label("加载更多选项")
                        .cursor_pointer()
                        .on_click(move |_, _, cx| {
                            owner.update(cx, |state, cx| state.request_more(cx))
                        })
                        .child("加载更多"),
                );
            }
            if let Some(footer) = &p.footer {
                panel = panel.child(panel_slot(
                    footer(interactive, window, cx),
                    1,
                    &entity,
                    &id,
                    maximum / 3.,
                ));
            }
            panel.into_any_element()
        },
        window,
        cx,
    )
}

fn status_button(
    id: impl Into<ElementId>,
    message: SharedString,
    owner: &Entity<YueSelectState>,
    theme: YueTheme,
    interactive: bool,
) -> AnyElement {
    let owner = owner.clone();
    div()
        .flex()
        .flex_col()
        .text_color(rgba(theme, "--error-foreground"))
        .child(message)
        .child(
            Button::new(id)
                .accessibility_label("重试加载")
                .disabled(!interactive)
                .cursor_pointer()
                .on_click(move |_, _, cx| owner.update(cx, |state, cx| state.retry_request(cx)))
                .child("重试"),
        )
        .into_any_element()
}
fn render_row(
    row: &Row,
    state: &YueSelectState,
    p: &Presentation,
    theme: YueTheme,
    interactive: bool,
    window: &mut Window,
    cx: &mut Context<YueSelectState>,
) -> AnyElement {
    let id: ElementId = (
        p.id.clone(),
        match row {
            Row::Group(key, _) => format!("group:{key}"),
            Row::Option(option) => format!("option:{}", option.key),
            Row::Create(query) => format!("create:{query}"),
        },
    )
        .into();
    let height = row_height(row, p, theme, window);
    let padding = metric(
        theme,
        &format!("--input-padding-inline-{}", p.size.suffix()),
        window,
    );
    let font = metric(
        theme,
        &format!("--input-font-size-{}", p.size.suffix()),
        window,
    );
    let icon = metric(
        theme,
        &format!("--input-icon-size-{}", p.size.suffix()),
        window,
    );
    if let Row::Group(_, label) = row {
        return div()
            .id(id)
            .h(height)
            .px(padding)
            .flex()
            .items_center()
            .text_sm()
            .text_color(rgba(theme, "--text-secondary"))
            .child(label.clone())
            .into_any_element();
    }
    let key = row.active_key().expect("selectable row identity");
    let enabled = interactive && state.eligible(row) && state.editable();
    let active = state.active.as_ref() == Some(&key);
    let selected = matches!(row,Row::Option(option) if state.value.contains(&option.key));
    let target = enabled
        && if state.keyboard_navigation {
            active
        } else {
            state.hovered.as_ref() == Some(&key)
        };
    let amount = transition(
        (id.clone(), "hover"),
        if target { 1_f32 } else { 0_f32 },
        Transition::new(if interactive {
            duration(theme, "--menu-item-duration")
        } else {
            Duration::ZERO
        })
        .ease(easing(theme, "--menu-item-ease")),
        window,
        cx,
    );
    let mut hover = rgba(theme, "--menu-item-background-hover");
    hover.a *= amount;
    let selected_bg = if selected {
        rgba(theme, "--select-option-background-selected")
    } else {
        gpui_kit::rgba(0)
    };
    let fg = rgba(
        theme,
        if !enabled {
            "--input-color-disabled"
        } else if selected {
            "--select-option-color-selected"
        } else {
            "--menu-item-color"
        },
    );
    let name = match row {
        Row::Option(option) => option.label.clone(),
        Row::Create(query) => format!("创建「{query}」").into(),
        _ => unreachable!(),
    };
    let owner = cx.entity();
    let activation = key.clone();
    let a11y_activation = key.clone();
    let a11y_owner = owner.clone();
    let hovered = key.clone();
    let moving = key.clone();
    let mouse_owner = owner.clone();
    let hover_owner = owner.clone();
    let mut shell = div()
        .id(id.clone())
        .test_support()
        .h(height)
        .w_full()
        .px(padding)
        .flex()
        .items_center()
        .gap(metric(theme, "--input-gap", window))
        .text_size(font)
        .text_color(fg)
        .relative()
        .rounded(rems(theme, "--menu-item-border-radius"))
        .bg(selected_bg)
        .when(interactive, |this| {
            this.role(Role::ListBoxOption)
                .aria_label(name.clone())
                .aria_selected(selected)
        })
        .when(
            interactive && active && state.list_focus.is_focused(window),
            |this| this.aria_active_descendant(),
        )
        .when(enabled, |this| {
            this.cursor_pointer()
                .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                    a11y_owner.update(cx, |state, cx| {
                        state.activate(
                            a11y_activation.clone(),
                            SelectChangeSource::Keyboard,
                            window,
                            cx,
                        )
                    });
                })
                .on_click(move |_, window, cx| {
                    owner.update(cx, |state, cx| {
                        state.activate(activation.clone(), SelectChangeSource::Pointer, window, cx)
                    })
                })
                .on_mouse_move(move |_, _, cx| {
                    mouse_owner.update(cx, |state, cx| {
                        if state.active.as_ref() != Some(&moving) || state.keyboard_navigation {
                            state.active = Some(moving.clone());
                            state.keyboard_navigation = false;
                            cx.notify();
                        }
                    })
                })
        })
        .on_hover(move |hover, _, cx| {
            hover_owner.update(cx, |state, cx| {
                if *hover {
                    if state.hovered.as_ref() != Some(&hovered) {
                        state.hovered = Some(hovered.clone());
                        cx.notify();
                    }
                } else if state.hovered.as_ref() == Some(&hovered) {
                    state.hovered = None;
                    cx.notify();
                }
            })
        });
    shell = shell.child(
        div()
            .absolute()
            .inset_0()
            .rounded(rems(theme, "--menu-item-border-radius"))
            .bg(hover),
    );
    if let Row::Option(option) = row {
        if let Some(renderer) = &p.option_renderer {
            // The slot owns vertical alignment. A custom Tag can keep its
            // horizontal self_start without overriding the row's centering.
            shell = shell.child(
                div()
                    .min_w_0()
                    .flex_1()
                    .h_full()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .child(renderer(
                        &SelectOptionContext {
                            option: option.clone(),
                            selected,
                            active,
                            disabled: !enabled,
                            query: state.query(cx),
                            size: p.size,
                        },
                        window,
                        cx,
                    )),
            );
        } else {
            shell = shell.child(div().size(icon).flex_shrink_0().when_some(
                option.icon,
                |this, image| {
                    this.child(
                        image
                            .size(gpui_kit::rems(
                                f32::from(icon) / f32::from(window.rem_size()),
                            ))
                            .color_value(fg),
                    )
                },
            ));
            let mut text = div().min_w_0().flex_1().flex().flex_col().child(
                YueOverflowTooltip::new((id.clone(), "label"), theme, option.label.clone())
                    .width(gpui_kit::relative(1.)),
            );
            if p.row_layout == SelectRowLayout::Description
                && let Some(description) = &option.description
            {
                text = text.child(
                    div()
                        .truncate()
                        .text_sm()
                        .text_color(rgba(theme, "--text-secondary"))
                        .child(description.clone()),
                );
            }
            shell = shell.child(text);
            if let Some(meta) = &option.meta {
                shell = shell.child(div().text_sm().flex_shrink_0().child(meta.clone()));
            }
        }
    } else {
        shell = shell.child(div().flex_1().min_w_0().truncate().child(name));
    }
    shell
        .child(div().size(icon).flex_shrink_0().when(selected, |this| {
            this.child(
                Icon::new(outline::Check)
                    .size(gpui_kit::rems(
                        f32::from(icon) / f32::from(window.rem_size()),
                    ))
                    .color_value(fg),
            )
        }))
        .a11y_disabled(!enabled)
        .into_any_element()
}

// Measure the actual slot after layout. The retained measurement updates the
// list budget on the next frame; custom content is capped to keep results usable.
fn panel_slot(
    content: AnyElement,
    slot: usize,
    owner: &Entity<YueSelectState>,
    id: &ElementId,
    maximum: Pixels,
) -> AnyElement {
    let owner = owner.downgrade();
    div()
        .id((id.clone(), format!("slot:{slot}")))
        .relative()
        .w_full()
        .flex_shrink_0()
        .max_h(maximum)
        .overflow_y_scroll()
        .child(content)
        .child(
            canvas(
                move |bounds, window, cx| {
                    let height = bounds.size.height;
                    let _ = owner.update(cx, |state, _| {
                        if state.slot_heights[slot] != height {
                            state.slot_heights[slot] = height;
                            let owner = owner.clone();
                            window.on_next_frame(move |_, cx| {
                                let _ = owner.update(cx, |_, cx| cx.notify());
                            });
                        }
                    });
                },
                |_, _, _, _| {},
            )
            .absolute()
            .inset_0(),
        )
        .into_any_element()
}
