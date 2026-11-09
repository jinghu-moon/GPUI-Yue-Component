use super::*;
use crate::{
    MotionDuration, MotionEase,
    theme::{TokenKey, keys as k},
    tokens::{rems, rgba},
};
use gpui_kit::base::{Presence, TestSupportExt as _, Transition};
use gpui_kit::{
    App, AvailableSpace, Bounds, BoxShadow, Element, GlobalElementId, InspectorElementId,
    InteractiveElement as _, LayoutId, ParentElement as _, Pixels, RenderOnce, Role,
    StatefulInteractiveElement as _, Style, Styled as _, Window, div, prelude::FluentBuilder as _,
    px, relative, size,
};
use std::time::Duration;

impl BadgeTone {
    pub(super) fn tokens(self) -> (TokenKey, TokenKey) {
        match self {
            Self::Default => (k::BADGE_BACKGROUND, k::BADGE_FOREGROUND),
            Self::Primary => (k::BADGE_PRIMARY_BACKGROUND, k::BADGE_PRIMARY_FOREGROUND),
            Self::Success => (k::BADGE_SUCCESS_BACKGROUND, k::BADGE_SUCCESS_FOREGROUND),
            Self::Warning => (k::BADGE_WARNING_BACKGROUND, k::BADGE_WARNING_FOREGROUND),
            Self::Danger => (k::BADGE_DANGER_BACKGROUND, k::BADGE_DANGER_FOREGROUND),
        }
    }
}

#[derive(Clone)]
struct Presentation {
    theme: YueTheme,
    text: Option<SharedString>,
    numeric: bool,
    label: Option<SharedString>,
    size: BadgeSize,
    shape: BadgeShape,
    background: Rgba,
    foreground: Rgba,
    separator: Rgba,
    name: SharedString,
    semantics: bool,
}

impl Presentation {
    fn element(
        &self,
        id: &ElementId,
        attached: bool,
        live: bool,
        opacity: f32,
        rem_size: Pixels,
    ) -> AnyElement {
        let theme = self.theme;
        let small = self.size == BadgeSize::Small;
        let dot = self.text.is_none();
        let height = rems(
            theme,
            match (dot, small) {
                (true, true) => k::BADGE_DOT_SIZE_SM,
                (true, false) => k::BADGE_DOT_SIZE,
                (false, true) => k::BADGE_HEIGHT_SM,
                (false, false) => k::BADGE_HEIGHT,
            },
        );
        let padding = rems(
            theme,
            if self.numeric {
                k::BADGE_COUNT_PADDING_INLINE
            } else if small {
                k::BADGE_PADDING_INLINE_SM
            } else {
                k::BADGE_PADDING_INLINE
            },
        );
        let font = if self.numeric {
            crate::typography::number_font(theme)
        } else {
            crate::typography::ui_font(theme)
        };
        let mut mark = div()
            .id((id.clone(), "mark"))
            .test_support()
            .flex()
            .items_center()
            .justify_center()
            .flex_shrink_0()
            .h(height)
            .min_w(height)
            .bg(self.background)
            .text_color(self.foreground)
            .text_size(rems(theme, k::BADGE_FONT_SIZE))
            .font_family(font.family)
            .font_features(crate::typography::tabular_nums())
            .line_height(relative(1.))
            .when(dot || self.shape == BadgeShape::Pill, |d| d.rounded_full())
            .when(!dot && self.shape == BadgeShape::Rounded, |d| {
                d.rounded(rems(theme, k::BADGE_SHAPE_RADIUS))
            });
        if let Some(text) = &self.text {
            mark = mark
                .px(padding)
                .child(div().min_w_0().truncate().child(text.clone()));
            if !self.numeric {
                mark = mark.max_w(rems(theme, k::BADGE_TEXT_MAX_WIDTH));
            }
        } else {
            mark = mark.w(height);
        }
        if attached {
            mark = mark.shadow(vec![BoxShadow {
                color: self.separator.into(),
                offset: point(px(0.), px(0.)),
                blur_radius: px(0.),
                spread_radius: rems(theme, k::BADGE_SEPARATOR_WIDTH).to_pixels(rem_size),
                inset: false,
            }]);
        }
        let mut root = div()
            .id((id.clone(), "badge"))
            .test_support()
            .flex()
            .items_center()
            .flex_shrink_0()
            .opacity(opacity)
            .child(mark);
        if let Some(label) = &self.label {
            root = root.gap(rems(theme, k::BADGE_LABEL_GAP)).child(
                div()
                    .font_family(crate::typography::ui_font(theme).family)
                    .text_size(rems(theme, k::FONT_SIZE_MD))
                    .text_color(rgba(theme, k::TEXT_SECONDARY))
                    .child(label.clone()),
            );
        }
        if live && self.semantics {
            root = root.role(Role::Caption).aria_label(self.name.clone());
        }
        root.into_any_element()
    }
}

