use super::*;
use gpui_kit::base::keyed_tree::{
    KeyedTreeModel, TreeApply, TreeCheckMode, TreeFilter, TreeLoadTicket, TreeNode,
    TreeSelectionMode,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Projection metadata. Business records remain borrowed from the original source.
pub struct TableOutlineRow {
    key: TableRowKey,
    record: TableRowKey,
    depth: usize,
    detail: bool,
    expanded: bool,
    branch: bool,
}
impl TableOutlineRow {
    pub fn key(&self) -> &TableRowKey {
        &self.key
    }
    pub fn record_key(&self) -> &TableRowKey {
        &self.record
    }
    pub fn depth(&self) -> usize {
        self.depth
    }
    pub fn is_detail(&self) -> bool {
        self.detail
    }
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }
    pub fn is_branch(&self) -> bool {
        self.branch
    }
}
pub struct TableOutlineSource<S: TableSource> {
    source: Rc<S>,
    revision: u64,
    order: Vec<TableRowKey>,
    positions: HashMap<TableRowKey, usize>,
    rows: HashMap<TableRowKey, TableOutlineRow>,
    root_total: usize,
    page_roots: usize,
    record_count: usize,
}
impl<S: TableSource> TableOutlineSource<S> {
    pub fn source(&self) -> &Rc<S> {
        &self.source
    }
    pub fn record(&self, row: &TableOutlineRow) -> Option<&S::Row> {
        self.source.row(&row.record)
    }
    pub fn root_total(&self) -> usize {
        self.root_total
    }
}
impl<S: TableSource> TableSource for TableOutlineSource<S> {
    type Row = TableOutlineRow;
    fn scope(&self) -> &SharedString {
        self.source.scope()
    }
    fn revision(&self) -> u64 {
        self.revision
    }
    fn len(&self) -> usize {
        self.order.len()
    }
    fn row_key(&self, ix: usize) -> Option<&TableRowKey> {
        self.order.get(ix)
    }
    fn position_of(&self, key: &TableRowKey) -> Option<usize> {
        self.positions.get(key).copied()
    }
    fn row(&self, key: &TableRowKey) -> Option<&Self::Row> {
        self.rows.get(key)
    }
    fn contains_key(&self, key: &TableRowKey) -> bool {
        self.source.contains_key(key)
    }
    fn row_revision(&self, key: &TableRowKey) -> u64 {
        self.rows
            .get(key)
            .map(|row| self.source.row_revision(&row.record))
            .unwrap_or(self.revision)
    }
    fn is_detail(&self, key: &TableRowKey) -> bool {
        self.rows.get(key).is_some_and(|row| row.detail)
    }
    fn page_len(&self) -> usize {
        self.page_roots
    }
    fn record_len(&self) -> usize {
        self.record_count
    }
}

