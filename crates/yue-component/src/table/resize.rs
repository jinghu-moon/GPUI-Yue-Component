use super::*;
use gpui_kit::{Context, Pixels, px};

#[derive(Clone, Debug)]
pub struct TableColumnResizeRequest {
    id: TableRequestId,
    scope: SharedString,
    revision: u64,
    key: TableColumnKey,
    width: Rems,
}
impl TableColumnResizeRequest {
    pub fn id(&self) -> TableRequestId {
        self.id
    }
    pub fn column(&self) -> &TableColumnKey {
        &self.key
    }
    pub fn width(&self) -> Rems {
        self.width
    }
    pub fn apply_to<S: TableSource>(
        &self,
        accepted: &TableView<S>,
    ) -> Result<TableView<S>, TableError> {
        if self.scope != *accepted.source.scope() || self.revision != accepted.revision {
            return Err(TableError::StaleRequest);
        }
        let mut columns = accepted.columns.to_vec();
        let column = columns
            .iter_mut()
            .find(|c| c.key() == &self.key)
            .ok_or(TableError::MissingKey)?;
        if !column.resizable || !column.visible {
            return Err(TableError::UnsupportedCombination(
                "column is not resizable",
            ));
        }
        column.width = self.width;
        column.flex = 0.;
        Ok(accepted
            .clone()
            .with_columns(columns)
            .based_on(accepted.revision))
    }
}
pub(super) struct ResizeSession {
    key: TableColumnKey,
    start: Pixels,
    pointer: Option<Pixels>,
    current: Pixels,
    minimum: Pixels,
    maximum: Pixels,
    rem: Pixels,
    held_key: Option<SharedString>,
}
pub(super) struct ResizeLifetime {
    pub _activation: gpui_kit::Subscription,
}

impl<D: TableDelegate> YueTableState<D> {
    pub(super) fn begin_resize(
        &mut self,
        key: &TableColumnKey,
        pointer: Option<Pixels>,
        w: &Window,
        cx: &mut Context<Self>,
    ) -> Result<(), TableError> {
        if self.resize.as_ref().is_some_and(|s| &s.key == key) {
            return Ok(());
        }
        if self
            .presentation
            .as_ref()
            .is_some_and(|p| p.disabled || p.readonly)
        {
            return Err(TableError::UnsupportedCombination(
                "table is disabled or readonly",
            ));
        }
        let view = self.view().ok_or(TableError::Unbound)?;
        if matches!(
            view.load,
            TableLoadState::Loading | TableLoadState::Refreshing
        ) {
            return Err(TableError::UnsupportedCombination("table is busy"));
        }
        let column = view
            .columns
            .iter()
            .find(|c| c.key() == key)
            .ok_or(TableError::MissingKey)?;
        if !column.visible || !column.resizable {
            return Err(TableError::UnsupportedCombination(
                "column is not resizable",
            ));
        }
        let start = self.column_width(key).ok_or(TableError::MissingKey)?;
        self.resize = Some(ResizeSession {
            key: key.clone(),
            start,
            pointer,
            current: start,
            minimum: column.minimum.to_pixels(w.rem_size()),
            maximum: column.maximum.to_pixels(w.rem_size()),
            rem: w.rem_size(),
            held_key: None,
        });
        self.resize_preview = None;
        cx.notify();
        Ok(())
    }
    pub(super) fn pointer_resize(&mut self, x: Pixels, cx: &mut Context<Self>) {
        let Some(s) = &mut self.resize else {
            return;
        };
        let Some(origin) = s.pointer else {
            return;
        };
        let next = (s.start + x - origin).clamp(s.minimum, s.maximum);
        if !f32::from(next).is_finite() || next == s.current {
            return;
        }
        s.current = next;
        self.resize_preview = Some((s.key.clone(), rems(f32::from(next) / f32::from(s.rem))));
        self.resolved_signature = None;
        cx.notify();
    }
    pub(super) fn keyboard_resize(
        &mut self,
        key: &TableColumnKey,
        event: &gpui_kit::KeyDownEvent,
        w: &Window,
        cx: &mut Context<Self>,
    ) {
        let k = event.keystroke.key.as_str();
        if k == "escape" {
            self.cancel_resize(cx);
            cx.stop_propagation();
            return;
        }
        if !matches!(k, "left" | "right") {
            return;
        }
        if self.begin_resize(key, None, w, cx).is_err() {
            return;
        }
        let s = self.resize.as_mut().unwrap();
        let token = if event.keystroke.modifiers.shift {
            "--resizable-keyboard-step-large"
        } else {
            "--resizable-keyboard-step"
        };
        let step = crate::tokens::rems(crate::current_theme(cx), token).to_pixels(w.rem_size());
        s.current =
            (s.current + step * if k == "left" { -1. } else { 1. }).clamp(s.minimum, s.maximum);
        s.held_key = Some(event.keystroke.key.clone().into());
        self.resize_preview = Some((s.key.clone(), rems(f32::from(s.current) / f32::from(s.rem))));
        self.resolved_signature = None;
        cx.stop_propagation();
        cx.notify();
    }
    pub(super) fn resize_key_up(&mut self, key: &str, cx: &mut Context<Self>) {
        if self
            .resize
            .as_ref()
            .is_some_and(|s| s.held_key.as_ref().is_some_and(|k| k.as_str() == key))
        {
            self.finish_resize(cx);
            cx.stop_propagation();
        }
    }
    pub(super) fn resize_blur(&mut self, key: &TableColumnKey, cx: &mut Context<Self>) {
        if self
            .resize
            .as_ref()
            .is_some_and(|s| &s.key == key && s.pointer.is_none())
        {
            self.finish_resize(cx);
        }
    }
    pub(super) fn has_pointer_resize(&self) -> bool {
        self.resize.as_ref().is_some_and(|s| s.pointer.is_some())
    }
    pub(super) fn finish_resize(&mut self, cx: &mut Context<Self>) {
        let Some(s) = self.resize.take() else {
            return;
        };
        if (s.current - s.start).abs() < px(0.001) || self.requests.len() >= 64 {
            self.cancel_resize(cx);
            return;
        }
        let Some(view) = self.view() else {
            self.cancel_resize(cx);
            return;
        };
        let scope = view.source.scope().clone();
        let revision = view.revision;
        self.sequence += 1;
        let id = TableRequestId {
            owner: self.owner,
            sequence: self.sequence,
        };
        self.pending = Some(id);
        self.requests.insert(id);
        self.resize_ticket = Some(id);
        // The pointer/key session is over. Pending is intent, not accepted width;
        // asynchronous hosts keep drawing the committed configuration.
        self.resize_preview = None;
        self.resolved_signature = None;
        cx.emit(TableEvent::ColumnResizeRequested(
            TableColumnResizeRequest {
                id,
                scope,
                revision,
                key: s.key,
                width: rems(f32::from(s.current) / f32::from(s.rem)),
            },
        ));
        cx.notify();
    }
    pub(super) fn cancel_resize(&mut self, cx: &mut Context<Self>) {
        if self.resize.take().is_some() || self.resize_preview.take().is_some() {
            self.resize_preview = None;
            self.resolved_signature = None;
            cx.notify();
        }
    }
}

pub(super) struct HandleLifetime {
    pub focus: gpui_kit::FocusHandle,
    pub _blur: gpui_kit::Subscription,
}
pub(super) struct DragMarker;
impl gpui_kit::Render for DragMarker {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl gpui_kit::IntoElement {
        gpui_kit::div()
    }
}
