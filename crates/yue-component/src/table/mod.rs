//! Stateful comparison tables. Data is accepted as one immutable snapshot;
//! scrolling, frozen-header geometry and scrollbar behavior belong to Base.
mod cells;
mod columns;
mod element;
mod reorder;
mod static_grid;
pub use columns::{
    TableColumnConfigurationRequest, TableColumnDraft, TableColumnPreference,
    TableColumnPreferences,
};
pub use reorder::{
    TableColumnMoveRequest, TableMovePlacement, TableRowDrag, TableRowMoveRequest,
    TableRowMoveScope,
};
mod range;
pub use range::{TableRangeRequest, TableRangeRows};
mod spans;
pub use spans::{TableSpan, TableSpanPlan};
mod layout;
mod resize;
pub use resize::TableColumnResizeRequest;
mod edit;
mod model;
mod outline;
mod query;
mod sizing;
pub use edit::{TableEditMode, TableEditMove, TableEditSession, TableEditStage, TableEditToken};
pub use layout::*;
pub use outline::{TableOutline, TableOutlineRow, TableOutlineSource};
pub use query::*;
pub use sizing::TableRowSizing;
mod state;
pub use model::*;
pub use state::*;

use gpui_kit::{
    AnyElement, App, ElementId, Entity, IntoElement, Rems, RenderOnce, SharedString, Window, rems,
};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TableSize {
    Small,
    #[default]
    Medium,
    Large,
}
impl TableSize {
    pub(super) fn height(self, theme: crate::YueTheme) -> Rems {
        use crate::theme::keys as k;
        crate::tokens::rems(
            theme,
            match self {
                Self::Small => k::SIZE_32,
                Self::Medium => k::SIZE_40,
                Self::Large => k::SIZE_48,
            },
        )
    }
    pub(super) fn font(self) -> crate::theme::TokenKey {
        use crate::theme::keys as k;
        match self {
            Self::Small => k::FONT_SIZE_SM,
            Self::Medium => k::FONT_SIZE_MD,
            Self::Large => k::FONT_SIZE_LG,
        }
    }
    pub(super) fn padding(self) -> crate::theme::TokenKey {
        use crate::theme::keys as k;
        match self {
            Self::Small => k::SPACE_12,
            Self::Medium => k::SPACE_16,
            Self::Large => k::SIZE_20,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TableBorders {
    None,
    #[default]
    Rows,
    Grid,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TableRenderMode {
    Static,
    #[default]
    Rows,
    Cells,
}
/// Read-only capabilities of a validated builder and its current accepted view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableCapabilities {
    mode: TableRenderMode,
    row_reordering: bool,
    column_reordering: bool,
    editing: bool,
    host_scroll: bool,
}
impl TableCapabilities {
    pub fn render_mode(&self) -> TableRenderMode {
        self.mode
    }
    pub fn rowspan(&self) -> bool {
        self.mode != TableRenderMode::Rows
    }
    pub fn measured_rows(&self) -> bool {
        self.mode == TableRenderMode::Rows && !self.host_scroll
    }
    pub fn row_reordering(&self) -> bool {
        self.row_reordering
    }
    pub fn column_reordering(&self) -> bool {
        self.column_reordering
    }
    pub fn editing(&self) -> bool {
        self.editing
    }
    pub fn host_scroll(&self) -> bool {
        self.host_scroll
    }
}
type Slot = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;
#[derive(Clone)]
pub(super) struct Presentation {
    id: ElementId,
    size: TableSize,
    borders: TableBorders,
    outer_border: bool,
    striped: bool,
    hoverable: bool,
    height: Rems,
    row_height: Option<Rems>,
    mode: TableRenderMode,
    label: SharedString,
    header_visible: bool,
    header_fixed: bool,
    disabled: bool,
    readonly: bool,
    toolbar: Option<Slot>,
    footer: Option<Slot>,
    empty: Option<Slot>,
    surface: crate::theme::TokenKey,
    resolved: Rc<layout::ResolvedColumns>,
    frozen: (usize, usize),
    footer_fixed: bool,
    host: Option<(gpui_kit::ScrollHandle, Rems, Rems)>,
    range_prefetch: Option<usize>,
    tail: Option<(SharedString, Rems, bool)>,
    row_drag: TableRowDrag,
    row_move_scope: TableRowMoveScope,
    drag_preview: Option<reorder::DragPreview>,
}
/// Retain the state in the host. Rows mode always supplies its own finite
/// viewport (20rem by default); Static grows naturally inside its parent.
#[derive(IntoElement)]
pub struct YueTable<D: TableDelegate> {
    state: Entity<YueTableState<D>>,
    p: Presentation,
}
impl<D: TableDelegate> YueTable<D> {
    pub fn new(id: impl Into<ElementId>, state: &Entity<YueTableState<D>>) -> Self {
        Self {
            state: state.clone(),
            p: Presentation {
                id: id.into(),
                size: TableSize::Medium,
                borders: TableBorders::Rows,
                outer_border: false,
                striped: false,
                hoverable: true,
                height: rems(20.),
                row_height: None,
                mode: TableRenderMode::Rows,
                label: "数据表格".into(),
                header_visible: true,
                header_fixed: true,
                disabled: false,
                readonly: false,
                toolbar: None,
                footer: None,
                empty: None,
                surface: crate::theme::keys::SURFACE,
                resolved: Rc::default(),
                frozen: (0, 0),
                footer_fixed: false,
                host: None,
                range_prefetch: None,
                tail: None,
                row_drag: TableRowDrag::None,
                row_move_scope: TableRowMoveScope::FullResult,
                drag_preview: None,
            },
        }
    }
    pub fn row_drag(mut self, mode: TableRowDrag) -> Self {
        self.p.row_drag = mode;
        self
    }
    pub fn row_move_scope(mut self, scope: TableRowMoveScope) -> Self {
        self.p.row_move_scope = scope;
        self
    }
    pub fn size(mut self, size: TableSize) -> Self {
        self.p.size = size;
        self
    }
    /// Observe body demand from the actual virtual viewport. Network tasks and
    /// cancellation belong to the host; duplicate/old completion is rejected.
    pub fn tail_loading(
        mut self,
        cursor: impl Into<SharedString>,
        distance: Rems,
        has_more: bool,
    ) -> Self {
        assert!(distance.0.is_finite() && distance.0 >= 0.);
        self.p.tail = Some((cursor.into(), distance, has_more));
        self
    }
    pub fn range_loading(mut self, prefetch_rows: usize) -> Self {
        self.p.range_prefetch = Some(prefetch_rows);
        self
    }
    pub fn borders(mut self, borders: TableBorders) -> Self {
        self.p.borders = borders;
        self
    }
    pub fn outer_border(mut self, enabled: bool) -> Self {
        self.p.outer_border = enabled;
        self
    }
    pub fn striped(mut self, enabled: bool) -> Self {
        self.p.striped = enabled;
        self
    }
    pub fn hoverable(mut self, enabled: bool) -> Self {
        self.p.hoverable = enabled;
        self
    }
    pub fn height(mut self, height: Rems) -> Self {
        assert!(height.0.is_finite() && height.0 > 0.);
        self.p.height = height;
        self
    }
    /// Complete fixed row height, including padding and separators.
    pub fn row_height(mut self, height: Rems) -> Self {
        assert!(height.0.is_finite() && height.0 > 0.);
        self.p.row_height = Some(height);
        self
    }
    pub fn render_mode(mut self, mode: TableRenderMode) -> Self {
        self.p.mode = mode;
        self
    }
    /// Validate the complete builder, including loading and host combinations.
    pub fn validate(&self, cx: &App) -> Result<(), TableError> {
        let view = self.state.read(cx).view().ok_or(TableError::Unbound)?;
        cells::validate_mode(view, &self.p)
    }
    pub fn capabilities(&self, cx: &App) -> Result<TableCapabilities, TableError> {
        self.validate(cx)?;
        let view = self.state.read(cx).view().unwrap();
        let writable = !self.p.disabled
            && !self.p.readonly
            && !matches!(
                view.load,
                TableLoadState::Loading | TableLoadState::Refreshing
            );
        Ok(TableCapabilities {
            mode: self.p.mode,
            row_reordering: writable
                && self.p.row_drag != TableRowDrag::None
                && view.source.can_move_rows()
                && view.query.sorts().is_empty()
                && (!(view.query.has_filters() || view.query.page().is_some())
                    || self.p.row_move_scope == TableRowMoveScope::MappedVisibleResult),
            column_reordering: writable
                && view
                    .columns
                    .iter()
                    .any(|c| c.visible && c.allowed && c.configurable && c.reorderable),
            editing: writable
                && view
                    .columns
                    .iter()
                    .any(|c| c.visible && c.allowed && c.editable),
            host_scroll: self.p.host.is_some(),
        })
    }
    pub fn try_render_mode(mut self, mode: TableRenderMode, cx: &App) -> Result<Self, TableError> {
        self.p.mode = mode;
        if let Some(view) = self.state.read(cx).view() {
            cells::validate_mode(view, &self.p)?;
        }
        Ok(self)
    }
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.p.label = label.into();
        self
    }
    pub fn header_visible(mut self, visible: bool) -> Self {
        self.p.header_visible = visible;
        self
    }
    pub fn header_fixed(mut self, fixed: bool) -> Self {
        self.p.header_fixed = fixed;
        self
    }
    /// Counts refer to data rows, independently of the header and column footer.
    pub fn frozen_rows(mut self, start: usize, end: usize) -> Self {
        self.p.frozen = (start, end);
        self
    }
    pub fn footer_fixed(mut self, fixed: bool) -> Self {
        self.p.footer_fixed = fixed;
        self
    }
    /// Borrow one page/panel scroll owner. The table occupies natural content
    /// size; top/bottom reserve host chrome and constrain its frozen bands.
    pub fn scroll_host(mut self, host: &gpui_kit::ScrollHandle, top: Rems, bottom: Rems) -> Self {
        assert!(top.0.is_finite() && top.0 >= 0. && bottom.0.is_finite() && bottom.0 >= 0.);
        self.p.host = Some((host.clone(), top, bottom));
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.p.disabled = disabled;
        self
    }
    pub fn readonly(mut self, readonly: bool) -> Self {
        self.p.readonly = readonly;
        self
    }
    /// Opaque backing for header/footer bands; ordinary cells remain transparent.
    pub fn surface_background(mut self, token: crate::theme::TokenKey) -> Self {
        self.p.surface = token;
        self
    }
    pub fn toolbar<E: IntoElement>(
        mut self,
        render: impl Fn(&mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.p.toolbar = Some(Rc::new(move |w, cx| render(w, cx).into_any_element()));
        self
    }
    pub fn footer<E: IntoElement>(
        mut self,
        render: impl Fn(&mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.p.footer = Some(Rc::new(move |w, cx| render(w, cx).into_any_element()));
        self
    }
    pub fn empty<E: IntoElement>(
        mut self,
        render: impl Fn(&mut Window, &mut App) -> E + 'static,
    ) -> Self {
        self.p.empty = Some(Rc::new(move |w, cx| render(w, cx).into_any_element()));
        self
    }
}
impl<D: TableDelegate> RenderOnce for YueTable<D> {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        use gpui_kit::{
            InteractiveElement as _, ParentElement as _, StatefulInteractiveElement as _,
        };
        if let Some(view) = self.state.read(cx).view()
            && let Err(error) = cells::validate_mode(view, &self.p)
        {
            // Invalid combinations never publish half a presentation or replace accepted data.
            return gpui_kit::div()
                .id(self.p.id.clone())
                .role(gpui_kit::Role::Status)
                .aria_label("表格配置无效")
                .child(format!("表格配置无效：{error}"))
                .into_any_element();
        }
        self.state
            .update(cx, |state, _| state.presentation = Some(self.p));
        self.state.into_any_element()
    }
}
