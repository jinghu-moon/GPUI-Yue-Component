use super::TableSize;
use gpui_kit::{
    AnyElement, App, ElementId, IntoElement, ParentElement as _, Pixels, Rems, SharedString,
    Styled as _, Window, div, rems,
};
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    rc::Rc,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TableError {
    EmptyKey,
    DuplicateRowKey(TableRowKey),
    DuplicateColumnKey(TableColumnKey),
    InvalidSource,
    InvalidGeometry,
    InvalidSelection,
    StaleView,
    StaleSource,
    StaleRequest,
    MissingKey,
    Unbound,
    UnsupportedOperation(&'static str),
    TooManyPendingRequests,
    Virtualizer(crate::VirtualizerError),
}
impl std::fmt::Display for TableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for TableError {}
impl From<crate::VirtualizerError> for TableError {
    fn from(value: crate::VirtualizerError) -> Self {
        Self::Virtualizer(value)
    }
}

macro_rules! key {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(SharedString);
        impl $name {
            pub fn try_new(value: impl Into<SharedString>) -> Result<Self, TableError> {
                let value = value.into();
                if value.trim().is_empty() {
                    Err(TableError::EmptyKey)
                } else {
                    Ok(Self(value))
                }
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
            pub fn element_id(&self) -> ElementId {
                self.0.clone().into()
            }
        }
    };
}
key!(TableRowKey);
key!(TableColumnKey);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableCellKey {
    row: TableRowKey,
    column: TableColumnKey,
}
impl TableCellKey {
    pub fn new(row: TableRowKey, column: TableColumnKey) -> Self {
        Self { row, column }
    }
    pub fn row(&self) -> &TableRowKey {
        &self.row
    }
    pub fn column(&self) -> &TableColumnKey {
        &self.column
    }
}

/// Immutable after publication. Implementations must not mutate records behind
/// shared references; publish a new source/revision instead. Lookups should be O(1)/O(log N).
pub trait TableSource: 'static {
    type Row: 'static;
    fn scope(&self) -> &SharedString;
    fn revision(&self) -> u64;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn row_key(&self, position: usize) -> Option<&TableRowKey>;
    fn position_of(&self, key: &TableRowKey) -> Option<usize>;
    fn row(&self, key: &TableRowKey) -> Option<&Self::Row>;
}

