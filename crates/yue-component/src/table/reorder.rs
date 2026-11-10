use super::*;
use gpui_kit::{Bounds, Context, Pixels, Point, px};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TableRowDrag {
    #[default]
    None,
    Handle,
    Row,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TableRowMoveScope {
    #[default]
    FullResult,
    MappedVisibleResult,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableMovePlacement {
    Before,
    After,
}
#[derive(Clone, Debug)]
pub struct TableRowMoveRequest {
    id: TableRequestId,
    scope: SharedString,
    source: u64,
    revision: u64,
    key: TableRowKey,
    target: TableRowKey,
    placement: TableMovePlacement,
}
fn move_key<K: Clone + PartialEq>(
    keys: &mut Vec<K>,
    key: &K,
    target: &K,
    placement: TableMovePlacement,
) -> Result<(), TableError> {
    let from = keys
        .iter()
        .position(|k| k == key)
        .ok_or(TableError::MissingKey)?;
    if key == target {
        return Ok(());
    }
    let value = keys.remove(from);
    let to = keys
        .iter()
        .position(|k| k == target)
        .ok_or(TableError::MissingKey)?
        + usize::from(placement == TableMovePlacement::After);
    keys.insert(to, value);
    Ok(())
}
impl TableRowMoveRequest {
    pub fn id(&self) -> TableRequestId {
        self.id
    }
    pub fn key(&self) -> &TableRowKey {
        &self.key
    }
    pub fn target(&self) -> &TableRowKey {
        &self.target
    }
    pub fn placement(&self) -> TableMovePlacement {
        self.placement
    }
    pub fn ordered_keys<S: TableSource>(
        &self,
        view: &TableView<S>,
    ) -> Result<Vec<TableRowKey>, TableError> {
        if self.scope != *view.source.scope()
            || self.source != view.source.revision()
            || self.revision != view.revision()
        {
            return Err(TableError::StaleRequest);
        }
        let mut keys = (0..view.source.len())
            .filter_map(|i| view.source.row_key(i).cloned())
            .collect();
        move_key(&mut keys, &self.key, &self.target, self.placement)?;
        Ok(keys)
    }
    pub fn apply_to_rows<R: 'static>(
        &self,
        view: &TableView<TableRows<R>>,
    ) -> Result<TableView<TableRows<R>>, TableError> {
        let keys = self.ordered_keys(view)?;
        let mut next = view.clone();
        next.source = Rc::new(view.source.reordered(view.source.revision() + 1, &keys)?);
        Ok(next.based_on(view.revision()))
    }
}
#[derive(Clone, Debug)]
pub struct TableColumnMoveRequest {
    id: TableRequestId,
    scope: SharedString,
    revision: u64,
    key: TableColumnKey,
    target: TableColumnKey,
    placement: TableMovePlacement,
}
impl TableColumnMoveRequest {
    pub fn id(&self) -> TableRequestId {
        self.id
    }
    pub fn key(&self) -> &TableColumnKey {
        &self.key
    }
    pub fn target(&self) -> &TableColumnKey {
        &self.target
    }
    pub fn placement(&self) -> TableMovePlacement {
        self.placement
    }
    pub fn apply_to<S: TableSource>(
        &self,
        view: &TableView<S>,
    ) -> Result<TableView<S>, TableError> {
        if self.scope != *view.source.scope() || self.revision != view.revision() {
            return Err(TableError::StaleRequest);
        }
        validate_column_move(view, &self.key, &self.target)?;
        let mut keys = view.columns.iter().map(|c| c.key().clone()).collect();
        move_key(&mut keys, &self.key, &self.target, self.placement)?;
        let prefs = (*view.preferences).clone().with_order(keys);
        prefs.apply_to(view)
    }
}
fn validate_column_move<S: TableSource>(
    view: &TableView<S>,
    key: &TableColumnKey,
    target: &TableColumnKey,
) -> Result<(), TableError> {
    let a = view
        .columns
        .iter()
        .find(|c| c.key() == key)
        .ok_or(TableError::MissingKey)?;
    let b = view
        .columns
        .iter()
        .find(|c| c.key() == target)
        .ok_or(TableError::MissingKey)?;
    if !a.visible
        || !b.visible
        || !a.allowed
        || !b.allowed
        || !a.configurable
        || !b.configurable
        || !a.reorderable
        || !b.reorderable
        || a.pin != b.pin
        || a.groups != b.groups
    {
        return Err(TableError::UnsupportedOperation(
            "column move crosses permission, group or pin boundary",
        ));
    }
    let from = view.columns.iter().position(|c| c.key() == key).unwrap();
    let to = view.columns.iter().position(|c| c.key() == target).unwrap();
    if view.columns[from.min(to)..=from.max(to)]
        .iter()
        .any(|c| !c.allowed || !c.configurable || !c.reorderable)
    {
        return Err(TableError::UnsupportedOperation(
            "column move crosses protected column",
        ));
    }
    Ok(())
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum MoveKey {
    Row(TableRowKey),
    Column(TableColumnKey),
}
#[derive(Clone)]
pub(super) struct DragPreview {
    pub target: MoveKey,
    pub placement: TableMovePlacement,
}
pub(super) struct DragSession {
    pub key: MoveKey,
    pub target: Option<DragPreview>,
    pub pointer: Option<Point<Pixels>>,
    pub start: Option<Point<Pixels>>,
    pub moved: bool,
    pub sequence: u64,
    pub frame_pending: bool,
}
impl<D: TableDelegate> YueTableState<D> {
    fn move_allowed(&self) -> Result<(), TableError> {
        if self
            .view()
            .is_some_and(|v| matches!(v.load, TableLoadState::Loading | TableLoadState::Refreshing))
            || self.edit.is_some()
            || self
                .presentation
                .as_ref()
                .is_some_and(|p| p.disabled || p.readonly)
            || self.resize.is_some()
        {
            return Err(TableError::UnsupportedOperation(
                "reordering is unavailable",
            ));
        }
        if self.requests.len() >= 64 {
            return Err(TableError::TooManyPendingRequests);
        }
        Ok(())
    }
    fn validate_row_move(&self, key: &TableRowKey, target: &TableRowKey) -> Result<(), TableError> {
        self.move_allowed()?;
        let v = self.view().ok_or(TableError::Unbound)?;
        let p = self.presentation.as_ref().ok_or(TableError::Unbound)?;
        if p.row_drag == TableRowDrag::None
            || !v.source.can_move_rows()
            || !v.query.sorts().is_empty()
            || ((v.query.has_filters() || v.query.page().is_some())
                && p.row_move_scope != TableRowMoveScope::MappedVisibleResult)
        {
            return Err(TableError::UnsupportedOperation(
                "row order cannot be mapped to this query",
            ));
        }
        for k in [key, target] {
            let i = v.source.position_of(k).ok_or(TableError::MissingKey)?;
            if v.source.is_detail(k)
                || v.source.is_placeholder(k)
                || v.disabled_rows.contains(k)
                || i < p.frozen.0
                || i >= v.source.len().saturating_sub(p.frozen.1)
            {
                return Err(TableError::UnsupportedOperation(
                    "row is not a movable target",
                ));
            }
        }
        Ok(())
    }
    pub fn request_row_move(
        &mut self,
        key: &TableRowKey,
        target: &TableRowKey,
        placement: TableMovePlacement,
        cx: &mut Context<Self>,
    ) -> Result<Option<TableRequestId>, TableError> {
        self.validate_row_move(key, target)?;
        let v = self.view().unwrap();
        let a = v.source.position_of(key).unwrap();
        let b = v.source.position_of(target).unwrap();
        if key == target
            || (placement == TableMovePlacement::Before && a + 1 == b)
            || (placement == TableMovePlacement::After && b + 1 == a)
        {
            return Ok(None);
        }
        let (scope, source, revision) =
            (v.source.scope().clone(), v.source.revision(), v.revision());
        self.sequence += 1;
        let id = TableRequestId {
            owner: self.owner,
            sequence: self.sequence,
        };
        self.requests.insert(id);
        self.pending = Some(id);
        cx.emit(TableEvent::RowMoveRequested(TableRowMoveRequest {
            id,
            scope,
            source,
            revision,
            key: key.clone(),
            target: target.clone(),
            placement,
        }));
        Ok(Some(id))
    }
    pub fn request_column_move(
        &mut self,
        key: &TableColumnKey,
        target: &TableColumnKey,
        placement: TableMovePlacement,
        cx: &mut Context<Self>,
    ) -> Result<Option<TableRequestId>, TableError> {
        self.move_allowed()?;
        let v = self.view().ok_or(TableError::Unbound)?;
        validate_column_move(v, key, target)?;
        let a = v.columns.iter().position(|c| c.key() == key).unwrap();
        let b = v.columns.iter().position(|c| c.key() == target).unwrap();
        if key == target
            || (placement == TableMovePlacement::Before && a + 1 == b)
            || (placement == TableMovePlacement::After && b + 1 == a)
        {
            return Ok(None);
        }
        let (scope, revision) = (v.source.scope().clone(), v.revision());
        self.sequence += 1;
        let id = TableRequestId {
            owner: self.owner,
            sequence: self.sequence,
        };
        self.requests.insert(id);
        self.pending = Some(id);
        cx.emit(TableEvent::ColumnMoveRequested(TableColumnMoveRequest {
            id,
            scope,
            revision,
            key: key.clone(),
            target: target.clone(),
            placement,
        }));
        Ok(Some(id))
    }
    pub(super) fn begin_drag(
        &mut self,
        key: MoveKey,
        pointer: Option<Point<Pixels>>,
        cx: &mut Context<Self>,
    ) -> Result<(), TableError> {
        match &key {
            MoveKey::Row(k) => self.validate_row_move(k, k)?,
            MoveKey::Column(k) => {
                self.move_allowed()?;
                validate_column_move(self.view().ok_or(TableError::Unbound)?, k, k)?;
            }
        }
        self.cancel_drag(cx);
        self.drag_sequence += 1;
        self.drag = Some(DragSession {
            key,
            target: None,
            start: pointer,
            pointer,
            moved: pointer.is_none(),
            sequence: self.drag_sequence,
            frame_pending: false,
        });
        cx.notify();
        Ok(())
    }
    pub(super) fn cancel_drag(&mut self, cx: &mut Context<Self>) {
        if self.drag.take().is_some() {
            cx.notify();
        }
    }
    pub(super) fn finish_drag(&mut self, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.take() else { return };
        if drag.moved
            && let Some(target) = drag.target
        {
            match (drag.key, target.target) {
                (MoveKey::Row(k), MoveKey::Row(t)) => {
                    let _ = self.request_row_move(&k, &t, target.placement, cx);
                }
                (MoveKey::Column(k), MoveKey::Column(t)) => {
                    let _ = self.request_column_move(&k, &t, target.placement, cx);
                }
                _ => {}
            }
        }
        cx.notify();
    }
    pub(super) fn keyboard_drag(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        let Some(drag) = &self.drag else { return false };
        if drag.pointer.is_some() {
            return false;
        }
        if key == "escape" {
            self.cancel_drag(cx);
            return true;
        }
        // The focused YueButton commits on native key-up. Handling key-down
        // here would let the following click start a second session.
        if key == "enter" || key == "space" {
            return false;
        }
        let direction = match (&drag.key, key) {
            (MoveKey::Row(_), "up") | (MoveKey::Column(_), "left") => -1isize,
            (MoveKey::Row(_), "down") | (MoveKey::Column(_), "right") => 1,
            _ => return false,
        };
        let v = self.view().unwrap();
        let current = drag.target.as_ref().map_or(&drag.key, |t| &t.target);
        let (from, len) = match current {
            MoveKey::Row(k) => (v.source.position_of(k).unwrap(), v.source.len()),
            MoveKey::Column(k) => (
                v.columns.iter().position(|c| c.key() == k).unwrap(),
                v.columns.len(),
            ),
        };
        let mut index = from as isize + direction;
        while index >= 0 && (index as usize) < len {
            let target = match &drag.key {
                MoveKey::Row(_) => MoveKey::Row(v.source.row_key(index as usize).unwrap().clone()),
                MoveKey::Column(_) => MoveKey::Column(v.columns[index as usize].key().clone()),
            };
            let valid = match (&drag.key, &target) {
                (MoveKey::Row(k), MoveKey::Row(t)) => self.validate_row_move(k, t).is_ok(),
                (MoveKey::Column(k), MoveKey::Column(t)) => validate_column_move(v, k, t).is_ok(),
                _ => false,
            };
            if valid {
                self.drag.as_mut().unwrap().target = Some(DragPreview {
                    target: target.clone(),
                    placement: if direction < 0 {
                        TableMovePlacement::Before
                    } else {
                        TableMovePlacement::After
                    },
                });
                self.reveal_drag_target(&target, cx);
                break;
            }
            index += direction;
        }
        cx.notify();
        true
    }
    fn reveal_drag_target(&mut self, target: &MoveKey, cx: &mut Context<Self>) {
        match target {
            MoveKey::Row(key) => {
                let _ = self.scroll_to_row(key, Default::default(), cx);
            }
            MoveKey::Column(key) => {
                let Some(view) = self.view() else { return };
                let Some(index) = self
                    .resolved
                    .indices
                    .iter()
                    .position(|i| view.columns[*i].key() == key)
                else {
                    return;
                };
                if self.resolved.pins[index] != TablePin::None {
                    return;
                }
                let viewport = self.drag_viewport();
                let handle = self.scroll_handle();
                let offset = handle.offset();
                let logical = if self.presentation.as_ref().is_some_and(|p| p.host.is_some()) {
                    viewport.left() - self.table_bounds.get().left()
                } else {
                    -offset.x
                };
                let left = self.resolved.offsets[index];
                let right = left + self.resolved.widths[index];
                let next = if left < logical + self.resolved.leading {
                    left - self.resolved.leading
                } else if right > logical + viewport.size.width - self.resolved.trailing {
                    right - viewport.size.width + self.resolved.trailing
                } else {
                    logical
                };
                handle.set_offset(gpui_kit::point(
                    (offset.x - (next - logical)).clamp(-handle.max_offset().x, px(0.)),
                    offset.y,
                ));
            }
        }
    }
    pub(super) fn pointer_drag(
        &mut self,
        point: Point<Pixels>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(d) = self.drag.as_mut() else { return };
        if d.pointer.is_none() {
            return;
        }
        d.pointer = Some(point);
        d.moved |= d
            .start
            .is_some_and(|start| (point.x - start.x).abs() + (point.y - start.y).abs() > px(4.));
        if !d.moved {
            return;
        }
        self.hit_drag_target(point, cx);
        self.drag_scroll(w, cx);
        cx.notify();
    }
    fn drag_viewport(&self) -> Bounds<Pixels> {
        let bounds = self.table_bounds.get();
        if let Some((host, top, bottom)) = self.presentation.as_ref().and_then(|p| p.host.as_ref())
        {
            let mut b = host.bounds();
            b.origin.y += top.to_pixels(self.layout_rem);
            b.size.height = (b.size.height
                - top.to_pixels(self.layout_rem)
                - bottom.to_pixels(self.layout_rem))
            .max(px(0.));
            bounds.intersect(&b)
        } else {
            bounds
        }
    }
    fn hit_drag_target(&mut self, point: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(drag) = &self.drag else { return };
        let Some(v) = self.view() else { return };
        let Some(p) = &self.presentation else { return };
        let viewport = self.drag_viewport();
        let natural = self.table_bounds.get();
        let scroll = self.scroll_handle().offset();
        if p.mode == TableRenderMode::Static
            && let MoveKey::Row(key) = &drag.key
        {
            let target = self.static_row_bounds.iter().find_map(|(k, b)| {
                if b.contains(&point) && self.validate_row_move(key, k).is_ok() {
                    Some(DragPreview {
                        target: MoveKey::Row(k.clone()),
                        placement: if point.y < b.center().y {
                            TableMovePlacement::Before
                        } else {
                            TableMovePlacement::After
                        },
                    })
                } else {
                    None
                }
            });
            self.drag.as_mut().unwrap().target = target;
            return;
        }
        let target = match &drag.key {
            MoveKey::Row(key) => {
                let coordinate = f64::from(point.y - natural.top())
                    + if p.host.is_some() {
                        0.
                    } else {
                        f64::from(-scroll.y)
                    };
                let rows = self.rows.read(cx);
                let Some(vp) = rows.viewport() else { return };
                let Ok(snapshot) = rows.snapshot(vp) else {
                    return;
                };
                let (mut lo, mut hi) = (0, snapshot.len());
                while lo < hi {
                    let mid = (lo + hi) / 2;
                    let item = snapshot.item(mid).unwrap();
                    if item.start() + f64::from(item.extent()) <= coordinate {
                        lo = mid + 1
                    } else {
                        hi = mid
                    }
                }
                let headers = cells::header_count(&Presentation {
                    resolved: self.resolved.clone(),
                    ..p.clone()
                });
                let i = lo.saturating_sub(headers);
                v.source
                    .row_key(i)
                    .filter(|_| {
                        lo >= headers && point.y >= viewport.top() && point.y <= viewport.bottom()
                    })
                    .and_then(|target| {
                        self.validate_row_move(key, target).ok()?;
                        let item = snapshot.item(lo).ok()?;
                        Some(DragPreview {
                            target: MoveKey::Row(target.clone()),
                            placement: if coordinate < item.start() + f64::from(item.extent()) / 2.
                            {
                                TableMovePlacement::Before
                            } else {
                                TableMovePlacement::After
                            },
                        })
                    })
            }
            MoveKey::Column(key) => {
                let x = point.x - viewport.left();
                let offset = if p.host.is_some() {
                    viewport.left() - natural.left()
                } else {
                    -scroll.x
                };
                let logical = if x < self.resolved.leading {
                    x
                } else if x >= viewport.size.width - self.resolved.trailing {
                    self.resolved.total - (viewport.size.width - x)
                } else {
                    x + offset
                };
                self.resolved.indices.iter().enumerate().find_map(|(i, c)| {
                    let col = &v.columns[*c];
                    if logical >= self.resolved.offsets[i]
                        && logical < self.resolved.offsets[i] + self.resolved.widths[i]
                        && validate_column_move(v, key, col.key()).is_ok()
                    {
                        Some(DragPreview {
                            target: MoveKey::Column(col.key().clone()),
                            placement: if logical
                                < self.resolved.offsets[i] + self.resolved.widths[i] / 2.
                            {
                                TableMovePlacement::Before
                            } else {
                                TableMovePlacement::After
                            },
                        })
                    } else {
                        None
                    }
                })
            }
        };
        self.drag.as_mut().unwrap().target = target;
    }
    fn drag_scroll(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(drag) = &self.drag else { return };
        let Some(pointer) = drag.pointer else { return };
        let viewport = self.drag_viewport();
        let edge = px(24.);
        let horizontal = matches!(drag.key, MoveKey::Column(_));
        let (value, start, end) = if horizontal {
            (pointer.x, viewport.left(), viewport.right())
        } else {
            (pointer.y, viewport.top(), viewport.bottom())
        };
        let step = if value < start + edge {
            -px(12.)
        } else if value > end - edge {
            px(12.)
        } else {
            px(0.)
        };
        if step == px(0.) {
            return;
        }
        let handle = self.scroll_handle();
        let old = handle.offset();
        let max = handle.max_offset();
        let mut next = old;
        if horizontal {
            next.x = (old.x - step).clamp(-max.x, px(0.));
        } else {
            next.y = (old.y - step).clamp(-max.y, px(0.));
        }
        if next == old {
            return;
        }
        handle.set_offset(next);
        w.refresh();
        let drag = self.drag.as_mut().unwrap();
        if drag.frame_pending {
            return;
        }
        drag.frame_pending = true;
        let seq = drag.sequence;
        let owner = cx.weak_entity();
        w.on_next_frame(move |w, cx| {
            let _ = owner.update(cx, |s, cx| {
                if !s.drag.as_ref().is_some_and(|d| d.sequence == seq) {
                    return;
                }
                let d = s.drag.as_mut().unwrap();
                d.frame_pending = false;
                let point = d.pointer.unwrap();
                s.hit_drag_target(point, cx);
                s.drag_scroll(w, cx);
                cx.notify();
            });
        });
        // on_next_frame already schedules frame demand and is safe in input dispatch.
        w.refresh();
    }
}
pub(super) fn handle<D: TableDelegate>(
    id: &'static str,
    key: MoveKey,
    owner: &gpui_kit::WeakEntity<YueTableState<D>>,
    focus: Option<&gpui_kit::FocusHandle>,
    tab_stop: bool,
    disabled: bool,
    cx: &gpui_kit::App,
) -> gpui_kit::AnyElement {
    use gpui_kit::{InteractiveElement as _, IntoElement as _, ParentElement as _, Styled as _};
    let theme = crate::current_theme(cx);
    let pointer_owner = owner.clone();
    let pointer_key = key.clone();
    let keyboard_owner = owner.clone();
    let keyboard_key = key.clone();
    let keys_owner = owner.clone();
    let label = match &key {
        MoveKey::Row(k) => format!("移动行 {}", k.as_str()),
        MoveKey::Column(k) => format!("移动列 {}", k.as_str()),
    };
    let mut button = crate::YueButton::icon(id, theme, crate::YueIcon::DragHandle, label)
        .variant(crate::ButtonVariant::Ghost)
        .size(crate::ButtonSize::Small)
        .disabled(disabled)
        .tab_stop(tab_stop)
        .accessibility_description(
            "拖动排序；空格开始键盘移动，方向键选择位置，Enter确认，Escape取消",
        )
        .on_click(move |event, _, cx| {
            if event.is_keyboard() {
                let _ = keyboard_owner.update(cx, |s, cx| {
                    if s.drag.is_some() {
                        s.finish_drag(cx);
                    } else {
                        let _ = s.begin_drag(keyboard_key.clone(), None, cx);
                    }
                });
            }
        });
    if let Some(focus) = focus {
        button = button.track_focus(focus);
    }
    let mut wrapper = gpui_kit::div()
        .w_full()
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .child(button);
    if !disabled {
        wrapper = wrapper
            .on_mouse_down(gpui_kit::MouseButton::Left, move |event, w, cx| {
                cx.stop_propagation();
                let _ = pointer_owner.update(cx, |s, cx| {
                    s.focus.focus(w, cx);
                    let _ = s.begin_drag(pointer_key.clone(), Some(event.position), cx);
                });
            })
            .on_key_down(move |event, _, cx| {
                let handled = keys_owner
                    .update(cx, |s, cx| {
                        s.keyboard_drag(event.keystroke.key.as_str(), cx)
                    })
                    .unwrap_or(false);
                if handled {
                    cx.stop_propagation();
                }
            });
    }
    wrapper.into_any_element()
}
