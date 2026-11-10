//! Bounded, natural-height span layout. It builds every origin once and measures
//! it at the resolved column width; no virtual viewport or estimated row height.
use gpui_kit::{
    AnyElement, App, AvailableSpace, Bounds, Element, ElementId, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Pixels, Style, Window, point, px, size,
};
pub(super) struct Cell {
    pub row: usize,
    pub rows: usize,
    pub left: Pixels,
    pub width: Pixels,
    pub element: AnyElement,
}
type RowsCallback = Box<dyn Fn(Vec<Bounds<Pixels>>, &mut App)>;
pub(super) struct StaticGrid {
    pub cells: Vec<Cell>,
    pub count: usize,
    pub width: Pixels,
    pub minimum: Pixels,
    pub on_rows: RowsCallback,
}
pub(super) struct Geometry {
    heights: Vec<Pixels>,
    offsets: Vec<Pixels>,
}
impl IntoElement for StaticGrid {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for StaticGrid {
    type RequestLayoutState = Geometry;
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        Some("static-span-grid".into())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        w: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Geometry) {
        let mut heights = vec![self.minimum; self.count];
        let measured: Vec<_> = self
            .cells
            .iter_mut()
            .map(|c| {
                c.element
                    .layout_as_root(
                        size(
                            AvailableSpace::Definite(c.width),
                            AvailableSpace::MinContent,
                        ),
                        w,
                        cx,
                    )
                    .height
            })
            .collect();
        for (cell, height) in self
            .cells
            .iter()
            .zip(&measured)
            .filter(|(c, _)| c.rows == 1)
        {
            heights[cell.row] = heights[cell.row].max(*height);
        }
        for (cell, height) in self.cells.iter().zip(measured).filter(|(c, _)| c.rows > 1) {
            let current: Pixels = heights[cell.row..cell.row + cell.rows]
                .iter()
                .copied()
                .fold(px(0.), |a, b| a + b);
            let extra = (height - current).max(px(0.)) / cell.rows as f32;
            for h in &mut heights[cell.row..cell.row + cell.rows] {
                *h += extra;
            }
        }
        // Snap tracks once before accumulating origins; independent cell rounding
        // must not leave a seam at a fractional rowspan boundary.
        let scale = w.scale_factor();
        for h in &mut heights {
            *h = px((f32::from(*h) * scale).ceil() / scale);
        }
        let mut total = px(0.);
        let offsets = heights
            .iter()
            .map(|h| {
                let top = total;
                total += *h;
                top
            })
            .collect();
        let layout = w.request_layout(
            Style {
                size: size(self.width.into(), total.into()),
                ..Default::default()
            },
            [],
            cx,
        );
        (layout, Geometry { heights, offsets })
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        g: &mut Geometry,
        w: &mut Window,
        cx: &mut App,
    ) {
        for c in &mut self.cells {
            let height = g.heights[c.row..c.row + c.rows]
                .iter()
                .copied()
                .fold(px(0.), |a, b| a + b);
            c.element.layout_as_root(
                size(
                    AvailableSpace::Definite(c.width),
                    AvailableSpace::Definite(height),
                ),
                w,
                cx,
            );
            c.element
                .prepaint_at(bounds.origin + point(c.left, g.offsets[c.row]), w, cx);
        }
        (self.on_rows)(
            g.heights
                .iter()
                .enumerate()
                .map(|(i, h)| {
                    Bounds::new(
                        bounds.origin + point(px(0.), g.offsets[i]),
                        size(self.width, *h),
                    )
                })
                .collect(),
            cx,
        );
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Geometry,
        _: &mut (),
        w: &mut Window,
        cx: &mut App,
    ) {
        for c in &mut self.cells {
            c.element.paint(w, cx);
        }
    }
}

