use super::*;
use crate::{VirtualGrid, VirtualGridCellKey, VirtualGridSpan, VirtualizerSource};
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::{
    Context, InteractiveElement as _, ParentElement as _, Pixels, StatefulInteractiveElement as _,
    Styled as _, div,
};
use std::collections::HashMap;
pub(super) fn header_id(level: usize) -> ElementId {
    if level == 0 {
        state::header_key()
    } else {
        ("table-header-level", level).into()
    }
}
pub(super) fn header_count(p: &Presentation) -> usize {
    if p.header_visible {
        if p.mode == TableRenderMode::Cells {
            p.resolved.depth + 1
        } else {
            1
        }
    } else {
        0
    }
}
pub(super) fn selection_id() -> ElementId {
    "table-selection-track".into()
}
pub(super) struct ColumnSource {
    keys: Vec<ElementId>,
    positions: HashMap<ElementId, usize>,
    pub sizes: Vec<Pixels>,
}
impl ColumnSource {
    pub fn new<S: TableSource>(view: &TableView<S>, resolved: &layout::ResolvedColumns) -> Self {
        let mut keys = Vec::new();
        let mut sizes = Vec::new();
        if view.selection_mode != TableSelectionMode::None {
            keys.push(selection_id());
            sizes.push(resolved.selection);
        }
        for (i, index) in resolved.indices.iter().enumerate() {
            keys.push(view.columns[*index].key().element_id());
            sizes.push(resolved.widths[i]);
        }
        Self {
            positions: keys
                .iter()
                .enumerate()
                .map(|(i, k)| (k.clone(), i))
                .collect(),
            keys,
            sizes,
        }
    }
}
impl VirtualizerSource for ColumnSource {
    fn len(&self) -> usize {
        self.keys.len()
    }
    fn key_at(&self, index: usize) -> ElementId {
        self.keys[index].clone()
    }
    fn index_of(&self, key: &ElementId) -> Option<usize> {
        self.positions.get(key).copied()
    }
}
pub(super) struct CellPart {
    pub column: Option<usize>,
    pub width: Pixels,
    pub rows: usize,
    pub columns: usize,
    pub header_level: usize,
}
pub(super) fn validate_mode<S: TableSource>(
    view: &TableView<S>,
    p: &Presentation,
) -> Result<(), TableError> {
    if p.mode == TableRenderMode::Cells && (view.sizing.is_measured() || view.has_details) {
        return Err(TableError::UnsupportedCombination(
            "Cells requires Fixed/Known and no full-width details",
        ));
    }
    if p.host.is_some() && (p.mode == TableRenderMode::Static || view.sizing.is_measured()) {
        return Err(TableError::UnsupportedCombination(
            "scroll_host requires Rows/Cells with Fixed/Known",
        ));
    }
    for span in view.spans.descriptors() {
        if p.mode != TableRenderMode::Cells && span.row_span() != 1 {
            return Err(TableError::UnsupportedCombination(
                "rowspan requires Cells with Fixed/Known",
            ));
        }
        let r = view
            .source
            .position_of(span.origin().row())
            .ok_or(TableError::MissingKey)?;
        let end = r + span.row_span();
        let start_count = p.frozen.0.min(view.source.len());
        let end_start = view
            .source
            .len()
            .saturating_sub(p.frozen.1)
            .max(start_count);
        if r < start_count && end > start_count || r < end_start && end > end_start {
            return Err(TableError::UnsupportedCombination(
                "span crosses frozen row boundary",
            ));
        }
    }
    Ok(())
}
pub(super) fn span_set<S: TableSource>(
    view: &TableView<S>,
    p: &Presentation,
) -> crate::VirtualGridSpanSet {
    let rows = header_count(p);
    let selection = usize::from(view.selection_mode != TableSelectionMode::None);
    let mut spans = Vec::new();
    // Header cells use the same sparse span index as body cells.
    for level in 0..rows {
        let mut ix = 0;
        while ix < p.resolved.indices.len() {
            let column = &view.columns[p.resolved.indices[ix]];
            if column.groups.len() < level {
                ix += 1;
                continue;
            }
            if let Some(group) = column.groups.get(level) {
                let mut end = ix + 1;
                while end < p.resolved.indices.len()
                    && p.resolved.pins[end] == p.resolved.pins[ix]
                    && view.columns[p.resolved.indices[end]]
                        .groups
                        .get(level)
                        .is_some_and(|g| g.key() == group.key())
                {
                    end += 1;
                }
                if end - ix > 1 {
                    spans.push(VirtualGridSpan::new(
                        VirtualGridCellKey::new(header_id(level), column.key().element_id()),
                        1,
                        end - ix,
                    ));
                }
                ix = end;
            } else {
                if rows - level > 1 {
                    spans.push(VirtualGridSpan::new(
                        VirtualGridCellKey::new(header_id(level), column.key().element_id()),
                        rows - level,
                        1,
                    ));
                }
                ix += 1;
            }
        }
    }
    if selection > 0 && rows > 1 {
        spans.push(VirtualGridSpan::new(
            VirtualGridCellKey::new(header_id(0), selection_id()),
            rows,
            1,
        ));
    }
    for span in view.spans.descriptors() {
        spans.push(VirtualGridSpan::new(
            VirtualGridCellKey::new(
                state::row_id(view.source.scope(), span.origin().row()),
                span.origin().column().element_id(),
            ),
            span.row_span(),
            span.column_span(),
        ));
    }
    crate::VirtualGridSpanSet::new(spans)
}
pub(super) fn render<D: TableDelegate>(
    state: &mut YueTableState<D>,
    view: &Rc<TableView<D::Source>>,
    p: &Presentation,
    w: &mut Window,
    cx: &mut Context<YueTableState<D>>,
) -> AnyElement {
    let rows = header_count(p);
    let selection = usize::from(view.selection_mode != TableSelectionMode::None);
    let signature = (
        Rc::as_ptr(&p.resolved) as usize,
        view.spans.identity(),
        rows,
    );
    if state.span_signature != Some(signature) {
        state.span_set = span_set(view, p);
        state.span_signature = Some(signature);
    }
    let view = view.clone();
    let p = p.clone();
    let delegate = state.delegate.clone();
    let owner = cx.weak_entity();
    let active = state.active.clone();
    let edit = state.edit.clone();
    let focus = state.focus.clone();
    let control = state.control_focus.clone();
    let scroll = state.scroll.clone();
    let render_view = view.clone();
    let rp = p.clone();
    let epoch = state.measurements.epoch;
    let scroll_host = p.host.as_ref().map_or(&state.scroll, |(host, _, _)| host);
    let mut grid = VirtualGrid::new(
        (p.id.clone(), "viewport"),
        &state.rows,
        &state.columns,
        scroll_host,
        move |item, w, cx| {
            let grid_column = item.column().index();
            let index = grid_column.checked_sub(selection);
            let level = item.row().index();
            let part = CellPart {
                column: index,
                width: item.extent().width,
                rows: item.row_span(),
                columns: item.column_span(),
                header_level: level,
            };
            if level < rows
                && let Some(index) = index
                && let Some(group) = render_view.columns[rp.resolved.indices[index]]
                    .groups
                    .get(level)
            {
                let first = (0..index).all(|i| {
                    !render_view.columns[rp.resolved.indices[i]]
                        .groups
                        .get(level)
                        .is_some_and(|g| g.key() == group.key())
                });
                let mut groupcell = div()
                    .id((ElementId::from("table-group"), group.key().clone()))
                    .test_support()
                    .aria_label(if first {
                        group.title().clone()
                    } else {
                        "".into()
                    })
                    .aria_row_index(level)
                    .aria_column_index(grid_column)
                    .aria_column_span(item.column_span())
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(crate::tokens::rgba(crate::current_theme(cx), rp.surface))
                    .child(group.title().clone());
                if first {
                    groupcell = groupcell.role(gpui_kit::Role::ColumnHeader);
                }
                return groupcell.into_any_element();
            }
            let logical_index = if level < rows {
                0
            } else {
                level - rows + usize::from(rp.header_visible)
            };
            element::row(
                &render_view,
                logical_index,
                Some(item.extent().height),
                Some(&part),
                &rp,
                &delegate,
                &owner,
                active.as_ref(),
                edit.as_ref(),
                epoch,
                &scroll,
                &focus,
                &control,
                w,
                cx,
            )
        },
    )
    .frozen_rows(
        (if p.header_fixed || p.frozen.0 > 0 {
            rows
        } else {
            0
        }) + p.frozen.0.min(view.source.len()),
    )
    .frozen_end_rows(
        p.frozen.1.min(view.source.len().saturating_sub(p.frozen.0))
            + usize::from(p.footer_fixed && view.columns.iter().any(|c| c.footer.is_some())),
    )
    .frozen_columns(
        selection
            + p.resolved
                .pins
                .iter()
                .take_while(|pin| **pin == TablePin::Start)
                .count(),
    )
    .frozen_end_columns(
        p.resolved
            .pins
            .iter()
            .rev()
            .take_while(|pin| **pin == TablePin::End)
            .count(),
    );
    if let Some(key) = &state.active {
        grid = grid
            .cell_focus_handle(
                VirtualGridCellKey::new(
                    state::row_id(view.source.scope(), key.row()),
                    key.column().element_id(),
                ),
                state.control_focus.clone(),
            )
            .focus_fallback(state.focus.clone());
    }
    if let Some(edit) = &state.edit {
        let keys = if edit.mode() == TableEditMode::Row {
            view.columns
                .iter()
                .filter(|col| col.editable && col.visible)
                .map(|col| {
                    VirtualGridCellKey::new(
                        state::row_id(view.source.scope(), edit.key().row()),
                        col.key().element_id(),
                    )
                })
                .collect()
        } else {
            vec![VirtualGridCellKey::new(
                state::row_id(view.source.scope(), edit.key().row()),
                edit.key().column().element_id(),
            )]
        };
        grid = grid.retained_cells(keys);
    }
    if let Some((_, top, bottom)) = &p.host {
        grid = grid.in_scroll_host(top.to_pixels(w.rem_size()), bottom.to_pixels(w.rem_size()));
    }
    grid.span_set(&state.span_set, cx)
        .expect("validated table span plan")
        .test_support()
        .into_any_element()
}
