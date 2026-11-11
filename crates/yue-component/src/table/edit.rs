use super::*;
use gpui_kit::{Context, EntityId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableEditMode {
    Cell,
    Row,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TableEditMove {
    #[default]
    Stay,
    Next,
    Previous,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TableEditStage {
    Editing,
    Validating,
    Saving,
    Error(SharedString),
    Conflict,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableEditToken {
    owner: EntityId,
    session: u64,
    draft: u64,
}
#[derive(Clone, Debug)]
pub struct TableEditSession {
    token: TableEditToken,
    key: TableCellKey,
    mode: TableEditMode,
    stage: TableEditStage,
    scope: SharedString,
    revision: u64,
    rules: Vec<(TableColumnKey, u64)>,
    movement: TableEditMove,
}
impl TableEditSession {
    pub fn token(&self) -> &TableEditToken {
        &self.token
    }
    pub fn key(&self) -> &TableCellKey {
        &self.key
    }
    pub fn mode(&self) -> TableEditMode {
        self.mode
    }
    pub fn stage(&self) -> &TableEditStage {
        &self.stage
    }
    pub fn base_revision(&self) -> u64 {
        self.revision
    }
    pub fn includes(&self, key: &TableCellKey) -> bool {
        key.row() == self.key.row()
            && (self.mode == TableEditMode::Row
                && self.rules.iter().any(|(column, _)| column == key.column())
                || key == &self.key)
    }
}
impl<D: TableDelegate> YueTableState<D> {
    pub fn edit_session(&self) -> Option<&TableEditSession> {
        self.edit.as_ref()
    }
    /// Register the actual retained controls so deletion can repair focus.
    pub fn set_edit_focus_handles(
        &mut self,
        handles: Vec<gpui_kit::FocusHandle>,
        cx: &mut Context<Self>,
    ) -> Result<(), TableError> {
        if self.edit.is_none() || handles.len() > 64 {
            return Err(TableError::UnsupportedOperation(
                "invalid editor focus handles",
            ));
        }
        self.editor_focuses = handles;
        cx.notify();
        Ok(())
    }
    pub fn request_edit(
        &mut self,
        key: TableCellKey,
        mode: TableEditMode,
        cx: &mut Context<Self>,
    ) -> Result<(), TableError> {
        #[cfg(feature = "table-profile")]
        let _perf = gpui_kit::perf_stats::scope("table.edit.request_edit");
        self.validate_edit(&key)?;
        if self.edit.is_some() {
            return Err(TableError::UnsupportedOperation("finish the current edit"));
        }
        cx.emit(TableEvent::EditRequested { key, mode });
        Ok(())
    }
    fn validate_edit(&self, key: &TableCellKey) -> Result<(), TableError> {
        let view = self.view().ok_or(TableError::Unbound)?;
        if self
            .presentation
            .as_ref()
            .is_some_and(|p| p.disabled || p.readonly)
            || matches!(
                view.load,
                TableLoadState::Loading | TableLoadState::Refreshing
            )
            || view.disabled_rows.contains(key.row())
        {
            return Err(TableError::UnsupportedOperation("editing is unavailable"));
        }
        if view.source.position_of(key.row()).is_none()
            || view.source.is_detail(key.row())
            || view.source.is_placeholder(key.row())
            || !view
                .columns
                .iter()
                .any(|column| column.key() == key.column() && column.editable && column.visible)
        {
            return Err(TableError::MissingKey);
        }
        Ok(())
    }
    /// Hosts create/initialize real controls first, then accept the edit session.
    pub fn start_edit(
        &mut self,
        key: TableCellKey,
        mode: TableEditMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<TableEditToken, TableError> {
        #[cfg(feature = "table-profile")]
        let _perf = gpui_kit::perf_stats::scope("table.edit.start_edit");
        self.validate_edit(&key)?;
        if self.edit.is_some() {
            return Err(TableError::UnsupportedOperation("finish the current edit"));
        }
        self.cancel_resize(cx);
        let view = self.view().unwrap();
        let rules = view
            .columns
            .iter()
            .filter(|column| {
                column.editable && (mode == TableEditMode::Row || column.key() == key.column())
            })
            .map(|column| (column.key().clone(), column.edit_revision))
            .collect();
        let scope = view.source.scope().clone();
        let revision = view.source.row_revision(key.row());
        self.edit_sequence += 1;
        let token = TableEditToken {
            owner: self.owner,
            session: self.edit_sequence,
            draft: 0,
        };
        self.edit = Some(TableEditSession {
            token: token.clone(),
            key: key.clone(),
            mode,
            stage: TableEditStage::Editing,
            scope,
            revision,
            rules,
            movement: TableEditMove::Stay,
        });
        self.set_active(key, window, cx);
        self.control_focus.focus(window, cx);
        cx.notify();
        Ok(token)
    }
    /// Call from the retained controls' Change events; values stay in those controls.
    pub fn edit_draft_changed(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Result<TableEditToken, TableError> {
        let session = self.edit.as_mut().ok_or(TableError::StaleRequest)?;
        if session.stage == TableEditStage::Conflict {
            return Err(TableError::StaleSource);
        }
        session.token.draft += 1;
        session.stage = TableEditStage::Editing;
        cx.notify();
        Ok(session.token.clone())
    }
    pub fn is_current_edit(&self, token: &TableEditToken) -> bool {
        self.edit.as_ref().is_some_and(|session| {
            &session.token == token && session.stage != TableEditStage::Conflict
        })
    }
    /// The host reads current Input/Select values when handling this event.
    pub fn request_edit_save(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Result<TableEditToken, TableError> {
        #[cfg(feature = "table-profile")]
        let _perf = gpui_kit::perf_stats::scope("table.edit.request_edit_save");
        self.request_edit_save_with_move(TableEditMove::Stay, cx)
    }
    pub fn request_edit_save_with_move(
        &mut self,
        movement: TableEditMove,
        cx: &mut Context<Self>,
    ) -> Result<TableEditToken, TableError> {
        let session = self.edit.as_mut().ok_or(TableError::StaleRequest)?;
        if !matches!(
            session.stage,
            TableEditStage::Editing | TableEditStage::Error(_)
        ) {
            return Err(TableError::UnsupportedOperation(
                "edit is not ready to save",
            ));
        }
        session.stage = TableEditStage::Validating;
        // A retry of unchanged text is a new attempt too. Old validation/save
        // completions must not settle that attempt merely because the text matches.
        session.token.draft += 1;
        session.movement = movement;
        let token = session.token.clone();
        cx.emit(TableEvent::EditSaveRequested(token.clone()));
        cx.notify();
        Ok(token)
    }
    pub fn accept_edit_validation(
        &mut self,
        token: &TableEditToken,
        cx: &mut Context<Self>,
    ) -> Result<(), TableError> {
        if !self.is_current_edit(token)
            || self.edit.as_ref().unwrap().stage != TableEditStage::Validating
        {
            return Err(TableError::StaleRequest);
        }
        self.edit.as_mut().unwrap().stage = TableEditStage::Saving;
        cx.notify();
        Ok(())
    }
    pub fn reject_edit(
        &mut self,
        token: &TableEditToken,
        error: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Result<(), TableError> {
        if !self.is_current_edit(token)
            || !matches!(
                self.edit.as_ref().unwrap().stage,
                TableEditStage::Validating | TableEditStage::Saving
            )
        {
            return Err(TableError::StaleRequest);
        }
        self.edit.as_mut().unwrap().stage = TableEditStage::Error(error.into());
        cx.notify();
        Ok(())
    }
    pub fn cancel_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.edit.take().is_some() {
            self.editor_focuses.clear();
            self.focus.focus(window, cx);
            cx.emit(TableEvent::EditEnded);
            cx.notify();
        }
    }
    /// Accept only this session/draft's save. Failed commits retain the editor.
    pub fn commit_edit(
        &mut self,
        token: &TableEditToken,
        candidate: TableView<D::Source>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<TableCommitOutcome, TableError> {
        #[cfg(feature = "table-profile")]
        let _perf = gpui_kit::perf_stats::scope("table.edit.commit_edit");
        if !self.is_current_edit(token)
            || self.edit.as_ref().unwrap().stage != TableEditStage::Saving
        {
            return Err(TableError::StaleRequest);
        }
        let session = self.edit.take().unwrap();
        if candidate.source.scope() != &session.scope
            || candidate.source.position_of(session.key.row()).is_none()
            || candidate.source.is_detail(session.key.row())
        {
            self.edit = Some(session);
            return Err(TableError::StaleSource);
        }
        let result = self.commit_view(candidate, None, cx);
        match result {
            Ok(outcome) => {
                self.editor_focuses.clear();
                self.focus.focus(window, cx);
                cx.emit(TableEvent::EditEnded);
                if let Some(key) = next_edit(self.view().unwrap(), &session) {
                    let _ = self.request_edit(key, session.mode, cx);
                }
                Ok(outcome)
            }
            Err(error) => {
                self.edit = Some(session);
                Err(error)
            }
        }
    }
    pub(super) fn reconcile_edit(
        &mut self,
        candidate: &TableView<D::Source>,
        cx: &mut Context<Self>,
    ) {
        #[cfg(feature = "table-profile")]
        let _perf = gpui_kit::perf_stats::scope("table.edit.reconcile_edit");
        let Some(session) = &mut self.edit else {
            return;
        };
        if candidate.source.scope() != &session.scope
            || !candidate.source.contains_key(session.key.row())
        {
            self.edit = None;
            self.repair_control_focus = true;
            cx.emit(TableEvent::EditEnded);
            return;
        }
        if candidate.source.row_revision(session.key.row()) != session.revision
            || session.rules.iter().any(|(key, revision)| {
                !candidate.columns.iter().any(|column| {
                    column.key() == key && column.editable && column.edit_revision == *revision
                })
            })
        {
            session.stage = TableEditStage::Conflict;
        }
    }
}

fn next_edit<S: TableSource>(
    view: &TableView<S>,
    session: &TableEditSession,
) -> Option<TableCellKey> {
    if session.movement == TableEditMove::Stay {
        return None;
    }
    let columns = view
        .columns
        .iter()
        .filter(|column| column.visible && column.editable)
        .collect::<Vec<_>>();
    let mut row = view.source.position_of(session.key.row())?;
    let mut column = columns
        .iter()
        .position(|column| column.key() == session.key.column())?;
    let forward = session.movement == TableEditMove::Next;
    if session.mode == TableEditMode::Cell
        && ((forward && column + 1 < columns.len()) || (!forward && column > 0))
    {
        column = if forward { column + 1 } else { column - 1 };
    } else {
        loop {
            row = if forward {
                row.checked_add(1).filter(|ix| *ix < view.source.len())?
            } else {
                row.checked_sub(1)?
            };
            let key = view.source.row_key(row)?;
            if !view.source.is_detail(key) && !view.disabled_rows.contains(key) {
                break;
            }
        }
        column = if forward || session.mode == TableEditMode::Row {
            0
        } else {
            columns.len() - 1
        };
    }
    Some(TableCellKey::new(
        view.source.row_key(row)?.clone(),
        columns[column].key().clone(),
    ))
}
