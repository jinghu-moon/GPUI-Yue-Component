use super::*;
use crate::{
    YueOverflowTooltip,
    motion::{self, MotionDuration, MotionEase},
    theme::{TokenKey, keys as k},
    tokens::{mix_rgba, number, rems, rgba, shadow},
};
use gpui_kit::base::{TestSupportExt as _, TextSelection};
use gpui_kit::{
    App, ClickEvent, Div, FocusHandle, FontWeight, InteractiveElement as _, MouseButton,
    ParentElement as _, Pixels, Point, RenderOnce, Role, StatefulInteractiveElement as _,
    Styled as _, Window, div, prelude::FluentBuilder as _, relative,
};
use std::{cell::Cell, rc::Rc};

impl CardSize {
    fn choose(self, sm: TokenKey, md: TokenKey, lg: TokenKey) -> TokenKey {
        match self {
            Self::Small => sm,
            Self::Medium => md,
            Self::Large => lg,
        }
    }
    fn padding(self) -> TokenKey {
        self.choose(k::CARD_PADDING_SM, k::CARD_PADDING_MD, k::CARD_PADDING_LG)
    }
    fn gap(self) -> TokenKey {
        self.choose(k::CARD_GAP_SM, k::CARD_GAP_MD, k::CARD_GAP_LG)
    }
    fn title(self) -> TokenKey {
        self.choose(
            k::CARD_TITLE_FONT_SIZE_SM,
            k::CARD_TITLE_FONT_SIZE_MD,
            k::CARD_TITLE_FONT_SIZE_LG,
        )
    }
    fn body(self) -> TokenKey {
        self.choose(
            k::CARD_BODY_FONT_SIZE_SM,
            k::CARD_BODY_FONT_SIZE_MD,
            k::CARD_BODY_FONT_SIZE_LG,
        )
    }
    fn description(self) -> TokenKey {
        self.choose(
            k::CARD_DESCRIPTION_FONT_SIZE_SM,
            k::CARD_DESCRIPTION_FONT_SIZE_MD,
            k::CARD_DESCRIPTION_FONT_SIZE_LG,
        )
    }
}

#[derive(Clone)]
struct Command {
    handler: crate::button::ClickHandler,
    focus: FocusHandle,
}
#[derive(Default)]
struct Press {
    origin: Cell<Option<Point<Pixels>>>,
    cancelled: Cell<bool>,
}

// Only explicit static regions are wired. Independent slots have no parent click listener.
fn static_region(
    base: Div,
    command: Option<&Command>,
    id: ElementId,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let base = base.id(id.clone()).test_support();
    let Some(command) = command.cloned() else {
        return base.into_any_element();
    };
    let press = window
        .use_keyed_state((id, "press"), cx, |_, _| Rc::new(Press::default()))
        .read(cx)
        .clone();
    let down = press.clone();
    let moved = press.clone();
    base.on_mouse_down(MouseButton::Left, move |e, w, cx| {
        down.origin.set(Some(e.position));
        down.cancelled.set(TextSelection::has_selection(w, cx));
    })
    .on_mouse_move(move |e, _, _| {
        if let Some(start) = moved.origin.get() {
            // Pointer slop in GPUI logical pixels, independent of rem zoom.
            if (e.position - start).magnitude() > 5. {
                moved.cancelled.set(true);
            }
        }
    })
    .on_click(move |event, w, cx| {
        let origin = press.origin.take();
        if let ClickEvent::Mouse(mouse) = event
            && !event.is_secondary()
            && origin.is_some()
            && !press.cancelled.get()
            && (mouse.up.position - mouse.down.position).magnitude() <= 5.
            && !TextSelection::has_selection(w, cx)
        {
            command.focus.focus(w, cx);
            (command.handler)(event, w, cx);
        }
    })
    .into_any_element()
}

