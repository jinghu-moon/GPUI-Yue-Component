//! Stateful comparison tables. Data is accepted as one immutable snapshot;
//! scrolling, frozen-header geometry and scrollbar behavior belong to Base.
mod element;
mod layout;
mod resize;
pub use resize::TableColumnResizeRequest;
mod model;
pub use layout::*;
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
            },
        }
    }
    pub fn size(mut self, size: TableSize) -> Self {
        self.p.size = size;
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
        self.state
            .update(cx, |state, _| state.presentation = Some(self.p));
        self.state
    }
}
