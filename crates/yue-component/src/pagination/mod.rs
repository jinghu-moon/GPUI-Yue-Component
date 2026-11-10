//! Controlled pagination, composed from Base page planning and existing Yue controls.
mod element;
mod model;
mod state;
use crate::YueTheme;
use gpui_kit::{ElementId, Entity, IntoElement, SharedString};
pub use model::{
    PaginationChange, PaginationChangeSource, PaginationError, PaginationMode, PaginationSize,
    PaginationSizeChangePolicy, PaginationValue, PaginationVariant,
};
pub use state::YuePaginationState;
use std::rc::Rc;

gpui_kit::actions!(yue_pagination, [NextField, PreviousField]);
pub(crate) fn init(cx: &mut gpui_kit::App) {
    cx.bind_keys([
        gpui_kit::KeyBinding::new("tab", NextField, Some("YuePaginationJump > Input")),
        gpui_kit::KeyBinding::new(
            "shift-tab",
            PreviousField,
            Some("YuePaginationJump > Input"),
        ),
    ]);
}

/// Hosts subscribe to PaginationChange and explicitly accept with State::set_value.
/// Keep one state per mounted control; setters run in updates, not in render.
///
/// ```
/// use yue_component::{YuePagination, YuePaginationState, PaginationValue, gpui_kit};
/// use gpui_kit::{AppContext as _, Context, Entity, IntoElement, Render, Subscription, Window};
/// struct Orders { pagination: Entity<YuePaginationState>, _subscription: Subscription }
/// impl Orders {
///     fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
///         let pagination = cx.new(|cx| YuePaginationState::new(
///             PaginationValue::new(257, 1, 10).unwrap(), window, cx));
///         let subscription = cx.subscribe_in(&pagination, window, |_, state, request, window, cx| {
///             // Validate/request data here; explicitly commit when accepted.
///             state.update(cx, |state, cx| state.set_value(request.value(), window, cx));
///         });
///         Self { pagination, _subscription: subscription }
///     }
/// }
/// impl Render for Orders {
///     fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
///         YuePagination::new("orders", &self.pagination).show_total(true).page_size_selector(true)
///     }
/// }
/// ```
#[derive(IntoElement)]
pub struct YuePagination {
    id: ElementId,
    state: Entity<YuePaginationState>,
    theme: Option<YueTheme>,
    size: PaginationSize,
    variant: PaginationVariant,
    mode: PaginationMode,
    total: bool,
    formatter: Option<Rc<dyn Fn(PaginationValue) -> SharedString>>,
    selector: bool,
    jumper: bool,
    first_last: bool,
    ellipsis_jump: bool,
    visible: usize,
    hide_single: bool,
    labels: Option<(SharedString, SharedString)>,
    name: SharedString,
    layout: crate::control_layout::ControlLayout,
}
crate::control_layout::control_layout!(YuePagination);
impl YuePagination {
    pub fn new(id: impl Into<ElementId>, state: &Entity<YuePaginationState>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            theme: None,
            size: PaginationSize::Medium,
            variant: PaginationVariant::Ghost,
            mode: PaginationMode::Pages,
            total: false,
            formatter: None,
            selector: false,
            jumper: false,
            first_last: false,
            ellipsis_jump: true,
            visible: 7,
            hide_single: false,
            labels: None,
            name: "分页".into(),
            layout: Default::default(),
        }
    }
    pub fn theme(mut self, theme: YueTheme) -> Self {
        self.theme = Some(theme);
        self
    }
    pub fn size(mut self, size: PaginationSize) -> Self {
        self.size = size;
        self
    }
    pub fn variant(mut self, variant: PaginationVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn mode(mut self, mode: PaginationMode) -> Self {
        self.mode = mode;
        self
    }
    pub fn show_total(mut self, show: bool) -> Self {
        self.total = show;
        self
    }
    pub fn total_format(
        mut self,
        format: impl Fn(PaginationValue) -> SharedString + 'static,
    ) -> Self {
        self.formatter = Some(Rc::new(format));
        self.total = true;
        self
    }
    pub fn page_size_selector(mut self, show: bool) -> Self {
        self.selector = show;
        self
    }
    pub fn show_jumper(mut self, show: bool) -> Self {
        self.jumper = show;
        self
    }
    pub fn first_last(mut self, show: bool) -> Self {
        self.first_last = show;
        self
    }
    pub fn ellipsis_jump(mut self, enabled: bool) -> Self {
        self.ellipsis_jump = enabled;
        self
    }
    /// Bound work independently of total pages. Endpoints/ellipses follow Base's plan.
    ///
    /// # Panics
    /// Panics unless `count` is within `5..=15` (default: 7).
    pub fn visible_pages(mut self, count: usize) -> Self {
        assert!((5..=15).contains(&count));
        self.visible = count;
        self
    }
    pub fn hide_on_single_page(mut self, hide: bool) -> Self {
        self.hide_single = hide;
        self
    }
    /// Show text instead of icons for previous/next navigation.
    ///
    /// # Panics
    /// Panics when either label is blank.
    pub fn navigation_labels(
        mut self,
        previous: impl Into<SharedString>,
        next: impl Into<SharedString>,
    ) -> Self {
        let labels = (previous.into(), next.into());
        assert!(!labels.0.trim().is_empty() && !labels.1.trim().is_empty());
        self.labels = Some(labels);
        self
    }
    /// Name this navigation landmark (default: "分页").
    ///
    /// # Panics
    /// Panics when the name is blank.
    pub fn accessibility_label(mut self, name: impl Into<SharedString>) -> Self {
        let name = name.into();
        assert!(!name.trim().is_empty());
        self.name = name;
        self
    }
}

#[cfg(all(test, feature = "headless-tests"))]
mod tests;
