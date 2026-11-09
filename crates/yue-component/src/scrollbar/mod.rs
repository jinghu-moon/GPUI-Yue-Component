//! Floating scrollbar presentation over the existing Base scroll owner.
use crate::{
    YueTheme,
    tokens::{cubic_bezier, duration, rems, rgba},
};
use gpui_kit::base::{
    Scrollbar, ScrollbarBehavior, ScrollbarHandle, ScrollbarMode, ScrollbarMotion, ScrollbarStyles,
    ScrollbarTheme,
};
use gpui_kit::{App, Bounds, ElementId, IntoElement, Pixels, RenderOnce, Window};

#[cfg(all(test, feature = "headless-tests"))]
mod tests;

/// When an overflowing viewport reveals its floating thumb.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollbarVisibility {
    /// Reveal on scroll or at the edge; fade after activity ends.
    #[default]
    Auto,
    /// Reveal while the pointer is inside the scrolling viewport.
    Hover,
    /// Keep visible whenever this axis overflows.
    Always,
}

/// A scrollbar overlay. The supplied handle remains the only scroll owner.
#[derive(IntoElement)]
pub struct YueScrollbar {
    base: Scrollbar,
    theme: Option<YueTheme>,
    visibility: ScrollbarVisibility,
}

impl YueScrollbar {
    pub fn vertical<H: ScrollbarHandle + Clone>(id: impl Into<ElementId>, handle: &H) -> Self {
        Self::from_base(Scrollbar::vertical(handle).id(id))
    }
    pub fn horizontal<H: ScrollbarHandle + Clone>(id: impl Into<ElementId>, handle: &H) -> Self {
        Self::from_base(Scrollbar::horizontal(handle).id(id))
    }
    pub fn both<H: ScrollbarHandle + Clone>(id: impl Into<ElementId>, handle: &H) -> Self {
        Self::from_base(Scrollbar::new(handle).id(id))
    }
    fn from_base(base: Scrollbar) -> Self {
        Self {
            base,
            theme: None,
            visibility: ScrollbarVisibility::Auto,
        }
    }
    pub fn visibility(mut self, visibility: ScrollbarVisibility) -> Self {
        self.visibility = visibility;
        self
    }
    pub fn theme(mut self, theme: YueTheme) -> Self {
        self.theme = Some(theme);
        self
    }
    /// Override overlay bounds, preserving the handle's actual scroll range.
    pub fn viewport_bounds(mut self, bounds: Bounds<Pixels>) -> Self {
        self.base = self.base.viewport_bounds(bounds);
        self
    }
    pub fn viewport_from_layout(mut self) -> Self {
        self.base = self.base.viewport_from_layout();
        self
    }
}

impl RenderOnce for YueScrollbar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.base.theme(project(
            self.theme.unwrap_or_else(|| crate::current_theme(cx)),
            window,
            self.visibility,
        ))
    }
}

pub(crate) fn install(cx: &mut App) {
    // Internal Base consumers (editor, text blocks) receive the same design,
    // resolved per window rather than storing one window's Pixels globally.
    gpui_kit::base::Theme::global_mut(cx).scrollbar =
        ScrollbarTheme::new().with_resolver(|window, cx| {
            project(crate::current_theme(cx), window, ScrollbarVisibility::Auto)
        });
}

fn project(theme: YueTheme, window: &Window, visibility: ScrollbarVisibility) -> ScrollbarTheme {
    let metric = |name| rems(theme, name).to_pixels(window.rem_size());
    let inset = metric("--scrollbar-thumb-inset");
    let min_length = metric("--scrollbar-thumb-min-length");
    let active_width = metric("--scrollbar-thumb-width-active");
    let clear = gpui_kit::transparent_black();
    let styles = ScrollbarStyles::default()
        .track(|s| {
            s.width(metric("--scrollbar-hit-width"))
                .bg(clear)
                .border_color(clear)
        })
        .track_hover(|s| s.bg(clear).border_color(clear))
        .track_active(|s| s.bg(clear).border_color(clear))
        .thumb(|s| {
            s.width(metric("--scrollbar-thumb-width"))
                .inset(inset)
                .radius(active_width / 2.)
                .min_length(min_length)
                .bg(rgba(theme, "--scrollbar-thumb-color"))
        })
        .thumb_hover(|s| {
            s.width(metric("--scrollbar-thumb-width-hover"))
                .inset(inset)
                .radius(active_width / 2.)
                .min_length(min_length)
                .bg(rgba(theme, "--scrollbar-thumb-color-hover"))
        })
        .thumb_active(|s| {
            s.width(active_width)
                .inset(inset)
                .radius(active_width / 2.)
                .min_length(min_length)
                .bg(rgba(theme, "--scrollbar-thumb-color-active"))
        });
    let curve = |name| {
        let (a, b, c, d) = cubic_bezier(theme, name);
        [a, b, c, d]
    };
    let motion = ScrollbarMotion::default()
        .with_idle(duration(theme, "--scrollbar-idle-delay"))
        .with_enter(duration(theme, "--scrollbar-duration-enter"))
        .with_exit(duration(theme, "--scrollbar-duration-exit"))
        .with_expand(duration(theme, "--scrollbar-duration-expand"))
        .with_color(duration(theme, "--scrollbar-duration-color"))
        .with_fade_easing(curve("--scrollbar-ease-fade"))
        .with_expand_easing(curve("--scrollbar-ease-expand"));
    ScrollbarTheme::new()
        .with_styles(styles)
        .with_motion(motion)
        .with_mode(match visibility {
            ScrollbarVisibility::Auto => ScrollbarMode::Scrolling,
            ScrollbarVisibility::Hover => ScrollbarMode::Hover,
            ScrollbarVisibility::Always => ScrollbarMode::Always,
        })
        .with_behavior(
            ScrollbarBehavior::new()
                .with_edge_reveal(true)
                .with_viewport_hover(visibility == ScrollbarVisibility::Hover)
                .with_track_hover_expansion(true)
                .with_floating_geometry(true)
                .with_frame_driven_drag(true)
                .with_page_step(0.9),
        )
}
