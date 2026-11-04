//! Cell-level two-axis presentation. Both axes reuse VirtualizerState; the host
//! owns exactly one ScrollHandle and applies both proposals in one transaction.
use super::element::ViewStamp;
use super::*;
use crate::TestSupportExt as _;
use gpui::{
    AnyElement, App, AvailableSpace, Bounds, ContentMask, Div, Element, Entity, GlobalElementId,
    Hitbox, InspectorElementId, InteractiveElement, IntoElement, LayoutId, ParentElement,
    ScrollHandle, Stateful, StatefulInteractiveElement, StyleRefinement, Styled, Window, div,
    point, px, size,
};

#[derive(Clone)]
pub struct VirtualGridCell {
    row: VirtualizerItem,
    column: VirtualizerItem,
    frozen_row: bool,
    frozen_column: bool,
}
impl VirtualGridCell {
    /// Frame-local presentation flags; identity and preparation stay unchanged.
    pub fn is_frozen_row(&self) -> bool {
        self.frozen_row
    }
    pub fn is_frozen_column(&self) -> bool {
        self.frozen_column
    }

    pub fn row(&self) -> &VirtualizerItem {
        &self.row
    }
    pub fn column(&self) -> &VirtualizerItem {
        &self.column
    }
    pub fn identity(&self) -> VirtualGridCellIdentity {
        VirtualGridCellIdentity {
            row: self.row.identity(),
            column: self.column.identity(),
        }
    }
}
/// Content identity for preparation; geometry remains on the frame's cell.
#[derive(Clone)]
pub struct VirtualGridCellIdentity {
    row: VirtualizerItemIdentity,
    column: VirtualizerItemIdentity,
}
impl VirtualGridCellIdentity {
    pub fn row(&self) -> &VirtualizerItemIdentity {
        &self.row
    }
    pub fn column(&self) -> &VirtualizerItemIdentity {
        &self.column
    }
}
#[derive(Clone)]
pub struct VirtualGridSnapshot {
    rows: VirtualizerSnapshot,
    columns: VirtualizerSnapshot,
}
impl VirtualGridSnapshot {
    pub fn new(
        rows: VirtualizerSnapshot,
        columns: VirtualizerSnapshot,
    ) -> Result<Self, VirtualizerError> {
        if rows.axis() != Axis::Vertical
            || columns.axis() != Axis::Horizontal
            || !rows.viewport().shares_host(columns.viewport())
            || rows.viewport_revision() != columns.viewport_revision()
            || rows.viewport().extent() != columns.viewport().cross_extent()
            || columns.viewport().extent() != rows.viewport().cross_extent()
        {
            return Err(VirtualizerError::InvalidGeometry("grid axes"));
        }
        Ok(Self { rows, columns })
    }
    pub fn rows(&self) -> &VirtualizerSnapshot {
        &self.rows
    }
    pub fn columns(&self) -> &VirtualizerSnapshot {
        &self.columns
    }
    pub fn cells(&self) -> impl Iterator<Item = VirtualGridCell> + '_ {
        // Cells cost O(R*C); source/geometry preparation costs only O(R+C).
        let mut rows = self.rows.working_items();
        let mut row = rows.next().map(|item| item.expect("valid grid row"));
        let columns = if row.is_some() {
            self.columns
                .working_items()
                .map(|item| item.expect("valid grid column"))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let mut column = 0;
        std::iter::from_fn(move || {
            if columns.is_empty() {
                return None;
            }
            if column == columns.len() {
                column = 0;
                row = rows.next().map(|item| item.expect("valid grid row"));
            }
            let row = row.as_ref()?;
            let cell = VirtualGridCell {
                frozen_row: self.rows.is_frozen(row.index()),
                frozen_column: self.columns.is_frozen(columns[column].index()),
                row: row.clone(),
                column: columns[column].clone(),
            };
            column += 1;
            Some(cell)
        })
    }
}
type Renderer = Rc<dyn Fn(VirtualGridCell, &mut Window, &mut App) -> AnyElement>;
type ViewFactory = Rc<dyn Fn(VirtualGridCellIdentity, &mut Window, &mut App) -> gpui::AnyView>;
type CellKey = (ElementId, ElementId);
#[derive(Default)]
struct CellViews {
    entries: std::collections::HashMap<CellKey, ([ViewStamp; 2], gpui::AnyView)>,
    touched: std::collections::HashSet<CellKey>,
}
pub struct VirtualGrid {
    id: ElementId,
    rows: Entity<VirtualizerState>,
    columns: Entity<VirtualizerState>,
    scroll: ScrollHandle,
    base: Stateful<Div>,
    render: Renderer,
    view_factory: Option<ViewFactory>,
    content_revision: u64,
    frozen_rows: usize,
    frozen_columns: usize,
}
impl VirtualGrid {
    /// Preflight configuration without claiming a mount, querying the source,
    /// or writing a scroll position. Dynamic cell-derived row heights are not
    /// supported: both dimensions must be provided by the caller.
    pub fn validate_axes(
        rows: &VirtualizerState,
        columns: &VirtualizerState,
    ) -> Result<(), VirtualizerError> {
        if rows.axis() != Axis::Vertical || columns.axis() != Axis::Horizontal {
            return Err(VirtualizerError::InvalidGeometry("grid axes"));
        }
        if rows.attachment() != VirtualizerAttachment::Attached
            || columns.attachment() != VirtualizerAttachment::Attached
        {
            return Err(VirtualizerError::Unsupported("grid requires attached axes"));
        }
        if rows.sizing_kind() == VirtualizerSizingKind::Measured
            || columns.sizing_kind() == VirtualizerSizingKind::Measured
        {
            return Err(VirtualizerError::Unsupported(
                "grid requires explicit row/column dimensions",
            ));
        }
        if !rows.shares_binding(columns) {
            return Err(VirtualizerError::BindingConflict);
        }
        Ok(())
    }
    /// States must be Attached, share a binding, and use Fixed/Known sizes.
    /// Cells do not determine row/column sizes; callers supply explicit geometry.
    pub fn new<R: IntoElement>(
        id: impl Into<ElementId>,
        rows: &Entity<VirtualizerState>,
        columns: &Entity<VirtualizerState>,
        scroll: &ScrollHandle,
        render: impl Fn(VirtualGridCell, &mut Window, &mut App) -> R + 'static,
    ) -> Self {
        let id = id.into();
        let row_owner = rows.downgrade();
        let column_owner = columns.downgrade();
        let mut base = div()
            .id(id.clone())
            .size_full()
            .overflow_scroll()
            .track_scroll(scroll)
            .on_scroll_wheel(move |_, _, cx| {
                let _ = row_owner.update(cx, |s, cx| s.user_scroll_activity(cx));
                let _ = column_owner.update(cx, |s, cx| s.user_scroll_activity(cx));
            });
        base.style().allow_concurrent_scroll = Some(true);
        Self {
            id,
            rows: rows.clone(),
            columns: columns.clone(),
            scroll: scroll.clone(),
            base,
            render: Rc::new(move |cell, w, cx| render(cell, w, cx).into_any_element()),
            view_factory: None,
            content_revision: 0,
            frozen_rows: 0,
            frozen_columns: 0,
        }
    }
    /// Prepare one view per live content identity, independently of frame geometry.
    /// External factory inputs require content_revision or axis content invalidation.
    /// Child entity Render/layout/paint and its own notify contract remain unchanged.
    pub fn new_with_cell_views<V: Into<gpui::AnyView>>(
        id: impl Into<ElementId>,
        rows: &Entity<VirtualizerState>,
        columns: &Entity<VirtualizerState>,
        scroll: &ScrollHandle,
        factory: impl Fn(VirtualGridCellIdentity, &mut Window, &mut App) -> V + 'static,
    ) -> Self {
        let mut grid = Self::new(id, rows, columns, scroll, |_, _, _| div());
        grid.view_factory = Some(Rc::new(move |cell, w, cx| factory(cell, w, cx).into()));
        grid
    }
    /// Freeze the leading rows in current source order, clamped to source length.
    /// Oversized prefixes materialize only their visible portion and retained keys.
    pub fn frozen_rows(mut self, count: usize) -> Self {
        self.frozen_rows = count;
        self
    }
    /// Freeze leading columns using the same native scroll owner and cell IDs.
    pub fn frozen_columns(mut self, count: usize) -> Self {
        self.frozen_columns = count;
        self
    }
    pub fn content_revision(mut self, revision: u64) -> Self {
        self.content_revision = revision;
        self
    }
}
impl Styled for VirtualGrid {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}
impl InteractiveElement for VirtualGrid {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}
impl StatefulInteractiveElement for VirtualGrid {}
impl IntoElement for VirtualGrid {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
pub struct VirtualGridFrame {
    cells: Vec<(ElementId, AnyElement, Bounds<Pixels>)>,
    partitioned: bool,
    views: Option<Rc<std::cell::RefCell<CellViews>>>,
}
struct GridMount {
    rows: gpui::WeakEntity<VirtualizerState>,
    columns: gpui::WeakEntity<VirtualizerState>,
    row_revision: u64,
    column_revision: u64,
    _subscriptions: Vec<gpui::Subscription>,
    views: Option<Rc<std::cell::RefCell<CellViews>>>,
}
impl Element for VirtualGrid {
    type RequestLayoutState = VirtualGridFrame;
    type PrepaintState = Option<Hitbox>;
    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn a11y_role(&self) -> Option<gpui::Role> {
        self.base.a11y_role()
    }
    fn write_a11y_info(&self, node: &mut gpui::accesskit::Node) {
        self.base.write_a11y_info(node);
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        w: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, VirtualGridFrame) {
        let rows = self.rows.read(cx);
        let columns = self.columns.read(cx);
        Self::validate_axes(rows, columns).expect("invalid virtual grid configuration");
        let global = id.expect("grid identity");
        for state in [&self.rows, &self.columns] {
            state
                .update(cx, |s, _| {
                    s.claim_attached_mount(w.window_handle().window_id(), global)
                })
                .unwrap();
        }
        let rows = self.rows.clone();
        let columns = self.columns.clone();
        let row_revision = rows.read(cx).binding_revision();
        let column_revision = columns.read(cx).binding_revision();
        let mount = w.use_keyed_state(
            (self.id.clone(), "grid-mount"),
            cx,
            move |_, cx: &mut gpui::Context<GridMount>| {
                let subscriptions = vec![
                    cx.observe(&rows, |_, _, cx| cx.notify()),
                    cx.observe(&columns, |_, _, cx| cx.notify()),
                    cx.on_release(|mount: &mut GridMount, cx| {
                        let _ = mount
                            .rows
                            .update(cx, |s, cx| s.release_mount(mount.row_revision, cx));
                        let _ = mount
                            .columns
                            .update(cx, |s, cx| s.release_mount(mount.column_revision, cx));
                    }),
                ];
                GridMount {
                    rows: rows.downgrade(),
                    columns: columns.downgrade(),
                    row_revision,
                    column_revision,
                    _subscriptions: subscriptions,
                    views: None,
                }
            },
        );
        mount.update(cx, |m, cx| {
            if m.rows != self.rows || m.row_revision != row_revision {
                m.views = None;
                let _ = m
                    .rows
                    .update(cx, |s, cx| s.release_mount(m.row_revision, cx));
                m.rows = self.rows.downgrade();
                m.row_revision = row_revision;
                m._subscriptions[0] = cx.observe(&self.rows, |_, _, cx| cx.notify());
            }
            if m.columns != self.columns || m.column_revision != column_revision {
                m.views = None;
                let _ = m
                    .columns
                    .update(cx, |s, cx| s.release_mount(m.column_revision, cx));
                m.columns = self.columns.downgrade();
                m.column_revision = column_revision;
                m._subscriptions[1] = cx.observe(&self.columns, |_, _, cx| cx.notify());
            }
        });
        let views = mount.update(cx, |m, _| {
            if self.view_factory.is_some() {
                Some(m.views.get_or_insert_with(|| Rc::default()).clone())
            } else {
                m.views = None;
                None
            }
        });
        if let (Some(factory), Some(views)) = (self.view_factory.clone(), views.clone()) {
            views.borrow_mut().touched.clear();
            let rows = self.rows.clone();
            let columns = self.columns.clone();
            let row_source = rows.read(cx).source_for_render();
            let column_source = columns.read(cx).source_for_render();
            let revision = self.content_revision;
            self.render = Rc::new(move |cell, w, cx| {
                let key = (cell.row.key().clone(), cell.column.key().clone());
                let stamp = [
                    (
                        row_source.revision_at(cell.row.index()),
                        revision,
                        rows.read(cx).item_content_revision(cell.row.index()),
                        cell.row.index(),
                    ),
                    (
                        column_source.revision_at(cell.column.index()),
                        revision,
                        columns.read(cx).item_content_revision(cell.column.index()),
                        cell.column.index(),
                    ),
                ];
                let existing = {
                    let mut cache = views.borrow_mut();
                    cache.touched.insert(key.clone());
                    cache
                        .entries
                        .get(&key)
                        .filter(|(old, _)| *old == stamp)
                        .map(|(_, view)| view.clone())
                };
                let view = existing.unwrap_or_else(|| {
                    let view = factory(cell.identity(), w, cx);
                    views
                        .borrow_mut()
                        .entries
                        .insert(key, (stamp, view.clone()));
                    view
                });
                view.into_any_element()
            });
        }
        let layout =
            self.base
                .interactivity()
                .request_layout(id, inspector, w, cx, |style, w, cx| {
                    w.request_layout(style, None, cx)
                });
        (
            layout,
            VirtualGridFrame {
                cells: Vec::new(),
                partitioned: false,
                views,
            },
        )
    }
    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        frame: &mut VirtualGridFrame,
        w: &mut Window,
        cx: &mut App,
    ) -> Option<Hitbox> {
        let clip = bounds.intersect(&w.content_mask().bounds);
        let visible = clip.size.width > px(0.)
            && clip.size.height > px(0.)
            && w.is_visible()
            && w.is_window_active();
        for state in [&self.rows, &self.columns] {
            state.update(cx, |s, cx| {
                s.update_focus(w, cx);
                if !visible || cx.reduce_motion() {
                    s.settle_motion();
                }
            });
        }
        let offset = self.scroll.precise_offset();
        let now = cx.background_executor().now();
        let observe = |state: &VirtualizerState, offset: f64, main, cross| {
            let old = state.viewport();
            VirtualizerViewport::new(
                state.binding().unwrap(),
                old.map_or(1, |v| v.revision() + 1),
                offset.max(0.0),
                main,
                cross,
            )
            .input_revision(old.map_or(0, |v| {
                v.input_revision_value() + u64::from((v.offset() - offset).abs() > 0.01)
            }))
        };
        let revision = self
            .rows
            .read(cx)
            .viewport()
            .map_or(0, |v| v.revision())
            .max(self.columns.read(cx).viewport().map_or(0, |v| v.revision()))
            + 1;
        let mut rv = observe(
            self.rows.read(cx),
            -offset.y,
            bounds.size.height,
            bounds.size.width,
        );
        let mut cv = observe(
            self.columns.read(cx),
            -offset.x,
            bounds.size.width,
            bounds.size.height,
        );
        let row_count = if self.frozen_rows == 0 {
            0
        } else {
            self.frozen_rows
                .min(self.rows.read(cx).source_for_render().len())
        };
        let column_count = if self.frozen_columns == 0 {
            0
        } else {
            self.frozen_columns
                .min(self.columns.read(cx).source_for_render().len())
        };
        let frozen_height = Pixels::from(
            self.rows
                .read(cx)
                .frozen_extent(row_count)
                .min(f64::from(bounds.size.height)),
        );
        let frozen_width = Pixels::from(
            self.columns
                .read(cx)
                .frozen_extent(column_count)
                .min(f64::from(bounds.size.width)),
        );
        rv = rv
            .with_revision(revision)
            .with_frozen_prefix(row_count, frozen_height);
        cv = cv
            .with_revision(revision)
            .with_frozen_prefix(column_count, frozen_width);
        let (rs, rr) = self.rows.update(cx, |s, _| s.prepare(&rv, now)).unwrap();
        let (cs, cr) = self.columns.update(cx, |s, _| s.prepare(&cv, now)).unwrap();
        let y = rr
            .as_ref()
            .map_or(rv.offset(), VirtualizerScrollRequest::offset)
            .clamp(
                0.0,
                (rs.total_extent() - f64::from(bounds.size.height)).max(0.0),
            );
        let x = cr
            .as_ref()
            .map_or(cv.offset(), VirtualizerScrollRequest::offset)
            .clamp(
                0.0,
                (cs.total_extent() - f64::from(bounds.size.width)).max(0.0),
            );
        self.scroll.set_precise_offset(point(-x, -y));
        for (state, viewport, request, applied) in [
            (&self.rows, &rv, rr.as_ref(), y),
            (&self.columns, &cv, cr.as_ref(), x),
        ] {
            state.update(cx, |s, cx| {
                if let Some(request) = request {
                    s.confirm(request, applied, viewport).unwrap();
                } else {
                    s.commit(&viewport.clone().with_offset(applied)).unwrap();
                }
                s.arm_scroll_idle(cx);
                for event in s.take_events() {
                    cx.emit(event);
                }
            });
        }
        let rows = self.rows.read(cx).snapshot(&rv.with_offset(y)).unwrap();
        let columns = self.columns.read(cx).snapshot(&cv.with_offset(x)).unwrap();
        let content_size = size(
            Pixels::from(columns.total_extent()),
            Pixels::from(rows.total_extent()),
        );
        let snapshot = VirtualGridSnapshot::new(rows, columns).unwrap();
        frame.partitioned = row_count > 0 || column_count > 0;
        let hitbox = self.base.interactivity().prepaint(
            id,
            inspector,
            bounds,
            content_size,
            w,
            cx,
            |_, _, hitbox, w, cx| {
                let regions = [
                    Bounds::new(
                        bounds.origin + point(frozen_width, frozen_height),
                        bounds.size - size(frozen_width, frozen_height),
                    ),
                    Bounds::new(
                        bounds.origin + point(frozen_width, px(0.)),
                        size(bounds.size.width - frozen_width, frozen_height),
                    ),
                    Bounds::new(
                        bounds.origin + point(px(0.), frozen_height),
                        size(frozen_width, bounds.size.height - frozen_height),
                    ),
                    Bounds::new(bounds.origin, size(frozen_width, frozen_height)),
                ];
                let mut ordinary = (!frame.partitioned).then(|| snapshot.cells());
                let mut ordered = if frame.partitioned {
                    snapshot.cells().collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                ordered.sort_by_key(|cell| {
                    usize::from(cell.frozen_row) + 2 * usize::from(cell.frozen_column)
                });
                let mut ordered = ordered.into_iter();
                let cells = std::iter::from_fn(|| {
                    ordinary
                        .as_mut()
                        .map_or_else(|| ordered.next(), |it| it.next())
                });
                w.with_content_mask(Some(ContentMask { bounds: clip }), |w| {
                    for cell in cells {
                        let row_key = cell.row.key().clone();
                        let column_key = cell.column.key().clone();
                        let extent = size(cell.column.extent(), cell.row.extent());
                        let origin = bounds.origin
                            + point(
                                Pixels::from(
                                    cell.column.start() - if cell.frozen_column { 0. } else { x },
                                ),
                                Pixels::from(
                                    cell.row.start() - if cell.frozen_row { 0. } else { y },
                                ),
                            );
                        let region =
                            usize::from(cell.frozen_row) + 2 * usize::from(cell.frozen_column);
                        let mask = regions[region].intersect(&clip);
                        let build = |w: &mut Window, cx: &mut App| {
                            let mut element = div()
                                .id(column_key)
                                .w(extent.width)
                                .h(extent.height)
                                .child((self.render)(cell, w, cx))
                                .test_support()
                                .into_any_element();
                            w.with_id(row_key.clone(), |w| {
                                element.layout_as_root(extent.map(AvailableSpace::Definite), w, cx);
                                element.prepaint_at(origin, w, cx);
                            });
                            frame.cells.push((row_key, element, mask));
                        };
                        if row_count > 0 || column_count > 0 {
                            w.with_content_mask(Some(ContentMask { bounds: mask }), |w| {
                                build(w, cx)
                            });
                        } else {
                            build(w, cx);
                        }
                    }
                });
                hitbox
            },
        );
        if let Some(views) = &frame.views {
            let mut cache = views.borrow_mut();
            let CellViews { entries, touched } = &mut *cache;
            entries.retain(|key, _| touched.contains(key));
        }
        if visible
            && (self.rows.read(cx).has_scroll_command()
                || self.columns.read(cx).has_scroll_command())
        {
            w.request_animation_frame();
        }
        hitbox
    }
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        frame: &mut VirtualGridFrame,
        hitbox: &mut Option<Hitbox>,
        w: &mut Window,
        cx: &mut App,
    ) {
        self.base.interactivity().paint(
            id,
            inspector,
            bounds,
            hitbox.as_ref(),
            w,
            cx,
            |_, w, cx| {
                w.with_content_mask(Some(ContentMask { bounds }), |w| {
                    for (row, element, mask) in &mut frame.cells {
                        if frame.partitioned {
                            w.with_content_mask(Some(ContentMask { bounds: *mask }), |w| {
                                w.with_id(row.clone(), |w| element.paint(w, cx));
                            });
                        } else {
                            w.with_id(row.clone(), |w| element.paint(w, cx));
                        }
                    }
                });
            },
        );
    }
}
