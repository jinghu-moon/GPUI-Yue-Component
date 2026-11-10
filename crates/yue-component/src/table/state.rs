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
}
struct RowSource<S: TableSource> {
    source: Option<Rc<S>>,
    header: bool,
    footer: bool,
}
fn footer_key() -> ElementId {
    (ElementId::from("table-band"), "footer").into()
}
fn header_key() -> ElementId {
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
        self.source.as_ref().map_or(0, |s| s.len())
            + usize::from(self.header)
            + usize::from(self.footer)
    }
    fn key_at(&self, index: usize) -> ElementId {
        if self.header && index == 0 {
            return header_key();
        }
        if self.footer && index + 1 == self.len() {
            return footer_key();
        }
        let source = self.source.as_ref().unwrap();
        row_id(
            source.scope(),
            source.row_key(index - usize::from(self.header)).unwrap(),
        )
    }
    fn index_of(&self, key: &ElementId) -> Option<usize> {
        if self.header && *key == header_key() {
            return Some(0);
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
                    .map(|i| i + usize::from(self.header));
            }
        }
        None
    }
    fn revision_at(&self, _: usize) -> u64 {
        self.source.as_ref().map_or(0, |s| s.revision())
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
    pub(super) geometry: Option<(u64, f32, f32, bool)>,
    pub(super) scroll_target: Option<(TableRowKey, VirtualizerScrollOptions)>,
    owner: EntityId,
    sequence: u64,
    pending: Option<TableRequestId>,
    revision: u64,
    requests: std::collections::HashSet<TableRequestId>,
    pub(super) repair_control_focus: bool,
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
                header: false,
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
        Self {
            delegate: delegate.downgrade(),
            accepted: None,
            presentation: None,
            focus: cx.focus_handle(),
            control_focus: cx.focus_handle(),
            active: None,
            anchor: None,
            rows: cx.new(|_| row_state),
            columns: cx.new(|_| column_state),
            scroll: ScrollHandle::new(),
            geometry: None,
            scroll_target: None,
            owner: cx.entity().entity_id(),
            sequence: 0,
            pending: None,
            revision: 0,
            requests: Default::default(),
            repair_control_focus: false,
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
        &self.scroll
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
            if !same_scope || candidate.columns.is_empty() || candidate.source.is_empty() {
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
                candidate.source.row_key(position)?.clone()
            };
            let column = candidate
                .columns
                .iter()
                .find(|c| c.key() == active.column())
                .unwrap_or(&candidate.columns[0])
                .key()
                .clone();
            Some(TableCellKey::new(key, column))
        });
        let layout_changed = self.accepted.as_ref().is_none_or(|old| {
            !same_source
                || !Rc::ptr_eq(&old.columns, &candidate.columns)
                || old.selection_mode != candidate.selection_mode
        });
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
        self.accepted = Some(Rc::new(candidate));
        // A full external publication invalidates requests prepared for the old source.
        if !same_source {
            self.pending = None;
            self.requests.clear();
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
        self.requests.remove(&id);
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
    pub(super) fn sync_layout(&mut self, p: &Presentation, w: &Window, cx: &mut Context<Self>) {
        let Some(view) = self.accepted.as_ref() else {
            return;
        };
        let height = p
            .row_height
            .unwrap_or_else(|| p.size.height(crate::current_theme(cx)))
            .to_pixels(w.rem_size());
        let width = view
            .columns
            .iter()
            .map(|c| c.width.to_pixels(w.rem_size()))
            .fold(px(0.), |a, b| a + b)
            + if view.selection_mode != TableSelectionMode::None {
                crate::tokens::rems(crate::current_theme(cx), crate::theme::keys::SIZE_48)
                    .to_pixels(w.rem_size())
            } else {
                px(0.)
            };
        // Selection/load-only publications keep the source/axis geometry intact.
        let signature = (
            view.source.revision(),
            f32::from(height),
            f32::from(width),
            p.header_visible,
        );
        if self.geometry != Some(signature) {
            let source = Rc::new(RowSource {
                source: Some(view.source.clone()),
                header: p.header_visible,
                footer: view.columns.iter().any(|c| c.footer.is_some()),
            });
            self.rows.update(cx, |s, cx| {
                s.replace_source(
                    source,
                    VirtualizerSizing::fixed(height),
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
                s.replace_source(
                    Rc::new(OneTrack),
                    VirtualizerSizing::fixed(width.max(px(1.))),
                    VirtualizerChange::Replace {
                        preserve_anchor: true,
                    },
                )
                .expect("validated table track");
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
    }
    pub(super) fn set_active(
        &mut self,
        key: TableCellKey,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.active.as_ref() != Some(&key) {
            self.active = Some(key.clone());
            cx.emit(TableEvent::ActiveChanged(Some(key.clone())));
        }
        self.focus.focus(window, cx);
        let _ = self.scroll_to_row(key.row(), VirtualizerScrollOptions::default(), cx);
    }
}
impl<D: TableDelegate> Render for YueTableState<D> {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        element::render(self, w, cx)
    }
}
