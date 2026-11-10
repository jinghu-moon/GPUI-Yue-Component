use super::*;
use crate::{
    VirtualizerBinding, VirtualizerChange, VirtualizerLayout, VirtualizerScrollOptions,
    VirtualizerSizing, VirtualizerSource, VirtualizerState,
};
use gpui_kit::{
    AppContext as _, Axis, Context, EntityId, EventEmitter, FocusHandle, Focusable, Render,
    ScrollHandle, px,
};
use model::SelectionIntent;

pub trait TableDelegate: Sized + 'static {
    type Source: TableSource;
    fn render_cell(
        &mut self,
        row: &<Self::Source as TableSource>::Row,
        column: &TableColumn<<Self::Source as TableSource>::Row>,
        cell: &TableCellContext,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        column.render(row, cell, window, cx)
    }
    fn render_detail(
        &mut self,
        _row: &<Self::Source as TableSource>::Row,
        _width: gpui_kit::Pixels,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        gpui_kit::div().into_any_element()
    }
}
struct RowSource<S: TableSource> {
    source: Option<Rc<S>>,
    headers: usize,
    footer: bool,
}
pub(super) fn footer_key() -> ElementId {
    (ElementId::from("table-band"), "footer").into()
}
pub(super) fn header_key() -> ElementId {
    (ElementId::from("table-band"), "header").into()
}
pub(super) fn row_id(scope: &SharedString, key: &TableRowKey) -> ElementId {
    (
        (ElementId::from("table-record"), scope.clone()).into(),
        key.as_str().to_owned(),
    )
        .into()
}
impl<S: TableSource> VirtualizerSource for RowSource<S> {
    fn len(&self) -> usize {
        self.source.as_ref().map_or(0, |s| s.len()) + self.headers + usize::from(self.footer)
    }
    fn key_at(&self, index: usize) -> ElementId {
        if index < self.headers {
            return cells::header_id(index);
        }
        if self.footer && index + 1 == self.len() {
            return footer_key();
        }
        let source = self.source.as_ref().unwrap();
        row_id(
            source.scope(),
            source.row_key(index - self.headers).unwrap(),
        )
    }
    fn index_of(&self, key: &ElementId) -> Option<usize> {
        if let Some(index) = (0..self.headers).find(|index| cells::header_id(*index) == *key) {
            return Some(index);
        }
        if self.footer && *key == footer_key() {
            return Some(self.len() - 1);
        }
        if let ElementId::NamedChild(parent, child) = key {
            let scope: ElementId = (
                ElementId::from("table-record"),
                self.source.as_ref()?.scope().clone(),
            )
                .into();
            if **parent == scope {
                return self
                    .source
                    .as_ref()?
                    .position_of(&TableRowKey::try_new(child.clone()).ok()?)
                    .map(|i| i + self.headers);
            }
        }
        None
    }
    fn revision_at(&self, index: usize) -> u64 {
        self.source.as_ref().map_or(0, |s| {
            s.row_key(index.saturating_sub(self.headers))
                .map_or(s.revision(), |key| s.row_revision(key))
        })
    }
}
struct OneTrack;
impl VirtualizerSource for OneTrack {
    fn len(&self) -> usize {
        1
    }
    fn key_at(&self, _: usize) -> ElementId {
        "table-row-track".into()
    }
    fn index_of(&self, key: &ElementId) -> Option<usize> {
        (*key == ElementId::from("table-row-track")).then_some(0)
    }
}

