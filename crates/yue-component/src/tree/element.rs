use super::*;
use crate::{
    EmptySize, InputType, YueEmpty, YueInput, YueVirtualList, outline,
    tokens::{rems, rgba},
};
use gpui_kit::base::{
    A11yDisabledExt as _, A11ySelectionExt as _, Button, SelectionA11y, TestSupportExt as _,
    keyed_tree::tree_row,
};
use gpui_kit::{
    AccessibleAction, Context, Focusable as _, InteractiveElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, canvas, div, prelude::FluentBuilder as _,
};

pub(crate) fn row_height(p: &Presentation, w: &Window, cx: &App) -> gpui_kit::Pixels {
    crate::choice_field::metric(
        p.theme.unwrap_or_else(|| crate::current_theme(cx)),
        &format!("--tree-row-height-{}", p.size.suffix()),
        w,
    )
}
pub(super) fn render(
    state: &mut YueTreeState,
    window: &mut Window,
    cx: &mut Context<YueTreeState>,
) -> AnyElement {
    let Some(p) = state.presentation.clone() else {
        return div().into_any_element();
    };
    let theme = p.theme.unwrap_or_else(|| crate::current_theme(cx));
    let signature = format!(
        "{:?}/{:?}/{:?}/{:?}/{:?}/{}",
        theme.mode(),
        theme.accent(),
        theme.neutral(),
        window.rem_size(),
        state.viewport_width,
        window.scale_factor()
    );
    if state.motion.geometry_key != signature {
        state.motion.geometry_key = signature;
        state.motion.requested = false;
        state.geometry = None;
    }
    state.repair_action_focus(window, cx);
    state.layout_row_height = row_height(&p, window, cx);
    if !window.is_window_active() || !window.is_visible() {
        state.motion.requested = false;
        if state.motion.running() {
            state.geometry = None;
        }
    }
    state.sync_geometry(cx);
    let owner = cx.entity();
    let render_owner = owner.clone();
    let rp = p.clone();
    let height = p
        .height
        .unwrap_or_else(|| crate::choice_field::metric(theme, "--tree-max-height", window));
    state.page_rows = (f32::from(height) / f32::from(state.layout_row_height))
        .floor()
        .max(1.) as isize;
    let mut root = div()
        .id(p.id.clone())
        .test_support()
        .flex()
        .flex_col()
        .w_full()
        .min_w_0()
        .gap(rems(theme, "--tree-gap"));
    if p.search {
        let input = state.input.clone();
        let owner = owner.clone();
        root = root.child(
            div()
                .on_key_down(move |e, window, cx| {
                    if matches!(e.keystroke.key.as_str(), "down" | "enter") {
                        owner.update(cx, |s, cx| {
                            if !s.disabled && !s.composing(window, cx) {
                                s.focus(window, cx);
                                cx.stop_propagation();
                            }
                        })
                    }
                })
                .child(
                    YueInput::new((p.id.clone(), "query"), theme, &input)
                        .input_type(InputType::Search)
                        .placeholder("搜索已加载节点")
                        .accessibility_label("搜索树节点")
                        .size(p.size.input())
                        .block(),
                ),
        );
    }
    let focus = state.focus.clone().tab_stop(p.tab_stop && !state.disabled);
    let tree = div()
        .id((p.id.clone(), "tree"))
        .test_support()
        .role(gpui_kit::Role::Tree)
        .aria_label(p.name.clone())
        .track_focus(&focus)
        .h(height)
        .min_h_0()
        .w_full()
        .key_context("YueTree")
        .on_action(cx.listener(|s, _: &EnterActions, w, cx| s.enter_actions(w, cx)))
        .on_action(cx.listener(|s, _: &ActionsNext, w, cx| s.leave_actions(false, false, w, cx)))
        .on_action(cx.listener(|s, _: &ActionsPrevious, w, cx| s.leave_actions(true, false, w, cx)))
        .on_action(cx.listener(|s, _: &ExitActions, w, cx| s.leave_actions(false, true, w, cx)))
        .on_key_down(
            cx.listener(|state, e: &gpui_kit::KeyDownEvent, window, cx| {
                if state.disabled || !state.interactive {
                    return;
                }
                let key = e.keystroke.key.as_str();
                let modifiers = e.keystroke.modifiers;
                let movement = match key {
                    "up" => Some(TreeMove::Previous),
                    "down" => Some(TreeMove::Next),
                    "left" => Some(TreeMove::Parent),
                    "right" => Some(TreeMove::Child),
                    "home" => Some(TreeMove::First),
                    "end" => Some(TreeMove::Last),
                    "pageup" => Some(TreeMove::Page(-state.page_rows)),
                    "pagedown" => Some(TreeMove::Page(state.page_rows)),
                    _ => None,
                };
                if let Some(movement) = movement {
                    let before = state.selected_keys().to_vec();
                    if modifiers.shift {
                        state.model.begin_range_selection();
                    }
                    state.navigate(movement, cx);
                    if modifiers.shift
                        && matches!(movement, TreeMove::Previous | TreeMove::Next)
                        && state.model.selection_mode() == TreeSelectionMode::Multiple
                        && !state.toggle_selection
                        && let Some(k) = state.active_key().cloned()
                    {
                        state.model.select(
                            &k,
                            gpui_kit::base::keyed_tree::TreeSelectionGesture::default()
                                .range(true)
                                .additive(modifiers.control),
                        );
                        let after = state.selected_keys().to_vec();
                        if before != after {
                            cx.emit(TreeEvent::SelectionChanged(TreeChange {
                                before,
                                after,
                                key: k,
                                source: TreeChangeSource::Keyboard,
                            }));
                        }
                    }
                    cx.stop_propagation();
                    return;
                }
                match key {
                    "enter" | "space" => {
                        if let Some(k) = state.active_key().cloned() {
                            if key == "space" && state.model.check_mode() != TreeCheckMode::None {
                                state.check(k, TreeChangeSource::Keyboard, cx)
                            } else {
                                state.activate(k, TreeChangeSource::Keyboard, modifiers, true, cx)
                            }
                        }
                        cx.stop_propagation();
                    }
                    "a" if modifiers.control
                        && !state.toggle_selection
                        && state.model.selection_mode() == TreeSelectionMode::Multiple =>
                    {
                        let before = state.selected_keys().to_vec();
                        state.model.select_visible();
                        let after = state.selected_keys().to_vec();
                        if before != after {
                            cx.emit(TreeEvent::SelectionChanged(TreeChange {
                                before,
                                after,
                                key: state.active_key().cloned().unwrap_or_default(),
                                source: TreeChangeSource::Keyboard,
                            }));
                        }
                        cx.notify();
                        cx.stop_propagation();
                    }
                    "f" if modifiers.control
                        && state.presentation.as_ref().is_some_and(|p| p.search) =>
                    {
                        state.input.read(cx).focus_handle(cx).focus(window, cx);
                        cx.stop_propagation();
                    }
                    "f10" if modifiers.shift => {
                        if let Some(k) = state.active_key().cloned() {
                            cx.emit(TreeEvent::ContextRequested(k));
                        }
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }),
        );
    let tree = if state.model.rows().is_empty() {
        tree.child(if let Some(render) = &p.empty {
            render(window, cx)
        } else {
            YueEmpty::new((p.id.clone(), "empty"))
                .size(EmptySize::Small)
                .description(if state.query_observed.is_empty() {
                    "暂无节点"
                } else {
                    "未找到匹配节点"
                })
                .inset(false)
                .into_any_element()
        })
    } else {
        tree.child(
            YueVirtualList::new(
                (p.id.clone(), "rows"),
                &state.virtualizer,
                move |item, window, cx| {
                    render_slot(
                        &render_owner,
                        item.index(),
                        item.extent(),
                        &rp,
                        theme,
                        window,
                        cx,
                    )
                },
            )
            .theme(theme)
            .semantics(None, None),
        )
    };
    let measured = owner.downgrade();
    let animating = state.motion.running();
    root = root
        .relative()
        .child(super::motion::CaptureRow {
            child: tree
                .a11y_selection(SelectionA11y::default().disabled(state.disabled).multiple(
                    state.model.selection_mode() == TreeSelectionMode::Multiple
                        || state.model.check_mode() != TreeCheckMode::None,
                ))
                .into_any_element(),
            key: None,
            cache: state.motion.paint.clone(),
        })
        .child(
            canvas(
                move |bounds, window, cx| {
                    if animating && bounds.intersects(&window.content_mask().bounds) {
                        window.request_animation_frame();
                    }
                    let _ = measured.update(cx, |s, _| {
                        if s.viewport_width != bounds.size.width {
                            s.viewport_width = bounds.size.width;
                            let measured = measured.clone();
                            window.on_next_frame(move |_, cx| {
                                let _ = measured.update(cx, |_, cx| cx.notify());
                            });
                        }
                    });
                },
                |_, _, _, _| {},
            )
            .absolute()
            .inset_0(),
        );
    if let Some(TreeLoadState::Loading) = state.model.load_state(None) {
        root = root.child(
            crate::YueSpinner::new((p.id.clone(), "root-loading"))
                .active(true)
                .label("正在加载节点"),
        );
    }
    if let Some(TreeLoadState::Failed(error)) = state.model.load_state(None) {
        let owner = owner.clone();
        root = root.child(
            Button::new((p.id.clone(), "root-retry"))
                .disabled(state.disabled || !state.interactive)
                .child(format!("{error} · 重试"))
                .on_click(move |_, _, cx| owner.update(cx, |s, cx| s.request_load(None, cx))),
        );
    }
    root.into_any_element()
}
fn render_slot(
    owner: &Entity<YueTreeState>,
    index: usize,
    extent: gpui_kit::Pixels,
    p: &Presentation,
    theme: YueTheme,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    if extent <= gpui_kit::px(0.01) {
        return div().h(extent).into_any_element();
    }
    let state = owner.read(cx);
    let Some(row) = state.motion.rows.get(index) else {
        return div().into_any_element();
    };
    let key = row.key().clone();
    let model_index = state
        .model
        .positions()
        .get(&gpui_kit::ElementId::Name(key.clone()))
        .copied();
    let cache = state.motion.paint.clone();
    let ghost = state.motion.ghost(&key);
    let content = if let Some(index) = model_index {
        let child = render_row(owner, index, p, theme, window, cx);
        if cx.reduce_motion() {
            child
        } else {
            super::motion::CaptureRow {
                child,
                key: Some(key),
                cache,
            }
            .into_any_element()
        }
    } else {
        canvas(
            |_, _, _| {},
            move |bounds, _, w, _| {
                if let Some(snapshot) = &ghost {
                    w.paint_snapshot(snapshot, bounds.origin);
                }
            },
        )
        .h(row_height(p, window, cx))
        .w_full()
        .into_any_element()
    };
    div()
        .h(extent)
        .w_full()
        .overflow_hidden()
        .child(content)
        .into_any_element()
}
fn render_row(
    owner: &Entity<YueTreeState>,
    index: usize,
    p: &Presentation,
    theme: YueTheme,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let state = owner.read(cx);
    let rows = state.model.rows();
    let Some(row) = rows.get(index) else {
        return div().into_any_element();
    };
    let Some(node) = state.node(row.key()).cloned() else {
        return div().into_any_element();
    };
    let key = row.key().clone();
    let active = state.active_key() == Some(&key);
    let selected = state.selected_keys().contains(&key);
    let check = state.model.check_mode() != TreeCheckMode::None;
    let mark = state.mark(&key);
    let can_check = state.model.can_check(&key);
    let expanded = state.model.is_expanded(&key);
    let interactive = state.interactive && !state.disabled && !node.is_disabled();
    let h = row_height(p, window, cx);
    let indent =
        crate::choice_field::metric(theme, &format!("--tree-indent-{}", p.size.suffix()), window);
    let icon_size = crate::choice_field::metric(
        theme,
        &format!("--input-icon-size-{}", p.size.suffix()),
        window,
    );
    let font = rems(theme, &format!("--input-font-size-{}", p.size.suffix()));
    let gap = rems(theme, "--tree-gap");
    // Row geometry belongs to the tree, including when a custom action is taller.
    let mut content = div().flex().items_center().gap(gap).w_full().h(h).min_w_0();
    // Clamp deep indentation to leave an actionable title lane in narrow popups.
    let max_indent = (state.viewport_width / 3.).min(indent * 12.);
    let depth_indent = (indent * row.depth() as f32).min(max_indent);
    let mut indentation = div().relative().w(depth_indent).h(h).flex_shrink_0();
    if p.lines {
        let line_width = rems(theme, "--tree-line-width").to_pixels(window.rem_size());
        // Lay out the same column as ancestor arrows, then paint about its
        // center. Snapping both line edges in layout can shift a fractional
        // rem-scaled width off this spine by half a device pixel.
        let arrow_start = gap.to_pixels(window.rem_size());
        for (depth, continuation) in row.continuations().iter().enumerate() {
            let x = indent * depth as f32 + arrow_start;
            if x < depth_indent && *continuation {
                indentation = indentation.child(
                    div()
                        .id((p.id.clone(), format!("line:{key}:{depth}")))
                        .test_support()
                        .absolute()
                        .left(x)
                        .top_0()
                        .h_full()
                        .w(icon_size)
                        .child(
                            canvas(
                                |_, _, _| {},
                                move |mut bounds, _, window, _| {
                                    bounds.origin.x = bounds.center().x - line_width / 2.;
                                    bounds.size.width = line_width;
                                    window.paint_quad(gpui_kit::fill(
                                        bounds,
                                        rgba(theme, "--tree-line-color"),
                                    ));
                                },
                            )
                            .size_full(),
                        ),
                );
            }
        }
    }
    content = content.child(indentation);
    let loading = state.model.load_state(Some(&key)).cloned();
    let has_icons = p.icons || !state.node_icons.is_empty();
    let node_icon = state.node_icons.get(&key).copied();
    let has_focus = state.focus.is_focused(window);
    let mut arrow = Button::new((p.id.clone(), format!("expand:{key}")))
        .relative()
        .tab_stop(false)
        .disabled(!interactive || node.is_leaf())
        .size(icon_size)
        .accessibility_label(if expanded {
            "折叠节点"
        } else {
            "展开节点"
        });
    let expand_owner = owner.clone();
    let expand_key = key.clone();
    arrow = arrow.on_click(move |_, window, cx| {
        cx.stop_propagation();
        expand_owner.update(cx, |s, cx| {
            s.focus(window, cx);
            s.expand(expand_key.clone(), cx)
        })
    });
    if !node.is_leaf() {
        let enabled = !cx.reduce_motion() && window.is_visible() && window.is_window_active();
        let now = cx.background_executor().now();
        let rotation = gpui_kit::base::transition_at(
            (p.id.clone(), format!("rotation:{key}")),
            if expanded { 0.25_f32 } else { 0. },
            if enabled {
                crate::motion::transition(
                    theme,
                    crate::MotionDuration::Interaction,
                    crate::MotionEase::Standard,
                )
            } else {
                gpui_kit::base::Transition::new(std::time::Duration::ZERO)
            },
            now,
            false,
            window,
            cx,
        );
        let rotating = enabled
            && matches!(
                rotation.status,
                gpui_kit::base::MotionStatus::Running | gpui_kit::base::MotionStatus::Delayed
            );
        arrow = arrow
            .child(
                Icon::new(outline::ChevronRight)
                    .rotation(rotation.value)
                    .color_value(rgba(theme, "--tree-color"))
                    .size(rems(
                        theme,
                        &format!("--input-icon-size-{}", p.size.suffix()),
                    )),
            )
            .child(
                canvas(
                    move |bounds, w, _| {
                        if rotating && bounds.intersects(&w.content_mask().bounds) {
                            w.request_animation_frame();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            );
    }
    content = content.child(if matches!(loading, Some(TreeLoadState::Loading)) {
        crate::YueSpinner::new((p.id.clone(), format!("loading:{key}")))
            .active(true)
            .label("正在加载子节点")
            .into_any_element()
    } else {
        arrow.into_any_element()
    });
    if check {
        let check_owner = owner.clone();
        let check_key = key.clone();
        let disabled = !interactive || !can_check;
        let mut slot = div()
            .id((p.id.clone(), format!("check:{key}")))
            .test_support()
            .w(icon_size)
            .h(icon_size)
            .flex_shrink_0();
        if node.is_checkable() {
            slot = slot.child(crate::checkbox::embedded_mark(
                theme, p.size, mark, disabled,
            ));
            if !disabled {
                slot = slot.on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    check_owner.update(cx, |s, cx| {
                        s.focus(window, cx);
                        s.check(check_key.clone(), TreeChangeSource::Pointer, cx)
                    })
                });
            }
        }
        content = content.child(slot.on_click(|_, _, cx| cx.stop_propagation()));
    }
    if has_icons {
        let icon = node_icon.unwrap_or_else(|| {
            if node.is_leaf() {
                Icon::new(outline::File)
            } else {
                Icon::new(outline::Folder)
            }
        });
        content = content.child(
            icon.size(rems(
                theme,
                &format!("--input-icon-size-{}", p.size.suffix()),
            ))
            .color_value(rgba(theme, "--tree-color")),
        );
    }
    let label = if let Some(render) = &p.label {
        render(&node, window, cx)
    } else {
        crate::YueOverflowTooltip::new(
            (p.id.clone(), format!("title:{key}")),
            theme,
            node.label().clone(),
        )
        .width(gpui_kit::relative(1.))
        .into_any_element()
    };
    content = content.child(div().flex_1().min_w_0().overflow_hidden().child(label));
    if let Some(render) = &p.extra {
        content = content.child(div().flex_shrink_0().child(render(&node, window, cx)));
    }
    if let Some(render) = &p.actions {
        content = content.child(
            div()
                .id((p.id.clone(), format!("actions:{key}")))
                .key_context("YueTreeActions")
                .flex_shrink_0()
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(render(&node, window, cx)),
        );
    }
    if let Some(TreeLoadState::Failed(error)) = loading {
        let retry_owner = owner.clone();
        let retry_key = key.clone();
        content = content.child(
            Button::new((p.id.clone(), format!("retry:{key}")))
                .disabled(!interactive)
                .tab_stop(false)
                .accessibility_label(format!("{error}，重试加载"))
                .child("重试")
                .on_click(move |_, _, cx| {
                    cx.stop_propagation();
                    retry_owner.update(cx, |s, cx| s.request_load(Some(retry_key.clone()), cx))
                }),
        );
    }
    let click_owner = owner.clone();
    let click_key = key.clone();
    let a11y_owner = owner.clone();
    let a11y_key = key.clone();
    let a11y_focus_owner = owner.clone();
    let a11y_focus_key = key.clone();
    let a11y_expand_owner = owner.clone();
    let a11y_expand_key = key.clone();
    let context_owner = owner.clone();
    let context_key = key.clone();
    tree_row(
        (p.id.clone(), format!("node:{key}")),
        &node,
        row,
        expanded,
        selected,
        (check && node.is_checkable()).then_some(mark),
        active && has_focus,
    )
    .test_support()
    .h(h)
    .flex_shrink_0()
    .w_full()
    .min_w_0()
    .px(rems(theme, "--tree-padding-inline"))
    .rounded(rems(theme, "--tree-row-radius"))
    .text_size(font)
    .text_color(rgba(
        theme,
        if !interactive {
            "--tree-color-disabled"
        } else if selected {
            "--tree-color-selected"
        } else {
            "--tree-color"
        },
    ))
    .when(selected, |d| {
        d.bg(rgba(theme, "--tree-background-selected"))
    })
    .when(interactive, |d| {
        d.hover(move |style| style.bg(rgba(theme, "--tree-background-hover")))
    })
    .relative()
    .when(active && has_focus, |d| {
        d.child(
            div()
                .absolute()
                .inset_0()
                .rounded(rems(theme, "--tree-row-radius"))
                .border_1()
                .border_color(rgba(theme, "--tree-outline-color")),
        )
    })
    .on_click(move |event, window, cx| {
        click_owner.update(cx, |s, cx| {
            s.focus(window, cx);
            s.activate(
                click_key.clone(),
                TreeChangeSource::Pointer,
                event.modifiers(),
                s.toggle_selection || event.click_count() == 2,
                cx,
            )
        })
    })
    .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
        a11y_owner.update(cx, |s, cx| {
            s.focus(window, cx);
            if check && s.model.node(&a11y_key).is_some_and(|n| n.is_checkable()) {
                s.check(a11y_key.clone(), TreeChangeSource::Accessibility, cx);
            } else {
                s.activate(
                    a11y_key.clone(),
                    TreeChangeSource::Accessibility,
                    Default::default(),
                    true,
                    cx,
                );
            }
        })
    })
    .when(interactive, |d| {
        d.on_a11y_action(AccessibleAction::Focus, move |_, window, cx| {
            a11y_focus_owner.update(cx, |s, cx| {
                if s.disabled || !s.interactive {
                    return;
                }
                s.model.set_active(a11y_focus_key.clone());
                s.focus(window, cx);
                s.sync_geometry(cx);
                s.scroll_active(cx);
                cx.notify();
            });
        })
    })
    .when(interactive && !node.is_leaf(), |d| {
        d.on_a11y_action(
            if expanded {
                AccessibleAction::Collapse
            } else {
                AccessibleAction::Expand
            },
            move |_, window, cx| {
                a11y_expand_owner.update(cx, |s, cx| {
                    // Native Expand/Collapse requests are idempotent, unlike a toggle.
                    if s.model.is_expanded(&a11y_expand_key) == expanded {
                        s.focus(window, cx);
                        s.expand(a11y_expand_key.clone(), cx);
                    }
                });
            },
        )
    })
    .on_mouse_down(gpui_kit::MouseButton::Right, move |_, _, cx| {
        context_owner.update(cx, |_, cx| {
            cx.emit(TreeEvent::ContextRequested(context_key.clone()))
        })
    })
    .child(content)
    .a11y_disabled(!interactive)
    .into_any_element()
}