pub(super) fn render<D: super::TableDelegate>(
    state: &mut super::YueTableState<D>,
    view: &std::rc::Rc<super::TableView<D::Source>>,
    p: &super::Presentation,
    w: &mut Window,
    cx: &mut gpui_kit::Context<super::YueTableState<D>>,
) -> StaticGrid {
    use super::{TableSelectionMode, TableSource};
    use gpui_kit::{InteractiveElement as _, ParentElement as _, Styled as _};
    let minimum = p
        .row_height
        .unwrap_or_else(|| p.size.height(crate::current_theme(cx)))
        .to_pixels(w.rem_size());
    let rectangles: Vec<_> = view
        .spans
        .descriptors()
        .iter()
        .map(|s| {
            (
                view.source.position_of(s.origin().row()).unwrap(),
                p.resolved
                    .indices
                    .iter()
                    .position(|i| view.columns[*i].key() == s.origin().column())
                    .unwrap(),
                s.row_span(),
                s.column_span(),
            )
        })
        .collect();
    let mut cells = Vec::new();
    let selection = view.selection_mode != TableSelectionMode::None;
    for r in 0..view.source.len() {
        let key = view.source.row_key(r).unwrap();
        let row_id = super::state::row_id(view.source.scope(), key);
        if view.source.is_detail(key) {
            let element = super::element::row(
                view,
                r + usize::from(p.header_visible),
                None,
                None,
                p,
                &state.delegate,
                &cx.weak_entity(),
                state.active.as_ref(),
                state.edit.as_ref(),
                state.measurements.epoch,
                state.scroll_handle(),
                &state.focus,
                &state.control_focus,
                w,
                cx,
            );
            cells.push(Cell {
                row: r,
                rows: 1,
                left: px(0.),
                width: p.resolved.total,
                element: gpui_kit::div()
                    .id((row_id, "detail"))
                    .w(p.resolved.total)
                    .h_full()
                    .child(element)
                    .into_any_element(),
            });
            continue;
        }
        let range = if selection { 0 } else { 1 }..p.resolved.indices.len() + 1;
        for track in range {
            let c = track.checked_sub(1);
            let span = c.and_then(|c| {
                rectangles.iter().find(|(row, col, rows, cols)| {
                    (*row..row + rows).contains(&r) && (*col..col + cols).contains(&c)
                })
            });
            if let (Some(c), Some((row, col, _, _))) = (c, span)
                && (*row != r || *col != c)
            {
                continue;
            }
            let (rows, columns) = span.map_or((1, 1), |s| (s.2, s.3));
            let (left, width, id) = c.map_or(
                (px(0.), p.resolved.selection, "selection".to_owned()),
                |c| {
                    (
                        p.resolved.offsets[c],
                        p.resolved.widths[c..c + columns]
                            .iter()
                            .copied()
                            .fold(px(0.), |a, b| a + b),
                        view.columns[p.resolved.indices[c]]
                            .key()
                            .as_str()
                            .to_owned(),
                    )
                },
            );
            let part = super::cells::CellPart {
                column: c,
                width,
                rows,
                columns,
                header_level: 0,
                natural: true,
            };
            let element = super::element::row(
                view,
                r + usize::from(p.header_visible),
                None,
                Some(&part),
                p,
                &state.delegate,
                &cx.weak_entity(),
                state.active.as_ref(),
                state.edit.as_ref(),
                state.measurements.epoch,
                state.scroll_handle(),
                &state.focus,
                &state.control_focus,
                w,
                cx,
            );
            // Stable cell prefix isolates the row helper's retained control IDs.
            cells.push(Cell {
                row: r,
                rows,
                left,
                width,
                element: gpui_kit::div()
                    .id((row_id.clone(), id))
                    .w(width)
                    .h_full()
                    .child(element)
                    .into_any_element(),
            });
        }
    }
    let keys: Vec<_> = (0..view.source.len())
        .map(|i| view.source.row_key(i).unwrap().clone())
        .collect();
    let owner = cx.weak_entity();
    StaticGrid {
        cells,
        count: view.source.len(),
        width: p.resolved.total,
        minimum,
        on_rows: Box::new(move |bounds, cx| {
            let _ = owner.update(cx, |s, _| {
                s.static_row_bounds.extend(keys.iter().cloned().zip(bounds))
            });
        }),
    }
}