/// Optional slice adapter. Keeps the caller's allocation; Row need not implement Clone.
pub struct TableRows<R: 'static> {
    scope: SharedString,
    revision: u64,
    rows: Rc<[R]>,
    keys: Vec<TableRowKey>,
    positions: HashMap<TableRowKey, usize>,
}
impl<R: 'static> TableRows<R> {
    pub fn try_new(
        scope: impl Into<SharedString>,
        revision: u64,
        rows: Rc<[R]>,
        key_of: impl Fn(&R) -> SharedString,
    ) -> Result<Self, TableError> {
        let scope = scope.into();
        if scope.trim().is_empty() {
            return Err(TableError::EmptyKey);
        }
        let mut keys = Vec::with_capacity(rows.len());
        let mut positions = HashMap::with_capacity(rows.len());
        for (index, row) in rows.iter().enumerate() {
            let key = TableRowKey::try_new(key_of(row))?;
            if positions.insert(key.clone(), index).is_some() {
                return Err(TableError::DuplicateRowKey(key));
            }
            keys.push(key);
        }
        Ok(Self {
            scope,
            revision,
            rows,
            keys,
            positions,
        })
    }
}
impl<R: 'static> TableSource for TableRows<R> {
    type Row = R;
    fn scope(&self) -> &SharedString {
        &self.scope
    }
    fn revision(&self) -> u64 {
        self.revision
    }
    fn len(&self) -> usize {
        self.rows.len()
    }
    fn row_key(&self, position: usize) -> Option<&TableRowKey> {
        self.keys.get(position)
    }
    fn position_of(&self, key: &TableRowKey) -> Option<usize> {
        self.positions.get(key).copied()
    }
    fn row(&self, key: &TableRowKey) -> Option<&R> {
        self.positions.get(key).map(|index| &self.rows[*index])
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TableAlign {
    #[default]
    Start,
    Center,
    End,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TableVerticalAlign {
    Start,
    #[default]
    Center,
    End,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TableTextOverflow {
    #[default]
    Ellipsis,
    Wrap,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TableSelectionMode {
    #[default]
    None,
    Single,
    Multiple,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum TableLoadState {
    #[default]
    Ready,
    Loading,
    Refreshing,
    Failed(SharedString),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TableCommitOutcome {
    #[default]
    Applied,
    Unchanged,
}

pub struct TableCellContext {
    pub(super) key: TableCellKey,
    pub(super) position: usize,
    pub(super) selected: bool,
    pub(super) active: bool,
    pub(super) width: Pixels,
    pub(super) control_focus: Option<gpui_kit::FocusHandle>,
    pub(super) size: TableSize,
    pub(super) disabled: bool,
    pub(super) readonly: bool,
    pub(super) busy: bool,
}
impl TableCellContext {
    pub fn key(&self) -> &TableCellKey {
        &self.key
    }
    pub fn row_position(&self) -> usize {
        self.position
    }
    pub fn is_selected(&self) -> bool {
        self.selected
    }
    pub fn is_active(&self) -> bool {
        self.active
    }
    pub fn width(&self) -> Pixels {
        self.width
    }
    /// For the primary real control of the active interactive cell only.
    /// Attach this with track_focus; retain additional control state in the host.
    pub fn control_focus(&self) -> Option<&gpui_kit::FocusHandle> {
        self.control_focus.as_ref()
    }
    pub fn size(&self) -> TableSize {
        self.size
    }
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
    pub fn is_readonly(&self) -> bool {
        self.readonly
    }
    pub fn is_busy(&self) -> bool {
        self.busy
    }
}
type Renderer<R> = Rc<dyn Fn(&R, &TableCellContext, &mut Window, &mut App) -> AnyElement>;
type Text<R> = Rc<dyn Fn(&R) -> SharedString>;
pub struct TableColumn<R: 'static> {
    pub(super) key: TableColumnKey,
    pub(super) title: SharedString,
    pub(super) width: Rems,
    pub(super) align: TableAlign,
    pub(super) header_align: Option<TableAlign>,
    pub(super) vertical: TableVerticalAlign,
    pub(super) overflow: TableTextOverflow,
    pub(super) text: Option<Text<R>>,
    pub(super) renderer: Option<Renderer<R>>,
    pub(super) header: Option<super::Slot>,
    pub(super) footer: Option<SharedString>,
    pub(super) interactive: bool,
}
impl<R: 'static> TableColumn<R> {
    pub fn text<T: Into<SharedString>>(
        key: impl Into<SharedString>,
        title: impl Into<SharedString>,
        accessor: impl Fn(&R) -> T + 'static,
    ) -> Self {
        Self {
            key: TableColumnKey(key.into()),
            title: title.into(),
            width: rems(12.),
            align: TableAlign::Start,
            header_align: None,
            vertical: TableVerticalAlign::Center,
            overflow: TableTextOverflow::Ellipsis,
            text: Some(Rc::new(move |row| accessor(row).into())),
            renderer: None,
            header: None,
            footer: None,
            interactive: false,
        }
    }
    pub fn custom<E: IntoElement>(
        key: impl Into<SharedString>,
        title: impl Into<SharedString>,
        render: impl Fn(&R, &TableCellContext, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        let mut column = Self::text(key, title, |_| "");
        column.text = None;
        column.renderer = Some(Rc::new(move |r, cell, w, cx| {
            render(r, cell, w, cx).into_any_element()
        }));
        column
    }
    pub fn key(&self) -> &TableColumnKey {
        &self.key
    }
    pub fn title(&self) -> &SharedString {
        &self.title
    }
    pub fn width(mut self, width: Rems) -> Self {
        self.width = width;
        self
    }
    pub fn align(mut self, align: TableAlign) -> Self {
        self.align = align;
        self
    }
    pub fn header_align(mut self, align: TableAlign) -> Self {
        self.header_align = Some(align);
        self
    }
    pub fn vertical_align(mut self, align: TableVerticalAlign) -> Self {
        self.vertical = align;
        self
    }
    pub fn overflow(mut self, overflow: TableTextOverflow) -> Self {
        self.overflow = overflow;
        self
    }
    pub fn header<E: IntoElement>(
        mut self,
        render: impl Fn(&mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.header = Some(Rc::new(move |w, cx| render(w, cx).into_any_element()));
        self
    }
    pub fn footer(mut self, content: impl Into<SharedString>) -> Self {
        self.footer = Some(content.into());
        self
    }
    /// Enter/F2 enters this cell's real controls. Renderers use is_active() to
    /// expose their Tab stops; other cells must keep their controls out of Tab order.
    pub fn interactive(mut self, enabled: bool) -> Self {
        self.interactive = enabled;
        self
    }
    pub fn render(
        &self,
        row: &R,
        cell: &TableCellContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        if let Some(render) = &self.renderer {
            return render(row, cell, window, cx);
        }
        let value = self.text.as_ref().expect("text column")(row);
        match self.overflow {
            TableTextOverflow::Wrap => div().w_full().min_w_0().child(value).into_any_element(),
            TableTextOverflow::Ellipsis => {
                crate::YueOverflowTooltip::new("text", crate::current_theme(cx), value)
                    .width(gpui_kit::relative(1.))
                    .focusable_when_overflow(false)
                    .into_any_element()
            }
        }
    }
}

pub struct TableView<S: TableSource> {
    pub(super) source: Rc<S>,
    pub(super) columns: Rc<[TableColumn<S::Row>]>,
    pub(super) selection: BTreeSet<TableRowKey>,
    pub(super) disabled_rows: BTreeSet<TableRowKey>,
    pub(super) selection_mode: TableSelectionMode,
    pub(super) load: TableLoadState,
    pub(super) revision: u64,
    pub(super) base_revision: Option<u64>,
}
impl<S: TableSource> Clone for TableView<S> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            columns: self.columns.clone(),
            selection: self.selection.clone(),
            disabled_rows: self.disabled_rows.clone(),
            selection_mode: self.selection_mode,
            load: self.load.clone(),
            revision: self.revision,
            base_revision: self.base_revision,
        }
    }
}
impl<S: TableSource> TableView<S> {
    pub fn new(source: Rc<S>, columns: impl Into<Rc<[TableColumn<S::Row>]>>) -> Self {
        Self {
            source,
            columns: columns.into(),
            selection: BTreeSet::new(),
            disabled_rows: BTreeSet::new(),
            selection_mode: TableSelectionMode::None,
            load: TableLoadState::Ready,
            revision: 0,
            base_revision: None,
        }
    }
    pub fn source(&self) -> &Rc<S> {
        &self.source
    }
    pub fn shared_columns(&self) -> &Rc<[TableColumn<S::Row>]> {
        &self.columns
    }
    pub fn columns(&self) -> &[TableColumn<S::Row>] {
        &self.columns
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn selection_kind(&self) -> TableSelectionMode {
        self.selection_mode
    }
    pub fn selected_keys(&self) -> &BTreeSet<TableRowKey> {
        &self.selection
    }
    pub fn selection_mode(mut self, mode: TableSelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }
    pub fn selection(mut self, keys: impl IntoIterator<Item = TableRowKey>) -> Self {
        self.selection = keys.into_iter().collect();
        self
    }
    pub fn disabled_rows(mut self, keys: impl IntoIterator<Item = TableRowKey>) -> Self {
        self.disabled_rows = keys.into_iter().collect();
        self
    }
    pub fn load_state(mut self, state: TableLoadState) -> Self {
        self.load = state;
        self
    }
    pub fn load(&self) -> &TableLoadState {
        &self.load
    }
    /// Optimistic concurrency guard. Clone an accepted view for configuration changes.
    pub fn based_on(mut self, revision: u64) -> Self {
        self.base_revision = Some(revision);
        self
    }
    pub(super) fn same_inputs(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.source, &other.source)
            && Rc::ptr_eq(&self.columns, &other.columns)
            && self.selection == other.selection
            && self.disabled_rows == other.disabled_rows
            && self.selection_mode == other.selection_mode
            && self.load == other.load
    }
    pub(super) fn validate(&self, validate_source: bool) -> Result<(), TableError> {
        if validate_source {
            if self.source.scope().trim().is_empty() {
                return Err(TableError::EmptyKey);
            }
            let mut keys = HashSet::with_capacity(self.source.len());
            for position in 0..self.source.len() {
                let key = self
                    .source
                    .row_key(position)
                    .ok_or(TableError::InvalidSource)?;
                if key.as_str().trim().is_empty() {
                    return Err(TableError::EmptyKey);
                }
                if !keys.insert(key) {
                    return Err(TableError::DuplicateRowKey(key.clone()));
                }
                if self.source.position_of(key) != Some(position) || self.source.row(key).is_none()
                {
                    return Err(TableError::InvalidSource);
                }
            }
        }
        let mut keys = HashSet::new();
        let mut width = 0.;
        for column in self.columns.iter() {
            if column.key.as_str().trim().is_empty() {
                return Err(TableError::EmptyKey);
            }
            if !keys.insert(&column.key) {
                return Err(TableError::DuplicateColumnKey(column.key.clone()));
            }
            let extent = column.width.0;
            if !extent.is_finite() || extent <= 0. {
                return Err(TableError::InvalidGeometry);
            }
            width += extent;
        }
        if !width.is_finite() {
            return Err(TableError::InvalidGeometry);
        }
        if self
            .disabled_rows
            .iter()
            .any(|key| self.source.position_of(key).is_none())
        {
            return Err(TableError::InvalidSelection);
        }
        if (self.selection_mode == TableSelectionMode::None && !self.selection.is_empty())
            || (self.selection_mode == TableSelectionMode::Single && self.selection.len() > 1)
            || self.selection.iter().any(|key| {
                self.source.position_of(key).is_none() || self.disabled_rows.contains(key)
            })
        {
            return Err(TableError::InvalidSelection);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TableRequestId {
    pub(super) owner: gpui_kit::EntityId,
    pub(super) sequence: u64,
}
#[derive(Clone, Debug)]
pub(super) enum SelectionIntent {
    Toggle(TableRowKey),
    Range(TableRowKey, TableRowKey),
    All(bool),
    Clear,
}
#[derive(Clone, Debug)]
pub struct TableSelectionRequest {
    pub(super) id: TableRequestId,
    pub(super) scope: SharedString,
    pub(super) intent: SelectionIntent,
    pub(super) base_revision: u64,
}
impl TableSelectionRequest {
    pub fn id(&self) -> TableRequestId {
        self.id
    }
    pub fn base_revision(&self) -> u64 {
        self.base_revision
    }
    /// Rebase the intent on the latest accepted selection, not an obsolete whole view.
    pub fn apply_to<S: TableSource>(
        &self,
        accepted: &TableView<S>,
    ) -> Result<TableView<S>, TableError> {
        if self.scope != *accepted.source.scope() {
            return Err(TableError::StaleRequest);
        }
        let mut next = accepted.clone().based_on(accepted.revision);
        match &self.intent {
            SelectionIntent::Clear => next.selection.clear(),
            SelectionIntent::All(checked) => {
                if next.selection_mode != TableSelectionMode::Multiple {
                    return Err(TableError::InvalidSelection);
                }
                next.selection.clear();
                if *checked {
                    for position in 0..next.source.len() {
                        let key = next
                            .source
                            .row_key(position)
                            .ok_or(TableError::InvalidSource)?;
                        if !next.disabled_rows.contains(key) {
                            next.selection.insert(key.clone());
                        }
                    }
                }
            }
            SelectionIntent::Toggle(key) => {
                if next.selection_mode == TableSelectionMode::None
                    || next.source.position_of(key).is_none()
                    || next.disabled_rows.contains(key)
                {
                    return Err(TableError::InvalidSelection);
                }
                if next.selection_mode == TableSelectionMode::Single {
                    next.selection.clear();
                    next.selection.insert(key.clone());
                } else if !next.selection.remove(key) {
                    next.selection.insert(key.clone());
                }
            }
            SelectionIntent::Range(start, end) => {
                if next.selection_mode != TableSelectionMode::Multiple {
                    return Err(TableError::InvalidSelection);
                }
                let a = next
                    .source
                    .position_of(start)
                    .ok_or(TableError::MissingKey)?;
                let b = next.source.position_of(end).ok_or(TableError::MissingKey)?;
                next.selection.clear();
                for position in a.min(b)..=a.max(b) {
                    let key = next
                        .source
                        .row_key(position)
                        .ok_or(TableError::InvalidSource)?;
                    if !next.disabled_rows.contains(key) {
                        next.selection.insert(key.clone());
                    }
                }
            }
        }
        Ok(next)
    }
}
#[derive(Clone, Debug)]
pub enum TableEvent {
    SelectionRequested(TableSelectionRequest),
    ActiveChanged(Option<TableCellKey>),
}