pub struct YueTableState<D: TableDelegate> {
    pub(super) delegate: gpui_kit::WeakEntity<D>,
    pub(super) accepted: Option<Rc<TableView<D::Source>>>,
    pub(super) presentation: Option<Presentation>,
    pub(super) focus: FocusHandle,
    pub(super) active: Option<TableCellKey>,
    pub(super) control_focus: FocusHandle,
    pub(super) anchor: Option<TableRowKey>,
    pub(super) rows: Entity<VirtualizerState>,
    pub(super) columns: Entity<VirtualizerState>,
    pub(super) scroll: ScrollHandle,
    pub(super) geometry: Option<sizing::Geometry>,
    pub(super) measurements: sizing::Measurements,
    pub(super) resolved: Rc<layout::ResolvedColumns>,
    pub(super) available_width: gpui_kit::Pixels,
    pub(super) resolved_signature: Option<(usize, f32, f32, bool)>,
    pub(super) scroll_target: Option<(TableRowKey, VirtualizerScrollOptions)>,
    pub(super) resize: Option<resize::ResizeSession>,
    pub(super) resize_preview: Option<(TableColumnKey, gpui_kit::Rems)>,
    pub(super) resize_ticket: Option<TableRequestId>,
    pub(super) query_draft: Option<(TableRequestId, TableQuery)>,
    pub(super) edit: Option<TableEditSession>,
    pub(super) edit_sequence: u64,
    pub(super) editor_focuses: Vec<FocusHandle>,
    pub(super) owner: EntityId,
    pub(super) sequence: u64,
    pub(super) pending: Option<TableRequestId>,
    revision: u64,
    pub(super) requests: std::collections::HashSet<TableRequestId>,
    pub(super) repair_control_focus: bool,
    pub(super) span_set: crate::VirtualGridSpanSet,
    pub(super) span_signature: Option<(usize, usize, usize)>,
    pub(super) column_target: Option<(TableColumnKey, VirtualizerScrollOptions)>,
    pub(super) range_pending: Option<TableRangeRequest>,
    pub(super) range_failure: Option<std::ops::Range<usize>>,
    pub(super) tail_seen: Option<SharedString>,
    pub(super) tail_failed: Option<SharedString>,
    _range_subscription: gpui_kit::Subscription,
    pub(super) drag: Option<reorder::DragSession>,
    pub(super) drag_sequence: u64,
    pub(super) static_row_bounds:
        std::collections::HashMap<TableRowKey, gpui_kit::Bounds<gpui_kit::Pixels>>,
    pub(super) table_bounds: Rc<std::cell::Cell<gpui_kit::Bounds<gpui_kit::Pixels>>>,
    pub(super) layout_rem: gpui_kit::Pixels,
}
impl<D: TableDelegate> EventEmitter<TableEvent> for YueTableState<D> {}
impl<D: TableDelegate> Focusable for YueTableState<D> {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl<D: TableDelegate> YueTableState<D> {
    pub fn new(delegate: &Entity<D>, cx: &mut Context<Self>) -> Self {
        let binding = VirtualizerBinding::new();
        let row_state = VirtualizerState::new_attached(
            Axis::Vertical,
            Rc::new(RowSource::<D::Source> {
                source: None,
                headers: 0,
                footer: false,
            }),
            VirtualizerSizing::fixed(px(40.)),
            &binding,
        )
        .expect("valid empty table axis");
        let column_state = VirtualizerState::new_attached(
            Axis::Horizontal,
            Rc::new(OneTrack),
            VirtualizerSizing::fixed(px(1.)),
            &binding,
        )
        .expect("valid table track");
        let rows = cx.new(|_| row_state);
        let range_subscription =
            cx.subscribe(&rows, |s, _, event: &crate::VirtualizerEvent, cx| {
                if let crate::VirtualizerEvent::RangeChanged { rendered, .. } = event {
                    s.observe_range(rendered.clone(), cx);
                }
            });
        Self {
            delegate: delegate.downgrade(),
            accepted: None,
            presentation: None,
            focus: cx.focus_handle(),
            control_focus: cx.focus_handle(),
            active: None,
            anchor: None,
            rows,
            columns: cx.new(|_| column_state),
            scroll: ScrollHandle::new(),
            geometry: None,
            measurements: sizing::Measurements::default(),
            resolved: Rc::default(),
            available_width: px(1024.),
            resolved_signature: None,
            scroll_target: None,
            resize: None,
            resize_preview: None,
            resize_ticket: None,
            query_draft: None,
            edit: None,
            edit_sequence: 0,
            editor_focuses: Vec::new(),
            owner: cx.entity().entity_id(),
            sequence: 0,
            pending: None,
            revision: 0,
            requests: Default::default(),
            repair_control_focus: false,
            span_set: Default::default(),
            span_signature: None,
            column_target: None,
            range_pending: None,
            range_failure: None,
            tail_seen: None,
            tail_failed: None,
            _range_subscription: range_subscription,
            drag: None,
            drag_sequence: 0,
            static_row_bounds: Default::default(),
            table_bounds: Rc::default(),
            layout_rem: px(16.),
        }
    }
    pub fn view(&self) -> Option<&Rc<TableView<D::Source>>> {
        self.accepted.as_ref()
    }
    pub fn active_cell(&self) -> Option<&TableCellKey> {
        self.active.as_ref()
    }
    pub fn pending_request(&self) -> Option<TableRequestId> {
        self.pending
    }
    pub fn scroll_handle(&self) -> &ScrollHandle {
        self.presentation
            .as_ref()
            .and_then(|p| p.host.as_ref())
            .map_or(&self.scroll, |(host, _, _)| host)
    }
    pub fn column_width(&self, key: &TableColumnKey) -> Option<gpui_kit::Pixels> {
        let view = self.view()?;
        self.resolved
            .indices
            .iter()
            .position(|i| view.columns[*i].key() == key)
            .map(|i| self.resolved.widths[i])
    }
    pub fn effective_pin(&self, key: &TableColumnKey) -> Option<TablePin> {
        let view = self.view()?;
        self.resolved
            .indices
            .iter()
            .position(|i| view.columns[*i].key() == key)
            .map(|i| self.resolved.pins[i])
    }
    /// The only publication entry. All fallible validation precedes mutation;
    /// source callbacks cannot reenter the owner (they receive no App/Context).
    pub fn commit_view(
        &mut self,
        mut candidate: TableView<D::Source>,
        response_to: Option<TableRequestId>,
        cx: &mut Context<Self>,
    ) -> Result<TableCommitOutcome, TableError> {
        if response_to
            .is_some_and(|ticket| !self.requests.contains(&ticket) || ticket.owner != self.owner)
        {
            return Err(TableError::StaleRequest);
        }
        if candidate
            .base_revision
            .is_some_and(|revision| revision != self.revision)
        {
            return Err(TableError::StaleView);
        }
        let same_source = self
            .accepted
            .as_ref()
            .is_some_and(|old| Rc::ptr_eq(&old.source, &candidate.source));
        if !same_source
            && self.accepted.as_ref().is_some_and(|old| {
                old.source.scope() == candidate.source.scope()
                    && candidate.source.revision() <= old.source.revision()
            })
        {
            return Err(TableError::StaleSource);
        }
        candidate.validate(!same_source)?;
        if let Some(p) = &self.presentation {
            cells::validate_mode(&candidate, p)?;
        }
        if let Some(session) = &self.edit {
            let old = self.view().unwrap();
            if candidate.source.scope() == old.source.scope()
                && candidate.source.contains_key(session.key().row())
                && (candidate.source.position_of(session.key().row()).is_none()
                    || candidate.query != old.query
                    || !candidate
                        .columns
                        .iter()
                        .any(|column| column.key() == session.key().column() && column.visible))
            {
                return Err(TableError::UnsupportedOperation(
                    "finish editing before changing structure",
                ));
            }
        }
        candidate.page_selection = (
            candidate.source.record_len().saturating_sub(
                candidate
                    .disabled_rows
                    .iter()
                    .filter(|key| candidate.source.position_of(key).is_some())
                    .count(),
            ),
            candidate
                .selection
                .iter()
                .filter(|key| candidate.source.position_of(key).is_some())
                .count(),
        );
        if let Some((id, target)) = &self.query_draft
            && response_to == Some(*id)
            && self
                .accepted
                .as_ref()
                .is_some_and(|old| !target.same_rules(&old.columns, &candidate.columns))
        {
            return Err(TableError::StaleRequest);
        }
        if self
            .accepted
            .as_ref()
            .is_some_and(|old| old.same_inputs(&candidate))
        {
            if let Some(ticket) = response_to {
                self.settle(ticket);
            }
            return Ok(TableCommitOutcome::Unchanged);
        }
        let active = self.active.as_ref().and_then(|active| {
            let same_scope = self
                .accepted
                .as_ref()
                .is_some_and(|old| old.source.scope() == candidate.source.scope());
            if !same_scope
                || !candidate.columns.iter().any(|c| c.visible)
                || candidate.source.is_empty()
            {
                return None;
            }
            let key = if candidate.source.position_of(active.row()).is_some() {
                active.row().clone()
            } else {
                let position = self
                    .accepted
                    .as_ref()?
                    .source
                    .position_of(active.row())?
                    .min(candidate.source.len() - 1);
                (position..candidate.source.len())
                    .chain((0..position).rev())
                    .filter_map(|ix| candidate.source.row_key(ix))
                    .find(|key| !candidate.source.is_detail(key))?
                    .clone()
            };
            let column = candidate
                .columns
                .iter()
                .find(|c| c.visible && c.key() == active.column())
                .unwrap_or_else(|| candidate.columns.iter().find(|c| c.visible).unwrap())
                .key()
                .clone();
            Some(TableCellKey::new(key, column))
        });
        let layout_changed = self.accepted.as_ref().is_none_or(|old| {
            !same_source
                || !Rc::ptr_eq(&old.columns, &candidate.columns)
                || old.selection_mode != candidate.selection_mode
        });
        if layout_changed
            || !same_source
            || self
                .accepted
                .as_ref()
                .is_some_and(|old| old.query != candidate.query)
        {
            self.cancel_resize(cx);
            self.cancel_drag(cx);
        }
        self.revision += 1;
        candidate.revision = self.revision;
        candidate.base_revision = Some(self.revision);
        self.repair_control_focus |= self.active != active;
        self.active = active;
        if self
            .anchor
            .as_ref()
            .is_some_and(|key| candidate.source.position_of(key).is_none())
        {
            self.anchor = None;
        }
        if self
            .accepted
            .as_ref()
            .is_some_and(|old| old.source.scope() != candidate.source.scope())
        {
            self.scroll.set_offset(gpui_kit::point(px(0.), px(0.)));
            self.anchor = None;
            self.scroll_target = None;
        }
        if let Some((id, target)) = &self.query_draft
            && self
                .accepted
                .as_ref()
                .is_some_and(|old| !target.same_rules(&old.columns, &candidate.columns))
        {
            let id = *id;
            self.settle(id);
        }
        let completing_tail = response_to.is_some_and(|id| {
            self.range_pending
                .as_ref()
                .is_some_and(|r| r.id() == id && r.tail_cursor().is_some())
        });
        if (!same_source && !completing_tail)
            || self.accepted.as_ref().is_some_and(|old| {
                old.source.scope() != candidate.source.scope() || old.query != candidate.query
            })
        {
            self.tail_seen = None;
            self.tail_failed = None;
        }
        self.reconcile_edit(&candidate, cx);
        if self.accepted.as_ref().is_some_and(|old| {
            old.query != candidate.query || !Rc::ptr_eq(&old.columns, &candidate.columns)
        }) {
            if let Some(request) = self.range_pending.take() {
                if request.tail_cursor().is_some() && !completing_tail {
                    self.tail_seen = None;
                    self.tail_failed = None;
                }
                self.requests.remove(&request.id);
            }
            self.range_failure = None;
        }
        if !same_source {
            self.span_signature = None;
        }
        self.accepted = Some(Rc::new(candidate));
        // A full external publication invalidates requests prepared for the old source.
        if !same_source {
            self.range_pending = None;
            self.range_failure = None;
            self.pending = None;
            self.requests.clear();
            self.query_draft = None;
        } else if let Some(ticket) = response_to {
            self.settle(ticket);
        }
        if layout_changed {
            self.geometry = None;
        }
        cx.notify();
        Ok(TableCommitOutcome::Applied)
    }
    pub fn reject_request(
        &mut self,
        id: TableRequestId,
        cx: &mut Context<Self>,
    ) -> Result<(), TableError> {
        if !self.requests.contains(&id) {
            return Err(TableError::StaleRequest);
        }
        self.settle(id);
        cx.notify();
        Ok(())
    }
    fn settle(&mut self, id: TableRequestId) {
        if self.range_pending.as_ref().is_some_and(|r| r.id == id) {
            self.range_pending = None;
        }
        self.requests.remove(&id);
        if self
            .query_draft
            .as_ref()
            .is_some_and(|(ticket, _)| *ticket == id)
        {
            self.query_draft = None;
        }
        if self.resize_ticket == Some(id) {
            self.resize_ticket = None;
            self.resize_preview = None;
            self.resolved_signature = None;
        }
        self.pending = self.requests.iter().max_by_key(|id| id.sequence).copied();
    }
    pub fn request_toggle(
        &mut self,
        key: &TableRowKey,
        range: bool,
        cx: &mut Context<Self>,
    ) -> Result<TableRequestId, TableError> {
        let view = self.accepted.as_ref().ok_or(TableError::Unbound)?;
        if view.source.position_of(key).is_none() || view.disabled_rows.contains(key) {
            return Err(TableError::InvalidSelection);
        }
        let intent = if range && view.selection_mode == TableSelectionMode::Multiple {
            SelectionIntent::Range(
                self.anchor.clone().unwrap_or_else(|| key.clone()),
                key.clone(),
            )
        } else {
            SelectionIntent::Toggle(key.clone())
        };
        let id = self.request_selection(intent, cx)?;
        if !range {
            self.anchor = Some(key.clone());
        }
        Ok(id)
    }
    pub fn request_select_all(
        &mut self,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> Result<TableRequestId, TableError> {
        self.request_selection(SelectionIntent::All(selected), cx)
    }
    pub fn request_clear_selection(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Result<TableRequestId, TableError> {
        self.request_selection(SelectionIntent::Clear, cx)
    }
    fn request_selection(
        &mut self,
        intent: SelectionIntent,
        cx: &mut Context<Self>,
    ) -> Result<TableRequestId, TableError> {
        let view = self.accepted.as_ref().ok_or(TableError::Unbound)?;
        if self.presentation.as_ref().is_some_and(|p| p.disabled)
            || matches!(
                view.load,
                TableLoadState::Loading | TableLoadState::Refreshing
            )
        {
            return Err(TableError::InvalidSelection);
        }
        // A host that never settles requests must not grow an unbounded queue.
        if self.requests.len() >= 64 {
            return Err(TableError::TooManyPendingRequests);
        }
        self.sequence += 1;
        let id = TableRequestId {
            owner: self.owner,
            sequence: self.sequence,
        };
        let request = TableSelectionRequest {
            id,
            scope: view.source.scope().clone(),
            intent,
            base_revision: self.revision,
        };
        request.apply_to(view)?;
        self.pending = Some(id);
        self.requests.insert(id);
        cx.emit(TableEvent::SelectionRequested(request));
        Ok(id)
    }
    pub fn scroll_to_row(
        &mut self,
        key: &TableRowKey,
        options: VirtualizerScrollOptions,
        cx: &mut Context<Self>,
    ) -> Result<(), TableError> {
        if self
            .presentation
            .as_ref()
            .is_some_and(|p| p.mode == TableRenderMode::Static)
        {
            return Err(TableError::UnsupportedOperation(
                "Static table scrolls with its containing page",
            ));
        }
        if self
            .accepted
            .as_ref()
            .ok_or(TableError::Unbound)?
            .source
            .position_of(key)
            .is_none()
        {
            return Err(TableError::MissingKey);
        }
        self.scroll_target = Some((key.clone(), options));
        cx.notify();
        Ok(())
    }
    /// Locate both axes without revealing a covered cell as a second origin.
    pub fn scroll_to_cell(
        &mut self,
        key: &TableCellKey,
        options: VirtualizerScrollOptions,
        cx: &mut Context<Self>,
    ) -> Result<(), TableError> {
        let view = self.accepted.as_ref().ok_or(TableError::Unbound)?;
        let key = view.spans.origin_for(view, key);
        if !view
            .columns
            .iter()
            .any(|c| c.visible && c.key() == key.column())
        {
            return Err(TableError::MissingKey);
        }
        self.scroll_to_row(key.row(), options, cx)?;
        if self
            .presentation
            .as_ref()
            .is_some_and(|p| p.mode == TableRenderMode::Cells)
        {
            self.column_target = Some((key.column().clone(), options));
        }
        Ok(())
    }
    pub(super) fn sync_layout(&mut self, p: &Presentation, w: &Window, cx: &mut Context<Self>) {
        self.layout_rem = w.rem_size();
        let Some(view) = self.accepted.as_ref() else {
            return;
        };
        let height = p
            .row_height
            .unwrap_or_else(|| p.size.height(crate::current_theme(cx)))
            .to_pixels(w.rem_size());
        let signature = (
            Rc::as_ptr(&view.columns) as *const () as usize,
            f32::from(w.rem_size()),
            f32::from(self.available_width),
            view.selection_mode != TableSelectionMode::None,
        );
        if self.resolved_signature != Some(signature) {
            let selection = if signature.3 {
                crate::tokens::rems(crate::current_theme(cx), crate::theme::keys::SIZE_48)
                    .to_pixels(w.rem_size())
            } else {
                px(0.)
            };
            let preview = self.resize_preview.as_ref().map(|(key, width)| {
                view.columns
                    .iter()
                    .map(|c| {
                        let mut c = c.clone();
                        if c.key() == key {
                            c.width = *width;
                            c.flex = 0.;
                        }
                        c
                    })
                    .collect::<Vec<_>>()
            });
            self.resolved = Rc::new(layout::ResolvedColumns::resolve(
                preview.as_deref().unwrap_or(&view.columns),
                self.available_width,
                w.rem_size(),
                selection,
            ));
            self.resolved_signature = Some(signature);
        }
        let width = self.resolved.total;
        if view.sizing.is_measured() {
            let theme = crate::current_theme(cx);
            let line = match p.size {
                TableSize::Small => crate::theme::keys::LINE_HEIGHT_SM,
                TableSize::Medium => crate::theme::keys::LINE_HEIGHT_MD,
                TableSize::Large => crate::theme::keys::LINE_HEIGHT_LG,
            };
            let constraints = sizing::MeasurementConstraints {
                scope: view.source.scope().clone(),
                columns: Rc::as_ptr(&view.columns) as *const () as usize,
                widths: self
                    .resolved
                    .indices
                    .iter()
                    .enumerate()
                    .map(|(ix, column)| {
                        (
                            view.columns[*column].key().clone(),
                            f32::from(self.resolved.widths[ix]).to_bits(),
                        )
                    })
                    .collect(),
                rem: f32::from(w.rem_size()).to_bits(),
                minimum: f32::from(height).to_bits(),
                font: crate::tokens::rems(theme, p.size.font()).0.to_bits(),
                line: crate::tokens::rems(theme, line).0.to_bits(),
                padding: crate::tokens::rems(theme, p.size.padding()).0.to_bits(),
                family: w.text_style().font_family.clone(),
            };
            if self.measurements.constraints.as_ref() != Some(&constraints) {
                self.measurements.epoch += 1;
                self.measurements.constraints = Some(constraints);
                self.measurements.sizes.clear();
                self.measurements.pending.clear();
            }
        } else if self.measurements.constraints.take().is_some() {
            self.measurements.epoch += 1;
            self.measurements.sizes.clear();
            self.measurements.pending.clear();
        }
        // Selection/load-only publications keep the source/axis geometry intact.
        let signature = sizing::Geometry {
            mode: p.mode,
            source: view.source.revision(),
            height: f32::from(height),
            width: f32::from(width),
            header: p.header_visible,
            depth: self.resolved.depth,
            footer: view.columns.iter().any(|c| c.footer.is_some()),
            sizing: view.sizing.identity(),
            measurement_epoch: self.measurements.epoch,
        };
        if self.geometry.as_ref() != Some(&signature) {
            self.measurements.sizes.retain(|key, (revision, _, _)| {
                view.source.contains_key(key) && view.source.row_revision(key) == *revision
            });
            let headers = if p.header_visible {
                if p.mode == TableRenderMode::Cells {
                    self.resolved.depth + 1
                } else {
                    1
                }
            } else {
                0
            };
            let source = Rc::new(RowSource {
                source: Some(view.source.clone()),
                headers,
                footer: view.columns.iter().any(|c| c.footer.is_some()),
            });
            self.rows.update(cx, |s, cx| {
                let len = source.len();
                let sizing = if !matches!(view.sizing.kind, sizing::RowSizingKind::Fixed)
                    || p.header_visible && self.resolved.depth > 0
                {
                    let sizes = (0..len)
                        .map(|ix| {
                            if ix < headers {
                                return if p.mode == TableRenderMode::Cells {
                                    height
                                } else {
                                    height * (self.resolved.depth + 1) as f32
                                };
                            }
                            let position = ix - headers;
                            let Some(key) = view.source.row_key(position) else {
                                return height;
                            };
                            match &view.sizing.kind {
                                sizing::RowSizingKind::Fixed => height,
                                sizing::RowSizingKind::Known(heights) => {
                                    heights[key].to_pixels(w.rem_size())
                                }
                                sizing::RowSizingKind::Measured(estimate) => self
                                    .measurements
                                    .sizes
                                    .get(key)
                                    .map(|(_, height, _)| *height)
                                    .unwrap_or_else(|| estimate.to_pixels(w.rem_size())),
                            }
                        })
                        .collect::<Vec<_>>();
                    VirtualizerSizing::known(sizes)
                } else {
                    VirtualizerSizing::fixed(height)
                };
                s.replace_source(
                    source,
                    sizing,
                    VirtualizerChange::Replace {
                        preserve_anchor: true,
                    },
                )
                .expect("validated table rows");
                s.set_layout(VirtualizerLayout::default().overscan(height * 2.))
                    .expect("finite row geometry");
                cx.notify();
            });
            self.columns.update(cx, |s, cx| {
                let (source, sizing): (Rc<dyn VirtualizerSource>, VirtualizerSizing) =
                    if p.mode == TableRenderMode::Cells {
                        let source = cells::ColumnSource::new(view, &self.resolved);
                        let sizes = source.sizes.clone();
                        (Rc::new(source), VirtualizerSizing::known(sizes))
                    } else {
                        (
                            Rc::new(OneTrack),
                            if view.sizing.is_measured() {
                                VirtualizerSizing::known(vec![width.max(px(1.))])
                            } else {
                                VirtualizerSizing::fixed(width.max(px(1.)))
                            },
                        )
                    };
                s.replace_source(
                    source,
                    sizing,
                    VirtualizerChange::Replace {
                        preserve_anchor: true,
                    },
                )
                .expect("validated table columns");
                s.set_layout(VirtualizerLayout::default().overscan(height * 2.))
                    .unwrap();
                cx.notify();
            });
            self.geometry = Some(signature);
        }
        if let Some((key, options)) = self.scroll_target.take() {
            self.rows.update(cx, |s, cx| {
                let _ = s.scroll_to_key(row_id(view.source.scope(), &key), options);
                cx.notify();
            });
        }
        if let Some((key, options)) = self.column_target.take() {
            self.columns.update(cx, |s, cx| {
                let _ = s.scroll_to_key(key.element_id(), options);
                cx.notify();
            });
        }
    }
    pub(super) fn set_active(
        &mut self,
        key: TableCellKey,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = self
            .accepted
            .as_ref()
            .map_or_else(|| key.clone(), |v| v.spans.origin_for(v, &key));
        if self.active.as_ref() != Some(&key) {
            self.active = Some(key.clone());
            cx.emit(TableEvent::ActiveChanged(Some(key.clone())));
        }
        self.focus.focus(window, cx);
        let _ = self.scroll_to_row(key.row(), VirtualizerScrollOptions::default(), cx);
        if self
            .presentation
            .as_ref()
            .is_some_and(|p| p.mode == TableRenderMode::Cells)
        {
            self.column_target = Some((key.column().clone(), VirtualizerScrollOptions::default()));
        }
    }
}
impl<D: TableDelegate> Render for YueTableState<D> {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        element::render(self, w, cx)
    }
}
