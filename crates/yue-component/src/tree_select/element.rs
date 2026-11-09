use super::behavior::{Dismiss, Leave, LeaveBack, QueryAccept, QueryNext, QueryPrevious};
use super::*;
use crate::{
    Icon, TagSize, YueTag, YueTree,
    choice_field::{measure, metric, visible_tags},
    outline,
    popup::{
        PopupDismissReason, PopupDismissRoute, PopupHost, PopupId, PopupPlacement,
        PopupPointerMode, PopupSpec,
    },
    tokens::{rems, rgba, shadow},
};
use gpui_kit::base::{Align, Button, Placement, Select, StyledExt as _, TestSupportExt as _};
use gpui_kit::{
    Context, InteractiveElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, canvas, div, px,
};

fn tag_size(size: TreeSize) -> TagSize {
    match size {
        TreeSize::Small => TagSize::Small,
        TreeSize::Medium => TagSize::Medium,
        TreeSize::Large => TagSize::Large,
    }
}
fn label(item: &TreeSelectItemSnapshot, display: TreeSelectValueDisplay) -> SharedString {
    if display == TreeSelectValueDisplay::Path && !item.path.is_empty() {
        let mut path = item.path.iter().map(|s| s.as_str()).collect::<Vec<_>>();
        path.push(item.label.as_str());
        path.join(" / ").into()
    } else {
        item.label.clone()
    }
}
pub(super) fn render(
    s: &mut YueTreeSelectState,
    w: &mut Window,
    cx: &mut Context<YueTreeSelectState>,
) -> AnyElement {
    let Some(p) = s.presentation.clone() else {
        return div().into_any_element();
    };
    assert!(
        !p.name.trim().is_empty(),
        "TreeSelect requires accessibility_label or YueField label"
    );
    let theme = p.theme.unwrap_or_else(|| crate::current_theme(cx));
    let id = p.id.clone();
    let owner = cx.entity();
    let h = metric(theme, &format!("--input-height-{}", p.size.suffix()), w);
    let padding = metric(
        theme,
        &format!("--input-padding-inline-{}", p.size.suffix()),
        w,
    );
    let font = metric(theme, &format!("--input-font-size-{}", p.size.suffix()), w);
    let icon = metric(theme, &format!("--input-icon-size-{}", p.size.suffix()), w);
    let gap = metric(theme, "--input-gap", w);
    let width = p
        .width
        .unwrap_or_else(|| metric(theme, "--select-width", w));
    let measured = if s.measured_width > px(0.) {
        s.measured_width
    } else {
        width
    };
    let keys = if s.checked() {
        s.tree.read(cx).projected_checked(p.projection)
    } else {
        s.value(cx).keys().to_vec()
    };
    let items = keys.iter().map(|k| s.item(k, cx)).collect::<Vec<_>>();
    for key in &keys {
        s.tag_focus
            .entry(key.clone())
            .or_insert_with(|| cx.focus_handle());
    }
    s.visible_tags.clear();
    let value_label: SharedString = items
        .iter()
        .map(|i| label(i, p.display).to_string())
        .collect::<Vec<_>>()
        .join("、")
        .into();
    let editable = s.editable();
    let focused = s.focus.contains_focused(w, cx);
    let mut values = div()
        .flex()
        .items_center()
        .flex_1()
        .min_w_0()
        .gap(rems(theme, "--select-tag-gap"));
    let mut hidden = vec![];
    if items.is_empty() {
        values = values.child(
            div()
                .text_color(rgba(theme, "--input-placeholder-color"))
                .child(p.placeholder.clone()),
        );
    } else if let Some(render) = &p.value {
        values = values.child(render(&items, w, cx));
    } else if s.mode == TreeSelectMode::Single {
        values = values.child(
            div()
                .min_w_0()
                .overflow_hidden()
                .text_ellipsis()
                .child(value_label.clone()),
        );
    } else {
        let tag_font = metric(theme, &format!("--tag-font-size-{}", p.size.suffix()), w);
        let tag_padding = metric(
            theme,
            &format!("--tag-padding-inline-{}", p.size.suffix()),
            w,
        );
        let tag_gap = metric(theme, "--select-tag-gap", w);
        let budget = (measured
            - 2. * padding
            - 2. * icon
            - 2. * gap
            - p.prefix
                .as_ref()
                .map_or(px(0.), |x| measure(x, font, w) + gap))
        .max(px(0.));
        let widths = items
            .iter()
            .map(|item| {
                measure(&label(item, p.display), tag_font, w) + 2. * tag_padding + icon + tag_gap
            })
            .collect::<Vec<_>>();
        let show = if p.tags == SelectTagLayout::Wrap {
            items.len()
        } else {
            visible_tags(
                &widths,
                if p.tags == SelectTagLayout::Collapse {
                    p.collapse
                } else {
                    items.len()
                },
                budget,
                tag_gap,
                |count| measure(&format!("+{count}").into(), tag_font, w) + 2. * tag_padding,
            )
        };
        if p.tags == SelectTagLayout::Wrap {
            values = values.flex_wrap()
        }
        for (index, item) in items.iter().enumerate() {
            if index >= show {
                hidden.push(item.clone());
                continue;
            }
            s.visible_tags.push(item.key.clone());
            let mut tag = YueTag::new(
                (id.clone(), format!("tag:{}", item.key)),
                label(item, p.display),
            )
            .theme(theme)
            .size(tag_size(p.size))
            .max_width(budget.max(px(1.)))
            .overflow_tooltip(true)
            .disabled(s.disabled);
            if !item.resolved {
                tag = tag.accessibility_description("待解析，当前结果中没有此项");
            }
            if let Some(focus) = s.tag_focus.get(&item.key) {
                tag = tag.track_close_focus(focus);
            }
            if editable && !item.locked {
                let owner = owner.clone();
                let key = item.key.clone();
                tag = tag
                    .close_label(format!("移除 {}", item.label))
                    .on_close(move |_, w, cx| {
                        cx.stop_propagation();
                        owner.update(cx, |s, cx| s.remove(Some(key.clone()), w, cx))
                    });
            }
            if let Some(render) = &p.tag {
                tag = render(tag, item, w, cx);
            }
            values = values.child(tag);
        }
        if !hidden.is_empty() {
            let owner = owner.clone();
            values = values.child(
                Button::new((id.clone(), "more"))
                    .accessibility_label(format!("查看其余 {} 个选择", hidden.len()))
                    .disabled(!editable)
                    .track_focus(&s.collapsed_focus)
                    .px(tag_padding)
                    .child(format!("+{}", hidden.len()))
                    .on_click(move |_, w, cx| {
                        cx.stop_propagation();
                        owner.update(cx, |s, cx| {
                            let next = !s.collapsed;
                            if s.open {
                                s.close(false, w, cx)
                            }
                            s.collapsed = next;
                            if next {
                                if let Some(focus) = s
                                    .hidden_tags
                                    .iter()
                                    .find(|k| !s.item(k, cx).locked)
                                    .and_then(|k| s.tag_focus.get(k))
                                {
                                    focus.focus(w, cx)
                                } else {
                                    s.collapsed_focus.focus(w, cx)
                                }
                            }
                            cx.notify();
                        });
                    }),
            );
        }
    }
    s.hidden_tags = hidden.iter().map(|i| i.key.clone()).collect();
    if hidden.is_empty() {
        s.collapsed = false;
    }
    let activation_owner = owner.clone();
    let mut activation = div()
        .id((id.clone(), "trigger"))
        .test_support()
        .flex()
        .items_center()
        .flex_1()
        .min_w_0()
        .gap(gap)
        .on_click(move |_, w, cx| {
            activation_owner.update(cx, |s, cx| {
                let open = !s.open;
                if s.change_open(open, true, w, cx) {
                    s.content_focus(cx).focus(w, cx)
                } else {
                    s.focus.focus(w, cx)
                }
            })
        });
    if let Some(prefix) = &p.prefix_icon {
        activation = activation.child(prefix.size(rems(
            theme,
            &format!("--input-icon-size-{}", p.size.suffix()),
        )));
    }
    if let Some(prefix) = &p.prefix {
        activation = activation.child(div().flex_shrink_0().child(prefix.clone()))
    }
    activation = activation.child(values);
    let mut frame = div()
        .id((id.clone(), "frame"))
        .test_support()
        .refine_style(&crate::choice_field::frame_geometry(h, padding, gap, font))
        .refine_style(&crate::choice_field::surface(
            theme,
            p.status,
            s.disabled,
            s.readonly,
            p.variant == SelectVariant::Outline,
            focused || s.open,
        ))
        .text_color(rgba(
            theme,
            if s.disabled {
                "--input-color-disabled"
            } else {
                "--input-color"
            },
        ))
        .min_w(icon * 3. + padding * 2.)
        .child(activation);
    if p.tags == SelectTagLayout::Wrap && s.mode != TreeSelectMode::Single {
        frame = frame.items_start().py((h - metric(
            theme,
            &format!("--tag-height-{}", p.size.suffix()),
            w,
        )) / 2.)
    } else {
        frame = frame.h(h)
    }
    if p.clearable && items.iter().any(|i| !i.locked) {
        let owner = owner.clone();
        frame = frame.child(
            Button::new((id.clone(), "clear"))
                .accessibility_label("清除可移除的选择")
                .disabled(!editable)
                .size(icon)
                .on_click(move |_, w, cx| {
                    cx.stop_propagation();
                    owner.update(cx, |s, cx| s.remove(None, w, cx))
                })
                .child(Icon::new(outline::X).size(rems(
                    theme,
                    &format!("--input-icon-size-{}", p.size.suffix()),
                ))),
        );
    }
    let arrow_owner = owner.clone();
    frame = frame.child(
        Button::new((id.clone(), "arrow"))
            .tab_stop(false)
            .disabled(!editable)
            .size(icon)
            .accessibility_label(if s.open {
                "收起树选项"
            } else {
                "展开树选项"
            })
            .on_click(move |_, w, cx| {
                cx.stop_propagation();
                arrow_owner.update(cx, |s, cx| {
                    let open = !s.open;
                    if s.change_open(open, true, w, cx) {
                        s.content_focus(cx).focus(w, cx)
                    } else {
                        s.focus.focus(w, cx)
                    }
                })
            })
            .child(if s.loading || s.result == TreeSelectResultState::Loading {
                crate::YueSpinner::new((id.clone(), "loading"))
                    .active(true)
                    .label("正在加载选项")
                    .into_any_element()
            } else {
                Icon::new(outline::ChevronDown)
                    .size(rems(
                        theme,
                        &format!("--input-icon-size-{}", p.size.suffix()),
                    ))
                    .into_any_element()
            }),
    );
    if focused && !s.disabled {
        frame = frame.child(
            div()
                .absolute()
                .inset(
                    -metric(theme, "--input-focus-ring-offset", w)
                        - metric(theme, "--input-focus-ring-width", w),
                )
                .border(rems(theme, "--input-focus-ring-width"))
                .border_color(rgba(theme, "--input-focus-ring-color"))
                .rounded(rems(theme, "--input-border-radius")),
        );
    }
    let trigger = owner.clone();
    let confirm = owner.clone();
    let focus = s.focus.clone();
    let content_focus = s.content_focus(cx);
    let mut semantic = Select::new(id.clone())
        .key_context("YueTreeSelect")
        .popup_kind(gpui_kit::accesskit::HasPopup::Tree)
        .open(s.open)
        .disabled(s.disabled)
        .readonly(s.readonly)
        .required(p.required)
        .invalid(p.status == InputStatus::Error)
        .focus_handle(&focus)
        .content_focus_handle(&content_focus)
        .accessibility_label(p.name.clone())
        .accessibility_value(value_label)
        .on_open_request(move |open, w, cx| {
            trigger.update(cx, |s, cx| {
                if s.composing(w, cx) {
                    return s.open;
                }
                s.change_open(open, true, w, cx)
            })
        })
        .on_confirm(move |w, cx| {
            confirm.update(cx, |s, cx| {
                if s.composing(w, cx) {
                    return;
                }
                let key = s.tree.read(cx).active_key().cloned();
                if let Some(key) = key {
                    s.tree.update(cx, |tree, cx| {
                        tree.activate(
                            key,
                            crate::TreeChangeSource::Keyboard,
                            Default::default(),
                            true,
                            cx,
                        )
                    });
                }
                let _ = w;
            })
        })
        .child(frame)
        .w_full();
    if let Some(description) = &p.description {
        semantic = semantic.accessibility_description(description.clone())
    }
    if let Some(popup) = popup(s, &p, theme, false, hidden.clone(), w, cx) {
        semantic = semantic.child(popup)
    }
    if let Some(popup) = popup(s, &p, theme, true, hidden, w, cx) {
        semantic = semantic.child(popup)
    }
    let weak = owner.downgrade();
    let measurement = canvas(
        move |bounds, window, cx| {
            let mounted = weak.clone();
            let handle = window.window_handle();
            let _mount = window.use_keyed_state("tree-select-mount-session", cx, |_, cx| {
                cx.on_release(move |_, cx| {
                    let _ = handle.update(cx, |_, window, cx| {
                        let _ = mounted.update(cx, |s, cx| s.close_silent(window, cx));
                    });
                })
                .detach();
            });
            let width = bounds.size.width;
            let _ = weak.update(cx, |s, _| {
                if s.measured_width != width {
                    s.measured_width = width;
                    let weak = weak.clone();
                    window.on_next_frame(move |_, cx| {
                        let _ = weak.update(cx, |_, cx| cx.notify());
                    });
                }
            });
        },
        |_, _, _, _| {},
    )
    .absolute()
    .inset_0();
    let dismiss_owner = owner.clone();
    let mut root = div()
        .on_action(move |_: &Dismiss, w, cx| dismiss_owner.update(cx, |s, cx| s.dismiss(w, cx)))
        .id((id, "owner"))
        .relative()
        .flex()
        .flex_col()
        .min_w_0()
        .text_left()
        .capture_key_down(cx.listener(|s, e: &gpui_kit::KeyDownEvent, w, cx| {
            if matches!(e.keystroke.key.as_str(), "enter" | "escape") && s.composing(w, cx) {
                cx.stop_propagation();
            }
        }))
        .on_key_down(cx.listener(|s, e: &gpui_kit::KeyDownEvent, w, cx| {
            if s.composing(w, cx) {
                return;
            }
            let key = e.keystroke.key.as_str();
            if !s.open
                && !s.collapsed
                && s.focus.is_focused(w)
                && matches!(key, "enter" | "space" | "down" | "up")
            {
                if s.change_open(true, true, w, cx) {
                    s.content_focus(cx).focus(w, cx);
                }
                cx.stop_propagation();
                return;
            }
            if matches!(key, "backspace" | "delete")
                && !s.open
                && s.editable()
                && s.mode != TreeSelectMode::Single
            {
                let focused = s
                    .tag_focus
                    .iter()
                    .find(|(_, h)| h.is_focused(w))
                    .map(|(k, _)| k.clone());
                if let Some(key) = focused {
                    s.remove(Some(key), w, cx);
                    cx.stop_propagation();
                    return;
                }
                if key == "backspace"
                    && s.focus.is_focused(w)
                    && let Some(focus) = s
                        .visible_tags
                        .iter()
                        .rev()
                        .find(|k| !s.item(k, cx).locked)
                        .and_then(|k| s.tag_focus.get(k))
                {
                    focus.focus(w, cx);
                    cx.stop_propagation();
                    return;
                }
            }
            if key == "escape" && s.collapsed {
                s.collapsed = false;
                s.collapsed_focus.focus(w, cx);
                cx.notify();
                cx.stop_propagation();
            } else if key == "tab" && s.collapsed {
                s.leave(e.keystroke.modifiers.shift, w, cx);
                cx.stop_propagation();
            } else if key == "escape" && s.open {
                s.close(true, w, cx);
                cx.stop_propagation();
            } else if key == "tab" && s.open {
                s.leave(e.keystroke.modifiers.shift, w, cx);
                cx.stop_propagation();
            } else if matches!(key, "down" | "enter")
                && s.open
                && s.search != TreeSelectSearch::None
                && s.content_focus(cx).is_focused(w)
                && s.result == TreeSelectResultState::Ready
            {
                let focus = s.tree.read(cx).focus.clone();
                focus.focus(w, cx);
                cx.stop_propagation();
            } else if key == "f"
                && e.keystroke.modifiers.control
                && s.open
                && s.search != TreeSelectSearch::None
            {
                s.content_focus(cx).focus(w, cx);
                cx.stop_propagation();
            }
        }))
        .child(semantic)
        .child(s.anchor.capture())
        .child(measurement);
    root = if p.block {
        root.w_full()
    } else {
        root.w(width)
    };
    root.into_any_element()
}