/// Host-owned structural adapter. Expansion/loading use Base's keyed tree model;
/// detail disclosure has an independent set of explicitly named stable keys.
pub struct TableOutline<S: TableSource> {
    source: Rc<S>,
    tree: Option<KeyedTreeModel>,
    details: BTreeMap<TableRowKey, TableRowKey>,
    open_details: BTreeSet<TableRowKey>,
    check: TreeCheckMode,
}
fn tree_error(error: impl std::fmt::Display) -> TableError {
    TableError::InvalidQuery(error.to_string().into())
}
fn validate_nodes<S: TableSource>(source: &S, nodes: &[TreeNode]) -> Result<(), TableError> {
    let mut pending: Vec<_> = nodes.iter().collect();
    while let Some(node) = pending.pop() {
        let key = TableRowKey::try_new(node.key().clone())?;
        if source.row(&key).is_none() {
            return Err(TableError::MissingKey);
        }
        if let gpui_kit::base::keyed_tree::TreeChildren::Loaded(children) = node.child_state() {
            pending.extend(children);
        }
    }
    Ok(())
}
impl<S: TableSource> TableOutline<S> {
    pub fn new(source: Rc<S>) -> Self {
        Self {
            source,
            tree: None,
            details: BTreeMap::new(),
            open_details: BTreeSet::new(),
            check: TreeCheckMode::None,
        }
    }
    /// Opt in before publishing the tree. Default Table selection is independent.
    pub fn with_check_mode(mut self, mode: TreeCheckMode) -> Result<Self, TableError> {
        if self.tree.is_some() {
            return Err(TableError::UnsupportedOperation(
                "configure checking before the tree",
            ));
        }
        self.check = mode;
        Ok(self)
    }
    pub fn set_tree(&mut self, nodes: Vec<TreeNode>) -> Result<(), TableError> {
        validate_nodes(self.source.as_ref(), &nodes)?;
        let mut model = KeyedTreeModel::new(TreeSelectionMode::None, self.check);
        model.replace(nodes, false).map_err(tree_error)?;
        self.tree = Some(model);
        Ok(())
    }
    pub fn tree(&self) -> Option<&KeyedTreeModel> {
        self.tree.as_ref()
    }
    /// Apply controlled Table selection using Base's opt-in cascade rules.
    /// Unknown lazy subtrees keep Base's explicit Incomplete error.
    pub fn apply_selection(
        &mut self,
        request: &TableSelectionRequest,
        accepted: &TableView<TableOutlineSource<S>>,
    ) -> Result<TableView<TableOutlineSource<S>>, TableError> {
        if self.check == TreeCheckMode::None {
            return request.apply_to(accepted);
        }
        if request.scope != *self.source.scope()
            || accepted.source.source.scope() != self.source.scope()
        {
            return Err(TableError::StaleSource);
        }
        let tree = self
            .tree
            .as_mut()
            .ok_or(TableError::UnsupportedOperation("no tree"))?;
        let previous = tree.checked_keys().to_vec();
        let leaves = accepted
            .selection
            .iter()
            .filter(|key| tree.node(key.as_str()).is_some_and(|node| node.is_leaf()))
            .map(|key| SharedString::from(key.as_str().to_owned()))
            .collect();
        tree.set_checked(leaves).map_err(tree_error)?;
        let result = (|| {
            match &request.intent {
                model::SelectionIntent::Toggle(key) => {
                    tree.toggle_check(key.as_str()).map_err(tree_error)?;
                }
                model::SelectionIntent::Clear => {
                    tree.set_checked(vec![]).map_err(tree_error)?;
                }
                model::SelectionIntent::All(checked) => {
                    for ix in 0..accepted.source.len() {
                        let key = accepted.source.row_key(ix).unwrap();
                        if !accepted.source.is_detail(key)
                            && !accepted.disabled_rows.contains(key)
                            && (tree.mark(key.as_str())
                                == gpui_kit::base::keyed_tree::TreeMark::Checked)
                                != *checked
                        {
                            tree.toggle_check(key.as_str()).map_err(tree_error)?;
                        }
                    }
                }
                model::SelectionIntent::Range(_, _) => {
                    let next = request.apply_to(accepted)?;
                    tree.set_checked(
                        next.selection
                            .iter()
                            .filter(|key| {
                                tree.node(key.as_str()).is_some_and(|node| node.is_leaf())
                            })
                            .map(|key| SharedString::from(key.as_str().to_owned()))
                            .collect(),
                    )
                    .map_err(tree_error)?;
                }
            }
            let selection = tree
                .project_checked(gpui_kit::base::keyed_tree::TreeCheckProjection::All)
                .into_iter()
                .map(TableRowKey::try_new)
                .collect::<Result<Vec<_>, _>>()?;
            let next = accepted
                .clone()
                .selection(selection)
                .based_on(accepted.revision());
            next.validate(false)?;
            Ok(next)
        })();
        if result.is_err() {
            tree.set_checked(previous).map_err(tree_error)?;
        }
        result
    }
    pub fn sort_siblings(
        &mut self,
        mut compare: impl FnMut(&S::Row, &S::Row) -> std::cmp::Ordering,
    ) -> Result<(), TableError> {
        let source = &self.source;
        self.tree
            .as_mut()
            .ok_or(TableError::UnsupportedOperation("no tree"))?
            .sort_siblings(|a, b| {
                compare(
                    source
                        .row(&TableRowKey::try_new(a.key().clone()).unwrap())
                        .unwrap(),
                    source
                        .row(&TableRowKey::try_new(b.key().clone()).unwrap())
                        .unwrap(),
                )
            });
        Ok(())
    }
    pub fn toggle_tree(&mut self, key: &TableRowKey) -> Result<bool, TableError> {
        self.tree
            .as_mut()
            .ok_or(TableError::UnsupportedOperation("no tree"))?
            .toggle_expanded(key.as_str())
            .map_err(tree_error)
    }
    pub fn filter_tree(
        &mut self,
        query: &str,
        filter: Option<TreeFilter>,
    ) -> Result<(), TableError> {
        let tree = self
            .tree
            .as_mut()
            .ok_or(TableError::UnsupportedOperation("no tree"))?;
        tree.set_filter(filter);
        tree.filter(query);
        Ok(())
    }
    pub fn request_children(
        &mut self,
        key: &TableRowKey,
    ) -> Result<Option<TreeLoadTicket>, TableError> {
        Ok(self
            .tree
            .as_mut()
            .ok_or(TableError::UnsupportedOperation("no tree"))?
            .request_load(Some(key.as_str().into())))
    }
    pub fn reject_children(
        &mut self,
        ticket: &TreeLoadTicket,
        error: SharedString,
    ) -> Result<TreeApply, TableError> {
        Ok(self
            .tree
            .as_mut()
            .ok_or(TableError::UnsupportedOperation("no tree"))?
            .reject_load(ticket, error))
    }
    pub fn apply_children(
        &mut self,
        ticket: &TreeLoadTicket,
        nodes: Vec<TreeNode>,
        source: Rc<S>,
    ) -> Result<TreeApply, TableError> {
        if !self
            .tree
            .as_ref()
            .is_some_and(|tree| tree.is_current_load(ticket))
        {
            return Ok(TreeApply::Stale);
        }
        if source.scope() != self.source.scope() {
            return Err(TableError::StaleSource);
        }
        if !Rc::ptr_eq(&source, &self.source) && source.revision() <= self.source.revision() {
            return Err(TableError::StaleSource);
        }
        validate_nodes(source.as_ref(), &nodes)?;
        let tree = self
            .tree
            .as_mut()
            .ok_or(TableError::UnsupportedOperation("no tree"))?;
        if tree.keys().iter().any(|key| {
            source
                .row(&TableRowKey::try_new(key.clone()).unwrap())
                .is_none()
        }) {
            return Err(TableError::MissingKey);
        }
        let outcome = tree.apply_children(ticket, nodes).map_err(tree_error)?;
        if outcome == TreeApply::Applied {
            self.source = source;
        }
        Ok(outcome)
    }
    pub fn set_details(
        &mut self,
        pairs: impl IntoIterator<Item = (TableRowKey, TableRowKey)>,
    ) -> Result<(), TableError> {
        let mut values = BTreeMap::new();
        let mut seen = BTreeSet::new();
        for (parent, detail) in pairs {
            if self.source.row(&parent).is_none() {
                return Err(TableError::MissingKey);
            }
            if self.source.contains_key(&detail) || !seen.insert(detail.clone()) {
                return Err(TableError::DuplicateRowKey(detail));
            }
            if values.insert(parent.clone(), detail).is_some() {
                return Err(TableError::DuplicateRowKey(parent));
            }
        }
        self.open_details.retain(|key| values.contains_key(key));
        self.details = values;
        Ok(())
    }
    pub fn toggle_detail(&mut self, parent: &TableRowKey) -> Result<bool, TableError> {
        if !self.details.contains_key(parent) {
            return Err(TableError::MissingKey);
        }
        if !self.open_details.remove(parent) {
            self.open_details.insert(parent.clone());
        }
        Ok(self.open_details.contains(parent))
    }
    pub fn snapshot(
        &self,
        revision: u64,
        page: Option<TablePage>,
    ) -> Result<TableOutlineSource<S>, TableError> {
        let mut rows = HashMap::new();
        let mut visible = Vec::new();
        let mut roots = 0;
        if let Some(tree) = &self.tree {
            for key in tree.keys() {
                let key = TableRowKey::try_new(key.clone())?;
                let node = tree.node(key.as_str()).unwrap();
                rows.insert(
                    key.clone(),
                    TableOutlineRow {
                        record: key.clone(),
                        key,
                        depth: 0,
                        detail: false,
                        expanded: tree.is_expanded(node.key()),
                        branch: !node.is_leaf(),
                    },
                );
            }
            let projected = tree.rows();
            for row in projected.iter() {
                if row.depth() == 0 {
                    roots += 1;
                }
                let included = page.is_none_or(|p| (roots - 1) / p.size() == p.number() - 1);
                let key = TableRowKey::try_new(row.key().clone())?;
                rows.get_mut(&key).unwrap().depth = row.depth();
                if included {
                    visible.push(key);
                }
            }
        } else {
            roots = self.source.len();
            for ix in 0..self.source.len() {
                let key = self
                    .source
                    .row_key(ix)
                    .ok_or(TableError::InvalidSource)?
                    .clone();
                rows.insert(
                    key.clone(),
                    TableOutlineRow {
                        record: key.clone(),
                        key: key.clone(),
                        depth: 0,
                        detail: false,
                        expanded: false,
                        branch: false,
                    },
                );
                if page.is_none_or(|p| ix / p.size() == p.number() - 1) {
                    visible.push(key);
                }
            }
        }
        let record_count = visible.len();
        let page_roots = visible.iter().filter(|key| rows[*key].depth == 0).count();
        let mut order = Vec::with_capacity(visible.len() + self.open_details.len());
        for key in visible {
            order.push(key.clone());
            if self.open_details.contains(&key) {
                let detail = self.details[&key].clone();
                let depth = rows[&key].depth;
                rows.insert(
                    detail.clone(),
                    TableOutlineRow {
                        key: detail.clone(),
                        record: key,
                        depth,
                        detail: true,
                        expanded: false,
                        branch: false,
                    },
                );
                order.push(detail);
            }
        }
        let positions = order
            .iter()
            .enumerate()
            .map(|(ix, key)| (key.clone(), ix))
            .collect();
        Ok(TableOutlineSource {
            source: self.source.clone(),
            revision,
            order,
            positions,
            rows,
            root_total: roots,
            page_roots,
            record_count,
        })
    }
}
