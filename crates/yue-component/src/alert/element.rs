use super::*;
use crate::{
    ButtonSize, ButtonVariant, YueButton, YueIcon,
    motion::{MotionDuration, MotionEase, transition},
    theme::keys as k,
    tokens::{rems, rgba},
    typography::TextRole,
};
use gpui_kit::{
    InteractiveElement as _, RenderOnce, Role, StatefulInteractiveElement as _, Styled as _,
    TextRun,
    base::{
        MotionStatus, NoticeController, TestSupportExt as _, motion::MotionDisclosure,
        transition_at,
    },
    canvas, div,
    prelude::FluentBuilder as _,
    px,
};
use std::cell::RefCell;

#[derive(Default)]
struct Measurement {
    narrow: bool,
    overflow: bool,
    signature: Option<(
        SharedString,
        gpui_kit::Pixels,
        gpui_kit::Pixels,
        gpui_kit::Font,
        Option<usize>,
    )>,
}

pub(super) fn colors(
    theme: YueTheme,
    kind: AlertKind,
    variant: AlertVariant,
) -> (gpui_kit::Rgba, gpui_kit::Rgba, gpui_kit::Rgba) {
    let (bg, fg, border, solid, on) = match kind {
        AlertKind::Info => (
            k::INFO_BG,
            k::INFO_FOREGROUND,
            k::INFO_BORDER,
            k::ACCENT_SOLID,
            k::ON_ACCENT,
        ),
        AlertKind::Success => (
            k::SUCCESS_BG,
            k::SUCCESS_FOREGROUND,
            k::SUCCESS_BORDER,
            k::SUCCESS_SOLID,
            k::ON_SUCCESS_SOLID,
        ),
        AlertKind::Warning => (
            k::WARNING_BG,
            k::WARNING_FOREGROUND,
            k::WARNING_BORDER,
            k::WARNING_SOLID,
            k::ON_WARNING_SOLID,
        ),
        AlertKind::Error => (
            k::ERROR_BG,
            k::ERROR_FOREGROUND,
            k::ERROR_BORDER,
            k::ERROR_SOLID,
            k::ON_ERROR_SOLID,
        ),
    };
    if variant == AlertVariant::Solid {
        (rgba(theme, solid), rgba(theme, on), rgba(theme, solid))
    } else {
        (rgba(theme, bg), rgba(theme, fg), rgba(theme, border))
    }
}
impl RenderOnce for YueAlert {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = self.theme.unwrap_or_else(|| crate::current_theme(cx));
        let id = self.id;
        let state = w.use_keyed_state((id.clone(), "notice"), cx, NoticeController::new);
        let owner = w.current_view();
        state.update(cx, |s, cx| {
            s.sync(self.visible, self.duration, owner, self.focus_after, w, cx);
            s.handlers(self.on_close, self.on_closed);
        });
        let generation = state.read(cx).generation();
        let focus = state.read(cx).focus();
        let (bg, fg, border) = colors(theme, self.kind, self.variant);
        let small = self.size == AlertSize::Small;
        let has_description = self.description.is_some() || !self.children.is_empty();
        let foldable = self.description.is_some() && self.max_lines.is_some();
        let role = match self.announcement.unwrap_or(match self.kind {
            AlertKind::Info | AlertKind::Success => AlertAnnouncement::Polite,
            AlertKind::Warning | AlertKind::Error => AlertAnnouncement::Assertive,
        }) {
            AlertAnnouncement::Quiet => Role::Group,
            AlertAnnouncement::Polite => Role::Status,
            AlertAnnouncement::Assertive => Role::Alert,
        };
        let enter = transition(theme, MotionDuration::Enter, MotionEase::Enter);
        let exit = transition(theme, MotionDuration::Exit, MotionEase::Stow);
        let fade = transition_at(
            (id.clone(), "fade"),
            if self.visible { 1_f32 } else { 0. },
            if self.visible {
                enter.clone()
            } else {
                exit.clone()
            },
            cx.background_executor().now(),
            false,
            w,
            cx,
        );
        let measurement = w
            .use_keyed_state((id.clone(), "measurement"), cx, |_, _| {
                Rc::new(RefCell::new(Measurement::default()))
            })
            .read(cx)
            .clone();
        let narrow = measurement.borrow().narrow;
        let align_to_title = has_description || (narrow && self.action.is_some());
        let expand_focus = w
            .use_keyed_state((id.clone(), "expand-focus"), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let show_expand = foldable && measurement.borrow().overflow;
        if !show_expand && expand_focus.is_focused(w) {
            let focus = expand_focus.clone();
            w.on_next_frame(move |w, cx| {
                if focus.is_focused(w) {
                    w.blur(cx);
                    gpui_kit::base::Root::focus_next(w, cx);
                }
            });
        }
        let body = if self.visible {
            let icon = self.icon.unwrap_or_else(|| {
                Icon::new(match self.kind {
                    AlertKind::Info => YueIcon::Info,
                    AlertKind::Success => YueIcon::Success,
                    AlertKind::Warning => YueIcon::Warning,
                    AlertKind::Error => YueIcon::Error,
                })
            });
            let text_role = if small {
                TextRole::Caption
            } else {
                TextRole::Body
            };
            let text_style = text_role.style(theme);
            let first_line_height = px(text_style.line_height);
            let mut text = div()
                .id((id.clone(), "text"))
                .test_support()
                .flex()
                .flex_col()
                .min_w_0()
                .flex_1()
                .gap(rems(theme, k::SPACE_4))
                .child(
                    div()
                        .id((id.clone(), "message"))
                        .test_support()
                        .min_w_0()
                        .whitespace_normal()
                        .child(self.message.clone()),
                );
            if let Some(description) = self.description {
                let display = description.clone();
                let lines = self.max_lines;
                let observer = measurement.clone();
                let expanded = self.expanded;
                let capture = canvas(
                    move |bounds, w, cx| {
                        let style = w.text_style();
                        let size = style.font_size.to_pixels(w.rem_size());
                        let signature = (
                            description.clone(),
                            bounds.size.width,
                            size,
                            style.font(),
                            lines,
                        );
                        let mut cached = observer.borrow_mut();
                        if cached.signature.as_ref() != Some(&signature) {
                            let runs = [TextRun {
                                len: description.len(),
                                font: style.font(),
                                color: style.color,
                                ..Default::default()
                            }];
                            let overflow = lines.is_some_and(|n| {
                                w.text_system()
                                    .shape_text(
                                        description.clone(),
                                        size,
                                        &runs,
                                        Some(bounds.size.width.max(px(0.))),
                                        None,
                                    )
                                    .is_ok_and(|shaped| {
                                        shaped
                                            .iter()
                                            .map(|l| l.wrap_boundaries().len() + 1)
                                            .sum::<usize>()
                                            > n
                                    })
                            });
                            cached.signature = Some(signature);
                            if cached.overflow != overflow {
                                cached.overflow = overflow;
                                w.defer(cx, |w, _| w.refresh());
                            }
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0();
                // Shape against the same inherited typography as the displayed text.
                text = text.child(
                    div()
                        .id((id.clone(), "description"))
                        .test_support()
                        .relative()
                        .w_full()
                        .min_w_0()
                        .when(!expanded, |d| {
                            d.when_some(lines, |d, n| d.text_ellipsis().line_clamp(n))
                        })
                        .child(display)
                        .child(capture),
                );
            }
            text = text.children(self.children);
            if show_expand {
                let handler = self.on_expanded;
                let next = !self.expanded;
                text = text.child(
                    div().child(
                        YueButton::text(
                            (id.clone(), "expand"),
                            theme,
                            if self.expanded {
                                self.collapse_label
                            } else {
                                self.expand_label
                            },
                        )
                        .variant(if self.variant == AlertVariant::Solid {
                            ButtonVariant::Default
                        } else {
                            ButtonVariant::Ghost
                        })
                        .size(ButtonSize::Small)
                        .track_focus(&expand_focus)
                        .expanded(self.expanded)
                        .on_click(move |_, w, cx| {
                            if let Some(h) = &handler {
                                h(next, w, cx);
                            }
                        }),
                    ),
                );
            }
            let mut content = div()
                .id((id.clone(), "content"))
                .test_support()
                .flex()
                .min_w_0()
                .flex_1()
                .gap(rems(theme, k::SPACE_8))
                .when(!narrow, |d| {
                    if has_description {
                        d.items_start()
                    } else {
                        d.items_center()
                    }
                })
                .when(narrow, |d| d.flex_col());
            content = content.child(text);
            if let Some(action) = self.action {
                content = content.child(
                    div()
                        .id((id.clone(), "action"))
                        .test_support()
                        .min_w_0()
                        .max_w_full()
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .child(action),
                );
            }
            let hover = state.clone();
            let timer = state.clone();
            let measure = measurement.clone();
            let timed = self.duration.is_some();
            let capture = canvas(
                move |bounds, w, cx| {
                    let next = bounds.size.width < gpui_kit::rems(26.).to_pixels(w.rem_size());
                    {
                        let mut m = measure.borrow_mut();
                        if m.narrow != next {
                            m.narrow = next;
                            w.defer(cx, |w, _| w.refresh());
                        }
                    }
                    let exposed =
                        bounds.intersects(&w.content_mask().bounds) && bounds.size.height > px(0.);
                    let (progress, running) = timer.update(cx, |s, cx| s.sample(exposed, w, cx));
                    if running
                        || matches!(fade.status, MotionStatus::Running | MotionStatus::Delayed)
                    {
                        w.request_animation_frame();
                    }
                    (progress, timed)
                },
                move |bounds, (progress, timed), w, _| {
                    if timed {
                        let height = rems(theme, k::STATUS_BORDER_WIDTH).to_pixels(w.rem_size());
                        w.paint_quad(gpui_kit::fill(
                            gpui_kit::Bounds::new(
                                gpui_kit::point(bounds.left(), bounds.bottom() - height),
                                gpui_kit::size(bounds.size.width * progress, height),
                            ),
                            fg,
                        ));
                    }
                },
            )
            .absolute()
            .inset_0();
            let mut surface = text_style
                .apply(div())
                .id(id.clone())
                .test_support()
                .relative()
                .w_full()
                .min_w_0()
                .flex()
                .when(align_to_title, |d| d.items_start())
                .when(!align_to_title, |d| d.items_center())
                .gap(rems(theme, k::SPACE_8))
                .px(rems(theme, if small { k::SPACE_8 } else { k::SPACE_12 }))
                .py(rems(theme, if small { k::SPACE_4 } else { k::SPACE_8 }))
                .bg(bg)
                .text_color(fg)
                .track_focus(&focus)
                .role(role)
                .aria_label(self.message)
                .when(!self.banner, |d| d.rounded(rems(theme, k::RADIUS_MD)))
                .when(
                    self.variant == AlertVariant::Outlined && !self.banner,
                    |d| {
                        d.border(rems(theme, k::STATUS_BORDER_WIDTH))
                            .border_color(border)
                    },
                )
                .when(self.banner, |d| {
                    d.border_b(rems(theme, k::STATUS_BORDER_WIDTH))
                        .border_color(border)
                })
                .on_hover(move |value, w, cx| {
                    hover.update(cx, |s, cx| s.set_hovered(*value, w, cx))
                });
            if self.show_icon {
                let icon_size = rems(
                    theme,
                    if !small && has_description {
                        k::ICON_SIZE_MD
                    } else {
                        k::ICON_SIZE_SM
                    },
                );
                // A flex slot has no anonymous text baseline. Match the title's
                // line box; single-line surfaces additionally center the lane
                // against the command buttons' retained hit-target height.
                surface = surface.child(
                    div()
                        .id((id.clone(), "icon"))
                        .test_support()
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(icon_size)
                        .h(first_line_height)
                        .child(icon.color_value(fg).size(icon_size)),
                );
            }
            surface = surface.child(content);
            if self.closable {
                let close = state.clone();
                let button = if let Some(text) = self.close_text {
                    YueButton::text((id.clone(), "close"), theme, text)
                        .accessibility_label(self.close_label)
                } else {
                    let button = YueButton::icon(
                        (id.clone(), "close"),
                        theme,
                        YueIcon::Close,
                        self.close_label,
                    );
                    if let Some(icon) = self.close_icon {
                        button.replace_icon(icon)
                    } else {
                        button
                    }
                };
                surface = surface.child(
                    div().flex_shrink_0().flex().items_center().child(
                        button
                            .size(ButtonSize::Small)
                            .variant(if self.variant == AlertVariant::Solid {
                                ButtonVariant::Default
                            } else {
                                ButtonVariant::Ghost
                            })
                            .on_click(move |_, w, cx| {
                                close.update(cx, |s, cx| {
                                    s.request_close(AlertCloseReason::CloseButton, w, cx)
                                })
                            }),
                    ),
                );
            }
            Some(surface.child(capture).into_any_element())
        } else {
            None
        };
        let settled = state.downgrade();
        self.layout.apply(
            div().w_full().min_w_0().opacity(fade.value).child(
                MotionDisclosure::new((id, "disclosure"), self.visible, body)
                    .transitions(enter, exit)
                    .geometry_key(format!("{theme:?}/{:?}/{:?}", self.size, w.rem_size()))
                    .on_settled(move |open, w, cx| {
                        if let Some(s) = settled.upgrade() {
                            s.update(cx, |s, cx| s.settled(generation, open, w, cx));
                        }
                    }),
            ),
        )
    }
}
