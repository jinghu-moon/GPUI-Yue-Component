//! Select measurements shared by the field, panel and option rows.

use super::{model::*, *};

use crate::{InputSize, TagSize};

use gpui_kit::Pixels;

impl SelectSize {
    pub(super) fn suffix(self) -> &'static str {
        match self {
            Self::Small => "sm",
            Self::Medium => "md",
            Self::Large => "lg",
        }
    }
    pub(super) fn input(self) -> InputSize {
        match self {
            Self::Small => InputSize::Small,
            Self::Medium => InputSize::Medium,
            Self::Large => InputSize::Large,
        }
    }
    pub(super) fn tag(self) -> TagSize {
        match self {
            Self::Small => TagSize::Small,
            Self::Medium => TagSize::Medium,
            Self::Large => TagSize::Large,
        }
    }
}

pub(super) use crate::choice_field::{measure, metric};

pub(super) fn option_height(p: &Presentation, theme: YueTheme, window: &Window) -> Pixels {
    let name = if p.row_layout == SelectRowLayout::Description {
        format!("--select-option-description-height-{}", p.size.suffix())
    } else {
        format!("--select-option-height-{}", p.size.suffix())
    };
    metric(theme, &name, window)
}

pub(super) fn row_height(row: &Row, p: &Presentation, theme: YueTheme, window: &Window) -> Pixels {
    if matches!(row, Row::Group(..)) {
        metric(theme, "--input-height-sm", window)
    } else {
        option_height(p, theme, window)
    }
}
