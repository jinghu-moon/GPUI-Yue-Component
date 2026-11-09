use super::*;
use crate::{
    TreeApply, TreeChangeSource, TreeDataError, TreeEvent, TreeNode, TreeReconciliation,
    TreeRowAction, TreeSelectionMode,
    popup::{PopupAnchor, PopupId},
    tree::YueTreeState,
};
use gpui_kit::{AppContext as _, Context, EventEmitter, Focusable, Render, Subscription, Task};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
pub struct YueTreeSelectState {
    pub(super) tree: Entity<YueTreeState>,
    pub(super) mode: TreeSelectMode,
    pub(super) focus: FocusHandle,
    pub(super) collapsed_focus: FocusHandle,
    pub(super) anchor: PopupAnchor,
    pub(super) parent: Option<PopupId>,
    pub(super) open: bool,
    pub(super) collapsed: bool,
    pub(super) disabled: bool,
    pub(super) readonly: bool,
    pub(super) loading: bool,
    pub(super) search: TreeSelectSearch,
    pub(super) result: TreeSelectResultState,
    pub(super) presentation: Option<Presentation>,
    pub(super) measured_width: gpui_kit::Pixels,
    pub(super) content_width_cache: Option<(
        u64,
        gpui_kit::Pixels,
        gpui_kit::Pixels,
        String,
        gpui_kit::Pixels,
    )>,
    pub(super) slot_heights: [gpui_kit::Pixels; 2],
    pub(super) visible_tags: Vec<SharedString>,
    pub(super) hidden_tags: Vec<SharedString>,
    pub(super) cache: HashMap<SharedString, TreeSelectItemSnapshot>,
    pub(super) tag_focus: HashMap<SharedString, FocusHandle>,
    observed_query: SharedString,
    session: u64,
    generation: u64,
    pending: Option<TreeSelectSearchTicket>,
    debounce: Option<Task<()>>,
    debounce_duration: Duration,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<TreeSelectEvent> for YueTreeSelectState {}
impl YueTreeSelectState {
    pub fn new(
        mode: TreeSelectMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Self, TreeDataError> {
        if mode == TreeSelectMode::Checked(TreeCheckMode::None) {
            return Err(TreeDataError::Mode);
        }
        let (selection, check) = match mode {
            TreeSelectMode::Single => (TreeSelectionMode::Single, TreeCheckMode::None),
            TreeSelectMode::Multiple => (TreeSelectionMode::Multiple, TreeCheckMode::None),
            TreeSelectMode::Checked(c) => (TreeSelectionMode::None, c),
        };
        let tree = cx.new(|cx| {
            let mut s = YueTreeState::new(selection, check, window, cx);
            s.toggle_selection = true;
            s.row_action = if check == TreeCheckMode::None {
                TreeRowAction::Select
            } else {
                TreeRowAction::Check
            };
            s.interactive = false;
            if check != TreeCheckMode::Cascade {
                s.model.set_retain_missing(true).unwrap()
            }
            s
        });
        let input = tree.read(cx).input.clone();
        let subs = vec![
            cx.subscribe_in(&tree, window, |state, _, event, window, cx| match event {
                TreeEvent::SelectionChanged(change) | TreeEvent::CheckChanged(change) => {
                    if !state.open || !state.editable() {
                        return;
                    }
                    let source = match change.source() {
                        TreeChangeSource::Pointer => TreeSelectChangeSource::Pointer,
                        TreeChangeSource::Keyboard => TreeSelectChangeSource::Keyboard,
                        TreeChangeSource::Accessibility => TreeSelectChangeSource::Accessibility,
                    };
                    let before = state.value_for(change.before().to_vec());
                    state.emit_change(before, Some(change.key().clone()), source, cx);
                }
                TreeEvent::Activate { .. } if state.mode == TreeSelectMode::Single => {
                    if state.open {
                        state.close(true, window, cx)
                    }
                }
                TreeEvent::LoadRequested(ticket) => {
                    cx.emit(TreeSelectEvent::LoadRequested(ticket.clone()));
                }
                _ => {}
            }),
            cx.observe_in(&input, window, |state, input, window, cx| {
                if state.composing(window, cx) {
                    return;
                }
                let query = input.read(cx).value();
                if query == state.observed_query {
                    return;
                }
                state.observed_query = query;
                if state.search == TreeSelectSearch::Remote && state.open {
                    state.invalidate();
                    state.result = TreeSelectResultState::Loading;
                    state.content_focus(cx).focus(window, cx);
                    state.tree.update(cx, |s, cx| {
                        s.interactive = false;
                        cx.notify()
                    });
                    let timer = cx.background_executor().timer(state.debounce_duration);
                    state.debounce = Some(cx.spawn_in(window, async move |state, cx| {
                        timer.await;
                        let _ = state.update(cx, |s, cx| s.start_search(cx));
                    }));
                }
                cx.notify();
            }),
        ];
        Ok(Self {
            tree,
            mode,
            focus: cx.focus_handle(),
            collapsed_focus: cx.focus_handle(),
            anchor: PopupAnchor::default(),
            parent: None,
            open: false,
            collapsed: false,
            disabled: false,
            readonly: false,
            loading: false,
            search: TreeSelectSearch::None,
            result: TreeSelectResultState::Ready,
            presentation: None,
            measured_width: gpui_kit::px(0.),
            content_width_cache: None,
            slot_heights: [gpui_kit::px(0.); 2],
            visible_tags: vec![],
            hidden_tags: vec![],
            cache: HashMap::new(),
            tag_focus: HashMap::new(),
            observed_query: "".into(),
            session: 0,
            generation: 0,
            pending: None,
            debounce: None,
            debounce_duration: Duration::from_millis(200),
            _subscriptions: subs,
        })
    }
    pub fn mode(&self) -> TreeSelectMode {
        self.mode
    }
    pub fn value(&self, cx: &App) -> TreeSelectValue {
        let tree = self.tree.read(cx);
        self.value_for(
            if self.checked() {
                tree.checked_keys()
            } else {
                tree.selected_keys()
            }
            .to_vec(),
        )
    }
    fn value_for(&self, keys: Vec<SharedString>) -> TreeSelectValue {
        match self.mode {
            TreeSelectMode::Single => TreeSelectValue::Single(keys.into_iter().next()),
            TreeSelectMode::Multiple => TreeSelectValue::Multiple(keys),
            TreeSelectMode::Checked(_) => TreeSelectValue::Checked(keys),
        }
    }
    pub(super) fn checked(&self) -> bool {
        matches!(self.mode, TreeSelectMode::Checked(_))
    }
    pub(super) fn editable(&self) -> bool {
        !self.disabled && !self.readonly
    }
    pub fn is_open(&self) -> bool {
        self.open
    }
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
    pub fn is_readonly(&self) -> bool {
        self.readonly
    }
    pub fn result_state(&self) -> &TreeSelectResultState {
        &self.result
    }
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }
    pub fn focus(&self, w: &mut Window, cx: &mut App) {
        if !self.disabled {
            self.focus.focus(w, cx)
        }
    }
    pub fn query_input(&self, cx: &App) -> Entity<crate::InputState> {
        self.tree.read(cx).input.clone()
    }
    pub fn query(&self, cx: &App) -> SharedString {
        self.tree.read(cx).query(cx)
    }
    pub fn set_query(
        &mut self,
        q: impl Into<SharedString>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.tree.update(cx, |s, cx| s.set_query(q, w, cx));
    }
    pub fn set_nodes(
        &mut self,
        nodes: impl IntoIterator<Item = TreeNode>,
        cx: &mut Context<Self>,
    ) -> Result<TreeReconciliation, TreeDataError> {
        let r = self.tree.update(cx, |s, cx| s.set_nodes(nodes, cx))?;
        self.invalidate();
        self.result = TreeSelectResultState::Ready;
        self.sync_interaction(cx);
        self.refresh_cache(cx);
        cx.notify();
        Ok(r)
    }
    pub fn set_expanded_keys(
        &mut self,
        keys: Vec<SharedString>,
        cx: &mut Context<Self>,
    ) -> Result<(), TreeDataError> {
        self.tree.update(cx, |s, cx| s.set_expanded_keys(keys, cx))
    }
    pub fn expanded_keys(&self, cx: &App) -> Vec<SharedString> {
        self.tree.read(cx).expanded_keys()
    }
    pub fn reveal(&mut self, key: &str, cx: &mut Context<Self>) -> Result<(), TreeDataError> {
        self.tree.update(cx, |s, cx| s.reveal(key, cx))
    }
    pub fn set_node_icon(
        &mut self,
        key: impl Into<SharedString>,
        icon: crate::Icon,
        cx: &mut Context<Self>,
    ) {
        self.tree.update(cx, |s, cx| s.set_node_icon(key, icon, cx));
    }
    pub fn set_value(
        &mut self,
        value: TreeSelectValue,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), TreeDataError> {
        if !matches!(
            (&value, self.mode),
            (TreeSelectValue::Single(_), TreeSelectMode::Single)
                | (TreeSelectValue::Multiple(_), TreeSelectMode::Multiple)
                | (TreeSelectValue::Checked(_), TreeSelectMode::Checked(_))
        ) {
            return Err(TreeDataError::Mode);
        }
        if value == self.value(cx) {
            return Ok(());
        }
        let checked = self.checked();
        self.tree.update(cx, |s, cx| {
            if s.limit.is_some_and(|l| value.keys().len() > l) {
                return Err(TreeDataError::Limit);
            }
            if checked {
                s.set_checked(value.keys().to_vec(), cx)
            } else {
                s.set_selected_keys(value.keys().to_vec(), cx)
            }
        })?;
        self.close_silent(w, cx);
        self.refresh_cache(cx);
        cx.notify();
        Ok(())
    }
    pub fn set_selected_items(
        &mut self,
        items: impl IntoIterator<Item = TreeSelectItemSnapshot>,
        cx: &mut Context<Self>,
    ) {
        let selected: HashSet<_> = self.value(cx).keys().iter().cloned().collect();
        for item in items {
            if selected.contains(&item.key) {
                self.cache.insert(item.key.clone(), item);
            }
        }
        cx.notify();
    }
    pub fn selected_items(&self, cx: &App) -> Vec<TreeSelectItemSnapshot> {
        self.value(cx)
            .keys()
            .iter()
            .map(|k| self.item(k, cx))
            .collect()
    }
    pub(super) fn item(&self, key: &SharedString, cx: &App) -> TreeSelectItemSnapshot {
        let tree = self.tree.read(cx);
        if let Some(n) = tree.node(key) {
            let mut ancestors = tree.ancestors(key);
            ancestors.reverse();
            let path = ancestors
                .iter()
                .filter_map(|k| tree.node(k).map(|n| n.label().clone()))
                .collect();
            TreeSelectItemSnapshot {
                key: key.clone(),
                label: n.label().clone(),
                path,
                locked: tree.model.is_locked(key, self.checked()),
                resolved: true,
            }
        } else {
            self.cache
                .get(key)
                .cloned()
                .map(|mut s| {
                    s.resolved = false;
                    s
                })
                .unwrap_or_else(|| TreeSelectItemSnapshot::new(key.clone(), key.clone()))
        }
    }
    fn refresh_cache(&mut self, cx: &mut Context<Self>) {
        let items = self.selected_items(cx);
        self.cache = items.into_iter().map(|i| (i.key.clone(), i)).collect();
        let keys: HashSet<_> = self.value(cx).keys().iter().cloned().collect();
        self.tag_focus.retain(|k, _| keys.contains(k));
        for k in keys {
            self.tag_focus.entry(k).or_insert_with(|| cx.focus_handle());
        }
    }
    pub fn set_search(
        &mut self,
        search: TreeSelectSearch,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), TreeDataError> {
        if search == TreeSelectSearch::Remote
            && self.mode == TreeSelectMode::Checked(TreeCheckMode::Cascade)
        {
            return Err(TreeDataError::Mode);
        }
        if self.search == search {
            return Ok(());
        }
        self.close_silent(w, cx);
        self.search = search;
        self.tree.update(cx, |s, _| {
            s.local_filter = search != TreeSelectSearch::Remote
        });
        cx.notify();
        Ok(())
    }
    /// Custom local matcher; remote results remain host-owned.
    pub fn set_filter(&mut self, filter: Option<crate::tree::TreeFilter>, cx: &mut Context<Self>) {
        self.tree.update(cx, |s, cx| s.set_filter(filter, cx));
    }
    pub fn set_search_debounce(&mut self, duration: Duration) {
        self.debounce_duration = duration;
    }
    pub fn set_max_selected(
        &mut self,
        limit: Option<usize>,
        cx: &mut Context<Self>,
    ) -> Result<(), TreeDataError> {
        if self.mode == TreeSelectMode::Checked(TreeCheckMode::Cascade)
            || self.mode == TreeSelectMode::Single
        {
            return Err(TreeDataError::Mode);
        }
        if limit.is_some_and(|l| self.value(cx).keys().len() > l) {
            return Err(TreeDataError::Limit);
        }
        self.tree.update(cx, |s, _| s.limit = limit);
        Ok(())
    }
    pub fn set_leaves_only(
        &mut self,
        value: bool,
        cx: &mut Context<Self>,
    ) -> Result<(), TreeDataError> {
        if self.checked() {
            return Err(TreeDataError::Mode);
        }
        self.tree.update(cx, |s, _| s.leaves_only = value);
        Ok(())
    }
    pub fn set_disabled(&mut self, value: bool, w: &mut Window, cx: &mut Context<Self>) {
        self.disabled = value;
        if value {
            self.close_silent(w, cx)
        }
        self.sync_interaction(cx);
        cx.notify();
    }
    pub fn set_readonly(&mut self, value: bool, w: &mut Window, cx: &mut Context<Self>) {
        self.readonly = value;
        if value {
            self.close_silent(w, cx)
        }
        self.sync_interaction(cx);
        cx.notify();
    }
    pub fn set_loading(&mut self, value: bool, cx: &mut Context<Self>) {
        self.loading = value;
        cx.notify();
    }
    pub fn set_open(&mut self, value: bool, w: &mut Window, cx: &mut Context<Self>) {
        self.change_open(value, false, w, cx);
    }
    pub(super) fn sync_interaction(&mut self, cx: &mut App) {
        let interactive =
            self.open && self.editable() && self.result == TreeSelectResultState::Ready;
        let disabled = self.disabled;
        self.tree.update(cx, |s, cx| {
            s.interactive = interactive;
            s.disabled = disabled;
            cx.notify();
        });
    }
    pub(super) fn content_focus(&self, cx: &App) -> FocusHandle {
        if self.search != TreeSelectSearch::None {
            self.query_input(cx).read(cx).focus_handle(cx)
        } else {
            self.tree.read(cx).focus.clone()
        }
    }
    pub(super) fn composing(&self, w: &mut Window, cx: &mut App) -> bool {
        self.tree.update(cx, |s, cx| s.composing(w, cx))
    }
    pub(super) fn change_open(
        &mut self,
        value: bool,
        emit: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if value && !self.editable() {
            return false;
        }
        if self.open == value {
            return self.open;
        }
        self.open = value;
        self.collapsed = false;
        self.session += 1;
        if value {
            self.parent =
                self.anchor.handle().bounds().and_then(|b| {
                    crate::popup::PopupRuntime::for_window(w, cx).parent_at(b.center())
                });
            let selected = self.value(cx).keys().first().cloned();
            if let Some(k) = selected {
                self.tree.update(cx, |s, cx| {
                    let _ = s.reveal(&k, cx);
                });
            }
            if self.search == TreeSelectSearch::Remote {
                self.start_search(cx)
            }
        } else {
            self.invalidate();
            self.result = TreeSelectResultState::Ready;
            self.observed_query = "".into();
            self.tree.update(cx, |s, cx| s.set_query("", w, cx));
        }
        self.sync_interaction(cx);
        if emit {
            cx.emit(TreeSelectEvent::OpenChanged(self.open))
        }
        cx.notify();
        self.open
    }
    pub(super) fn close(&mut self, restore: bool, w: &mut Window, cx: &mut Context<Self>) {
        self.change_open(false, true, w, cx);
        self.collapsed = false;
        if restore && !self.disabled {
            self.focus.focus(w, cx)
        }
        cx.notify();
    }
    pub(super) fn close_silent(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.change_open(false, false, w, cx);
        self.collapsed = false;
        self.invalidate();
        self.result = TreeSelectResultState::Ready;
        self.observed_query = "".into();
        self.tree.update(cx, |s, cx| s.set_query("", w, cx));
        self.sync_interaction(cx);
    }
    fn invalidate(&mut self) {
        self.generation += 1;
        self.pending = None;
        self.debounce = None;
    }
    fn start_search(&mut self, cx: &mut Context<Self>) {
        if !self.open || self.search != TreeSelectSearch::Remote {
            return;
        }
        self.generation += 1;
        let ticket = TreeSelectSearchTicket {
            owner: cx.entity_id(),
            session: self.session,
            generation: self.generation,
        };
        self.pending = Some(ticket.clone());
        self.result = TreeSelectResultState::Loading;
        self.sync_interaction(cx);
        cx.emit(TreeSelectEvent::QueryRequested(TreeSelectQuery {
            query: self.query(cx),
            ticket,
        }));
        cx.notify();
    }
    pub fn retry_search(&mut self, cx: &mut Context<Self>) {
        self.start_search(cx);
    }
    pub fn apply_search_results(
        &mut self,
        ticket: &TreeSelectSearchTicket,
        nodes: Vec<TreeNode>,
        cx: &mut Context<Self>,
    ) -> Result<TreeApply, TreeDataError> {
        if self.pending.as_ref() != Some(ticket) || !self.open {
            return Ok(TreeApply::Stale);
        }
        self.refresh_cache(cx);
        let result = self.tree.update(cx, |s, cx| {
            let r = s.model.replace(nodes, true);
            if r.is_ok() {
                cx.notify()
            }
            r
        });
        if let Err(error) = result {
            self.result = TreeSelectResultState::Failed(error.to_string().into());
            self.pending = None;
            self.sync_interaction(cx);
            cx.notify();
            return Err(error);
        }
        self.result = TreeSelectResultState::Ready;
        self.pending = None;
        self.sync_interaction(cx);
        self.refresh_cache(cx);
        cx.notify();
        Ok(TreeApply::Applied)
    }
    pub fn reject_search(
        &mut self,
        ticket: &TreeSelectSearchTicket,
        error: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> TreeApply {
        if self.pending.as_ref() != Some(ticket) || !self.open {
            return TreeApply::Stale;
        }
        self.pending = None;
        self.result = TreeSelectResultState::Failed(error.into());
        self.sync_interaction(cx);
        cx.notify();
        TreeApply::Applied
    }
    pub fn apply_children(
        &mut self,
        t: &crate::TreeLoadTicket,
        n: Vec<TreeNode>,
        cx: &mut Context<Self>,
    ) -> Result<TreeApply, TreeDataError> {
        let result = self.tree.update(cx, |s, cx| s.apply_children(t, n, cx));
        self.refresh_cache(cx);
        cx.notify();
        result
    }
    pub fn reject_load(
        &mut self,
        t: &crate::TreeLoadTicket,
        error: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> TreeApply {
        self.tree.update(cx, |s, cx| s.reject_load(t, error, cx))
    }
    pub(super) fn emit_change(
        &mut self,
        before: TreeSelectValue,
        trigger_key: Option<SharedString>,
        source: TreeSelectChangeSource,
        cx: &mut Context<Self>,
    ) {
        let after = self.value(cx);
        if before == after {
            return;
        }
        self.refresh_cache(cx);
        cx.emit(TreeSelectEvent::Change(TreeSelectChange {
            before,
            after,
            trigger_key,
            source,
            items: self.selected_items(cx),
        }));
        cx.notify();
    }
    pub(super) fn remove(
        &mut self,
        key: Option<SharedString>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.editable() {
            return;
        }
        let before = self.value(cx);
        let checked = self.checked();
        let clear = key.is_none();
        let focus_index = key
            .as_ref()
            .and_then(|k| self.visible_tags.iter().position(|v| v == k));
        let locked = key.as_ref().is_some_and(|k| self.item(k, cx).locked);
        if locked {
            return;
        }
        let locked_values: Vec<_> = before
            .keys()
            .iter()
            .filter(|k| self.item(k, cx).locked)
            .cloned()
            .collect();
        let trigger_key = key.clone();
        self.tree.update(cx, |s, cx| {
            if let Some(key) = key {
                s.model.remove_value(&key, checked);
            } else {
                if checked {
                    s.model
                        .set_checked(locked_values)
                        .expect("subset of current value");
                } else {
                    s.model
                        .set_selected(locked_values)
                        .expect("subset of current value");
                }
            }
            cx.notify();
        });
        self.emit_change(
            before,
            trigger_key.clone(),
            if clear {
                TreeSelectChangeSource::Clear
            } else {
                TreeSelectChangeSource::TagRemove
            },
            cx,
        );
        if self.collapsed {
            if let Some(next) = self
                .hidden_tags
                .iter()
                .filter(|k| Some(*k) != trigger_key.as_ref() && !self.item(k, cx).locked)
                .find_map(|k| self.tag_focus.get(k))
            {
                next.focus(w, cx);
            } else {
                self.collapsed_focus.focus(w, cx);
            }
        } else if let Some(next) = focus_index
            .and_then(|i| {
                self.visible_tags
                    .iter()
                    .enumerate()
                    .filter(|(_, k)| Some(*k) != trigger_key.as_ref())
                    .min_by_key(|(j, _)| j.abs_diff(i))
                    .map(|(_, k)| k.clone())
            })
            .and_then(|k| self.tag_focus.get(&k))
        {
            next.focus(w, cx);
        } else {
            self.focus.focus(w, cx);
        }
    }
}
impl Focusable for YueTreeSelectState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl Render for YueTreeSelectState {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        super::element::render(self, w, cx)
    }
}