fn popup(
    s: &mut YueTreeSelectState,
    p: &Presentation,
    theme: YueTheme,
    collapsed: bool,
    hidden: Vec<TreeSelectItemSnapshot>,
    w: &mut Window,
    cx: &mut Context<YueTreeSelectState>,
) -> Option<AnyElement> {
    let present = if collapsed { s.collapsed } else { s.open };
    let owner = cx.entity();
    let id = p.id.clone();
    let gap = metric(theme, "--select-panel-gap", w);
    let margin = metric(theme, "--space-8", w);
    let available = PopupHost::available_size(None, Default::default(), margin, w);
    let trigger_width = if s.measured_width > px(0.) {
        s.measured_width
    } else {
        p.width
            .unwrap_or_else(|| metric(theme, "--select-width", w))
    };
    let width = match p.panel_width {
        SelectPanelWidth::Fixed(width) => width,
        SelectPanelWidth::MatchTrigger => trigger_width,
        SelectPanelWidth::Content => {
            let font = metric(theme, &format!("--input-font-size-{}", p.size.suffix()), w);
            let indent = metric(theme, &format!("--tree-indent-{}", p.size.suffix()), w);
            let face = format!("{:?}", w.text_style().font());
            let tree = s.tree.read(cx);
            let revision = tree.model.revision();
            let measured = if let Some((_, _, _, _, width)) =
                s.content_width_cache.as_ref().filter(|(r, f, i, n, _)| {
                    *r == revision && *f == font && *i == indent && *n == face
                }) {
                *width
            } else {
                let width = tree
                    .model
                    .rows()
                    .iter()
                    .map(|row| {
                        tree.node(row.key()).map_or(px(0.), |n| {
                            measure(n.label(), font, w)
                                + indent * (row.depth().min(12) as f32)
                                + 4. * font
                        })
                    })
                    .fold(px(0.), gpui_kit::Pixels::max);
                s.content_width_cache = Some((revision, font, indent, face, width));
                width
            };
            measured.max(trigger_width)
        }
    }
    .min(available.width)
    .max(px(1.));
    let side = s.anchor.handle().bounds().map_or(available.height, |b| {
        (w.viewport_size().height - b.bottom() - gap - margin).max(b.top() - gap - margin)
    });
    let maximum = available.height.min(side).max(px(1.));
    let dismiss = owner.clone();
    let mut spec = PopupSpec::new(
        PopupId::new(
            cx.entity_id(),
            (
                id.clone(),
                if collapsed { "tree-tags" } else { "tree-popup" },
            ),
        ),
        s.anchor.handle(),
        present,
        PopupPlacement::Side(Placement::Bottom, Align::Start),
    )
    .surface_gap(gap)
    .margin(margin)
    .pointer_mode(PopupPointerMode::Block)
    .dismiss_route(PopupDismissRoute::Runtime)
    .dismiss_on(false, true)
    .dismiss_unplaceable()
    .exclude(s.anchor.handle())
    .motion(crate::motion::dropdown_popup(theme))
    .on_dismiss(move |request, w, cx| {
        dismiss.update(cx, |s, cx| {
            if collapsed {
                s.collapsed = false;
                if request.reason() == PopupDismissReason::Escape {
                    s.collapsed_focus.focus(w, cx);
                }
                cx.notify();
            } else {
                s.close(request.reason() == PopupDismissReason::Escape, w, cx)
            }
        })
    });
    if let Some(parent) = &s.parent {
        spec = spec.parent(parent.clone())
    }
    let tree = s.tree.clone();
    let result = s.result.clone();
    let slot_heights = s.slot_heights;
    let tag_focus = s.tag_focus.clone();
    let search = s.search;
    let editable = s.editable();
    let open = s.open;
    let p = p.clone();
    let padding = metric(theme, "--popover-padding", w);
    let row_h = metric(theme, &format!("--tree-row-height-{}", p.size.suffix()), w);
    let shadows = shadow(theme, "--popover-shadow");
    PopupHost::new(spec)
        .mount(
            move |presentation, _, window, cx| {
                let interactive = presentation.is_interactive() && editable && open;
                let keyboard_owner = owner.clone();
                let next_owner = owner.clone();
                let previous_owner = owner.clone();
                let accept_owner = owner.clone();
                let leave_owner = owner.clone();
                let back_owner = owner.clone();
                let dismiss_owner = owner.clone();
                let mut panel = div()
                    .key_context("YueTreeSelectPanel")
                    .on_action(move |_: &QueryNext, w, cx| {
                        next_owner.update(cx, |s, cx| s.enter_results(w, cx))
                    })
                    .on_action(move |_: &QueryPrevious, w, cx| {
                        previous_owner.update(cx, |s, cx| s.enter_results(w, cx))
                    })
                    .on_action(move |_: &QueryAccept, w, cx| {
                        accept_owner.update(cx, |s, cx| s.enter_results(w, cx))
                    })
                    .on_action(move |_: &Leave, w, cx| {
                        leave_owner.update(cx, |s, cx| s.leave(false, w, cx))
                    })
                    .on_action(move |_: &LeaveBack, w, cx| {
                        back_owner.update(cx, |s, cx| s.leave(true, w, cx))
                    })
                    .on_action(move |_: &Dismiss, w, cx| {
                        dismiss_owner.update(cx, |s, cx| s.dismiss(w, cx))
                    })
                    .capture_key_down(move |e, w, cx| {
                        keyboard_owner.update(cx, |s, cx| {
                            if matches!(e.keystroke.key.as_str(), "enter" | "escape")
                                && s.composing(w, cx)
                            {
                                cx.stop_propagation();
                            }
                        });
                    })
                    .id((
                        id.clone(),
                        if collapsed { "hidden-values" } else { "panel" },
                    ))
                    .test_support()
                    .w(width)
                    .max_h(maximum)
                    .flex()
                    .flex_col()
                    .min_h_0()
                    .gap(metric(theme, "--space-4", window))
                    .p(padding)
                    .bg(rgba(theme, "--popover-background"))
                    .border(rems(theme, "--popover-border-width"))
                    .border_color(rgba(theme, "--popover-border-color"))
                    .rounded(rems(theme, "--popover-border-radius"))
                    .shadow(shadows.clone());
                if !presentation.is_interactive() {
                    panel = panel.occlude()
                }
                if collapsed {
                    let mut tags = div()
                        .id((id.clone(), "hidden-scroll"))
                        .max_h((maximum - 2. * padding).max(px(1.)))
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .gap(rems(theme, "--select-tag-gap"));
                    for item in &hidden {
                        let mut tag = YueTag::new(
                            (id.clone(), format!("hidden:{}", item.key)),
                            label(item, p.display),
                        )
                        .theme(theme)
                        .size(tag_size(p.size));
                        if let Some(focus) = tag_focus.get(&item.key) {
                            tag = tag.track_close_focus(focus);
                        }
                        if editable && !item.locked && presentation.is_interactive() {
                            let owner = owner.clone();
                            let key = item.key.clone();
                            tag = tag.close_label(format!("移除 {}", item.label)).on_close(
                                move |_, w, cx| {
                                    owner.update(cx, |s, cx| s.remove(Some(key.clone()), w, cx))
                                },
                            );
                        }
                        if let Some(render) = &p.tag {
                            tag = render(tag, item, window, cx);
                        }
                        tags = tags.child(tag)
                    }
                    return panel.child(tags).into_any_element();
                }
                if search != TreeSelectSearch::None && presentation.is_interactive() {
                    let input = tree.read(cx).input.clone();
                    let query_owner = owner.clone();
                    panel = panel.child(
                        div()
                            .key_context("YueTreeSelectQuery")
                            .on_key_down(move |e, w, cx| {
                                query_owner.update(cx, |s, cx| {
                                    if !s.composing(w, cx)
                                        && s.result == TreeSelectResultState::Ready
                                        && matches!(e.keystroke.key.as_str(), "down" | "enter")
                                    {
                                        let f = s.tree.read(cx).focus.clone();
                                        f.focus(w, cx);
                                        cx.stop_propagation();
                                    }
                                })
                            })
                            .child(
                                crate::YueInput::new((id.clone(), "query"), theme, &input)
                                    .size(p.size.input())
                                    .input_type(crate::InputType::Search)
                                    .placeholder("搜索节点")
                                    .accessibility_label("搜索树选项")
                                    .clearable(true)
                                    .block(),
                            ),
                    );
                }
                if let Some(header) = &p.header {
                    panel = panel.child(panel_slot(
                        header(interactive, window, cx),
                        0,
                        &owner,
                        &id,
                        maximum / 4.,
                    ))
                }
                let reserved = 2. * padding
                    + if search != TreeSelectSearch::None {
                        row_h + gap
                    } else {
                        px(0.)
                    }
                    + if p.header.is_some() {
                        slot_heights[0].max(row_h).min(maximum / 4.) + gap
                    } else {
                        px(0.)
                    }
                    + if p.footer.is_some() {
                        slot_heights[1].max(row_h).min(maximum / 4.) + gap
                    } else {
                        px(0.)
                    };
                let height = p
                    .height
                    .unwrap_or_else(|| metric(theme, "--select-list-max-height", window))
                    .min((maximum - reserved).max(px(1.)));
                tree.update(cx, |s, _| {
                    s.interactive = interactive && result == TreeSelectResultState::Ready
                });
                panel = match &result {
                    TreeSelectResultState::Ready => {
                        let mut widget = YueTree::new((id.clone(), "tree"), &tree)
                            .accessibility_label(format!("{}选项", p.name))
                            .theme(theme)
                            .size(p.size)
                            .height(height)
                            .tab_stop(false)
                            .show_lines(p.lines)
                            .show_icons(p.icons);
                        if let Some(render) = &p.label {
                            let render = render.clone();
                            widget = widget.render_label(move |n, w, cx| render(n, w, cx));
                        }
                        if let Some(render) = &p.extra {
                            let render = render.clone();
                            widget = widget.render_extra(move |n, w, cx| render(n, w, cx));
                        }
                        if let Some(render) = &p.empty {
                            let render = render.clone();
                            widget = widget.empty(move |w, cx| render(w, cx));
                        }
                        panel.child(widget)
                    }
                    TreeSelectResultState::Loading => panel.child(
                        crate::YueSpinner::new((id.clone(), "search-loading"))
                            .active(true)
                            .label("正在搜索节点"),
                    ),
                    TreeSelectResultState::Failed(error) => {
                        let owner = owner.clone();
                        panel.child(
                            Button::new((id.clone(), "search-retry"))
                                .disabled(!interactive)
                                .accessibility_label("重新搜索节点")
                                .child(format!("{error} · 重试"))
                                .on_click(move |_, _, cx| {
                                    owner.update(cx, |s, cx| s.retry_search(cx))
                                }),
                        )
                    }
                };
                if let Some(footer) = &p.footer {
                    panel = panel.child(panel_slot(
                        footer(interactive, window, cx),
                        1,
                        &owner,
                        &id,
                        maximum / 4.,
                    ))
                }
                panel.into_any_element()
            },
            w,
            cx,
        )
        .map(IntoElement::into_any_element)
}

fn panel_slot(
    content: AnyElement,
    slot: usize,
    owner: &Entity<YueTreeSelectState>,
    id: &ElementId,
    maximum: gpui_kit::Pixels,
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
                    let _ = owner.update(cx, |s, _| {
                        if s.slot_heights[slot] != bounds.size.height {
                            s.slot_heights[slot] = bounds.size.height;
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
