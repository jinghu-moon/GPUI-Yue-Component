use super::*;
use crate::{
    CheckboxState, VirtualGrid, YueCheckbox, YueEmpty, YueScrollbar, YueSpinner,
    tokens::{rems as token_rems, rgba},
};
use gpui_kit::base::{A11ySelectionExt as _, SelectionA11y, TestSupportExt as _};
use gpui_kit::{
    Context, InteractiveElement as _, ParentElement as _, Pixels, Role,
    StatefulInteractiveElement as _, Styled as _, div, px,
};

pub(super) fn render<D: TableDelegate>(
    state: &mut YueTableState<D>,
    w: &mut Window,
    cx: &mut Context<YueTableState<D>>,
) -> AnyElement {
    let Some(p) = state.presentation.clone() else {
        return div().into_any_element();
    };
    if state.repair_control_focus {
        state.repair_control_focus = false;
        if state.control_focus.contains_focused(w, cx) {
            state.focus.focus(w, cx);
        }
    }
    let theme = crate::current_theme(cx);
    let mut root = div()
        .id(p.id.clone())
        .test_support()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .text_color(rgba(theme, "--text-primary"))
        .text_size(token_rems(theme, p.size.font()))
        .line_height(token_rems(
            theme,
            match p.size {
                TableSize::Small => crate::theme::keys::LINE_HEIGHT_SM,
                TableSize::Medium => crate::theme::keys::LINE_HEIGHT_MD,
                TableSize::Large => crate::theme::keys::LINE_HEIGHT_LG,
            },
        ));
    if let Some(toolbar) = &p.toolbar {
        root = root.child(toolbar(w, cx));
    }
    let Some(view) = state.accepted.clone() else {
        return root.child(YueEmpty::new("unbound")).into_any_element();
    };
    // Also release the previous axis source when entering empty/no-column states.
    state.sync_layout(&p, w, cx);
    let busy = matches!(
        view.load,
        TableLoadState::Loading | TableLoadState::Refreshing
    );
    if busy && !view.source.is_empty() {
        root = root.child(YueSpinner::new("refreshing").label("正在刷新，保留当前结果"));
    }
    if let TableLoadState::Failed(message) = &view.load {
        root = root.child(
            div()
                .id("table-error")
                .test_support()
                .role(Role::Status)
                .py_2()
                .child(message.clone()),
        );
    }
    let mut table = div()
        .id("table-region")
        .test_support()
        .relative()
        .w_full()
        .min_w_0()
        .overflow_hidden()
        .role(Role::Table)
        .aria_label(p.label.clone())
        .aria_row_count(
            view.source.len()
                + usize::from(p.header_visible)
                + usize::from(view.columns.iter().any(|c| c.footer.is_some())),
        )
        .aria_column_count(
            view.columns.len() + usize::from(view.selection_mode != TableSelectionMode::None),
        )
        .track_focus(&state.focus.clone().tab_stop(!p.disabled))
        .on_key_down(cx.listener(|s, event: &gpui_kit::KeyDownEvent, w, cx| {
            if !s.focus.is_focused(w) && event.keystroke.key == "escape" {
                // Bubbled only after the actual editor/popup has declined Escape.
                s.focus.focus(w, cx);
                cx.stop_propagation();
                cx.notify();
                return;
            }
            if !s.focus.is_focused(w) {
                return;
            } // Inner inputs/buttons keep their normal keys and IME.
            navigate(s, event, w, cx);
        }));
    if p.outer_border {
        table = table
            .border_1()
            .border_color(rgba(theme, "--border-default"));
    }
    if view.columns.is_empty() || view.source.is_empty() {
        let message = if view.columns.is_empty() {
            "暂无可见列"
        } else {
            "暂无数据"
        };
        if p.mode == TableRenderMode::Rows {
            table = table.h(p.height);
        }
        if p.header_visible && !view.columns.is_empty() {
            let width = view
                .columns
                .iter()
                .map(|c| c.width.to_pixels(w.rem_size()))
                .fold(px(0.), |a, b| a + b)
                + if view.selection_mode != TableSelectionMode::None {
                    token_rems(theme, crate::theme::keys::SIZE_48).to_pixels(w.rem_size())
                } else {
                    px(0.)
                };
            let header = row(
                &view,
                0,
                &p,
                &state.delegate,
                &cx.weak_entity(),
                None,
                &state.focus,
                &state.control_focus,
                w,
                cx,
            );
            table = table
                .child(
                    div()
                        .id("empty-scroll")
                        .w_full()
                        .overflow_x_scroll()
                        .track_scroll(&state.scroll)
                        .child(div().w(width).min_w_full().child(header)),
                )
                .child(YueScrollbar::horizontal(
                    (p.id.clone(), "scrollbar"),
                    &state.scroll,
                ));
        }
        table = table.child(if busy && view.source.is_empty() {
            YueSpinner::new("initial-loading")
                .label("正在加载数据")
                .into_any_element()
        } else if let Some(empty) = &p.empty {
            empty(w, cx)
        } else {
            YueEmpty::new("empty")
                .description(message)
                .into_any_element()
        });
    } else {
        let owner = cx.weak_entity();
        let delegate = state.delegate.clone();
        let render_p = p.clone();
        let active = state.active.clone();
        let focus = state.focus.clone();
        let control_focus = state.control_focus.clone();
        let render_view = view.clone();
        if p.mode == TableRenderMode::Rows {
            let mut grid = VirtualGrid::new(
                (p.id.clone(), "viewport"),
                &state.rows,
                &state.columns,
                &state.scroll,
                move |item, w, cx| {
                    row(
                        &render_view,
                        item.row().index(),
                        &render_p,
                        &delegate,
                        &owner,
                        active.as_ref(),
                        &focus,
                        &control_focus,
                        w,
                        cx,
                    )
                },
            )
            .frozen_rows(usize::from(p.header_visible && p.header_fixed));
            if let Some(active) = &state.active {
                grid = grid
                    .cell_focus_handle(
                        crate::VirtualGridCellKey::new(
                            state::row_id(view.source.scope(), active.row()),
                            "table-row-track",
                        ),
                        state.control_focus.clone(),
                    )
                    .focus_fallback(state.focus.clone());
            }
            table = table
                .h(p.height)
                .child(grid.test_support())
                .child(YueScrollbar::both(
                    (p.id.clone(), "scrollbar"),
                    &state.scroll,
                ));
        } else {
            // Static keeps all rows in one natural-height block; horizontal overflow
            // still has a single owner for header, body and column footer.
            let total = view
                .columns
                .iter()
                .map(|c| c.width.to_pixels(w.rem_size()))
                .fold(px(0.), |a, b| a + b)
                + if view.selection_mode != TableSelectionMode::None {
                    token_rems(theme, crate::theme::keys::SIZE_48).to_pixels(w.rem_size())
                } else {
                    px(0.)
                };
            let mut content = div().w(total).min_w_full().flex().flex_col();
            for index in 0..view.source.len()
                + usize::from(p.header_visible)
                + usize::from(view.columns.iter().any(|c| c.footer.is_some()))
            {
                content = content.child(row(
                    &view,
                    index,
                    &p,
                    &delegate,
                    &owner,
                    active.as_ref(),
                    &focus,
                    &control_focus,
                    w,
                    cx,
                ));
            }
            table = table
                .child(
                    div()
                        .id("static-scroll")
                        .test_support()
                        .w_full()
                        .overflow_x_scroll()
                        .track_scroll(&state.scroll)
                        .child(content),
                )
                .child(YueScrollbar::horizontal(
                    (p.id.clone(), "scrollbar"),
                    &state.scroll,
                ));
        }
    }
    root = root.child(
        table.a11y_selection(
            SelectionA11y::default()
                .disabled(p.disabled)
                .readonly(p.readonly)
                .multiple(view.selection_mode == TableSelectionMode::Multiple),
        ),
    );
    if let Some(footer) = &p.footer {
        root = root.child(footer(w, cx));
    }
    root.into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn row<D: TableDelegate>(
    view: &Rc<TableView<D::Source>>,
    index: usize,
    p: &Presentation,
    delegate: &gpui_kit::WeakEntity<D>,
    owner: &gpui_kit::WeakEntity<YueTableState<D>>,
    active: Option<&TableCellKey>,
    focus: &gpui_kit::FocusHandle,
    control_focus: &gpui_kit::FocusHandle,
    w: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let theme = crate::current_theme(cx);
    let is_header = p.header_visible && index == 0;
    let position = index.saturating_sub(usize::from(p.header_visible));
    let is_footer = !is_header && position == view.source.len();
    let is_band = is_header || is_footer;
    let key = (!is_band).then(|| {
        view.source
            .row_key(position)
            .expect("validated source")
            .clone()
    });
    let selected = key.as_ref().is_some_and(|k| view.selection.contains(k));
    let mut row = div()
        .id(if let Some(key) = &key {
            state::row_id(view.source.scope(), key)
        } else if is_header {
            "table-header".into()
        } else {
            "table-footer".into()
        })
        .test_support()
        .role(Role::Row)
        .aria_row_index(index)
        .flex()
        .w_full()
        .min_w_0()
        .items_stretch();
    let height = p
        .row_height
        .unwrap_or_else(|| p.size.height(crate::current_theme(cx)));
    if p.mode == TableRenderMode::Rows {
        row = row.h(height).overflow_hidden();
    } else {
        row = row.min_h(height);
    }
    if is_band {
        row = row
            .bg(rgba(theme, p.surface))
            .font_weight(gpui_kit::FontWeight::SEMIBOLD);
    } else {
        row = row.aria_selected(selected);
        if selected {
            row = row.bg(rgba(theme, "--list-row-background-selected"));
        } else if p.striped && position % 2 == 1 {
            row = row.bg(rgba(theme, "--surface-subtle"));
        }
        if p.hoverable && !p.disabled && !selected {
            row = row.hover(|s| s.bg(rgba(theme, "--list-row-background-hover")));
        }
    }
    if p.borders != TableBorders::None {
        row = row
            .border_b_1()
            .border_color(rgba(theme, "--border-divider"));
    }
    if view.selection_mode != TableSelectionMode::None {
        let own = owner.clone();
        let own_focus = focus.clone();
        let key = key.clone();
        let selectable = view.source.len().saturating_sub(view.disabled_rows.len());
        let mark = if is_header {
            if view.selection.is_empty() {
                CheckboxState::Unchecked
            } else if view.selection.len() == selectable {
                CheckboxState::Checked
            } else {
                CheckboxState::Indeterminate
            }
        } else if selected {
            CheckboxState::Checked
        } else {
            CheckboxState::Unchecked
        };
        let disabled = p.disabled
            || matches!(
                view.load,
                TableLoadState::Loading | TableLoadState::Refreshing
            )
            || key.as_ref().is_some_and(|k| view.disabled_rows.contains(k));
        let select =
            if is_footer || (is_header && view.selection_mode == TableSelectionMode::Single) {
                div().into_any_element()
            } else if !is_header && view.selection_mode == TableSelectionMode::Single {
                crate::YueRadio::new("select", theme, selected)
                    .size(match p.size {
                        TableSize::Small => crate::RadioSize::Small,
                        TableSize::Medium => crate::RadioSize::Medium,
                        TableSize::Large => crate::RadioSize::Large,
                    })
                    .tab_stop(false)
                    .accessibility_label(format!("选择 {}", key.as_ref().unwrap().as_str()))
                    .disabled(disabled)
                    .on_change(move |_, _, w, cx| {
                        cx.stop_propagation();
                        own_focus.focus(w, cx);
                        let _ = own.update(cx, |s, cx| {
                            let key = key.as_ref().unwrap();
                            let column = s.view().unwrap().columns[0].key().clone();
                            s.set_active(TableCellKey::new(key.clone(), column), w, cx);
                            let _ = s.request_toggle(key, false, cx);
                        });
                    })
                    .into_any_element()
            } else {
                YueCheckbox::new("select", theme, mark)
                    .size(match p.size {
                        TableSize::Small => crate::CheckboxSize::Small,
                        TableSize::Medium => crate::CheckboxSize::Medium,
                        TableSize::Large => crate::CheckboxSize::Large,
                    })
                    .tab_stop(false)
                    .accessibility_label(if is_header {
                        "选择当前结果中的可选行".to_owned()
                    } else {
                        format!("选择 {}", key.as_ref().unwrap().as_str())
                    })
                    .disabled(disabled || (is_header && selectable == 0))
                    .on_change(move |mark, event, w, cx| {
                        cx.stop_propagation();
                        own_focus.focus(w, cx);
                        let _ = own.update(cx, |s, cx| {
                            if let Some(key) = &key {
                                let column = s.view().unwrap().columns[0].key().clone();
                                s.set_active(TableCellKey::new(key.clone(), column), w, cx);
                                let _ = s.request_toggle(key, event.modifiers().shift, cx);
                            } else {
                                let _ = s.request_select_all(mark == CheckboxState::Checked, cx);
                            }
                        });
                    })
                    .into_any_element()
            };
        row = row.child(
            div()
                .id("selection-cell")
                .test_support()
                .role(if is_header {
                    Role::ColumnHeader
                } else {
                    Role::Cell
                })
                .aria_column_index(0)
                .w(token_rems(theme, crate::theme::keys::SIZE_48))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .child(select),
        );
    }
    for (column_index, column) in view.columns.iter().enumerate() {
        let cell_key = key
            .as_ref()
            .map(|key| TableCellKey::new(key.clone(), column.key().clone()));
        let cell_active = cell_key.as_ref().is_some_and(|key| active == Some(key));
        let mut cell = div()
            .id(column.key.element_id())
            .test_support()
            .role(if is_header {
                Role::ColumnHeader
            } else {
                Role::Cell
            })
            .aria_column_index(
                column_index + usize::from(view.selection_mode != TableSelectionMode::None),
            )
            .w(column.width)
            .flex_shrink_0()
            .min_w_0()
            .flex()
            .flex_col()
            .px(token_rems(theme, p.size.padding()))
            .py(if p.mode == TableRenderMode::Static {
                token_rems(theme, crate::theme::keys::SPACE_4)
            } else {
                rems(0.)
            })
            .overflow_hidden();
        cell = match column.vertical {
            TableVerticalAlign::Start => cell.justify_start(),
            TableVerticalAlign::Center => cell.justify_center(),
            TableVerticalAlign::End => cell.justify_end(),
        };
        let align = if is_header {
            column.header_align.unwrap_or(column.align)
        } else {
            column.align
        };
        cell = match align {
            TableAlign::Start => cell.text_align(gpui_kit::TextAlign::Left),
            TableAlign::Center => cell.text_align(gpui_kit::TextAlign::Center),
            TableAlign::End => cell.text_align(gpui_kit::TextAlign::Right),
        };
        if p.borders == TableBorders::Grid && column_index + 1 < view.columns.len() {
            cell = cell
                .border_r_1()
                .border_color(rgba(theme, "--border-divider"));
        }
        // The overlay does not change text width or wrapping when focus moves.
        cell = cell.relative();
        let content = if is_footer {
            div()
                .child(column.footer.clone().unwrap_or_default())
                .into_any_element()
        } else if is_header {
            if let Some(header) = &column.header {
                header(w, cx)
            } else {
                div().child(column.title.clone()).into_any_element()
            }
        } else {
            let row = view
                .source
                .row(key.as_ref().unwrap())
                .expect("validated record");
            let cell_context = TableCellContext {
                key: cell_key.clone().unwrap(),
                position,
                selected,
                active: cell_active,
                control_focus: (cell_active && column.interactive).then(|| control_focus.clone()),
                size: p.size,
                disabled: p.disabled,
                readonly: p.readonly,
                busy: matches!(
                    view.load,
                    TableLoadState::Loading | TableLoadState::Refreshing
                ),
                width: (column.width.to_pixels(w.rem_size())
                    - token_rems(theme, p.size.padding()).to_pixels(w.rem_size()) * 2.
                    - if p.borders == TableBorders::Grid && column_index + 1 < view.columns.len() {
                        token_rems(theme, crate::theme::keys::BORDER_1).to_pixels(w.rem_size())
                    } else {
                        px(0.)
                    })
                .max(px(0.)),
            };
            delegate
                .update(cx, |d, cx| d.render_cell(row, column, &cell_context, w, cx))
                .unwrap_or_else(|_| div().into_any_element())
        };
        // Plain text is navigable by click; custom controls own their own focus and commands.
        if !is_band && column.text.is_some() && !p.disabled {
            let own = owner.clone();
            let key = cell_key.unwrap();
            cell = cell.on_mouse_down(gpui_kit::MouseButton::Left, move |_, w, cx| {
                let _ = own.update(cx, |s, cx| {
                    s.anchor = Some(key.row().clone());
                    s.set_active(key.clone(), w, cx);
                });
            });
        }
        let content_box = div()
            .id("cell-content")
            .test_support()
            .min_w_0()
            .w_full()
            .flex()
            .items_center();
        let content_box = match align {
            TableAlign::Start => content_box.justify_start(),
            TableAlign::Center => content_box.justify_center(),
            TableAlign::End => content_box.justify_end(),
        };
        cell = cell.child(content_box.child(content));
        if cell_active {
            cell = cell.aria_active_descendant();
        }
        if cell_active && focus.is_focused(w) {
            cell = cell.child(
                div()
                    .absolute()
                    .inset_0()
                    .border_1()
                    .border_color(rgba(theme, "--focus-ring-color")),
            );
        }
        row = row.child(cell);
    }
    row.into_any_element()
}

fn navigate<D: TableDelegate>(
    state: &mut YueTableState<D>,
    event: &gpui_kit::KeyDownEvent,
    w: &mut Window,
    cx: &mut Context<YueTableState<D>>,
) {
    let Some(p) = state.presentation.clone() else {
        return;
    };
    let theme = crate::current_theme(cx);
    let Some(view) = state.accepted.clone() else {
        return;
    };
    if p.disabled
        || view.source.is_empty()
        || view.columns.is_empty()
        || event.keystroke.modifiers.alt
        || event.keystroke.modifiers.platform
    {
        return;
    }
    let mut r = state
        .active
        .as_ref()
        .and_then(|key| view.source.position_of(key.row()))
        .unwrap_or(0);
    let mut c = state
        .active
        .as_ref()
        .and_then(|key| {
            view.columns
                .iter()
                .position(|col| col.key() == key.column())
        })
        .unwrap_or(0);
    let key = event.keystroke.key.as_str();
    let page = (p.height.0
        / p.row_height
            .unwrap_or_else(|| p.size.height(crate::current_theme(cx)))
            .0)
        .floor()
        .max(1.) as usize;
    match key {
        "enter" | "f2" if view.columns[c].interactive => {
            let active = TableCellKey::new(
                view.source.row_key(r).unwrap().clone(),
                view.columns[c].key().clone(),
            );
            state.set_active(active.clone(), w, cx);
            state.control_focus.focus(w, cx);
            cx.stop_propagation();
            return;
        }
        "up" => r = r.saturating_sub(1),
        "down" => r = (r + usize::from(state.active.is_some())).min(view.source.len() - 1),
        "left" => c = c.saturating_sub(1),
        "right" => c = (c + 1).min(view.columns.len() - 1),
        "home" => {
            c = 0;
            if event.keystroke.modifiers.control {
                r = 0;
            }
        }
        "end" => {
            c = view.columns.len() - 1;
            if event.keystroke.modifiers.control {
                r = view.source.len() - 1;
            }
        }
        "pageup" => r = r.saturating_sub(page),
        "pagedown" => r = (r + page).min(view.source.len() - 1),
        "space" => {
            let row = view.source.row_key(r).unwrap().clone();
            let _ = state.request_toggle(&row, event.keystroke.modifiers.shift, cx);
            cx.stop_propagation();
            return;
        }
        "a" if event.keystroke.modifiers.control
            && view.selection_mode == TableSelectionMode::Multiple =>
        {
            let _ = state.request_select_all(true, cx);
            cx.stop_propagation();
            return;
        }
        _ => return,
    }
    let row = view.source.row_key(r).unwrap().clone();
    if !event.keystroke.modifiers.shift {
        state.anchor = Some(row.clone());
    } else if state.anchor.is_none() {
        state.anchor = state
            .active
            .as_ref()
            .map(|k| k.row().clone())
            .or_else(|| view.source.row_key(0).cloned());
    }
    let key = TableCellKey::new(row.clone(), view.columns[c].key().clone());
    state.set_active(key, w, cx);
    let prefix = if view.selection_mode != TableSelectionMode::None {
        token_rems(theme, crate::theme::keys::SIZE_48).to_pixels(w.rem_size())
    } else {
        px(0.)
    };
    let left: Pixels = view.columns[..c]
        .iter()
        .map(|col| col.width.to_pixels(w.rem_size()))
        .fold(prefix, |a, b| a + b);
    let right = left + view.columns[c].width.to_pixels(w.rem_size());
    let offset = state.scroll.offset();
    let width = state.scroll.bounds().size.width;
    let x = if left < -offset.x {
        -left
    } else if right > -offset.x + width {
        width - right
    } else {
        offset.x
    };
    state.scroll.set_offset(gpui_kit::point(
        x.max(-state.scroll.max_offset().x).min(px(0.)),
        offset.y,
    ));
    if event.keystroke.modifiers.shift && view.selection_mode == TableSelectionMode::Multiple {
        let _ = state.request_toggle(&row, true, cx);
    }
    cx.stop_propagation();
}
