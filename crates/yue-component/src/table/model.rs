use super::TableRowSizing;
use super::TableSize;
use super::TableSpanPlan;
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
    InvalidQuery(SharedString),
    UnsupportedCombination(&'static str),
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
    /// Domain membership can include keys outside the displayed page. Remote
    /// sources may override this using a host-supplied selection domain.
    fn contains_key(&self, key: &TableRowKey) -> bool {
        self.row(key).is_some()
    }
    fn row_revision(&self, _key: &TableRowKey) -> u64 {
        self.revision()
    }
    fn is_detail(&self, _key: &TableRowKey) -> bool {
        false
    }
    /// An addressable, unloaded slot. It has no business row or selection membership.
    fn can_move_rows(&self) -> bool {
        false
    }
    fn is_placeholder(&self, _key: &TableRowKey) -> bool {
        false
    }
    /// O(1) summary for sources with unloaded slots; override together with is_placeholder.
    fn has_placeholders(&self) -> bool {
        false
    }
    /// Pagination units can be roots while visible rows include descendants/details.
    fn page_len(&self) -> usize {
        self.len()
    }
    fn record_len(&self) -> usize {
        self.len()
    }
}

/// Optional slice adapter. Keeps the caller's allocation; Row need not implement Clone.
pub struct TableRows<R: 'static> {
    scope: SharedString,
    revision: u64,
    rows: Rc<[R]>,
    keys: Vec<TableRowKey>,
    positions: HashMap<TableRowKey, usize>,
    storage: Option<Rc<HashMap<TableRowKey, usize>>>,
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
            storage: None,
        })
    }
}
impl<R: 'static> TableRows<R> {
    pub fn reordered(&self, revision: u64, keys: &[TableRowKey]) -> Result<Self, TableError> {
        if revision <= self.revision {
            return Err(TableError::StaleSource);
        }
        if keys.len() != self.keys.len() {
            return Err(TableError::InvalidSource);
        }
        let mut positions = HashMap::with_capacity(keys.len());
        for (i, k) in keys.iter().enumerate() {
            if !self.positions.contains_key(k) || positions.insert(k.clone(), i).is_some() {
                return Err(TableError::InvalidSource);
            }
        }
        Ok(Self {
            scope: self.scope.clone(),
            revision,
            rows: self.rows.clone(),
            keys: keys.to_vec(),
            positions,
            storage: Some(
                self.storage
                    .clone()
                    .unwrap_or_else(|| Rc::new(self.positions.clone())),
            ),
        })
    }
}
impl<R: 'static> TableSource for TableRows<R> {
    fn can_move_rows(&self) -> bool {
        true
    }
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
        self.storage
            .as_deref()
            .unwrap_or(&self.positions)
            .get(key)
            .map(|index| &self.rows[*index])
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
    pub(super) editing: bool,
    pub(super) edit_stage: Option<super::TableEditStage>,
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
    pub fn is_editing(&self) -> bool {
        self.editing
    }
    pub fn edit_stage(&self) -> Option<&super::TableEditStage> {
        self.edit_stage.as_ref()
    }
}
type Renderer<R> = Rc<dyn Fn(&R, &TableCellContext, &mut Window, &mut App) -> AnyElement>;
type Text<R> = Rc<dyn Fn(&R) -> SharedString>;
pub struct TableColumn<R: 'static> {
    pub(super) key: TableColumnKey,
    pub(super) title: SharedString,
    pub(super) width: Rems,
    pub(super) minimum: Rems,
    pub(super) maximum: Rems,
    pub(super) flex: f32,
    pub(super) visible: bool,
    pub(super) pin: super::TablePin,
    pub(super) responsive: Option<u32>,
    pub(super) resizable: bool,
    pub(super) version: u64,
    pub(super) allowed: bool,
    pub(super) configurable: bool,
    pub(super) reorderable: bool,
    pub(super) row_handle: bool,
    pub(super) groups: Rc<[super::TableHeaderGroup]>,
    pub(super) align: TableAlign,
    pub(super) header_align: Option<TableAlign>,
    pub(super) vertical: TableVerticalAlign,
    pub(super) overflow: TableTextOverflow,
    pub(super) text: Option<Text<R>>,
    pub(super) renderer: Option<Renderer<R>>,
    pub(super) header: Option<super::Slot>,
    pub(super) footer: Option<SharedString>,
    pub(super) interactive: bool,
    pub(super) editable: bool,
    pub(super) edit_revision: u64,
    pub(super) query: Option<Rc<super::query::QueryRule<R>>>,
}
impl<R: 'static> Clone for TableColumn<R> {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            title: self.title.clone(),
            width: self.width,
            minimum: self.minimum,
            maximum: self.maximum,
            flex: self.flex,
            visible: self.visible,
            pin: self.pin,
            responsive: self.responsive,
            resizable: self.resizable,
            version: self.version,
            allowed: self.allowed,
            configurable: self.configurable,
            reorderable: self.reorderable,
            row_handle: self.row_handle,
            groups: self.groups.clone(),
            align: self.align,
            header_align: self.header_align,
            vertical: self.vertical,
            overflow: self.overflow,
            text: self.text.clone(),
            renderer: self.renderer.clone(),
            header: self.header.clone(),
            footer: self.footer.clone(),
            interactive: self.interactive,
            editable: self.editable,
            edit_revision: self.edit_revision,
            query: self.query.clone(),
        }
    }
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
            minimum: rems(1.),
            maximum: rems(65536.),
            flex: 0.,
            visible: true,
            pin: super::TablePin::None,
            responsive: None,
            resizable: false,
            version: 1,
            allowed: true,
            configurable: true,
            reorderable: false,
            row_handle: false,
            groups: Rc::from([]),
            align: TableAlign::Start,
            header_align: None,
            vertical: TableVerticalAlign::Center,
            overflow: TableTextOverflow::Ellipsis,
            text: Some(Rc::new(move |row| accessor(row).into())),
            renderer: None,
            header: None,
            footer: None,
            interactive: false,
            editable: false,
            edit_revision: 0,
            query: None,
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
    pub fn min_width(mut self, width: Rems) -> Self {
        self.minimum = width;
        self
    }
    pub fn max_width(mut self, width: Rems) -> Self {
        self.maximum = width;
        self
    }
    /// Positive weights share remaining container space, subject to min/max.
    pub fn flex(mut self, weight: f32) -> Self {
        self.flex = weight;
        self
    }
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
    pub fn pin(mut self, pin: super::TablePin) -> Self {
        self.pin = pin;
        self
    }
    /// Larger priorities disappear first when declared widths do not fit.
    pub fn responsive_priority(mut self, priority: u32) -> Self {
        self.responsive = Some(priority);
        self
    }
    pub fn semantic_version(mut self, version: u64) -> Self {
        self.version = version;
        self
    }
    pub fn allowed(mut self, allowed: bool) -> Self {
        self.allowed = allowed;
        self
    }
    pub fn configurable(mut self, allowed: bool) -> Self {
        self.configurable = allowed;
        self
    }
    pub fn reorderable(mut self, enabled: bool) -> Self {
        self.reorderable = enabled;
        self
    }
    pub fn row_drag_handle(mut self, enabled: bool) -> Self {
        self.row_handle = enabled;
        self.interactive |= enabled;
        self
    }
    pub fn is_allowed(&self) -> bool {
        self.allowed
    }
    pub fn is_configurable(&self) -> bool {
        self.configurable
    }
    pub fn semantic_version_value(&self) -> u64 {
        self.version
    }
    pub fn resizable(mut self, enabled: bool) -> Self {
        self.resizable = enabled;
        self
    }
    pub fn header_path(mut self, groups: impl Into<Rc<[super::TableHeaderGroup]>>) -> Self {
        self.groups = groups.into();
        self
    }
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    pub fn pin_kind(&self) -> super::TablePin {
        self.pin
    }
    pub fn preferred_width(&self) -> Rems {
        self.width
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
    pub fn editable(mut self, enabled: bool) -> Self {
        self.editable = enabled;
        self
    }
    pub fn edit_revision(mut self, revision: u64) -> Self {
        self.edit_revision = revision;
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
    pub(super) query: super::TableQuery,
    pub(super) data_mode: super::TableDataMode,
    pub(super) total: super::TableTotal,
    pub(super) page_selection: (usize, usize),
    pub(super) sizing: TableRowSizing,
    pub(super) spans: TableSpanPlan,
    pub(super) has_details: bool,
    pub(super) declarations: Rc<[TableColumn<S::Row>]>,
    pub(super) preferences: Rc<super::TableColumnPreferences>,
    pub(super) schema: u64,
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
            query: self.query.clone(),
            data_mode: self.data_mode,
            total: self.total,
            page_selection: self.page_selection,
            sizing: self.sizing.clone(),
            spans: self.spans.clone(),
            has_details: self.has_details,
            declarations: self.declarations.clone(),
            preferences: self.preferences.clone(),
            schema: self.schema,
        }
    }
}
impl<S: TableSource> TableView<S> {
    pub fn new(source: Rc<S>, columns: impl Into<Rc<[TableColumn<S::Row>]>>) -> Self {
        let len = source.len();
        let total = super::TableTotal::Known(len);
        let declarations = columns.into();
        let columns = if declarations.iter().all(|c| c.allowed) {
            declarations.clone()
        } else {
            declarations
                .iter()
                .cloned()
                .map(|mut c| {
                    c.visible &= c.allowed;
                    c
                })
                .collect::<Vec<_>>()
                .into()
        };
        let has_details = (0..len)
            .filter_map(|i| source.row_key(i))
            .any(|k| source.is_detail(k));
        Self {
            source,
            columns,
            declarations,
            preferences: Rc::new(super::TableColumnPreferences::new(1)),
            schema: 1,
            selection: BTreeSet::new(),
            disabled_rows: BTreeSet::new(),
            selection_mode: TableSelectionMode::None,
            load: TableLoadState::Ready,
            revision: 0,
            base_revision: None,
            query: super::TableQuery::default(),
            data_mode: super::TableDataMode::Remote,
            total,
            page_selection: (len, 0),
            sizing: TableRowSizing::default(),
            spans: TableSpanPlan::default(),
            has_details,
        }
    }
    /// Replace a complete column configuration atomically, retaining stable keys.
    pub fn with_columns(mut self, columns: impl Into<Rc<[TableColumn<S::Row>]>>) -> Self {
        self.declarations = columns.into();
        let prefs = self.preferences.reconcile(&self);
        self.columns = prefs.resolve(&self);
        self.preferences = Rc::new(prefs);
        self
    }
    pub fn column_schema(&self) -> u64 {
        self.schema
    }
    pub fn with_column_schema(mut self, schema: u64) -> Self {
        self.schema = schema;
        let prefs = self.preferences.reconcile(&self);
        self.columns = prefs.resolve(&self);
        self.preferences = Rc::new(prefs);
        self
    }
    pub fn declared_columns(&self) -> &[TableColumn<S::Row>] {
        &self.declarations
    }
    pub fn column_preferences(&self) -> &super::TableColumnPreferences {
        &self.preferences
    }
    pub fn source(&self) -> &Rc<S> {
        &self.source
    }
    pub fn with_row_sizing(mut self, sizing: TableRowSizing) -> Self {
        self.sizing = sizing;
        self
    }
    pub fn with_spans(mut self, spans: TableSpanPlan) -> Self {
        self.spans = spans;
        self
    }
    pub fn spans(&self) -> &TableSpanPlan {
        &self.spans
    }
    pub fn row_sizing(&self) -> &TableRowSizing {
        &self.sizing
    }
    pub fn with_query(mut self, query: super::TableQuery) -> Self {
        self.query = query;
        self
    }
    pub fn query(&self) -> &super::TableQuery {
        &self.query
    }
    pub fn with_result(mut self, mode: super::TableDataMode, total: super::TableTotal) -> Self {
        self.data_mode = mode;
        self.total = total;
        self
    }
    pub fn data_mode(&self) -> super::TableDataMode {
        self.data_mode
    }
    pub fn total(&self) -> super::TableTotal {
        self.total
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
            && self.schema == other.schema
            && self.preferences == other.preferences
            && self.selection == other.selection
            && self.disabled_rows == other.disabled_rows
            && self.selection_mode == other.selection_mode
            && self.load == other.load
            && self.query == other.query
            && self.data_mode == other.data_mode
            && self.total == other.total
            && self.sizing.same(&other.sizing)
            && self.spans.same(&other.spans)
    }
    pub(super) fn validate(&self, validate_source: bool) -> Result<(), TableError> {
        if self.data_mode == super::TableDataMode::Local && self.source.has_placeholders() {
            return Err(TableError::UnsupportedCombination(
                "local query requires fully loaded records",
            ));
        }
        self.query.validate(&self.columns)?;
        self.sizing.validate(self.source.as_ref())?;
        if self
            .query
            .page()
            .is_some_and(|page| self.source.page_len() > page.size())
        {
            return Err(TableError::InvalidQuery(
                "returned page exceeds page size".into(),
            ));
        }
        if let super::TableTotal::Known(total) = self.total {
            if let Some(page) = self.query.page() {
                let start = (page.number() - 1)
                    .checked_mul(page.size())
                    .ok_or_else(|| TableError::InvalidQuery("page overflow".into()))?;
                if page.number() > total.div_ceil(page.size()).max(1)
                    || self.source.page_len() != page.size().min(total.saturating_sub(start))
                {
                    return Err(TableError::InvalidQuery(
                        "total and page records disagree".into(),
                    ));
                }
            } else if total < self.source.page_len() {
                return Err(TableError::InvalidQuery(
                    "total smaller than displayed records".into(),
                ));
            }
        }
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
                if self.source.position_of(key) != Some(position)
                    || (self.source.row(key).is_none() && !self.source.is_placeholder(key))
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
            if !column.minimum.0.is_finite()
                || column.minimum.0 <= 0.
                || !column.maximum.0.is_finite()
                || column.maximum.0 < column.minimum.0
                || !column.flex.is_finite()
                || column.flex < 0.
            {
                return Err(TableError::InvalidGeometry);
            }
            let extent = column.width.0;
            if !extent.is_finite() || extent <= 0. {
                return Err(TableError::InvalidGeometry);
            }
            width += extent;
        }
        super::layout::validate_groups(&self.columns)?;
        self.spans.validate(self)?;
        if !width.is_finite() {
            return Err(TableError::InvalidGeometry);
        }
        if self
            .disabled_rows
            .iter()
            .any(|key| !self.source.contains_key(key))
        {
            return Err(TableError::InvalidSelection);
        }
        if (self.selection_mode == TableSelectionMode::None && !self.selection.is_empty())
            || (self.selection_mode == TableSelectionMode::Single && self.selection.len() > 1)
            || self
                .selection
                .iter()
                .any(|key| !self.source.contains_key(key) || self.disabled_rows.contains(key))
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
                for position in 0..next.source.len() {
                    let key = next
                        .source
                        .row_key(position)
                        .ok_or(TableError::InvalidSource)?;
                    if *checked
                        && !next.disabled_rows.contains(key)
                        && !next.source.is_detail(key)
                        && !next.source.is_placeholder(key)
                    {
                        next.selection.insert(key.clone());
                    } else {
                        next.selection.remove(key);
                    }
                }
            }
            SelectionIntent::Toggle(key) => {
                if next.selection_mode == TableSelectionMode::None
                    || next.source.position_of(key).is_none()
                    || next.disabled_rows.contains(key)
                    || next.source.is_detail(key)
                    || next.source.is_placeholder(key)
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
                    if !next.disabled_rows.contains(key)
                        && !next.source.is_detail(key)
                        && !next.source.is_placeholder(key)
                    {
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
    RowMoveRequested(super::TableRowMoveRequest),
    ColumnMoveRequested(super::TableColumnMoveRequest),
    ColumnConfigurationRequested(super::TableColumnConfigurationRequest),
    RangeRequested(super::TableRangeRequest),
    EditRequested {
        key: TableCellKey,
        mode: super::TableEditMode,
    },
    EditSaveRequested(super::TableEditToken),
    EditEnded,
    SelectionRequested(TableSelectionRequest),
    ActiveChanged(Option<TableCellKey>),
    ColumnResizeRequested(super::TableColumnResizeRequest),
    QueryRequested(super::TableQueryRequest),
}