impl RenderOnce for YueBadge {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        assert!(
            self.child.is_none() || self.label.is_none(),
            "YueBadge label is standalone-only; describe the host instead"
        );
        let theme = self.theme.unwrap_or_else(|| crate::current_theme(cx));
        let (bg, fg) = self.tone.tokens();
        let (background, foreground) = self.colors.unwrap_or_else(|| {
            (
                rgba(
                    theme,
                    if self.content == Content::Dot && self.tone == BadgeTone::Default {
                        k::BADGE_COLOR
                    } else {
                        bg
                    },
                ),
                rgba(theme, fg),
            )
        });
        let visible = self.is_visible();
        let presentation = Presentation {
            theme,
            text: self.display_text(),
            numeric: matches!(self.content, Content::Count(_)),
            label: self.label.clone(),
            size: self.size,
            shape: self.shape,
            background,
            foreground,
            separator: self
                .separator_color
                .unwrap_or_else(|| rgba(theme, k::BADGE_SEPARATOR_COLOR)),
            name: self.accessible_name(),
            semantics: self.semantics,
        };
        BadgeElement {
            id: self.id,
            host: self.child,
            presentation,
            visible,
            animated: self.animated,
            offset: self.offset,
            body: None,
        }
    }
}

// Only optional transition snapshots survive frames. Static badges retain no
// content copy, entity, timer, or task in this element-local memory.
struct Memory {
    last: Option<Presentation>,
    animated: bool,
    attached: bool,
}

struct BadgeElement {
    id: ElementId,
    host: Option<AnyElement>,
    presentation: Presentation,
    visible: bool,
    animated: bool,
    offset: Point<Rems>,
    body: Option<AnyElement>,
}

impl IntoElement for BadgeElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for BadgeElement {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let attached = self.host.is_some();
        let (first, previous) =
            window.with_element_state(id.unwrap(), |state: Option<Memory>, _| {
                let first = state
                    .as_ref()
                    .is_none_or(|s| !s.animated || s.attached != attached);
                let mut state = state.unwrap_or(Memory {
                    last: None,
                    animated: false,
                    attached,
                });
                if first || !self.animated {
                    state.last = None;
                }
                if self.animated && self.visible {
                    state.last = Some(self.presentation.clone());
                }
                state.animated = self.animated;
                state.attached = attached;
                ((first, state.last.clone()), state)
            });
        let (render, opacity) = if self.animated {
            let duration = if self.visible {
                MotionDuration::Enter
            } else {
                MotionDuration::Exit
            };
            let transition = if first {
                Transition::new(Duration::ZERO)
            } else {
                crate::motion::transition(self.presentation.theme, duration, MotionEase::Fade)
            };
            let sample = Presence::new("visibility", self.visible)
                .transition(transition)
                .sample(window, cx);
            (sample.should_render(), sample.progress)
        } else {
            (self.visible, 1.)
        };
        if render {
            let presentation = if self.visible {
                Some(&self.presentation)
            } else {
                previous.as_ref()
            };
            self.body = presentation
                .map(|p| p.element(&self.id, attached, self.visible, opacity, window.rem_size()));
        } else {
            window.with_element_state(id.unwrap(), |state: Option<Memory>, _| {
                let mut state = state.unwrap();
                state.last = None;
                ((), state)
            });
        }
        let layout = if let Some(host) = &mut self.host {
            // Delegate the same layout node: wrapping a host must not change
            // its margin, percent width, flex growth, or measurement constraints.
            host.request_layout(window, cx)
        } else if let Some(body) = &mut self.body {
            body.request_layout(window, cx)
        } else {
            window.request_layout(
                Style {
                    size: size(px(0.).into(), px(0.).into()),
                    ..Default::default()
                },
                [],
                cx,
            )
        };
        (layout, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(host) = &mut self.host {
            host.prepaint(window, cx);
            if let Some(body) = &mut self.body {
                let measured = body.layout_as_root(
                    size(AvailableSpace::MaxContent, AvailableSpace::MaxContent),
                    window,
                    cx,
                );
                let offset = point(
                    self.offset.x.to_pixels(window.rem_size()),
                    self.offset.y.to_pixels(window.rem_size()),
                );
                let origin = point(
                    bounds.right() - measured.width / 2.,
                    bounds.top() - measured.height / 2.,
                ) + offset;
                body.prepaint_at(origin, window, cx);
            }
        } else if let Some(body) = &mut self.body {
            body.prepaint(window, cx);
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(host) = &mut self.host {
            host.paint(window, cx);
        }
        if let Some(body) = &mut self.body {
            body.paint(window, cx);
        }
    }
}