fn text_column(
    id: ElementId,
    title: SharedString,
    description: Option<SharedString>,
    truncate: bool,
    size: CardSize,
    theme: YueTheme,
) -> Div {
    let title_id: ElementId = (id.clone(), "title").into();
    let title_content = if truncate {
        YueOverflowTooltip::new(title_id, theme, title.clone()).into_any_element()
    } else {
        div().child(title.clone()).into_any_element()
    };
    div()
        .min_w_0()
        .flex_1()
        .flex()
        .flex_col()
        .gap(rems(theme, k::SPACE_4))
        .child(
            div()
                .id((id.clone(), "title-region"))
                .test_support()
                .min_w_0()
                .w_full()
                .role(Role::Label)
                .aria_label(title)
                .text_size(rems(theme, size.title()))
                .font_weight(FontWeight(number(theme, k::CARD_TITLE_WEIGHT)))
                .text_color(rgba(theme, k::CARD_TITLE_COLOR))
                .child(title_content),
        )
        .when_some(description.filter(|d| !d.trim().is_empty()), |el, d| {
            el.child(
                div()
                    .id((id, "description"))
                    .test_support()
                    .min_w_0()
                    .w_full()
                    .role(Role::Label)
                    .aria_label(d.clone())
                    .text_size(rems(theme, size.description()))
                    .text_color(rgba(theme, k::CARD_DESCRIPTION_COLOR))
                    .child(d),
            )
        })
}

impl RenderOnce for YueCardMeta {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = self.theme.unwrap_or_else(|| crate::current_theme(cx));
        div()
            .id(self.id.clone())
            .test_support()
            .min_w_0()
            .flex()
            .items_start()
            .gap(rems(theme, self.size.gap()))
            .when_some(self.leading, |el, leading| {
                el.child(div().flex_none().child(leading))
            })
            .child(text_column(
                self.id,
                self.title,
                self.description,
                self.truncate,
                self.size,
                theme,
            ))
    }
}

struct CoverEdges {
    radius: gpui_kit::Rems,
    top: bool,
    bottom: bool,
}

fn cover_element(
    cover: YueCardCover,
    id: ElementId,
    theme: YueTheme,
    edges: CoverEdges,
    command: Option<&Command>,
    w: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let CoverEdges {
        radius,
        top,
        bottom,
    } = edges;
    let side = matches!(
        cover.placement,
        CardCoverPlacement::Left | CardCoverPlacement::Right
    );
    let mut base = div().relative().min_w_0().flex_shrink_0().overflow_hidden();
    base = match cover.placement {
        CardCoverPlacement::Left => base.rounded_l(radius),
        CardCoverPlacement::Right => base.rounded_r(radius),
        _ => base
            .w_full()
            .when(top, |d| d.rounded_t(radius))
            .when(bottom, |d| d.rounded_b(radius)),
    };
    if side {
        base = base
            .w(cover
                .side_width
                .unwrap_or_else(|| relative(number(theme, k::CARD_COVER_SIDE_RATIO))))
            .min_h(rems(theme, k::CARD_COVER_SIDE_MIN_HEIGHT));
    } else {
        base = match cover.geometry {
            CoverGeometry::Ratio(ratio) => base.aspect_ratio(ratio),
            CoverGeometry::Height(h) => base.h(h),
        };
    }
    let mut corners = gpui_kit::Corners::all(gpui_kit::rems(0.));
    match cover.placement {
        CardCoverPlacement::Left => {
            corners.top_left = radius;
            corners.bottom_left = radius;
        }
        CardCoverPlacement::Right => {
            corners.top_right = radius;
            corners.bottom_right = radius;
        }
        _ => {
            if top {
                corners.top_left = radius;
                corners.top_right = radius;
            }
            if bottom {
                corners.bottom_left = radius;
                corners.bottom_right = radius;
            }
        }
    }
    static_region(
        base.child(div().absolute().inset_0().child((cover.content)(corners))),
        command,
        (id, "cover").into(),
        w,
        cx,
    )
    .into_any_element()
}

