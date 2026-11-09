use super::*;
use gpui_kit::base::{
    input::InputState,
    keyed_tree::{KeyedTreeModel, TreeSelectionGesture},
    virtualization::{
        VirtualizerChange, VirtualizerScrollOptions, VirtualizerSizing, VirtualizerSource,
        VirtualizerState,
    },
};
use gpui_kit::{
    AppContext as _, Context, EntityInputHandler as _, EventEmitter, FocusHandle, Focusable,
    Render, Subscription, px,
};
use std::collections::HashMap;
pub(crate) struct Source {
    rows: Rc<Vec<TreeRow>>,
    positions: Rc<HashMap<ElementId, usize>>,
}
impl VirtualizerSource for Source {
    fn len(&self) -> usize {
        self.rows.len()
    }
    fn key_at(&self, index: usize) -> ElementId {
        self.rows[index].key().clone().into()
    }
    fn index_of(&self, key: &ElementId) -> Option<usize> {
        self.positions.get(key).copied()
    }
}
pub struct YueTreeState {
    pub(crate) model: KeyedTreeModel,
    pub(crate) focus: FocusHandle,
    pub(crate) input: Entity<InputState>,
    pub(crate) virtualizer: Entity<VirtualizerState>,
    pub(crate) presentation: Option<Presentation>,
    pub(crate) row_action: TreeRowAction,
    pub(crate) toggle_selection: bool,
    pub(crate) leaves_only: bool,
    pub(crate) interactive: bool,
    pub(crate) disabled: bool,
    pub(crate) query_observed: SharedString,
    pub(crate) local_filter: bool,
    pub(crate) row_actions: HashMap<SharedString, Vec<FocusHandle>>,
    pub(crate) action_row: Option<SharedString>,
    pub(crate) node_icons: HashMap<SharedString, Icon>,
    pub(crate) limit: Option<usize>,
    pub(crate) layout_row_height: gpui_kit::Pixels,
    pub(crate) page_rows: isize,
    pub(crate) viewport_width: gpui_kit::Pixels,
    pub(crate) geometry: Option<(u64, gpui_kit::Pixels)>,
    pub(super) motion: super::motion::TreeMotion,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<TreeEvent> for YueTreeState {}
impl YueTreeState {
    pub fn new(
        selection: TreeSelectionMode,
        check: TreeCheckMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let virtualizer = cx.new(|_| {
            VirtualizerState::new(
                gpui_kit::Axis::Vertical,
                Rc::new(Source {
                    rows: Rc::new(vec![]),
                    positions: Rc::new(HashMap::new()),
                }),
                VirtualizerSizing::known(Vec::new()),
            )
            .expect("valid empty tree")
        });
        let input = cx.new(|cx| InputState::new(window, cx));
        let subscription = cx.observe_in(&input, window, |state, input, window, cx| {
            if state.composing(window, cx) {
                return;
            }
            let query = input.read(cx).value();
            if query != state.query_observed {
                state.query_observed = query.clone();
                if state.local_filter {
                    state.model.filter(&query);
                }
                cx.notify();
            }
        });
        Self {
            model: KeyedTreeModel::new(selection, check),
            focus: cx.focus_handle(),
            input,
            virtualizer,
            presentation: None,
            row_action: TreeRowAction::Select,
            toggle_selection: false,
            leaves_only: false,
            interactive: true,
            disabled: false,
            query_observed: "".into(),
            local_filter: true,
            row_actions: HashMap::new(),
            action_row: None,
            node_icons: HashMap::new(),
            limit: None,
            layout_row_height: px(32.),
            page_rows: 8,
            viewport_width: px(240.),
            geometry: None,
            motion: Default::default(),
            _subscriptions: vec![subscription],
        }
    }
    pub fn set_nodes(
        &mut self,
        nodes: impl IntoIterator<Item = TreeNode>,
        cx: &mut Context<Self>,
    ) -> Result<TreeReconciliation, TreeDataError> {
        let r = self.model.replace(nodes, false)?;
        self.node_icons.retain(|k, _| self.model.node(k).is_some());
        self.row_actions.retain(|k, _| self.model.node(k).is_some());
        cx.notify();
        Ok(r)
    }
    pub fn node(&self, key: &str) -> Option<&TreeNode> {
        self.model.node(key)
    }
    pub fn parent(&self, key: &str) -> Option<&SharedString> {
        self.model.parent(key)
    }
    pub fn ancestors(&self, key: &str) -> Vec<SharedString> {
        self.model.ancestors(key)
    }
    pub fn selected_keys(&self) -> &[SharedString] {
        self.model.selected_keys()
    }
    pub fn checked_keys(&self) -> &[SharedString] {
        self.model.checked_keys()
    }
    pub fn projected_checked(&self, projection: TreeCheckProjection) -> Vec<SharedString> {
        self.model.project_checked(projection)
    }
    pub fn mark(&self, key: &str) -> TreeMark {
        self.model.mark(key)
    }
    pub fn active_key(&self) -> Option<&SharedString> {
        self.model.active_key()
    }
    pub fn expanded_keys(&self) -> Vec<SharedString> {
        self.model.expanded_keys().cloned().collect()
    }
    pub fn set_selected_keys(
        &mut self,
        keys: Vec<SharedString>,
        cx: &mut Context<Self>,
    ) -> Result<(), TreeDataError> {
        self.model.set_selected(keys)?;
        cx.notify();
        Ok(())
    }
    pub fn set_checked(
        &mut self,
        keys: Vec<SharedString>,
        cx: &mut Context<Self>,
    ) -> Result<(), TreeDataError> {
        self.model.set_checked(keys)?;
        cx.notify();
        Ok(())
    }
    pub fn set_expanded_keys(
        &mut self,
        keys: Vec<SharedString>,
        cx: &mut Context<Self>,
    ) -> Result<(), TreeDataError> {
        self.model.set_expanded(keys)?;
        self.motion.requested = true;
        cx.notify();
        Ok(())
    }
    pub fn set_row_action(&mut self, action: TreeRowAction, cx: &mut Context<Self>) {
        self.row_action = action;
        cx.notify()
    }
    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        self.disabled = disabled;
        cx.notify()
    }
    pub fn set_node_icon(
        &mut self,
        key: impl Into<SharedString>,
        icon: Icon,
        cx: &mut Context<Self>,
    ) {
        self.node_icons.insert(key.into(), icon);
        cx.notify()
    }
    /// Register enabled controls used by render_actions in visual order.
    /// Those controls must use tab_stop(false); F2 owns entry and Tab traversal.
    pub fn set_row_action_handles(
        &mut self,
        key: impl Into<SharedString>,
        handles: Vec<FocusHandle>,
        cx: &mut Context<Self>,
    ) -> Result<(), TreeDataError> {
        let key = key.into();
        if self.model.node(&key).is_none() {
            return Err(TreeDataError::UnknownKey(key));
        }
        self.row_actions.insert(key, handles);
        cx.notify();
        Ok(())
    }
    pub(crate) fn repair_action_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.action_row.as_ref().is_some_and(|k| {
            self.model.active_key() != Some(k)
                || !self
                    .model
                    .positions()
                    .contains_key(&ElementId::Name(k.clone()))
        }) {
            self.action_row = None;
            self.focus.focus(window, cx);
        }
    }
    pub(crate) fn enter_actions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.interactive || self.disabled {
            return;
        }
        if let Some(key) = self.active_key().cloned()
            && let Some(handle) = self.row_actions.get(&key).and_then(|h| {
                h.iter()
                    .find(|h| gpui_kit::base::Root::is_focus_available(h, window))
            })
        {
            handle.focus(window, cx);
            self.action_row = Some(key);
            cx.notify();
        }
    }
    pub(crate) fn leave_actions(
        &mut self,
        back: bool,
        escape: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(key) = self.action_row.clone() else {
            return;
        };
        if !escape
            && let Some(handles) = self.row_actions.get(&key)
            && let Some(i) = handles.iter().position(|h| h.contains_focused(window, cx))
        {
            let target = if back { i.checked_sub(1) } else { Some(i + 1) };
            if let Some(target) = target
                .and_then(|i| handles.get(i))
                .filter(|h| gpui_kit::base::Root::is_focus_available(h, window))
            {
                target.focus(window, cx);
                return;
            }
        }
        self.action_row = None;
        self.focus.focus(window, cx);
        if !escape {
            if back {
                gpui_kit::base::Root::focus_prev(window, cx)
            } else {
                gpui_kit::base::Root::focus_next(window, cx)
            }
        }
        cx.notify();
    }
    pub fn set_filter(&mut self, filter: Option<TreeFilter>, cx: &mut Context<Self>) {
        self.model.set_filter(filter);
        cx.notify();
    }
    pub fn query_input(&self) -> &Entity<InputState> {
        &self.input
    }
    pub fn query(&self, cx: &App) -> SharedString {
        self.input.read(cx).value()
    }
    pub fn set_query(
        &mut self,
        query: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let query = query.into();
        self.query_observed = query.clone();
        self.input
            .update(cx, |s, cx| s.set_value(query.clone(), window, cx));
        if self.local_filter {
            self.model.filter(&query);
        }
        cx.notify();
    }
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        if !self.disabled {
            self.focus.focus(window, cx)
        }
    }
    pub fn reveal(&mut self, key: &str, cx: &mut Context<Self>) -> Result<(), TreeDataError> {
        self.model.reveal(key)?;
        self.sync_geometry(cx);
        self.scroll_active(cx);
        cx.notify();
        Ok(())
    }
    pub fn request_load(&mut self, key: Option<SharedString>, cx: &mut Context<Self>) {
        if let Some(t) = self.model.request_load(key) {
            cx.emit(TreeEvent::LoadRequested(t));
            cx.notify();
        }
    }
    pub fn apply_children(
        &mut self,
        ticket: &TreeLoadTicket,
        nodes: Vec<TreeNode>,
        cx: &mut Context<Self>,
    ) -> Result<TreeApply, TreeDataError> {
        let result = self.model.apply_children(ticket, nodes);
        if matches!(result, Ok(TreeApply::Applied))
            && ticket.target().is_some_and(|k| self.model.is_expanded(k))
        {
            self.motion.requested = true;
        }
        cx.notify();
        result
    }
    pub fn reject_load(
        &mut self,
        ticket: &TreeLoadTicket,
        error: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> TreeApply {
        let r = self.model.reject_load(ticket, error.into());
        if r == TreeApply::Applied {
            cx.notify()
        }
        r
    }
    pub(crate) fn composing(&self, window: &mut Window, cx: &mut App) -> bool {
        self.input
            .update(cx, |s, cx| s.marked_text_range(window, cx).is_some())
    }
    pub(crate) fn expand(&mut self, key: SharedString, cx: &mut Context<Self>) {
        if !self.interactive || self.disabled {
            return;
        }
        if self.model.toggle_expanded(&key).unwrap_or(false) {
            self.motion.requested = true;
            let expanded = self.model.is_expanded(&key);
            cx.emit(TreeEvent::ExpansionChanged {
                key: key.clone(),
                expanded,
            });
            if expanded {
                self.request_load(Some(key), cx)
            }
            cx.notify();
        }
    }
    pub(crate) fn activate(
        &mut self,
        key: SharedString,
        source: TreeChangeSource,
        modifiers: gpui_kit::Modifiers,
        activate: bool,
        cx: &mut Context<Self>,
    ) {
        if !self.interactive
            || self.disabled
            || !self.model.node(&key).is_some_and(|n| !n.is_disabled())
        {
            return;
        }
        if self.leaves_only && self.model.node(&key).is_some_and(|n| !n.is_leaf()) {
            self.model.set_active(key.clone());
            self.expand(key, cx);
            return;
        }
        if self.row_action == TreeRowAction::Expand {
            self.model.set_active(key.clone());
            self.expand(key, cx);
            return;
        }
        let check = self.row_action == TreeRowAction::Check;
        if (check && !self.model.can_check(&key))
            || (!check && !self.model.node(&key).is_some_and(|n| n.is_selectable()))
        {
            self.model.set_active(key);
            cx.notify();
            return;
        }
        let before = if check {
            self.model.checked_keys()
        } else {
            self.model.selected_keys()
        }
        .to_vec();
        if self.limit.is_some_and(|limit| before.len() >= limit) && !before.contains(&key) {
            self.model.set_active(key);
            cx.notify();
            return;
        }
        if check {
            self.model.set_active(key.clone());
            let _ = self.model.toggle_check(&key);
        } else {
            if self.model.selection_mode() == TreeSelectionMode::None {
                self.model.set_active(key.clone());
            }
            self.model.select(
                &key,
                TreeSelectionGesture::default()
                    .toggle(self.toggle_selection || modifiers.control)
                    .range(!self.toggle_selection && modifiers.shift)
                    .additive(modifiers.control),
            );
        }
        let after = if check {
            self.model.checked_keys()
        } else {
            self.model.selected_keys()
        }
        .to_vec();
        if before != after {
            let change = TreeChange {
                before,
                after,
                key: key.clone(),
                source,
            };
            cx.emit(if check {
                TreeEvent::CheckChanged(change)
            } else {
                TreeEvent::SelectionChanged(change)
            });
        }
        if activate {
            cx.emit(TreeEvent::Activate { key, source });
        }
        cx.notify();
    }
    pub(crate) fn check(
        &mut self,
        key: SharedString,
        source: TreeChangeSource,
        cx: &mut Context<Self>,
    ) {
        let action = self.row_action;
        self.row_action = TreeRowAction::Check;
        self.activate(key, source, Default::default(), false, cx);
        self.row_action = action;
    }
    pub(crate) fn navigate(&mut self, movement: TreeMove, cx: &mut Context<Self>) {
        if !self.interactive || self.disabled {
            return;
        }
        let before = self.model.active_key().cloned();
        let was_expanded = before.as_ref().map(|k| self.model.is_expanded(k));
        self.model.navigate(movement);
        if let Some(key) = before {
            let expanded = self.model.is_expanded(&key);
            if was_expanded != Some(expanded) {
                self.motion.requested = true;
                cx.emit(TreeEvent::ExpansionChanged {
                    key: key.clone(),
                    expanded,
                });
            }
            if expanded && movement == TreeMove::Child {
                self.request_load(Some(key), cx)
            }
        }
        if !self.motion.requested && self.motion.running() {
            // Navigation into a partially revealed row must expose its full
            // keyboard target before issuing a scroll-to-key command.
            self.geometry = None;
        }
        self.sync_geometry(cx);
        self.scroll_active(cx);
        cx.notify();
    }
    pub(crate) fn sync_geometry(&mut self, cx: &mut Context<Self>) {
        let height = self.layout_row_height;
        let geometry = (self.model.revision(), height);
        let now = cx.background_executor().now();
        let was_running = self.motion.running();
        let updates = self.motion.sample(now);
        if !updates.is_empty() {
            self.virtualizer.update(cx, |v, _| {
                v.update_sizes(&updates).expect("valid tree motion extents")
            });
        }
        let finished = was_running && !self.motion.running();
        let immediate = cx.reduce_motion();
        if self.geometry != Some(geometry) || finished || (immediate && self.motion.running()) {
            let theme = self
                .presentation
                .as_ref()
                .and_then(|p| p.theme)
                .unwrap_or_else(|| crate::current_theme(cx));
            let extents = if self.motion.requested
                && !immediate
                && self.geometry.is_some_and(|(_, h)| h == height)
            {
                self.motion.retarget(self.model.rows(), height, now, theme)
            } else {
                None
            };
            self.motion.requested = false;
            let extents = extents.unwrap_or_else(|| {
                self.motion.reset(self.model.rows(), self.model.positions());
                vec![height; self.motion.rows.len()]
            });
            let source = Rc::new(Source {
                rows: self.motion.rows.clone(),
                positions: self.motion.positions.clone(),
            });
            self.virtualizer.update(cx, |v, _| {
                v.replace_source(
                    source,
                    VirtualizerSizing::known(extents),
                    VirtualizerChange::Replace {
                        preserve_anchor: true,
                    },
                )
                .expect("valid keyed tree");
            });
            self.geometry = Some(geometry);
        }
        let retained = self
            .active_key()
            .map(|k| vec![ElementId::Name(k.clone())])
            .unwrap_or_default();
        self.virtualizer.update(cx, |v, _| {
            let _ = v.set_retained_keys(retained);
        });
    }
    pub(crate) fn scroll_active(&self, cx: &mut App) {
        if let Some(key) = self.active_key() {
            self.virtualizer.update(cx, |v, _| {
                let _ = v.scroll_to_key(
                    ElementId::Name(key.clone()),
                    VirtualizerScrollOptions::default(),
                );
            });
        }
    }
}
impl Focusable for YueTreeState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl Render for YueTreeState {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        super::element::render(self, w, cx)
    }
}
