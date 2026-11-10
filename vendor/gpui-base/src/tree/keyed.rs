//! Shared, presentation-free keyed tree behavior for trees and tree choice fields.
use gpui::{
    ElementId, InteractiveElement as _, SharedString, StatefulInteractiveElement as _, div,
};
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeSelectionMode {
    None,
    #[default]
    Single,
    Multiple,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeCheckMode {
    #[default]
    None,
    Strict,
    Cascade,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeCheckProjection {
    #[default]
    All,
    Parents,
    Leaves,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeMark {
    #[default]
    Unchecked,
    Checked,
    Mixed,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeChildren {
    Leaf,
    Loaded(Vec<TreeNode>),
    Unloaded,
}

/// Owned import data; the model flattens this once, never per rendered row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeNode {
    key: SharedString,
    label: SharedString,
    children: TreeChildren,
    disabled: bool,
    selectable: bool,
    checkable: bool,
    check_disabled: bool,
    keywords: SharedString,
}
impl TreeNode {
    pub fn new(key: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            children: TreeChildren::Leaf,
            disabled: false,
            selectable: true,
            checkable: true,
            check_disabled: false,
            keywords: "".into(),
        }
    }
    pub fn children(mut self, children: impl IntoIterator<Item = Self>) -> Self {
        let children: Vec<_> = children.into_iter().collect();
        self.children = if children.is_empty() {
            TreeChildren::Leaf
        } else {
            TreeChildren::Loaded(children)
        };
        self
    }
    pub fn unloaded(mut self) -> Self {
        self.children = TreeChildren::Unloaded;
        self
    }
    pub fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }
    pub fn selectable(mut self, value: bool) -> Self {
        self.selectable = value;
        self
    }
    pub fn checkable(mut self, value: bool) -> Self {
        self.checkable = value;
        self
    }
    pub fn check_disabled(mut self, value: bool) -> Self {
        self.check_disabled = value;
        self
    }
    pub fn keywords(mut self, value: impl Into<SharedString>) -> Self {
        self.keywords = value.into();
        self
    }
    pub fn key(&self) -> &SharedString {
        &self.key
    }
    pub fn label(&self) -> &SharedString {
        &self.label
    }
    pub fn child_state(&self) -> &TreeChildren {
        &self.children
    }
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
    pub fn is_selectable(&self) -> bool {
        self.selectable
    }
    pub fn is_checkable(&self) -> bool {
        self.checkable
    }
    pub fn is_check_disabled(&self) -> bool {
        self.check_disabled
    }
    pub fn is_leaf(&self) -> bool {
        matches!(self.children, TreeChildren::Leaf)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeDataError {
    EmptyKey,
    EmptyLabel,
    DuplicateKey(SharedString),
    UnknownKey(SharedString),
    InvalidValue(SharedString),
    Incomplete(SharedString),
    Mode,
    Limit,
}
impl std::fmt::Display for TreeDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for TreeDataError {}
#[derive(Clone, Debug, Default)]
pub struct TreeReconciliation {
    removed_selected: Vec<SharedString>,
    removed_checked: Vec<SharedString>,
}
impl TreeReconciliation {
    pub fn removed_selected(&self) -> &[SharedString] {
        &self.removed_selected
    }
    pub fn removed_checked(&self) -> &[SharedString] {
        &self.removed_checked
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeLoadTicket {
    owner: u64,
    generation: u64,
    serial: u64,
    target: Option<SharedString>,
}
impl TreeLoadTicket {
    pub fn target(&self) -> Option<&SharedString> {
        self.target.as_ref()
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeLoadState {
    Loading,
    Failed(SharedString),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeApply {
    Applied,
    Stale,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeMove {
    Previous,
    Next,
    First,
    Last,
    Page(isize),
    Parent,
    Child,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct TreeSelectionGesture {
    toggle: bool,
    range: bool,
    additive: bool,
}

impl TreeSelectionGesture {
    pub fn toggle(mut self, value: bool) -> Self {
        self.toggle = value;
        self
    }
    pub fn range(mut self, value: bool) -> Self {
        self.range = value;
        self
    }
    pub fn additive(mut self, value: bool) -> Self {
        self.additive = value;
        self
    }
}

#[derive(Clone)]
struct Record {
    node: Rc<TreeNode>,
    parent: Option<SharedString>,
    children: Vec<SharedString>,
    depth: usize,
}
#[derive(Clone, Debug)]
pub struct TreeRow {
    key: SharedString,
    depth: usize,
    position: usize,
    siblings: usize,
    parent_row: Option<usize>,
    line_index: Rc<Vec<(Option<usize>, bool)>>,
}
impl TreeRow {
    pub fn key(&self) -> &SharedString {
        &self.key
    }
    pub fn depth(&self) -> usize {
        self.depth
    }
    pub fn position(&self) -> usize {
        self.position
    }
    pub fn siblings(&self) -> usize {
        self.siblings
    }
    /// Build line guides only for a rendered row. Projection storage stays O(nodes),
    /// including a single deeply nested chain.
    pub fn continuations(&self) -> Vec<bool> {
        let mut out = Vec::with_capacity(self.depth);
        let mut parent = self.parent_row;
        while let Some(index) = parent {
            let (next, continues) = self.line_index[index];
            out.push(continues);
            parent = next;
        }
        out.reverse();
        out
    }
}
#[derive(Clone, Copy, Default)]
struct Summary {
    total: usize,
    chosen: usize,
    unknown: bool,
}
static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);

pub type TreeFilter = Rc<dyn Fn(&TreeNode, &str) -> bool>;

/// The sole owner of selection/checking/expansion. No GPUI entity per node.
pub struct KeyedTreeModel {
    selection: TreeSelectionMode,
    check: TreeCheckMode,
    records: HashMap<SharedString, Record>,
    roots: Vec<SharedString>,
    order: Vec<SharedString>,
    rows: Rc<Vec<TreeRow>>,
    positions: Rc<HashMap<ElementId, usize>>,
    selected: Vec<SharedString>,
    checked: Vec<SharedString>,
    checked_set: HashSet<SharedString>,
    expanded: HashSet<SharedString>,
    active: Option<SharedString>,
    anchor: Option<SharedString>,
    query: String,
    filter: Option<TreeFilter>,
    filtered: Option<HashSet<SharedString>>,
    folded: HashSet<SharedString>,
    summaries: HashMap<SharedString, Summary>,
    owner: u64,
    generation: u64,
    serial: u64,
    revision: u64,
    pending: HashMap<Option<SharedString>, TreeLoadTicket>,
    load: HashMap<Option<SharedString>, TreeLoadState>,
    retain_missing: bool,
}
impl KeyedTreeModel {
    pub fn new(selection: TreeSelectionMode, check: TreeCheckMode) -> Self {
        Self {
            selection,
            check,
            records: HashMap::new(),
            roots: vec![],
            order: vec![],
            rows: Rc::new(vec![]),
            positions: Rc::new(HashMap::new()),
            selected: vec![],
            checked: vec![],
            checked_set: HashSet::new(),
            expanded: HashSet::new(),
            active: None,
            anchor: None,
            query: String::new(),
            filter: None,
            filtered: None,
            folded: HashSet::new(),
            summaries: HashMap::new(),
            owner: NEXT_OWNER.fetch_add(1, Ordering::Relaxed),
            generation: 0,
            serial: 0,
            revision: 0,
            pending: HashMap::new(),
            load: HashMap::new(),
            retain_missing: false,
        }
    }
    pub fn selection_mode(&self) -> TreeSelectionMode {
        self.selection
    }
    pub fn check_mode(&self) -> TreeCheckMode {
        self.check
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn node(&self, key: &str) -> Option<&TreeNode> {
        self.records.get(key).map(|r| r.node.as_ref())
    }
    /// Children of a stored node. Stored `TreeNode`s expose metadata only;
    /// import subtrees are flattened, so traverse this index rather than child_state().
    pub fn children_keys(&self, key: &str) -> Option<&[SharedString]> {
        self.records.get(key).map(|r| r.children.as_slice())
    }
    pub fn parent(&self, key: &str) -> Option<&SharedString> {
        self.records.get(key)?.parent.as_ref()
    }
    pub fn keys(&self) -> &[SharedString] {
        &self.order
    }
    /// Stable sibling sorting keeps each subtree contiguous and all keyed state.
    /// The comparator receives metadata; callers can look up domain values by key.
    pub fn sort_siblings(
        &mut self,
        mut compare: impl FnMut(&TreeNode, &TreeNode) -> std::cmp::Ordering,
    ) {
        let mut roots = self.roots.clone();
        roots.sort_by(|a, b| compare(&self.records[a].node, &self.records[b].node));
        let mut children = HashMap::new();
        for (key, record) in &self.records {
            let mut keys = record.children.clone();
            keys.sort_by(|a, b| compare(&self.records[a].node, &self.records[b].node));
            children.insert(key.clone(), keys);
        }
        self.roots = roots;
        for (key, keys) in children {
            self.records.get_mut(&key).unwrap().children = keys;
        }
        let mut pending: Vec<_> = self.roots.iter().rev().cloned().collect();
        self.order.clear();
        while let Some(key) = pending.pop() {
            pending.extend(self.records[&key].children.iter().rev().cloned());
            self.order.push(key);
        }
        self.recompute();
        self.rebuild();
    }
    pub fn rows(&self) -> Rc<Vec<TreeRow>> {
        self.rows.clone()
    }
    pub fn positions(&self) -> Rc<HashMap<ElementId, usize>> {
        self.positions.clone()
    }
    pub fn selected_keys(&self) -> &[SharedString] {
        &self.selected
    }
    pub fn checked_keys(&self) -> &[SharedString] {
        &self.checked
    }
    pub fn active_key(&self) -> Option<&SharedString> {
        self.active.as_ref()
    }
    pub fn expanded_keys(&self) -> impl Iterator<Item = &SharedString> {
        self.order.iter().filter(|k| self.expanded.contains(*k))
    }
    pub fn ancestors(&self, key: &str) -> Vec<SharedString> {
        let mut out = vec![];
        let mut current = self.parent(key);
        while let Some(k) = current {
            out.push(k.clone());
            current = self.parent(k)
        }
        out
    }
    pub fn set_retain_missing(&mut self, retain: bool) -> Result<(), TreeDataError> {
        if retain && self.check == TreeCheckMode::Cascade {
            return Err(TreeDataError::Mode);
        }
        self.retain_missing = retain;
        Ok(())
    }
    pub fn replace(
        &mut self,
        nodes: impl IntoIterator<Item = TreeNode>,
        preserve_missing: bool,
    ) -> Result<TreeReconciliation, TreeDataError> {
        let (records, roots, order) = Self::import(nodes)?;
        let old_active = self.active.clone();
        let ancestors = old_active
            .as_ref()
            .map(|k| self.ancestors(k))
            .unwrap_or_default();
        let old_ix = old_active
            .as_ref()
            .and_then(|k| self.rows.iter().position(|r| r.key == *k))
            .unwrap_or(0);
        let mut report = TreeReconciliation::default();
        self.selected.retain(|k| {
            let keep = records
                .get(k)
                .map_or(preserve_missing && self.retain_missing, |r| {
                    r.node.selectable
                });
            if !keep {
                report.removed_selected.push(k.clone())
            }
            keep
        });
        self.checked.retain(|k| {
            let keep = records
                .get(k)
                .map_or(preserve_missing && self.retain_missing, |r| {
                    r.node.checkable && (self.check != TreeCheckMode::Cascade || r.node.is_leaf())
                });
            if !keep {
                report.removed_checked.push(k.clone())
            }
            keep
        });
        self.records = records;
        self.roots = roots;
        self.order = order;
        self.expanded
            .retain(|k| self.records.get(k).is_some_and(|r| !r.node.is_leaf()));
        self.generation += 1;
        self.pending.clear();
        self.load.clear();
        self.recompute();
        self.rebuild();
        if old_active
            .as_ref()
            .is_some_and(|k| !self.visible_enabled(k))
        {
            self.active = ancestors
                .into_iter()
                .find(|k| self.visible_enabled(k))
                .or_else(|| {
                    self.rows
                        .iter()
                        .skip(old_ix.min(self.rows.len().saturating_sub(1)))
                        .find(|r| self.enabled(&r.key))
                        .map(|r| r.key.clone())
                });
            self.ensure_active();
        }
        Ok(report)
    }
    fn import(
        nodes: impl IntoIterator<Item = TreeNode>,
    ) -> Result<
        (
            HashMap<SharedString, Record>,
            Vec<SharedString>,
            Vec<SharedString>,
        ),
        TreeDataError,
    > {
        let mut records = HashMap::new();
        let mut roots = vec![];
        let mut order = vec![];
        // Iterative traversal avoids call-stack growth on deeply nested input.
        let mut stack: Vec<_> = nodes
            .into_iter()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|n| (n, None, 0))
            .collect();
        while let Some((mut node, parent, depth)) = stack.pop() {
            if node.key.is_empty() {
                return Err(TreeDataError::EmptyKey);
            }
            if node.label.trim().is_empty() {
                return Err(TreeDataError::EmptyLabel);
            }
            if records.contains_key(&node.key) {
                return Err(TreeDataError::DuplicateKey(node.key));
            }
            let children = match std::mem::replace(&mut node.children, TreeChildren::Leaf) {
                TreeChildren::Loaded(c) => {
                    node.children = if c.is_empty() {
                        TreeChildren::Leaf
                    } else {
                        TreeChildren::Loaded(vec![])
                    };
                    c
                }
                other => {
                    node.children = other;
                    vec![]
                }
            };
            let key = node.key.clone();
            let child_keys = children.iter().map(|c| c.key.clone()).collect();
            if parent.is_none() {
                roots.push(key.clone())
            }
            order.push(key.clone());
            records.insert(
                key.clone(),
                Record {
                    node: Rc::new(node),
                    parent,
                    children: child_keys,
                    depth,
                },
            );
            stack.extend(
                children
                    .into_iter()
                    .rev()
                    .map(|n| (n, Some(key.clone()), depth + 1)),
            );
        }
        Ok((records, roots, order))
    }
    fn enabled(&self, key: &str) -> bool {
        self.node(key).is_some_and(|n| !n.disabled)
    }
    fn visible_enabled(&self, key: &str) -> bool {
        self.enabled(key) && self.positions.contains_key(&ElementId::Name(key.into()))
    }
    fn ensure_active(&mut self) {
        if !self
            .active
            .as_ref()
            .is_some_and(|k| self.visible_enabled(k))
        {
            self.active = self
                .rows
                .iter()
                .find(|r| self.enabled(&r.key))
                .map(|r| r.key.clone())
        }
    }
    pub fn set_active(&mut self, key: SharedString) -> bool {
        if !self.visible_enabled(&key) {
            return false;
        }
        self.active = Some(key);
        true
    }
    pub fn is_expanded(&self, key: &str) -> bool {
        if self.filtered.is_some() {
            !self.folded.contains(key)
        } else {
            self.expanded.contains(key)
        }
    }
    pub fn set_expanded(
        &mut self,
        keys: impl IntoIterator<Item = SharedString>,
    ) -> Result<(), TreeDataError> {
        let keys: HashSet<_> = keys.into_iter().collect();
        for k in &keys {
            if self.node(k).is_none() {
                return Err(TreeDataError::UnknownKey(k.clone()));
            }
        }
        self.expanded = keys;
        self.rebuild();
        Ok(())
    }
    pub fn toggle_expanded(&mut self, key: &str) -> Result<bool, TreeDataError> {
        let node = self
            .node(key)
            .ok_or_else(|| TreeDataError::UnknownKey(key.into()))?;
        if node.is_leaf() || node.disabled {
            return Ok(false);
        }
        if self.filtered.is_some() {
            if !self.folded.remove(key) {
                self.folded.insert(key.into());
            }
        } else if !self.expanded.remove(key) {
            self.expanded.insert(key.into());
        }
        let expanded = self.is_expanded(key);
        if !expanded
            && self
                .active
                .as_ref()
                .is_some_and(|k| self.ancestors(k).iter().any(|p| p == key))
        {
            self.active = Some(key.into())
        }
        self.rebuild();
        Ok(true)
    }
    pub fn reveal(&mut self, key: &str) -> Result<(), TreeDataError> {
        if self.node(key).is_none() {
            return Err(TreeDataError::UnknownKey(key.into()));
        }
        for ancestor in self.ancestors(key) {
            self.expanded.insert(ancestor.clone());
            self.folded.remove(&ancestor);
        }
        self.rebuild();
        if self.visible_enabled(key) {
            self.active = Some(key.into())
        }
        Ok(())
    }
    /// A synchronous pure matcher. Ancestor retention and temporary expansion
    /// remain owned by the model, regardless of the matching policy.
    pub fn set_filter(&mut self, filter: Option<TreeFilter>) {
        self.filter = filter;
        self.rebuild();
    }
    pub fn filter(&mut self, query: &str) {
        if self.query == query {
            return;
        }
        self.query = query.to_owned();
        self.folded.clear();
        self.rebuild()
    }
    fn rebuild(&mut self) {
        if self.query.trim().is_empty() {
            self.filtered = None
        } else {
            let q = self.query.to_lowercase();
            let mut matched = HashSet::new();
            // Reverse preorder propagates matches to ancestors in O(nodes).
            for k in self.order.iter().rev() {
                let r = &self.records[k];
                let matched_node = self.filter.as_ref().map_or_else(
                    || {
                        r.node.label.to_lowercase().contains(&q)
                            || r.node.keywords.to_lowercase().contains(&q)
                    },
                    |filter| filter(&r.node, &self.query),
                );
                if matched_node {
                    matched.insert(k.clone());
                }
                if matched.contains(k)
                    && let Some(p) = &r.parent
                {
                    matched.insert(p.clone());
                }
            }
            self.filtered = Some(matched);
        }
        let mut rows = vec![];
        let mut stack = vec![];
        let children: Vec<_> = self
            .roots
            .iter()
            .filter(|k| self.filtered.as_ref().is_none_or(|s| s.contains(*k)))
            .cloned()
            .collect();
        let count = children.len();
        for (ix, k) in children.into_iter().enumerate().rev() {
            stack.push((k, ix + 1, count, None))
        }
        let mut lines = vec![];
        let empty_lines = Rc::new(vec![]);
        while let Some((key, position, siblings, parent_row)) = stack.pop() {
            let r = &self.records[&key];
            rows.push(TreeRow {
                key: key.clone(),
                depth: r.depth,
                position,
                siblings,
                parent_row,
                line_index: empty_lines.clone(),
            });
            lines.push((parent_row, position < siblings));
            let parent_row = Some(lines.len() - 1);
            if self.is_expanded(&key) {
                let children: Vec<_> = r
                    .children
                    .iter()
                    .filter(|k| self.filtered.as_ref().is_none_or(|s| s.contains(*k)))
                    .cloned()
                    .collect();
                let count = children.len();
                for (ix, k) in children.into_iter().enumerate().rev() {
                    stack.push((k, ix + 1, count, parent_row))
                }
            }
        }
        let lines = Rc::new(lines);
        for row in &mut rows {
            row.line_index = lines.clone();
        }
        self.positions = Rc::new(
            rows.iter()
                .enumerate()
                .map(|(i, r)| (ElementId::Name(r.key.clone()), i))
                .collect(),
        );
        self.rows = Rc::new(rows);
        self.ensure_active();
        self.revision += 1;
    }
    pub fn navigate(&mut self, movement: TreeMove) {
        if self.rows.is_empty() {
            self.active = None;
            return;
        }
        let ix = self
            .active
            .as_ref()
            .and_then(|k| self.positions.get(&ElementId::Name(k.clone())).copied())
            .unwrap_or(0);
        let key = self.rows[ix].key.clone();
        let target = match movement {
            TreeMove::First => self
                .rows
                .iter()
                .find(|r| self.enabled(&r.key))
                .map(|r| r.key.clone()),
            TreeMove::Last => self
                .rows
                .iter()
                .rev()
                .find(|r| self.enabled(&r.key))
                .map(|r| r.key.clone()),
            TreeMove::Previous => self.rows[..ix]
                .iter()
                .rev()
                .find(|r| self.enabled(&r.key))
                .map(|r| r.key.clone()),
            TreeMove::Next => self.rows[ix + 1..]
                .iter()
                .find(|r| self.enabled(&r.key))
                .map(|r| r.key.clone()),
            TreeMove::Page(n) => {
                let mut target = ix;
                let mut count = n.unsigned_abs();
                while count > 0 {
                    let next = if n < 0 {
                        target.checked_sub(1)
                    } else {
                        (target + 1 < self.rows.len()).then_some(target + 1)
                    };
                    let Some(next) = next else {
                        break;
                    };
                    target = next;
                    if self.enabled(&self.rows[target].key) {
                        count -= 1;
                    }
                }
                self.rows
                    .get(target)
                    .filter(|r| self.enabled(&r.key))
                    .map(|r| r.key.clone())
            }
            TreeMove::Parent => {
                if self.node(&key).is_some_and(|n| !n.is_leaf()) && self.is_expanded(&key) {
                    let _ = self.toggle_expanded(&key);
                    None
                } else {
                    self.ancestors(&key)
                        .into_iter()
                        .find(|k| self.visible_enabled(k))
                }
            }
            TreeMove::Child => {
                if self.node(&key).is_some_and(|n| !n.is_leaf()) && !self.is_expanded(&key) {
                    let _ = self.toggle_expanded(&key);
                    None
                } else {
                    self.rows
                        .iter()
                        .skip(ix + 1)
                        .take_while(|r| r.depth > self.records[&key].depth)
                        .find(|r| self.enabled(&r.key))
                        .map(|r| r.key.clone())
                }
            }
        };
        if let Some(target) = target {
            self.active = Some(target);
        }
    }
    fn validate_keys(&self, keys: &[SharedString], checked: bool) -> Result<(), TreeDataError> {
        let mut seen = HashSet::new();
        for k in keys {
            if k.is_empty() {
                return Err(TreeDataError::EmptyKey);
            }
            if !seen.insert(k) {
                return Err(TreeDataError::DuplicateKey(k.clone()));
            }
            match self.node(k) {
                None if !self.retain_missing => return Err(TreeDataError::UnknownKey(k.clone())),
                Some(n)
                    if (!checked && !n.selectable)
                        || (checked
                            && (!n.checkable
                                || (self.check == TreeCheckMode::Cascade && !n.is_leaf()))) =>
                {
                    return Err(TreeDataError::InvalidValue(k.clone()));
                }
                _ => {}
            }
        }
        Ok(())
    }
    pub fn set_selected(&mut self, keys: Vec<SharedString>) -> Result<(), TreeDataError> {
        if (self.selection == TreeSelectionMode::None && !keys.is_empty())
            || (self.selection == TreeSelectionMode::Single && keys.len() > 1)
        {
            return Err(TreeDataError::Mode);
        }
        self.validate_keys(&keys, false)?;
        self.selected = keys;
        Ok(())
    }
    /// Capture the pre-navigation active item for the first Shift range step.
    pub fn begin_range_selection(&mut self) {
        if !self
            .anchor
            .as_ref()
            .is_some_and(|a| self.visible_enabled(a))
        {
            self.anchor = self.active.clone();
        }
    }
    pub fn select(&mut self, key: &str, gesture: TreeSelectionGesture) -> bool {
        if self.selection == TreeSelectionMode::None
            || !self.node(key).is_some_and(|n| n.selectable && !n.disabled)
        {
            return false;
        }
        let before = self.selected.clone();
        let old_active = self.active.clone();
        self.active = Some(key.into());
        if self.selection == TreeSelectionMode::Single {
            self.selected = vec![key.into()]
        } else if gesture.range {
            let anchor = self
                .anchor
                .as_ref()
                .filter(|a| self.visible_enabled(a))
                .cloned()
                .or(old_active);
            let a = anchor
                .as_ref()
                .and_then(|a| self.rows.iter().position(|r| r.key == *a))
                .unwrap_or(0);
            let b = self.rows.iter().position(|r| r.key == key).unwrap_or(a);
            if !gesture.additive {
                let locked: HashSet<_> = self
                    .selected
                    .iter()
                    .filter(|k| self.is_locked(k, false))
                    .cloned()
                    .collect();
                self.selected.retain(|k| locked.contains(k));
            }
            let mut chosen: HashSet<_> = self.selected.iter().cloned().collect();
            for row in &self.rows[a.min(b)..=a.max(b)] {
                if self
                    .node(&row.key)
                    .is_some_and(|n| n.selectable && !n.disabled)
                    && chosen.insert(row.key.clone())
                {
                    self.selected.push(row.key.clone())
                }
            }
        } else if gesture.toggle {
            if let Some(i) = self.selected.iter().position(|k| k == key) {
                self.selected.remove(i);
            } else {
                self.selected.push(key.into())
            }
            self.anchor = Some(key.into())
        } else {
            self.selected = vec![key.into()];
            self.anchor = Some(key.into())
        }
        before != self.selected
    }
    pub fn select_visible(&mut self) {
        if self.selection != TreeSelectionMode::Multiple {
            return;
        }
        let mut chosen: HashSet<_> = self.selected.iter().cloned().collect();
        for row in self.rows.iter() {
            if self
                .node(&row.key)
                .is_some_and(|n| n.selectable && !n.disabled)
                && chosen.insert(row.key.clone())
            {
                self.selected.push(row.key.clone())
            }
        }
    }
    pub fn set_checked(&mut self, keys: Vec<SharedString>) -> Result<(), TreeDataError> {
        if self.check == TreeCheckMode::None && !keys.is_empty() {
            return Err(TreeDataError::Mode);
        }
        self.validate_keys(&keys, true)?;
        self.checked = keys;
        self.recompute();
        Ok(())
    }
    pub fn is_locked(&self, key: &str, checked: bool) -> bool {
        self.node(key)
            .is_some_and(|n| n.disabled || (checked && n.check_disabled))
    }
    fn boundary(&self, key: &str) -> bool {
        self.node(key)
            .is_some_and(|n| n.disabled || n.check_disabled)
    }
    fn recompute(&mut self) {
        self.checked_set = self.checked.iter().cloned().collect();
        self.summaries.clear();
        for k in self.order.iter().rev() {
            let r = &self.records[k];
            let mut sum = Summary::default();
            if r.node.is_leaf() {
                if r.node.checkable && !self.boundary(k) {
                    sum.total = 1;
                    sum.chosen = usize::from(self.checked_set.contains(k));
                }
            } else {
                sum.unknown = matches!(r.node.children, TreeChildren::Unloaded);
                for child in &r.children {
                    if !self.boundary(child) {
                        let s = self.summaries[child];
                        sum.total += s.total;
                        sum.chosen += s.chosen;
                        sum.unknown |= s.unknown;
                    }
                }
            }
            self.summaries.insert(k.clone(), sum);
        }
        if self.check == TreeCheckMode::Cascade {
            let chosen = &self.checked_set;
            self.checked = self
                .order
                .iter()
                .filter(|k| chosen.contains(*k))
                .cloned()
                .collect();
        }
    }
    pub fn mark(&self, key: &str) -> TreeMark {
        if self.check != TreeCheckMode::Cascade || self.node(key).is_some_and(|n| n.is_leaf()) {
            return if self.checked_set.contains(key) {
                TreeMark::Checked
            } else {
                TreeMark::Unchecked
            };
        }
        let s = self.summaries.get(key).copied().unwrap_or_default();
        if s.total > 0 && s.total == s.chosen && !s.unknown {
            TreeMark::Checked
        } else if s.chosen > 0 {
            TreeMark::Mixed
        } else {
            TreeMark::Unchecked
        }
    }
    pub fn can_check(&self, key: &str) -> bool {
        self.check != TreeCheckMode::None
            && self
                .node(key)
                .is_some_and(|n| n.checkable && !n.disabled && !n.check_disabled)
            && (self.check != TreeCheckMode::Cascade
                || self.node(key).is_some_and(|n| n.is_leaf())
                || self
                    .summaries
                    .get(key)
                    .is_some_and(|s| s.total > 0 && !s.unknown))
    }
    fn leaves(&self, key: &str) -> Vec<SharedString> {
        let mut out = vec![];
        let mut stack = vec![SharedString::from(key)];
        while let Some(k) = stack.pop() {
            if self.boundary(&k) {
                continue;
            }
            let Some(r) = self.records.get(&k) else {
                continue;
            };
            if r.node.is_leaf() {
                if r.node.checkable {
                    out.push(k)
                }
            } else {
                stack.extend(r.children.iter().rev().cloned())
            }
        }
        out
    }
    pub fn toggle_check(&mut self, key: &str) -> Result<bool, TreeDataError> {
        if !self.can_check(key) {
            return if self.summaries.get(key).is_some_and(|s| s.unknown) {
                Err(TreeDataError::Incomplete(key.into()))
            } else {
                Ok(false)
            };
        }
        let chosen = self.mark(key) == TreeMark::Checked;
        let targets = if self.check == TreeCheckMode::Cascade {
            self.leaves(key)
        } else {
            vec![key.into()]
        };
        if chosen {
            let targets: HashSet<_> = targets.into_iter().collect();
            self.checked.retain(|k| !targets.contains(k));
        } else {
            self.checked.extend(
                targets
                    .into_iter()
                    .filter(|k| !self.checked_set.contains(k)),
            );
        }
        self.recompute();
        Ok(true)
    }
    pub fn remove_value(&mut self, key: &str, checked: bool) -> bool {
        if self.is_locked(key, checked) {
            return false;
        }
        if checked {
            let targets: HashSet<_> = if self.check == TreeCheckMode::Cascade {
                self.leaves(key)
            } else {
                vec![key.into()]
            }
            .into_iter()
            .collect();
            let before = self.checked.len();
            self.checked.retain(|k| !targets.contains(k));
            self.recompute();
            before != self.checked.len()
        } else {
            let before = self.selected.len();
            self.selected.retain(|k| k != key);
            before != self.selected.len()
        }
    }
    pub fn clear_values(&mut self, checked: bool) -> bool {
        let locked: HashSet<_> = if checked {
            &self.checked
        } else {
            &self.selected
        }
        .iter()
        .filter(|k| self.is_locked(k, checked))
        .cloned()
        .collect();
        let values = if checked {
            &mut self.checked
        } else {
            &mut self.selected
        };
        let before = values.len();
        values.retain(|k| locked.contains(k));
        let changed = before != values.len();
        if checked {
            self.recompute()
        }
        changed
    }
    pub fn project_checked(&self, projection: TreeCheckProjection) -> Vec<SharedString> {
        if self.check != TreeCheckMode::Cascade || projection == TreeCheckProjection::Leaves {
            return self.checked.clone();
        }
        let mut out = vec![];
        let mut covered = HashSet::new();
        for k in &self.order {
            let n = self.node(k).unwrap();
            if !n.checkable || self.mark(k) != TreeMark::Checked {
                continue;
            }
            if projection == TreeCheckProjection::All {
                out.push(k.clone());
                continue;
            }
            if n.is_leaf() && self.is_locked(k, true) {
                out.push(k.clone());
                continue;
            }
            if covered.contains(k) {
                continue;
            }
            out.push(k.clone());
            let mut stack = self.records[k].children.clone();
            while let Some(c) = stack.pop() {
                if self.boundary(&c) {
                    continue;
                }
                covered.insert(c.clone());
                stack.extend(self.records[&c].children.iter().cloned());
            }
        }
        out
    }
    pub fn load_state(&self, key: Option<&SharedString>) -> Option<&TreeLoadState> {
        self.load.get(&key.cloned())
    }
    pub fn request_load(&mut self, key: Option<SharedString>) -> Option<TreeLoadTicket> {
        if self.pending.contains_key(&key) {
            return None;
        }
        if let Some(k) = &key {
            if !self
                .node(k)
                .is_some_and(|n| !n.disabled && matches!(n.children, TreeChildren::Unloaded))
            {
                return None;
            }
        }
        self.serial += 1;
        let ticket = TreeLoadTicket {
            owner: self.owner,
            generation: self.generation,
            serial: self.serial,
            target: key.clone(),
        };
        self.pending.insert(key.clone(), ticket.clone());
        self.load.insert(key, TreeLoadState::Loading);
        Some(ticket)
    }
    pub fn reject_load(&mut self, ticket: &TreeLoadTicket, error: SharedString) -> TreeApply {
        if self.pending.get(&ticket.target) != Some(ticket) {
            return TreeApply::Stale;
        }
        self.pending.remove(&ticket.target);
        self.load
            .insert(ticket.target.clone(), TreeLoadState::Failed(error));
        TreeApply::Applied
    }
    /// Check before preparing a potentially expensive application result.
    pub fn is_current_load(&self, ticket: &TreeLoadTicket) -> bool {
        self.pending.get(&ticket.target) == Some(ticket)
    }
    pub fn apply_children(
        &mut self,
        ticket: &TreeLoadTicket,
        nodes: Vec<TreeNode>,
    ) -> Result<TreeApply, TreeDataError> {
        if self.pending.get(&ticket.target) != Some(ticket) {
            return Ok(TreeApply::Stale);
        }
        let (new_records, new_roots, new_order) = match Self::import(nodes) {
            Ok(v) => v,
            Err(e) => {
                self.reject_load(ticket, e.to_string().into());
                return Err(e);
            }
        };
        if let Some(k) = new_order
            .iter()
            .find(|k| ticket.target.is_some() && self.records.contains_key(*k))
        {
            let e = TreeDataError::DuplicateKey(k.clone());
            self.reject_load(ticket, e.to_string().into());
            return Err(e);
        }
        if let Some(parent) = &ticket.target {
            let record = self
                .records
                .get_mut(parent)
                .expect("valid ticket has target");
            record.children = new_roots.clone();
            Rc::make_mut(&mut record.node).children = if new_roots.is_empty() {
                TreeChildren::Leaf
            } else {
                TreeChildren::Loaded(vec![])
            };
            let depth = record.depth + 1;
            for k in &new_order {
                let mut r = new_records[k].clone();
                r.depth += depth;
                if r.parent.is_none() {
                    r.parent = Some(parent.clone())
                }
                self.records.insert(k.clone(), r);
            }
            let ix = self.order.iter().position(|k| k == parent).unwrap() + 1;
            self.order.splice(ix..ix, new_order);
        } else {
            // Root results are a complete model. Validation above precedes all mutation.
            self.records = new_records;
            self.roots = new_roots;
            self.order = new_order;
            self.expanded
                .retain(|k| self.records.get(k).is_some_and(|r| !r.node.is_leaf()));
            self.selected
                .retain(|k| self.records.get(k).is_some_and(|r| r.node.selectable));
            self.checked.retain(|k| {
                self.records.get(k).is_some_and(|r| {
                    r.node.checkable && (self.check != TreeCheckMode::Cascade || r.node.is_leaf())
                })
            });
            self.generation += 1;
            self.pending.clear();
            self.load.clear();
        }
        self.pending.remove(&ticket.target);
        self.load.remove(&ticket.target);
        self.recompute();
        self.rebuild();
        Ok(TreeApply::Applied)
    }
}

/// Keep keyed TreeItem semantics in Base; styled consumers supply only content/actions.
pub fn tree_row(
    id: impl Into<ElementId>,
    node: &TreeNode,
    row: &TreeRow,
    expanded: bool,
    selected: bool,
    mark: Option<TreeMark>,
    active: bool,
) -> gpui::Stateful<gpui::Div> {
    use gpui::prelude::FluentBuilder as _;
    div()
        .id(id)
        .role(gpui::Role::TreeItem)
        .aria_label(node.label.clone())
        .aria_selected(selected)
        .aria_level(row.depth + 1)
        .aria_position_in_set(row.position)
        .aria_size_of_set(row.siblings)
        .when(!node.is_leaf(), |d| d.aria_expanded(expanded))
        .when_some(mark, |d, m| {
            d.aria_toggled(match m {
                TreeMark::Unchecked => gpui::Toggled::False,
                TreeMark::Checked => gpui::Toggled::True,
                TreeMark::Mixed => gpui::Toggled::Mixed,
            })
        })
        .when(active, |d| d.aria_active_descendant())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> TreeNode {
        TreeNode::new("p", "Parent")
            .children([TreeNode::new("a", "Alpha"), TreeNode::new("b", "Beta")])
    }
    fn model() -> KeyedTreeModel {
        let mut m = KeyedTreeModel::new(TreeSelectionMode::Multiple, TreeCheckMode::Cascade);
        m.replace([fixture()], false).unwrap();
        m
    }
    #[test]
    fn cascade_projection_is_not_value() {
        let mut m = model();
        m.toggle_check("a").unwrap();
        assert_eq!(m.mark("p"), TreeMark::Mixed);
        m.toggle_check("b").unwrap();
        assert_eq!(
            m.project_checked(TreeCheckProjection::Parents),
            vec![SharedString::from("p")]
        );
        assert_eq!(m.checked_keys(), ["a", "b"]);
        m.remove_value("p", true);
        assert!(m.checked_keys().is_empty());
    }
    #[test]
    fn unknown_descendants_never_grant_future_permissions() {
        let mut m = model();
        m.replace(
            [TreeNode::new("p", "P")
                .children([TreeNode::new("a", "A"), TreeNode::new("u", "U").unloaded()])],
            false,
        )
        .unwrap();
        m.toggle_check("a").unwrap();
        assert!(!m.can_check("p"));
        assert_eq!(m.mark("p"), TreeMark::Mixed);
        let t = m.request_load(Some("u".into())).unwrap();
        m.apply_children(&t, vec![TreeNode::new("b", "B")]).unwrap();
        assert_eq!(m.checked_keys(), ["a"]);
        assert!(m.can_check("p"));
    }
    #[test]
    fn disabled_boundary_is_not_swallowed_by_parent_projection() {
        let mut m = model();
        m.replace(
            [TreeNode::new("p", "P").children([
                TreeNode::new("a", "A"),
                TreeNode::new("b", "B").check_disabled(true),
            ])],
            false,
        )
        .unwrap();
        m.set_checked(vec!["b".into()]).unwrap();
        m.toggle_check("p").unwrap();
        assert_eq!(
            m.project_checked(TreeCheckProjection::Parents),
            vec![SharedString::from("p"), "b".into()]
        );
        m.remove_value("p", true);
        assert_eq!(m.checked_keys(), ["b"]);
        assert!(!m.clear_values(true));
    }
    #[test]
    fn invalid_replace_is_atomic_and_identity_survives_reorder() {
        let mut m = model();
        m.set_selected(vec!["b".into()]).unwrap();
        assert!(
            m.replace([fixture(), TreeNode::new("a", "duplicate")], false)
                .is_err()
        );
        assert_eq!(m.selected_keys(), ["b"]);
        m.replace(
            [
                TreeNode::new("b", "Renamed"),
                fixture().children([TreeNode::new("a", "A")]),
            ],
            false,
        )
        .unwrap();
        assert_eq!(m.selected_keys(), ["b"]);
    }
    #[test]
    fn filter_expansion_is_temporary() {
        let mut m = model();
        assert_eq!(m.rows().len(), 1);
        m.filter("Alpha");
        assert_eq!(
            m.rows().iter().map(|r| r.key.as_str()).collect::<Vec<_>>(),
            ["p", "a"]
        );
        m.filter("");
        assert_eq!(m.rows().len(), 1);
        assert!(!m.is_expanded("p"));
    }
    #[test]
    fn collapse_reanchors_active_without_changing_selection() {
        let mut m = model();
        m.reveal("b").unwrap();
        m.set_selected(vec!["b".into()]).unwrap();
        m.toggle_expanded("p").unwrap();
        assert_eq!(m.active_key().unwrap(), "p");
        assert_eq!(m.selected_keys(), ["b"]);
        m.navigate(TreeMove::Next);
        assert_eq!(m.active_key().unwrap(), "p");
    }
    #[test]
    fn stale_and_foreign_tickets_cannot_modify_models() {
        let mut m = model();
        m.replace([TreeNode::new("u", "U").unloaded()], false)
            .unwrap();
        let t = m.request_load(Some("u".into())).unwrap();
        assert!(m.request_load(Some("u".into())).is_none());
        m.replace([TreeNode::new("u", "new").unloaded()], false)
            .unwrap();
        assert_eq!(m.apply_children(&t, vec![]).unwrap(), TreeApply::Stale);
        let mut other = model();
        assert_eq!(other.reject_load(&t, "oops".into()), TreeApply::Stale);
    }
    #[test]
    fn concurrent_loading_does_not_invalidate_siblings() {
        let mut m = model();
        m.replace(
            [
                TreeNode::new("x", "X").unloaded(),
                TreeNode::new("y", "Y").unloaded(),
            ],
            false,
        )
        .unwrap();
        let x = m.request_load(Some("x".into())).unwrap();
        let y = m.request_load(Some("y".into())).unwrap();
        m.apply_children(&x, vec![]).unwrap();
        assert!(m.node("x").unwrap().is_leaf());
        assert_eq!(
            m.apply_children(&y, vec![TreeNode::new("z", "Z")]).unwrap(),
            TreeApply::Applied
        );
    }
    #[test]
    fn strict_order_and_multi_gestures_keep_independent_keys() {
        let mut m = KeyedTreeModel::new(TreeSelectionMode::Multiple, TreeCheckMode::Strict);
        m.replace([fixture(), TreeNode::new("z", "Z")], false)
            .unwrap();
        m.set_expanded(vec!["p".into()]).unwrap();
        m.select("a", TreeSelectionGesture::default());
        m.select("b", TreeSelectionGesture::default().toggle(true));
        assert_eq!(m.selected_keys(), ["a", "b"]);
        m.select("p", TreeSelectionGesture::default().range(true));
        assert_eq!(m.selected_keys(), ["p", "a", "b"]);
        m.toggle_check("p").unwrap();
        m.toggle_check("a").unwrap();
        m.toggle_check("p").unwrap();
        m.toggle_check("p").unwrap();
        assert_eq!(m.checked_keys(), ["a", "p"]);
        assert_eq!(
            m.project_checked(TreeCheckProjection::Parents),
            m.checked_keys()
        );
    }
    #[test]
    fn range_without_anchor_starts_at_previous_active() {
        let mut m = model();
        m.set_expanded(vec!["p".into()]).unwrap();
        m.set_active("a".into());
        m.begin_range_selection();
        m.navigate(TreeMove::Next);
        let k = m.active_key().unwrap().clone();
        m.select(&k, TreeSelectionGesture::default().range(true));
        assert_eq!(m.selected_keys(), ["a", "b"]);
    }
    #[test]
    fn checkability_and_selection_validation_are_atomic() {
        let mut m = model();
        m.replace(
            [
                TreeNode::new("a", "A"),
                TreeNode::new("b", "B").checkable(false).selectable(false),
            ],
            false,
        )
        .unwrap();
        m.set_checked(vec!["a".into()]).unwrap();
        assert_eq!(
            m.set_checked(vec!["b".into()]),
            Err(TreeDataError::InvalidValue("b".into()))
        );
        assert_eq!(m.checked_keys(), ["a"]);
        assert_eq!(
            m.set_selected(vec!["b".into()]),
            Err(TreeDataError::InvalidValue("b".into()))
        );
        let mut m = KeyedTreeModel::new(TreeSelectionMode::Multiple, TreeCheckMode::Strict);
        m.replace([TreeNode::new("a", "A")], false).unwrap();
        m.set_checked(vec!["a".into()]).unwrap();
        m.set_retain_missing(true).unwrap();
        m.set_selected(vec!["pending".into()]).unwrap();
        assert_eq!(
            m.set_selected(vec!["".into()]),
            Err(TreeDataError::EmptyKey)
        );
        assert_eq!(m.selected_keys(), ["pending"]);
        assert_eq!(m.set_checked(vec!["".into()]), Err(TreeDataError::EmptyKey));
        assert_eq!(m.checked_keys(), ["a"]);
    }
    #[test]
    fn transparent_nodes_and_disabled_subtrees_define_separate_domains() {
        let mut m = model();
        m.replace(
            [TreeNode::new("p", "P").children([
                TreeNode::new("transparent", "T")
                    .checkable(false)
                    .children([TreeNode::new("a", "A")]),
                TreeNode::new("blocked", "B")
                    .disabled(true)
                    .children([TreeNode::new("b", "B")]),
            ])],
            false,
        )
        .unwrap();
        m.toggle_check("b").unwrap();
        m.toggle_check("p").unwrap();
        assert_eq!(m.checked_keys(), ["a", "b"]);
        m.remove_value("p", true);
        assert_eq!(m.checked_keys(), ["b"]);
        assert_eq!(m.mark("p"), TreeMark::Unchecked);
    }
    #[test]
    fn leaf_becoming_branch_removes_only_old_canonical_value() {
        let mut m = model();
        m.set_checked(vec!["a".into(), "b".into()]).unwrap();
        let r = m
            .replace(
                [TreeNode::new("p", "P").children([
                    TreeNode::new("a", "A").children([TreeNode::new("c", "C")]),
                    TreeNode::new("b", "B"),
                ])],
                false,
            )
            .unwrap();
        assert_eq!(r.removed_checked(), ["a"]);
        assert_eq!(m.checked_keys(), ["b"]);
        assert_eq!(m.mark("p"), TreeMark::Mixed);
    }
    #[test]
    fn root_load_replaces_same_keys_and_failed_retry_has_a_new_ticket() {
        let mut m = model();
        m.set_expanded(vec!["p".into()]).unwrap();
        m.set_checked(vec!["a".into(), "b".into()]).unwrap();
        m.set_selected(vec!["b".into()]).unwrap();
        let root = m.request_load(None).unwrap();
        assert_eq!(
            m.apply_children(
                &root,
                vec![TreeNode::new("p", "Parent").children([
                    TreeNode::new("a", "A").children([TreeNode::new("c", "C")]),
                    TreeNode::new("b", "B").selectable(false),
                ])]
            )
            .unwrap(),
            TreeApply::Applied
        );
        assert!(m.is_expanded("p"));
        assert_eq!(m.checked_keys(), ["b"]);
        assert!(m.selected_keys().is_empty());
        assert_eq!(m.mark("p"), TreeMark::Mixed);
        m.replace([TreeNode::new("u", "U").unloaded()], false)
            .unwrap();
        let first = m.request_load(Some("u".into())).unwrap();
        assert_eq!(m.reject_load(&first, "offline".into()), TreeApply::Applied);
        let second = m.request_load(Some("u".into())).unwrap();
        assert_ne!(first, second);
        assert_eq!(m.apply_children(&first, vec![]).unwrap(), TreeApply::Stale);
        assert_eq!(
            m.apply_children(&second, vec![]).unwrap(),
            TreeApply::Applied
        );
    }
    #[test]
    fn ten_thousand_deep_rows_use_shared_linear_line_storage() {
        let mut root = TreeNode::new("9999", "Leaf");
        for i in (0..9999).rev() {
            root = TreeNode::new(i.to_string(), "Branch").children([root]);
        }
        let mut m = KeyedTreeModel::new(TreeSelectionMode::Single, TreeCheckMode::None);
        m.replace([root], false).unwrap();
        m.set_expanded((0..9999).map(|i| i.to_string().into()))
            .unwrap();
        assert_eq!(m.rows.len(), 10000);
        let row = m.rows.last().unwrap();
        assert_eq!(row.depth(), 9999);
        assert_eq!(row.line_index.len(), 10000);
        assert!(Rc::ptr_eq(&m.rows[0].line_index, &row.line_index));
        assert_eq!(row.continuations().len(), 9999);
        m.navigate(TreeMove::Last);
        assert_eq!(m.active_key().unwrap(), "9999");
    }
    #[test]
    fn custom_matcher_keeps_ancestor_projection_and_never_mutates_values() {
        let mut m = model();
        m.set_selected(vec!["b".into()]).unwrap();
        m.set_filter(Some(Rc::new(|n, q| n.key().as_str() == q)));
        m.filter("a");
        assert_eq!(
            m.rows.iter().map(|r| r.key.as_str()).collect::<Vec<_>>(),
            ["p", "a"]
        );
        assert_eq!(m.selected_keys(), ["b"]);
        m.filter("");
        assert!(!m.is_expanded("p"));
    }
}
