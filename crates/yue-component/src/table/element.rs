use super::*;
use crate::{
    CheckboxState, VirtualGrid, YueCheckbox, YueEmpty, YueScrollbar, YueSpinner,
    tokens::{rems as token_rems, rgba},
};
use gpui_kit::base::{A11ySelectionExt as _, ElementExt as _, SelectionA11y, TestSupportExt as _};
use gpui_kit::{
    Context, InteractiveElement as _, ParentElement as _, Pixels, Role,
    StatefulInteractiveElement as _, Styled as _, div, px,
};

pub(super) fn render<D: TableDelegate>(
    state: &mut YueTableState<D>,
    w: &mut Window,
    cx: &mut Context<YueTableState<D>>,
) -> AnyElement {
    let Some(mut p) = state.presentation.clone() else {
        return div().into_any_element();
    };
    if state.repair_control_focus {
        state.repair_control_focus = false;
        if state.control_focus.contains_focused(w, cx)
            || state
                .editor_focuses
                .iter()
                .any(|focus| focus.contains_focused(w, cx))
        {
            state.focus.focus(w, cx);
        }
        if state.edit.is_none() {
            state.editor_focuses.clear();
        }
    }
    let weak = cx.weak_entity();
    w.use_keyed_state((p.id.clone(), "table-resize-lifetime"), cx, |w, cx| {
        let released = weak.clone();
        cx.on_release(move |_: &mut resize::ResizeLifetime, cx| {
            let _ = released.update(cx, |s, cx| {
                s.cancel_resize(cx);
                if s.edit.take().is_some() {
                    cx.emit(TableEvent::EditEnded);
                    cx.notify();
                }
            });
        })
        .detach();
        resize::ResizeLifetime {
            _activation: cx.observe_window_activation(w, move |_, w, cx| {
                if !w.is_window_active() {
                    let _ = weak.update(cx, |s, cx| s.cancel_resize(cx));
                }
            }),
        }
    });
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
    let pointer_owner = cx.weak_entity();
    root = root.child(
        gpui_kit::canvas(
            |_, _, _| {},
            move |_, _, w, _| {
                let moving = pointer_owner.clone();
                w.on_mouse_event(move |event: &gpui_kit::MouseMoveEvent, phase, _, cx| {
                    if !phase.capture() {
                        return;
                    }
                    let _ = moving.update(cx, |s, cx| {
                        if s.has_pointer_resize() {
                            if event.pressed_button == Some(gpui_kit::MouseButton::Left) {
                                s.pointer_resize(event.position.x, cx);
                            } else {
                                s.cancel_resize(cx);
                            }
                        }
                    });
                });
                w.on_mouse_event(move |event: &gpui_kit::MouseUpEvent, phase, _, cx| {
                    if !phase.capture() || event.button != gpui_kit::MouseButton::Left {
                        return;
                    }
                    let _ = pointer_owner.update(cx, |s, cx| {
                        if s.has_pointer_resize() {
                            s.pointer_resize(event.position.x, cx);
                            s.finish_resize(cx);
                        }
                    });
                });
            },
        )
        .absolute()
        .size_full(),
    );
    if let Some(toolbar) = &p.toolbar {
        root = root.child(toolbar(w, cx));
    }
    let Some(view) = state.accepted.clone() else {
        return root.child(YueEmpty::new("unbound")).into_any_element();
    };
    // Also release the previous axis source when entering empty/no-column states.
    state.sync_layout(&p, w, cx);
    p.resolved = state.resolved.clone();
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
    let observe_owner = cx.weak_entity();
    let border = if p.outer_border { px(2.) } else { px(0.) };
    let edit = state.edit.clone();
    let measurement_epoch = state.measurements.epoch;
    let mut table = div()
        .id("table-region")
        .on_prepaint(move |bounds, _, cx| {
            let width = (bounds.size.width - border).max(px(0.));
            let weak = observe_owner.clone();
            if observe_owner
                .read_with(cx, |s, _| s.available_width != width)
                .unwrap_or(false)
            {
                cx.defer(move |cx| {
                    let _ = weak.update(cx, |s, cx| {
                        if s.available_width != width {
                            s.available_width = width;
                            cx.notify();
                        }
                    });
                });
            }
        })
        .test_support()
        .relative()
        .w_full()
        .min_w_0()
        .overflow_hidden()
        .role(Role::Table)
        .aria_label(p.label.clone())
        .aria_row_count(
            view.source.len()
                + if p.header_visible {
                    p.resolved.depth + 1
                } else {
                    0
                }
                + usize::from(view.columns.iter().any(|c| c.footer.is_some())),
        )
        .aria_column_count(
            p.resolved.indices.len() + usize::from(view.selection_mode != TableSelectionMode::None),
        )
        .track_focus(&state.focus.clone().tab_stop(!p.disabled))
        .on_key_down(cx.listener(|s, event: &gpui_kit::KeyDownEvent, w, cx| {
            if event.keystroke.key == "escape" && s.edit.is_some() {
                s.cancel_edit(w, cx);
                cx.stop_propagation();
                return;
            }
            if event.keystroke.key == "escape" && (s.resize.is_some() || s.resize_preview.is_some())
            {
                s.cancel_resize(cx);
                cx.stop_propagation();
                return;
            }
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
    if p.resolved.indices.is_empty() || view.source.is_empty() {
        let message = if p.resolved.indices.is_empty() {
            "暂无可见列"
        } else {
            "暂无数据"
        };
        if p.mode == TableRenderMode::Rows {
            table = table.h(p.height);
        }
        if p.header_visible && !view.columns.is_empty() {
            let width = p.resolved.total;
            let header = row(
                &view,
                0,
                None,
                &p,
                &state.delegate,
                &cx.weak_entity(),
                None,
                None,
                measurement_epoch,
                &state.scroll,
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
        let row_scroll = state.scroll.clone();
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
                        Some(item.row().extent()),
                        &render_p,
                        &delegate,
                        &owner,
                        active.as_ref(),
                        edit.as_ref(),
                        measurement_epoch,
                        &row_scroll,
                        &focus,
                        &control_focus,
                        w,
                        cx,
                    )
                },
            )
            .frozen_rows(
                usize::from(p.header_visible && (p.header_fixed || p.frozen.0 > 0))
                    + p.frozen.0.min(view.source.len()),
            )
            .frozen_end_rows(
                p.frozen.1.min(view.source.len().saturating_sub(p.frozen.0))
                    + usize::from(
                        p.footer_fixed && view.columns.iter().any(|c| c.footer.is_some()),
                    ),
            );
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
            if let Some(session) = &state.edit {
                grid = grid.retained_cells(vec![crate::VirtualGridCellKey::new(
                    state::row_id(view.source.scope(), session.key().row()),
                    "table-row-track",
                )]);
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
            let total = p.resolved.total;
            let mut content = div().w(total).min_w_full().flex().flex_col();
            for index in 0..view.source.len()
                + usize::from(p.header_visible)
                + usize::from(view.columns.iter().any(|c| c.footer.is_some()))
            {
                content = content.child(row(
                    &view,
                    index,
                    None,
                    &p,
                    &delegate,
                    &owner,
                    active.as_ref(),
                    edit.as_ref(),
                    measurement_epoch,
                    &row_scroll,
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
    extent: Option<Pixels>,
    p: &Presentation,
    delegate: &gpui_kit::WeakEntity<D>,
    owner: &gpui_kit::WeakEntity<YueTableState<D>>,
    active: Option<&TableCellKey>,
    edit: Option<&TableEditSession>,
    measurement_epoch: u64,
    scroll: &gpui_kit::ScrollHandle,
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
    let positioned = p.resolved.has_pins() && p.mode == TableRenderMode::Rows
        || is_header && p.resolved.depth > 0;
    let mut cells = Vec::new();
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
        .aria_row_index(if is_header {
            0
        } else {
            index
                + if p.header_visible {
                    p.resolved.depth
                } else {
                    0
                }
        })
        .flex()
        .w_full()
        .min_w_0()
        .items_stretch();
    let minimum_height = p
        .row_height
        .unwrap_or_else(|| p.size.height(crate::current_theme(cx)))
        .to_pixels(w.rem_size());
    let height = if let Some(height) = extent.filter(|_| !is_band) {
        height
    } else if !is_band && p.mode == TableRenderMode::Rows {
        let key = key.as_ref().unwrap();
        match &view.sizing.kind {
            sizing::RowSizingKind::Fixed => minimum_height,
            sizing::RowSizingKind::Known(heights) => heights[key].to_pixels(w.rem_size()),
            sizing::RowSizingKind::Measured(estimate) => estimate.to_pixels(w.rem_size()),
        }
    } else {
        minimum_height
    };
    if key.as_ref().is_some_and(|key| view.source.is_detail(key)) {
        let key = key.as_ref().unwrap();
        let record = view.source.row(key).expect("validated detail");
        let content = delegate
            .update(cx, |d, cx| d.render_detail(record, p.resolved.total, w, cx))
            .unwrap_or_else(|_| div().into_any_element());
        let mut content = div()
            .id("detail-content")
            .test_support()
            .w_full()
            .min_w_0()
            .flex_shrink_0()
            .child(content);
        if view.sizing.is_measured() && p.mode == TableRenderMode::Rows {
            let key = key.clone();
            let revision = view.source.row_revision(&key);
            let epoch = measurement_epoch;
            let own = owner.clone();
            content = content.on_prepaint(move |bounds, _, cx| {
                let own = own.clone();
                let key = key.clone();
                let height = bounds.size.height.max(minimum_height);
                cx.defer(move |cx| {
                    let _ =
                        own.update(cx, |s, cx| s.measure_cell(key, epoch, revision, height, cx));
                });
            });
        }
        let detail = div()
            .id("detail-cell")
            .test_support()
            .role(Role::Cell)
            .aria_column_index(0)
            .aria_column_span(
                p.resolved.indices.len()
                    + usize::from(view.selection_mode != TableSelectionMode::None),
            )
            .w_full()
            .overflow_hidden()
            .child(content);
        return if p.mode == TableRenderMode::Static {
            row.child(detail).min_h(height).into_any_element()
        } else {
            row.child(detail.h(height))
                .h(height)
                .overflow_hidden()
                .into_any_element()
        };
    }
    let bands = if is_header { p.resolved.depth + 1 } else { 1 };
    if is_header && bands > 1 {
        row = row.role(Role::RowGroup);
    }
    if p.mode == TableRenderMode::Rows {
        row = row.h(height * bands as f32).overflow_hidden();
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
        let (selectable, page_selected) = view.page_selection;
        let mark = if is_header {
            if page_selected == 0 {
                CheckboxState::Unchecked
            } else if page_selected == selectable {
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
            || is_header && selectable == 0
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
                        "选择当前页的可选行".to_owned()
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
        let selection_cell = div()
            .id("selection-cell")
            .test_support()
            .role(if is_header {
                Role::ColumnHeader
            } else {
                Role::Cell
            })
            .aria_column_index(0)
            .aria_row_span(bands)
            .w(token_rems(theme, crate::theme::keys::SIZE_48))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .child(select)
            .into_any_element();
        if positioned {
            cells.push((
                TablePin::Start,
                px(0.),
                p.resolved.selection,
                0usize,
                bands,
                selection_cell,
            ));
        } else {
            row = row.child(selection_cell);
        }
    }
    for (column_index, declared_index) in p.resolved.indices.iter().enumerate() {
        let column = &view.columns[*declared_index];
        let width = p.resolved.widths[column_index];
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
            .aria_row_index(if is_header {
                column.groups.len()
            } else {
                index
                    + if p.header_visible {
                        p.resolved.depth
                    } else {
                        0
                    }
            })
            .aria_column_index(
                column_index + usize::from(view.selection_mode != TableSelectionMode::None),
            )
            .w(width)
            .flex_shrink_0()
            .min_w_0()
            .flex()
            .flex_col()
            .px(token_rems(theme, p.size.padding()))
            .py(
                if p.mode == TableRenderMode::Static || view.sizing.is_measured() && !is_band {
                    token_rems(theme, crate::theme::keys::SPACE_4)
                } else {
                    rems(0.)
                },
            )
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
        if p.borders == TableBorders::Grid && column_index + 1 < p.resolved.indices.len() {
            cell = cell
                .border_r_1()
                .border_color(rgba(theme, "--border-divider"));
        }
        // The overlay does not change text width or wrapping when focus moves.
        cell = cell.relative();
        if p.resolved.has_pins() && p.mode == TableRenderMode::Rows || bands > 1 {
            cell = cell.h_full();
        }
        let content = if is_footer {
            div()
                .child(column.footer.clone().unwrap_or_default())
                .into_any_element()
        } else if is_header {
            if let Some(header) = &column.header {
                header(w, cx)
            } else if column.is_sortable() {
                let sorted = view
                    .query
                    .sorts()
                    .iter()
                    .enumerate()
                    .find(|(_, s)| s.column() == column.key());
                let suffix = sorted.map_or(String::new(), |(ix, s)| {
                    format!(
                        " {}{}",
                        if s.direction() == TableSortDirection::Ascending {
                            "↑"
                        } else {
                            "↓"
                        },
                        if view.query.sorts().len() > 1 {
                            (ix + 1).to_string()
                        } else {
                            String::new()
                        }
                    )
                });
                let own = owner.clone();
                let key = column.key().clone();
                crate::YueButton::new("table-sort", format!("{}{suffix}", column.title()))
                    .variant(crate::ButtonVariant::Ghost)
                    .size(crate::ButtonSize::Small)
                    .tab_stop(false)
                    .accessibility_label(format!("按{}排序{suffix}", column.title()))
                    .accessibility_description(
                        "点击切换排序；Shift 加入多列排序；表格导航时 Alt+S 排序当前列",
                    )
                    .disabled(
                        p.disabled
                            || p.readonly
                            || matches!(
                                view.load,
                                TableLoadState::Loading | TableLoadState::Refreshing
                            ),
                    )
                    .on_click(move |event, _, cx| {
                        let _ = own.update(cx, |s, cx| {
                            let _ = s.request_sort(&key, event.modifiers().shift, cx);
                        });
                    })
                    .into_any_element()
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
                control_focus: (cell_active && (column.interactive || column.editable))
                    .then(|| control_focus.clone()),
                editing: edit
                    .as_ref()
                    .is_some_and(|session| session.includes(cell_key.as_ref().unwrap())),
                edit_stage: edit
                    .as_ref()
                    .filter(|session| session.includes(cell_key.as_ref().unwrap()))
                    .map(|session| session.stage().clone()),
                size: p.size,
                disabled: p.disabled,
                readonly: p.readonly,
                busy: matches!(
                    view.load,
                    TableLoadState::Loading | TableLoadState::Refreshing
                ),
                width: (width
                    - token_rems(theme, p.size.padding()).to_pixels(w.rem_size()) * 2.
                    - if p.borders == TableBorders::Grid
                        && column_index + 1 < p.resolved.indices.len()
                    {
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
        let editing = cell_key
            .as_ref()
            .is_some_and(|key| edit.as_ref().is_some_and(|session| session.includes(key)));
        if !is_band && (column.text.is_some() || column.editable) && !p.disabled && !editing {
            let own = owner.clone();
            let key = cell_key.unwrap();
            let editable = column.editable;
            cell = cell.on_mouse_down(gpui_kit::MouseButton::Left, move |event, w, cx| {
                let _ = own.update(cx, |s, cx| {
                    s.anchor = Some(key.row().clone());
                    s.set_active(key.clone(), w, cx);
                    if editable && event.click_count == 2 {
                        let _ = s.request_edit(key.clone(), TableEditMode::Cell, cx);
                    }
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
        let mut content_box = content_box.child(content).flex_shrink_0();
        if view.sizing.is_measured() && !is_band && p.mode == TableRenderMode::Rows {
            let key = key.as_ref().unwrap().clone();
            let revision = view.source.row_revision(&key);
            let epoch = measurement_epoch;
            let own = owner.clone();
            let padding = token_rems(theme, crate::theme::keys::SPACE_4).to_pixels(w.rem_size())
                * 2.
                + if p.borders == TableBorders::None {
                    px(0.)
                } else {
                    px(1.)
                };
            content_box = content_box.on_prepaint(move |bounds, _, cx| {
                let height = (bounds.size.height + padding).max(minimum_height);
                let own = own.clone();
                let key = key.clone();
                cx.defer(move |cx| {
                    let _ =
                        own.update(cx, |s, cx| s.measure_cell(key, epoch, revision, height, cx));
                });
            });
        }
        cell = cell.child(content_box);
        if is_header && column.resizable && !p.disabled && !p.readonly {
            let own = owner.clone();
            let blur_key = column.key().clone();
            let handle_state = w.use_keyed_state(
                (
                    (p.id.clone(), column.key().as_str().to_owned()).into(),
                    "resize-focus",
                ),
                cx,
                |w, cx| {
                    let focus = cx.focus_handle().tab_stop(true);
                    let blur = cx.on_blur(&focus, w, move |_, _, cx| {
                        let _ = own.update(cx, |s, cx| s.resize_blur(&blur_key, cx));
                    });
                    resize::HandleLifetime { focus, _blur: blur }
                },
            );
            let handle_focus = handle_state.read(cx).focus.clone();
            let down_focus = handle_focus.clone();
            let down_owner = owner.clone();
            let down_key = column.key().clone();
            let key_owner = owner.clone();
            let key = column.key().clone();
            let up_owner = owner.clone();
            let handle = gpui_kit::base::resize_handle::<(), resize::DragMarker>(
                "column-resize",
                gpui_kit::Axis::Horizontal,
            )
            .inside(gpui_kit::base::HandleEdge::Trailing)
            .with_focus(&handle_focus)
            .hit_width(token_rems(theme, "--resizable-hit-width").to_pixels(w.rem_size()))
            .line_width(token_rems(theme, "--resizable-line-width").to_pixels(w.rem_size()))
            .role(Role::Splitter)
            .aria_label(format!("调整{}列宽", column.title()))
            .on_mouse_down(gpui_kit::MouseButton::Left, move |event, w, cx| {
                cx.stop_propagation();
                down_focus.focus(w, cx);
                let _ = down_owner.update(cx, |s, cx| {
                    let _ = s.begin_resize(&down_key, Some(event.position.x), w, cx);
                });
            })
            .on_key_down(move |event, w, cx| {
                let _ = key_owner.update(cx, |s, cx| s.keyboard_resize(&key, event, w, cx));
            })
            .on_key_up(move |event, _, cx| {
                let _ = up_owner.update(cx, |s, cx| {
                    s.resize_key_up(event.keystroke.key.as_str(), cx)
                });
            });
            cell = cell.child(handle);
        }

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
        if is_header {
            cell = cell
                .aria_row_index(column.groups.len())
                .aria_row_span(bands - column.groups.len());
        }
        if positioned {
            cells.push((
                p.resolved.pins[column_index],
                p.resolved.offsets[column_index],
                width,
                if is_header { column.groups.len() } else { 0 },
                if is_header {
                    bands - column.groups.len()
                } else {
                    1
                },
                cell.into_any_element(),
            ));
        } else {
            row = row.child(cell);
        }
    }
    if is_header && bands > 1 {
        for level in 0..p.resolved.depth {
            let mut ix = 0;
            while ix < p.resolved.indices.len() {
                let column = &view.columns[p.resolved.indices[ix]];
                let Some(group) = column.groups.get(level) else {
                    ix += 1;
                    continue;
                };
                let mut end = ix + 1;
                while end < p.resolved.indices.len()
                    && view.columns[p.resolved.indices[end]]
                        .groups
                        .get(level)
                        .is_some_and(|g| g.key() == group.key())
                {
                    end += 1;
                }
                let width = p.resolved.widths[ix..end]
                    .iter()
                    .copied()
                    .fold(px(0.), |a, b| a + b);
                let groupcell = div()
                    .id((ElementId::from("table-group"), group.key().clone()))
                    .test_support()
                    .role(Role::ColumnHeader)
                    .aria_label(group.title().clone())
                    .aria_row_index(level)
                    .aria_column_index(
                        ix + usize::from(view.selection_mode != TableSelectionMode::None),
                    )
                    .aria_column_span(end - ix)
                    .flex()
                    .items_center()
                    .justify_center()
                    .size_full()
                    .border_b_1()
                    .border_color(rgba(theme, "--border-divider"))
                    .child(group.title().clone());
                cells.push((
                    p.resolved.pins[ix],
                    p.resolved.offsets[ix],
                    width,
                    level,
                    1,
                    groupcell.into_any_element(),
                ));
                ix = end;
            }
        }
    }
    if positioned {
        let offset = if p.mode == TableRenderMode::Rows {
            -scroll.precise_offset().x
        } else {
            0.
        };
        let offset = Pixels::from(offset);
        let pins = p.resolved.has_pins() && p.mode == TableRenderMode::Rows;
        let leading = if pins { p.resolved.leading } else { px(0.) };
        let trailing = if pins { p.resolved.trailing } else { px(0.) };
        let available = if pins {
            p.resolved.available
        } else {
            p.resolved.total
        };
        let background = if selected {
            rgba(theme, "--list-row-background-selected")
        } else if !is_band && p.striped && position % 2 == 1 {
            rgba(theme, "--surface-subtle")
        } else {
            rgba(theme, p.surface)
        };
        for level in 0..bands {
            let mut lanes = [Vec::new(), Vec::new(), Vec::new()];
            let remaining = height * (bands - level) as f32;
            for (pin, start, width, celllevel, span, cell) in
                cells.iter_mut().filter(|c| c.3 == level)
            {
                let part = match pin {
                    TablePin::Start if leading > px(0.) => 1,
                    TablePin::End if pins => 2,
                    _ => 0,
                };
                let left = match part {
                    1 => *start,
                    2 => *start - (p.resolved.total - trailing),
                    _ => *start - offset - leading,
                };
                let content = std::mem::replace(cell, div().into_any_element());
                lanes[part].push(
                    div()
                        .absolute()
                        .left(left)
                        .top_0()
                        .w(*width)
                        .h(height * (*span) as f32)
                        .child(div().size_full().child(content))
                        .into_any_element(),
                );
                let _ = celllevel;
            }
            let mut band = div()
                .id(("table-band-level", level))
                .absolute()
                .left_0()
                .top(height * level as f32)
                .w_full()
                .h(remaining);
            if is_header && bands > 1 {
                band = band.role(Role::Row).aria_row_index(level);
            }
            for (part, children) in lanes.into_iter().enumerate() {
                let (left, width) = match part {
                    1 => (offset, leading),
                    2 => (offset + available - trailing, trailing),
                    _ => (
                        offset + leading,
                        (available - leading - trailing).max(px(0.)),
                    ),
                };
                let lane = div()
                    .absolute()
                    .left(left)
                    .top_0()
                    .h_full()
                    .w(width)
                    .overflow_hidden()
                    .children(children);
                band = band.child(if part > 0 { lane.bg(background) } else { lane });
            }
            row = row.child(band);
        }
        row = row.relative();
        if p.mode == TableRenderMode::Static {
            row = row.h(height * bands as f32);
        }
    } else {
        row = row.children(cells.into_iter().map(|(_, _, _, _, _, cell)| cell));
    }
    row.into_any_element()
}

fn navigate<D: TableDelegate>(
    state: &mut YueTableState<D>,
    event: &gpui_kit::KeyDownEvent,
    w: &mut Window,
    cx: &mut Context<YueTableState<D>>,
) {
    let Some(mut p) = state.presentation.clone() else {
        return;
    };
    p.resolved = state.resolved.clone();
    let Some(view) = state.accepted.clone() else {
        return;
    };
    if event.keystroke.key == "s" && event.keystroke.modifiers.alt && !p.disabled {
        if let Some(active) = state.active.clone() {
            let _ = state.request_sort(active.column(), event.keystroke.modifiers.shift, cx);
            cx.stop_propagation();
        }
        return;
    }
    if p.disabled
        || view.source.is_empty()
        || p.resolved.indices.is_empty()
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
            p.resolved
                .indices
                .iter()
                .position(|i| view.columns[*i].key() == key.column())
        })
        .unwrap_or(0);
    let key = event.keystroke.key.as_str();
    let page_row = |forward: bool| -> usize {
        let rows = state.rows.read(cx);
        let Some(viewport) = rows.viewport() else {
            return r;
        };
        let Ok(snapshot) = rows.snapshot(viewport) else {
            return r;
        };
        let index = r + usize::from(p.header_visible);
        let Ok(item) = snapshot.item(index) else {
            return r;
        };
        let leading = usize::from(p.header_visible && (p.header_fixed || p.frozen.0 > 0))
            + p.frozen.0.min(view.source.len());
        let trailing = p.frozen.1.min(view.source.len().saturating_sub(p.frozen.0))
            + usize::from(p.footer_fixed && view.columns.iter().any(|c| c.footer.is_some()));
        let frozen = (0..leading)
            .chain(snapshot.len().saturating_sub(trailing)..snapshot.len())
            .filter_map(|ix| snapshot.item(ix).ok())
            .fold(px(0.), |sum, item| sum + item.extent());
        let extent = f64::from((viewport.extent() - frozen).max(px(0.)));
        if extent <= 0. {
            return r;
        }
        let target = item.start() + if forward { extent } else { -extent };
        rows.snapshot(&viewport.clone().with_offset(target.max(0.)))
            .ok()
            .map(|s| {
                s.visible_range()
                    .start
                    .saturating_sub(usize::from(p.header_visible))
                    .min(view.source.len() - 1)
            })
            .unwrap_or(r)
    };
    match key {
        "enter" | "f2" if view.columns[p.resolved.indices[c]].editable => {
            let key = TableCellKey::new(
                view.source.row_key(r).unwrap().clone(),
                view.columns[p.resolved.indices[c]].key().clone(),
            );
            let _ = state.request_edit(key, TableEditMode::Cell, cx);
            cx.stop_propagation();
            return;
        }
        "enter" | "f2" if view.columns[p.resolved.indices[c]].interactive => {
            let active = TableCellKey::new(
                view.source.row_key(r).unwrap().clone(),
                view.columns[p.resolved.indices[c]].key().clone(),
            );
            state.set_active(active.clone(), w, cx);
            state.control_focus.focus(w, cx);
            cx.stop_propagation();
            return;
        }
        "up" => r = r.saturating_sub(1),
        "down" => r = (r + usize::from(state.active.is_some())).min(view.source.len() - 1),
        "left" => c = c.saturating_sub(1),
        "right" => c = (c + 1).min(p.resolved.indices.len() - 1),
        "home" => {
            c = 0;
            if event.keystroke.modifiers.control {
                r = 0;
            }
        }
        "end" => {
            c = p.resolved.indices.len() - 1;
            if event.keystroke.modifiers.control {
                r = view.source.len() - 1;
            }
        }
        "pageup" => r = page_row(false),
        "pagedown" => r = page_row(true),
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
    if view
        .source
        .row_key(r)
        .is_some_and(|key| view.source.is_detail(key))
    {
        let forward = matches!(key, "down" | "pagedown" | "end");
        let replacement = if forward {
            (r..view.source.len())
                .find(|ix| !view.source.is_detail(view.source.row_key(*ix).unwrap()))
        } else {
            (0..=r)
                .rev()
                .find(|ix| !view.source.is_detail(view.source.row_key(*ix).unwrap()))
        };
        let Some(replacement) = replacement else {
            return;
        };
        r = replacement;
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
    let key = TableCellKey::new(
        row.clone(),
        view.columns[p.resolved.indices[c]].key().clone(),
    );
    state.set_active(key, w, cx);
    let left = p.resolved.offsets[c];
    let right = left + p.resolved.widths[c];
    let offset = state.scroll.offset();
    let width = state.scroll.bounds().size.width;
    let x = if p.resolved.pins[c] != TablePin::None {
        offset.x
    } else if left < -offset.x + p.resolved.leading {
        -left + p.resolved.leading
    } else if right > -offset.x + width - p.resolved.trailing {
        width - p.resolved.trailing - right
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
