//! Edge panels sharing the hardened Dialog session and overlay stack.
use crate::{YueDialog, YueTheme};
mod host;
pub use host::YueDrawerHost;
pub(crate) mod resize;
use gpui_kit::{
    AnyElement, App, DefiniteLength, ElementId, Entity, FocusHandle, IntoElement, ParentElement,
    Pixels, Point, RenderOnce, SharedString, Size, Window, point, px,
};
use std::{cell::Cell, rc::Rc};

pub use crate::dialog::{
    DialogChange as DrawerChange, DialogCloseReason as DrawerCloseReason,
    DialogDecision as DrawerDecision, DialogFooter as DrawerFooter,
    DialogOpenError as DrawerOpenError, DialogOutcome as DrawerOutcome,
    DialogSubmitRequest as DrawerSubmitRequest, DialogTicket as DrawerTicket,
    YueDialogEvent as YueDrawerEvent, YueDialogState as YueDrawerState,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DrawerPlacement {
    Left,
    #[default]
    Right,
    Top,
    Bottom,
}
impl DrawerPlacement {
    pub(crate) fn horizontal(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
    pub(crate) fn geometry(
        self,
        size: Size<Pixels>,
        extent: Pixels,
        progress: f32,
    ) -> (Point<Pixels>, Size<Pixels>) {
        let hidden = extent * (1. - progress);
        match self {
            Self::Left => (point(-hidden, px(0.)), gpui_kit::size(extent, size.height)),
            Self::Right => (
                point(size.width - extent + hidden, px(0.)),
                gpui_kit::size(extent, size.height),
            ),
            Self::Top => (point(px(0.), -hidden), gpui_kit::size(size.width, extent)),
            Self::Bottom => (
                point(px(0.), size.height - extent + hidden),
                gpui_kit::size(size.width, extent),
            ),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DrawerSize {
    Small,
    #[default]
    Medium,
    Large,
    Full,
}
impl DrawerSize {
    fn length(self) -> DefiniteLength {
        match self {
            Self::Small => gpui_kit::rems(20.).into(),
            Self::Medium => gpui_kit::rems(30.).into(),
            Self::Large => gpui_kit::rems(40.).into(),
            Self::Full => DefiniteLength::Fraction(1.),
        }
    }
}
pub(crate) struct Presentation {
    pub placement: DrawerPlacement,
    pub extent: DefiniteLength,
    pub resizable: bool,
    pub min: DefiniteLength,
    pub max: DefiniteLength,
    pub on_resize: Option<Rc<resize::ResizeCallback>>,
    pub host: Option<Rc<Cell<Option<gpui_kit::Bounds<Pixels>>>>>,
    pub memory: Option<resize::Memory>,
}
impl Presentation {
    pub fn extent(&self, available: Size<Pixels>, w: &Window) -> Pixels {
        let axis = if self.placement.horizontal() {
            available.width
        } else {
            available.height
        };
        let preferred = self.extent.to_pixels(axis.into(), w.rem_size());
        if f32::from(preferred).is_finite() {
            preferred.max(px(0.)).min(axis)
        } else {
            axis
        }
    }
    pub(crate) fn transition() -> gpui_kit::base::Transition {
        gpui_kit::base::Transition::new(std::time::Duration::from_millis(240)).easing(
            gpui_kit::base::Easing::CubicBezier {
                x1: 0.2,
                y1: 0.,
                x2: 0.38,
                y2: 0.9,
            },
        )
    }
}
/// Keep the declaration mounted while closed. The host owns all business data.
/// Drawer state/event/ticket types intentionally share Dialog's session contract.
#[derive(IntoElement)]
pub struct YueDrawer {
    inner: YueDialog,
    presentation: Presentation,
}
impl YueDrawer {
    fn inner_state(&self) -> &Entity<YueDrawerState> {
        &self.inner.state
    }
    pub fn new(id: impl Into<ElementId>, theme: YueTheme, state: &Entity<YueDrawerState>) -> Self {
        Self {
            inner: YueDialog::new(id, theme, state).confirm_on_enter(false),
            presentation: Presentation {
                placement: DrawerPlacement::Right,
                extent: DrawerSize::Medium.length(),
                resizable: false,
                min: gpui_kit::rems(12.).into(),
                max: DefiniteLength::Fraction(1.),
                on_resize: None,
                host: None,
                memory: None,
            },
        }
    }
    pub fn placement(mut self, value: DrawerPlacement) -> Self {
        self.presentation.placement = value;
        self
    }
    pub fn size(mut self, value: DrawerSize) -> Self {
        self.presentation.extent = value.length();
        self
    }
    /// Width on side placements, height on top/bottom placements.
    pub fn extent(mut self, value: impl Into<DefiniteLength>) -> Self {
        self.presentation.extent = value.into();
        self
    }
    pub fn resizable(mut self, value: bool) -> Self {
        self.presentation.resizable = value;
        self
    }
    /// Both limits resolve against the available main axis; viewport constraints win.
    pub fn resize_limits(
        mut self,
        min: impl Into<DefiniteLength>,
        max: impl Into<DefiniteLength>,
    ) -> Self {
        self.presentation.min = min.into();
        self.presentation.max = max.into();
        self
    }
    /// Called on actual user size changes; receives the effective logical pixels.
    pub fn on_resize(mut self, handler: impl Fn(Pixels, &mut Window, &mut App) + 'static) -> Self {
        self.presentation.on_resize = Some(Rc::new(handler));
        self
    }
    pub fn modal(mut self, value: bool) -> Self {
        self.inner = self.inner.modal(value);
        self
    }
    pub fn title(mut self, value: impl Into<SharedString>) -> Self {
        self.inner = self.inner.title(value);
        self
    }
    pub fn label(mut self, value: impl Into<SharedString>) -> Self {
        self.inner = self.inner.label(value);
        self
    }
    pub fn description(mut self, value: impl Into<SharedString>) -> Self {
        self.inner = self.inner.description(value);
        self
    }
    pub fn header(mut self, value: impl IntoElement) -> Self {
        self.inner = self.inner.header(value);
        self
    }
    pub fn extra(mut self, value: impl IntoElement) -> Self {
        self.inner = self.inner.extra(value);
        self
    }
    pub fn footer_content(mut self, value: impl IntoElement) -> Self {
        self.inner = self.inner.footer_content(value);
        self
    }
    pub fn header_visible(mut self, value: bool) -> Self {
        self.inner = self.inner.header_visible(value);
        self
    }
    pub fn footer(mut self, value: DrawerFooter) -> Self {
        self.inner = self.inner.footer(value);
        self
    }
    pub fn closable(mut self, value: bool) -> Self {
        self.inner = self.inner.closable(value);
        self
    }
    pub fn overlay(mut self, value: bool) -> Self {
        self.inner = self.inner.overlay(value);
        self
    }
    pub fn close_on_escape(mut self, value: bool) -> Self {
        self.inner = self.inner.close_on_escape(value);
        self
    }
    pub fn close_on_overlay_click(mut self, value: bool) -> Self {
        self.inner = self.inner.close_on_overlay_click(value);
        self
    }
    pub fn confirm_disabled(mut self, value: bool) -> Self {
        self.inner = self.inner.confirm_disabled(value);
        self
    }
    pub fn confirm_on_enter(mut self, value: bool) -> Self {
        self.inner = self.inner.confirm_on_enter(value);
        self
    }
    pub fn confirm_label(mut self, value: impl Into<SharedString>) -> Self {
        self.inner = self.inner.confirm_label(value);
        self
    }
    pub fn cancel_label(mut self, value: impl Into<SharedString>) -> Self {
        self.inner = self.inner.cancel_label(value);
        self
    }
    pub fn close_label(mut self, value: impl Into<SharedString>) -> Self {
        self.inner = self.inner.close_label(value);
        self
    }
    pub fn initial_focus(mut self, value: &FocusHandle) -> Self {
        self.inner = self.inner.initial_focus(value);
        self
    }
    pub fn theme(mut self, value: YueTheme) -> Self {
        self.inner = self.inner.theme(value);
        self
    }
    pub fn use_default_theme(mut self) -> Self {
        self.inner = self.inner.use_default_theme();
        self
    }
    pub fn on_confirm(
        mut self,
        handler: impl Fn(&DrawerSubmitRequest, &mut Window, &mut App) -> DrawerDecision + 'static,
    ) -> Self {
        self.inner = self.inner.on_confirm(handler);
        self
    }
    pub fn on_close_request(
        mut self,
        handler: impl Fn(DrawerCloseReason, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.inner = self.inner.on_close_request(handler);
        self
    }
}
impl ParentElement for YueDrawer {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.inner.extend(children);
    }
}
impl RenderOnce for YueDrawer {
    fn render(mut self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.inner.drawer = Some(self.presentation);
        self.inner
    }
}
#[cfg(all(test, feature = "headless-tests"))]
mod tests;