impl RenderOnce for YueCard {
    fn render(mut self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = self.theme.unwrap_or_else(|| crate::current_theme(cx));
        let id = self.id;
        let pad = rems(theme, self.size.padding());
        let gap = rems(theme, self.size.gap());
        let border = rems(theme, k::CARD_BORDER_WIDTH);
        let radius = rems(theme, k::CARD_RADIUS);
        let inner = gpui_kit::rems((radius.0 - border.0).max(0.));
        let primary_disabled = self
            .primary
            .as_ref()
            .is_some_and(|p| p.card_command().is_none());
        let mut command = None;
        if let Some(action) = self.primary.take() {
            let focus = action.card_focus().unwrap_or_else(|| {
                w.use_keyed_state((id.clone(), "primary-focus"), cx, |_, cx| cx.focus_handle())
                    .read(cx)
                    .clone()
            });
            command = action.card_command().map(|handler| Command {
                handler,
                focus: focus.clone(),
            });
            let action = action.track_focus(&focus);
            let footer = self
                .footer
                .get_or_insert_with(|| YueCardFooter::actions([]));
            footer.content =
                match std::mem::replace(&mut footer.content, FooterContent::Actions(Vec::new())) {
                    FooterContent::Actions(mut actions) => {
                        actions.push(action);
                        FooterContent::Actions(actions)
                    }
                    FooterContent::Custom(content) => FooterContent::Custom(
                        div()
                            .min_w_0()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .justify_between()
                            .gap(gap)
                            .child(content)
                            .child(action)
                            .into_any_element(),
                    ),
                };
        }
        let effect = if primary_disabled {
            CardHoverEffect::None
        } else {
            self.hover.unwrap_or(if command.is_some() {
                CardHoverEffect::Outline
            } else {
                CardHoverEffect::None
            })
        };
        // Keep the content minimum: percentage height may be unresolved in a
        // natural-height flex column, but must not compress a nested grid.
        let mut root = div()
            .id(id.clone())
            .test_support()
            .relative()
            .w_full()
            .h_full()
            .min_w_0()
            .flex()
            .flex_col()
            .border(border)
            .rounded(radius)
            .text_size(rems(theme, self.size.body()))
            .text_color(rgba(theme, k::CARD_BODY_COLOR));
        if let Some(label) = self.label {
            root = root.role(Role::Group).aria_label(label);
        }
        let progress = if effect != CardHoverEffect::None {
            let hover = w.use_keyed_state((id.clone(), "hover"), cx, |_, _| false);
            let value = *hover.read(cx);
            root = root.on_hover(w.listener_for(&hover, |state, next, _, cx| {
                if *state != *next {
                    *state = *next;
                    cx.notify();
                }
            }));
            gpui_kit::base::transition(
                (id.clone(), "hover-transition"),
                if value { 1. } else { 0. },
                motion::transition(theme, MotionDuration::Interaction, MotionEase::Standard),
                w,
                cx,
            )
        } else {
            0.
        };
        let plain = rgba(theme, k::CARD_BORDER_COLOR);
        let mut color = if self.variant == CardVariant::Outlined {
            plain
        } else {
            gpui_kit::Rgba::default()
        };
        color = mix_rgba(color, rgba(theme, k::CARD_BORDER_COLOR_HOVER), progress);
        root = root
            .border_color(if self.selected {
                rgba(theme, k::CARD_BORDER_COLOR_SELECTED)
            } else {
                color
            })
            .bg(rgba(
                theme,
                if self.selected {
                    k::CARD_BACKGROUND_SELECTED
                } else if self.variant == CardVariant::Filled {
                    k::CARD_BACKGROUND_FILLED
                } else {
                    k::CARD_BACKGROUND
                },
            ));
        let mut shadows = if self.variant == CardVariant::Elevated {
            shadow(theme, k::CARD_SHADOW)
        } else {
            Vec::new()
        };
        if effect == CardHoverEffect::Lift {
            for s in &mut shadows {
                s.color.a *= 1. - progress;
            }
            shadows.extend(
                shadow(theme, k::CARD_SHADOW_HOVER)
                    .into_iter()
                    .map(|mut s| {
                        s.color.a *= progress;
                        s
                    }),
            );
            if !cx.reduce_motion() {
                root = root.top(rems(theme, k::CARD_OFFSET_HOVER) * -progress);
            }
        }
        root = root.shadow(shadows);
        if self
            .footer
            .as_ref()
            .is_some_and(|f| matches!(&f.content, FooterContent::Actions(a) if a.is_empty()))
        {
            self.footer = None;
        }
        let has_header = self.header.is_some();
        let has_body = !self.body.is_empty();
        let has_footer = self.footer.is_some();
        let header_divided = self.header.as_ref().is_some_and(|h| h.divided);
        let placement = self.cover.as_ref().map(|c| c.placement);
        let side = matches!(
            placement,
            Some(CardCoverPlacement::Left | CardCoverPlacement::Right)
        );
        let before = placement == Some(CardCoverPlacement::BeforeHeader);
        let after = placement == Some(CardCoverPlacement::AfterHeader);
        // Grow from the measured content, rather than a zero flex basis. This
        // preserves both natural height and stretched cards with pinned footers.
        let mut column = div()
            .id((id.clone(), "column"))
            .test_support()
            .min_w_0()
            .flex_grow(1.)
            .flex()
            .flex_col();
        let mut leading_cover = None;
        let mut trailing_cover = None;
        if before && let Some(cover) = self.cover.take() {
            column = column.child(cover_element(
                cover,
                id.clone(),
                theme,
                CoverEdges {
                    radius: inner,
                    top: true,
                    bottom: !has_header && !has_body && !has_footer,
                },
                command.as_ref(),
                w,
                cx,
            ));
        }
        if let Some(header) = self.header {
            let tight = has_body && !after && !header.divided;
            let mut header_region = div().min_w_0().flex_none().p(pad).when(tight, |d| d.pb_0());
            match header.content {
                HeaderContent::Custom(content) => {
                    header_region = header_region.child(content);
                }
                HeaderContent::Standard {
                    title,
                    description,
                    leading,
                    extra,
                    truncate,
                } => {
                    header_region = header_region
                        .flex()
                        .flex_wrap()
                        .items_start()
                        .gap(gap)
                        .when_some(leading, |d, el| d.child(div().flex_none().child(el)))
                        .child(static_region(
                            div().min_w_0().flex_1().child(text_column(
                                id.clone(),
                                title,
                                description,
                                truncate,
                                self.size,
                                theme,
                            )),
                            command.as_ref(),
                            (id.clone(), "header-text").into(),
                            w,
                            cx,
                        ))
                        .when_some(extra, |d, el| {
                            d.child(div().min_w_0().max_w_full().flex_shrink_0().child(el))
                        });
                }
            }
            column = column.child(header_region.id((id.clone(), "header")).test_support());
            if header.divided && (has_body || after || has_footer) {
                column = column.child(
                    div()
                        .h(border)
                        .w_full()
                        .flex_none()
                        .bg(rgba(theme, k::CARD_DIVIDER_COLOR)),
                );
            }
        }
        if after && let Some(cover) = self.cover.take() {
            column = column.child(cover_element(
                cover,
                id.clone(),
                theme,
                CoverEdges {
                    radius: inner,
                    top: !has_header,
                    bottom: !has_body && !has_footer,
                },
                command.as_ref(),
                w,
                cx,
            ));
        }
        if has_body {
            let body = div()
                .min_w_0()
                .flex_grow(1.)
                .flex()
                .flex_col()
                .gap(gap)
                .when(self.body_inset, |d| d.p(pad))
                .when(self.body_inset && !after && header_divided, |d| d.pt(pad))
                .when(
                    self.body_inset && has_header && !after && !header_divided,
                    |d| d.pt(gap),
                )
                .children(self.body);
            // A missing header must keep a complete top inset.
            column = column.child(static_region(
                body,
                command.as_ref(),
                (id.clone(), "body").into(),
                w,
                cx,
            ));
        } else if has_footer {
            column = column.child(div().flex_1());
        }
        if let Some(footer) = self.footer {
            let mut foot = div()
                .id((id.clone(), "footer"))
                .test_support()
                .min_w_0()
                .flex_none()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(gap)
                .when(footer.inset, |d| d.p(pad));
            let header_owns_boundary = has_header && header_divided && !has_body && !after;
            if footer.divided
                && !header_owns_boundary
                && (has_body || placement.is_some() || has_header)
            {
                foot = foot
                    .border_t(border)
                    .border_color(rgba(theme, k::CARD_DIVIDER_COLOR));
            }
            foot = match footer.content {
                FooterContent::Custom(content) => foot.child(content),
                FooterContent::Actions(actions) => {
                    foot.children(actions.into_iter().enumerate().map(|(ix, action)| {
                        div()
                            .id(("action", ix))
                            .min_w(rems(theme, k::CARD_ACTION_MIN_WIDTH))
                            .flex_1()
                            .child(action.block())
                    }))
                }
            };
            column = column.child(foot);
        }
        if let Some(cover) = self.cover.take() {
            let left = cover.placement == CardCoverPlacement::Left;
            let el = cover_element(
                cover,
                id.clone(),
                theme,
                CoverEdges {
                    radius: inner,
                    top: true,
                    bottom: true,
                },
                command.as_ref(),
                w,
                cx,
            );
            if left {
                leading_cover = Some(el);
            } else {
                trailing_cover = Some(el);
            }
        }
        if side {
            root = root.flex_row().items_stretch();
        }
        self.layout.apply(
            root.when_some(leading_cover, |d, el| d.child(el))
                .child(column)
                .when_some(trailing_cover, |d, el| d.child(el)),
        )
    }
}
