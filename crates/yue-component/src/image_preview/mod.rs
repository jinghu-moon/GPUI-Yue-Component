//! Embedded static image viewing and aligned before/after comparison.
//! Hosts retain State and supply decoded, versioned resources. No window,
//! encoder, file service or renderer patch is owned by this component.
//!
//! ```no_run
//! use std::sync::Arc;
//! use gpui_kit::{App, AppContext as _, Entity, RenderImage, Window};
//! use yue_component::{ImagePreviewDocument, ImagePreviewError, ImagePreviewImage,
//!     ImagePreviewLoad, YueImagePreviewState};
//!
//! fn create_preview(image: Arc<RenderImage>, window: &mut Window, cx: &mut App)
//!     -> Result<Entity<YueImagePreviewState>, ImagePreviewError>
//! {
//!     let state = cx.new(|cx| YueImagePreviewState::new("preview", window, cx));
//!     let document = ImagePreviewDocument::new("document-1", 1)
//!         .with_source(ImagePreviewLoad::Ready(ImagePreviewImage::new(image, "截图")?));
//!     state.update(cx, |state, cx| state.set_document(document, window, cx))?;
//!     // Retain this Entity in the host; render YueImagePreview::new(&state)
//!     // inside a container with a definite height.
//!     Ok(state)
//! }
//! ```
mod element;
mod geometry;
mod model;
mod paint;
mod state;
pub use model::*;
pub use state::YueImagePreviewState;

use gpui_kit::{AnyElement, App, Entity, IntoElement, RenderOnce, SharedString, Window};
use std::rc::Rc;

type FooterRenderer = Box<dyn FnOnce(&mut Window, &mut App) -> AnyElement>;
type OverlayRenderer = Rc<dyn Fn(ImagePreviewOverlay, &mut Window, &mut App)>;

/// Read-only, logical-window geometry for non-interactive paint overlays.
/// Draw only within this viewport; image positions are normalized to the frame.
#[derive(Clone, Copy, Debug)]
pub struct ImagePreviewOverlay {
    viewport: gpui_kit::Bounds<gpui_kit::Pixels>,
    image_bounds: gpui_kit::Bounds<gpui_kit::Pixels>,
}
impl ImagePreviewOverlay {
    pub fn viewport(&self) -> gpui_kit::Bounds<gpui_kit::Pixels> {
        self.viewport
    }
    pub fn image_bounds(&self) -> gpui_kit::Bounds<gpui_kit::Pixels> {
        self.image_bounds
    }
    pub fn point(&self, u: f32, v: f32) -> gpui_kit::Point<gpui_kit::Pixels> {
        self.image_bounds.origin
            + gpui_kit::point(
                self.image_bounds.size.width * u,
                self.image_bounds.size.height * v,
            )
    }
}

#[derive(IntoElement)]
pub struct YueImagePreview {
    state: Entity<YueImagePreviewState>,
    theme: Option<crate::YueTheme>,
    animated: bool,
    disabled: bool,
    modes: bool,
    background_picker: bool,
    toolbar: ImagePreviewToolbar,
    toolbar_visible: bool,
    result_label: SharedString,
    footer: Option<FooterRenderer>,
    overlay: Option<OverlayRenderer>,
}
impl YueImagePreview {
    pub fn new(state: &Entity<YueImagePreviewState>) -> Self {
        Self {
            state: state.clone(),
            theme: None,
            animated: true,
            disabled: false,
            modes: true,
            background_picker: true,
            toolbar: ImagePreviewToolbar::Floating,
            toolbar_visible: true,
            result_label: "处理后".into(),
            footer: None,
            overlay: None,
        }
    }
    pub fn theme(mut self, theme: crate::YueTheme) -> Self {
        self.theme = Some(theme);
        self
    }
    pub fn animated(mut self, value: bool) -> Self {
        self.animated = value;
        self
    }
    pub fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }
    pub fn show_modes(mut self, value: bool) -> Self {
        self.modes = value;
        self
    }
    pub fn show_background_picker(mut self, value: bool) -> Self {
        self.background_picker = value;
        self
    }
    pub fn toolbar(mut self, position: ImagePreviewToolbar) -> Self {
        self.toolbar = position;
        self
    }
    pub fn show_toolbar(mut self, value: bool) -> Self {
        self.toolbar_visible = value;
        self
    }
    pub fn result_label(mut self, label: impl Into<SharedString>) -> Self {
        self.result_label = label.into();
        self
    }
    pub fn footer(
        mut self,
        renderer: impl FnOnce(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.footer = Some(Box::new(renderer));
        self
    }
    /// Paint-only slot. Do not register input handlers or perform side effects.
    pub fn image_overlay(
        mut self,
        renderer: impl Fn(ImagePreviewOverlay, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.overlay = Some(Rc::new(renderer));
        self
    }
}

gpui_kit::actions!(
    yue_image_preview,
    [
        ZoomIn, ZoomOut, ActualSize, FitAll, FitWidth, PanLeft, PanRight, PanUp, PanDown
    ]
);
pub(crate) fn init(cx: &mut App) {
    use gpui_kit::KeyBinding;
    cx.bind_keys([
        KeyBinding::new("+", ZoomIn, Some("YueImagePreview")),
        KeyBinding::new("=", ZoomIn, Some("YueImagePreview")),
        KeyBinding::new("-", ZoomOut, Some("YueImagePreview")),
        KeyBinding::new("0", FitAll, Some("YueImagePreview")),
        KeyBinding::new("1", ActualSize, Some("YueImagePreview")),
        KeyBinding::new("left", PanLeft, Some("YueImagePreview")),
        KeyBinding::new("right", PanRight, Some("YueImagePreview")),
        KeyBinding::new("up", PanUp, Some("YueImagePreview")),
        KeyBinding::new("down", PanDown, Some("YueImagePreview")),
        KeyBinding::new("shift-left", PanLeft, Some("YueImagePreview")),
        KeyBinding::new("shift-right", PanRight, Some("YueImagePreview")),
        KeyBinding::new("shift-up", PanUp, Some("YueImagePreview")),
        KeyBinding::new("shift-down", PanDown, Some("YueImagePreview")),
    ]);
}

#[cfg(test)]
mod tests;
