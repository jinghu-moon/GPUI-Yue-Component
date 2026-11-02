use gpui_kit::{RenderImage, SharedString};
use std::{fmt, sync::Arc};

/// Static decoded pixels. The host owns I/O, decoding, orientation and encoding.
#[derive(Clone, Debug, PartialEq)]
pub struct ImagePreviewImage {
    pub(super) image: Arc<RenderImage>,
    pub(super) description: SharedString,
}
impl ImagePreviewImage {
    pub fn new(
        image: Arc<RenderImage>,
        description: impl Into<SharedString>,
    ) -> Result<Self, ImagePreviewError> {
        let s = image.size(0);
        if image.frame_count() != 1 || s.width.0 <= 0 || s.height.0 <= 0 {
            return Err(ImagePreviewError::InvalidImage);
        }
        let description = description.into();
        Ok(Self {
            image,
            description: if description.trim().is_empty() {
                "图片预览".into()
            } else {
                description
            },
        })
    }
    pub fn pixel_size(&self) -> [u32; 2] {
        let s = self.image.size(0);
        [s.width.0 as u32, s.height.0 as u32]
    }
    pub fn description(&self) -> &SharedString {
        &self.description
    }
    pub fn image(&self) -> &Arc<RenderImage> {
        &self.image
    }
    pub(super) fn dimensions(&self) -> [f32; 2] {
        self.pixel_size().map(|v| v as f32)
    }
}

/// Explicit host-supplied state. Refreshing may retain a previously ready result.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ImagePreviewLoad {
    #[default]
    Empty,
    Loading,
    Ready(ImagePreviewImage),
    Failed(SharedString),
}
impl ImagePreviewLoad {
    pub fn ready(&self) -> Option<&ImagePreviewImage> {
        if let Self::Ready(image) = self {
            Some(image)
        } else {
            None
        }
    }
}

/// Immutable presentation input; submit through State, never from render.
#[derive(Clone, Debug, PartialEq)]
pub struct ImagePreviewDocument {
    pub(super) id: SharedString,
    pub(super) revision: u64,
    pub(super) source: ImagePreviewLoad,
    pub(super) result: ImagePreviewLoad,
    pub(super) full_frame: bool,
}
impl ImagePreviewDocument {
    pub fn new(id: impl Into<SharedString>, revision: u64) -> Self {
        Self {
            id: id.into(),
            revision,
            source: ImagePreviewLoad::Empty,
            result: ImagePreviewLoad::Empty,
            full_frame: false,
        }
    }
    pub fn with_source(mut self, source: ImagePreviewLoad) -> Self {
        self.source = source;
        self
    }
    pub fn with_result(mut self, result: ImagePreviewLoad) -> Self {
        self.result = result;
        self
    }
    /// Assert that both images represent the same complete, oriented frame.
    /// Aspect-ratio validation is also required; this does not stretch a crop.
    pub fn full_frame(mut self, aligned: bool) -> Self {
        self.full_frame = aligned;
        self
    }
    pub fn id(&self) -> &SharedString {
        &self.id
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn source(&self) -> &ImagePreviewLoad {
        &self.source
    }
    pub fn result(&self) -> &ImagePreviewLoad {
        &self.result
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ImagePreviewMode {
    #[default]
    Source,
    Compare,
    Result,
}
impl ImagePreviewMode {
    pub(super) fn value(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Compare => "compare",
            Self::Result => "result",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ImagePreviewFit {
    #[default]
    FitAll,
    FitWidth,
    Manual,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ImagePreviewBackground {
    #[default]
    CheckerDark,
    CheckerLight,
    White,
    Black,
}
impl ImagePreviewBackground {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::CheckerDark => "深色棋盘格",
            Self::CheckerLight => "浅色棋盘格",
            Self::White => "纯白",
            Self::Black => "纯黑",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ImagePreviewToolbar {
    #[default]
    Floating,
    Footer,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImagePreviewChangeSource {
    Pointer,
    Keyboard,
    Toolbar,
    Accessibility,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImagePreviewSnapshot {
    pub(super) document_id: Option<SharedString>,
    pub(super) revision: u64,
    pub(super) result_revision: Option<u64>,
    pub(super) mode: ImagePreviewMode,
    pub(super) fit: ImagePreviewFit,
    pub(super) zoom: f32,
    pub(super) center: [f32; 2],
    pub(super) split: f32,
    pub(super) background: ImagePreviewBackground,
    pub(super) reference_size: Option<[u32; 2]>,
}
impl ImagePreviewSnapshot {
    pub fn document_id(&self) -> Option<&SharedString> {
        self.document_id.as_ref()
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn result_revision(&self) -> Option<u64> {
        self.result_revision
    }
    pub fn mode(&self) -> ImagePreviewMode {
        self.mode
    }
    pub fn fit(&self) -> ImagePreviewFit {
        self.fit
    }
    pub fn zoom(&self) -> f32 {
        self.zoom
    }
    /// Image-space position at the viewport's safe-area midpoint. Moving a small
    /// image through the surrounding space can place this outside `[0, 1]`.
    pub fn center(&self) -> [f32; 2] {
        self.center
    }
    pub fn split(&self) -> f32 {
        self.split
    }
    pub fn background(&self) -> ImagePreviewBackground {
        self.background
    }
    pub fn reference_size(&self) -> Option<[u32; 2]> {
        self.reference_size
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImagePreviewEvent {
    ModeChanged(ImagePreviewSnapshot, ImagePreviewChangeSource),
    ViewChanged(ImagePreviewSnapshot, ImagePreviewChangeSource),
    SplitChanged(ImagePreviewSnapshot, ImagePreviewChangeSource),
    BackgroundChanged(ImagePreviewSnapshot, ImagePreviewChangeSource),
    RetryRequested {
        document_id: SharedString,
        revision: u64,
        result: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImagePreviewError {
    InvalidImage,
    EmptyDocumentId,
    StaleRevision,
    UnavailableMode,
    InvalidValue,
}
impl fmt::Display for ImagePreviewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidImage => "image must contain one non-empty static frame",
            Self::EmptyDocumentId => "document identity must not be empty",
            Self::StaleRevision => "document revision is older than the current request",
            Self::UnavailableMode => "requested preview mode is unavailable or unaligned",
            Self::InvalidValue => "preview value must be finite and valid",
        })
    }
}
impl std::error::Error for ImagePreviewError {}
