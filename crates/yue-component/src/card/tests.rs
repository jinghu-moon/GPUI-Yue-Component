use super::*;
use crate::{Accent, ListGrid, ThemeMode, YueList, YueListItem};
use gpui_kit::base::{
    Root, TestSupportExt as _,
    test_support::{ElementSnapshot, snapshots},
};
use gpui_kit::{
    AppContext as _, Context, Entity, InteractiveElement as _, KeyDownEvent, KeyUpEvent, Keystroke,
    MouseButton, MouseDownEvent, MouseUpEvent, ParentElement as _, Render, Styled as _,
    TestAppContext, VisualTestContext, Window, div, point, px,
};

struct Probe {
    width: f32,
    size: CardSize,
    variant: CardVariant,
    cover: Option<CardCoverPlacement>,
    header: bool,
    body: bool,
    footer: bool,
    grid: bool,
    disabled: bool,
    loading: bool,
    primary: bool,
    selected: bool,
    clicks: usize,
    secondary: usize,
    custom: bool,
    selectable: bool,
    lift: bool,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut card = YueCard::new("card")
            .size(self.size)
            .variant(self.variant)
            .selected(self.selected);
        if self.lift {
            card = card.hover_effect(CardHoverEffect::Lift);
        }
        if self.header {
            card = card.header(if self.custom {
                YueCardHeader::custom(YueButton::new("custom", "自定义").on_click(cx.listener(
                    |p, _, _, cx| {
                        p.secondary += 1;
                        cx.notify();
                    },
                )))
            } else {
                YueCardHeader::new("6205-ZZ 长标题需要自然换行")
                    .description("内径25 外径52 轴承工单")
                    .extra(
                        YueButton::new("extra", "收藏").on_click(cx.listener(|p, _, _, cx| {
                            p.secondary += 1;
                            cx.notify();
                        })),
                    )
            });
        }
        if self.body {
            card = if self.selectable {
                card.child(gpui_kit::base::SelectableText::new(
                    "selectable-body",
                    "Select this work order description without opening its details.",
                ))
            } else {
                card.child(
                    div()
                        .id("body-text")
                        .test_support()
                        .child("长中文正文在受限宽度下换行，不能撑出卡片。"),
                )
            };
        }
        if let Some(position) = self.cover {
            card = card.cover(
                YueCardCover::new(div().size_full().bg(gpui_kit::rgb(0x778899)))
                    .placement(position),
            );
        }
        if self.footer {
            card = card.footer(YueCardFooter::new(
                YueButton::new("secondary", "归档").on_click(cx.listener(|p, _, _, cx| {
                    p.secondary += 1;
                    cx.notify();
                })),
            ));
        }
        if self.primary {
            card = card.primary_action(
                YueButton::new("primary", "详情")
                    .disabled(self.disabled)
                    .loading(self.loading)
                    .on_click(cx.listener(|p, _, _, cx| {
                        p.clicks += 1;
                        cx.notify();
                    })),
            );
        }
        let content = if self.grid {
            let grid = ListGrid::min_width(px(170.), px(12.)).unwrap();
            YueList::new("cards")
                .grid(grid)
                .child(YueListItem::new("item-a").inset(false).content(card))
                .child(
                    YueListItem::new("item-b").inset(false).content(
                        YueCard::new("short")
                            .header(YueCardHeader::new("短标题"))
                            .child("短正文")
                            .footer(YueCardFooter::new("页脚")),
                    ),
                )
                .child(
                    YueListItem::new("item-c")
                        .inset(false)
                        .content(YueCard::new("third").child("第三张")),
                )
                .into_any_element()
        } else {
            card.into_any_element()
        };
        div().p_4().w(px(self.width)).child(content)
    }
}
fn open(cx: &mut TestAppContext) -> (Entity<Probe>, &mut VisualTestContext) {
    cx.update(|cx| crate::init(cx, crate::YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap());
    let mut saved = None;
    let (_, cx) = cx.add_window_view(|w, cx| {
        let view = cx.new(|_| Probe {
            width: 420.,
            size: CardSize::Medium,
            variant: CardVariant::Outlined,
            cover: None,
            header: true,
            body: true,
            footer: true,
            grid: false,
            disabled: false,
            loading: false,
            primary: false,
            selected: false,
            clicks: 0,
            secondary: 0,
            custom: false,
            selectable: false,
            lift: false,
        });
        saved = Some(view.clone());
        Root::new(view, w, cx)
    });
    draw(cx);
    (saved.unwrap(), cx)
}
fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| {
        w.refresh();
        w.draw(cx).clear(cx);
    });
}
fn get(cx: &mut VisualTestContext, id: impl Into<ElementId>) -> ElementSnapshot {
    let id = id.into();
    cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .find(|n| n.path().last() == Some(&id))
            .unwrap_or_else(|| panic!("missing {id:?}"))
    })
}
fn part(cx: &mut VisualTestContext, id: &str) -> ElementSnapshot {
    get(cx, (ElementId::from("card"), id.to_owned()))
}
fn click(cx: &mut VisualTestContext, id: impl Into<ElementId>) {
    let p = get(cx, id).bounds().center();
    cx.simulate_click(p, Default::default());
    draw(cx);
}
fn key(cx: &mut VisualTestContext, key: &str) {
    let keystroke = Keystroke::parse(key).unwrap();
    cx.simulate_event(KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(KeyUpEvent { keystroke });
    draw(cx);
}

#[gpui_kit::test]
fn omitted_regions_and_cover_order_have_natural_geometry(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    for header in [false, true] {
        for body in [false, true] {
            for footer in [false, true] {
                view.update(cx, |p, cx| {
                    p.header = header;
                    p.body = body;
                    p.footer = footer;
                    cx.notify();
                });
                draw(cx);
                let nodes = cx.update(|w, _| snapshots(w));
                for (name, exists) in [("header", header), ("body", body), ("footer", footer)] {
                    assert_eq!(
                        nodes.iter().any(|n| n.path().last()
                            == Some(&ElementId::from((ElementId::from("card"), name)))),
                        exists
                    );
                }
                let card = get(cx, "card").bounds();
                if body {
                    let body = part(cx, "body").bounds();
                    assert!(body.top() >= card.top() && body.bottom() <= card.bottom());
                }
            }
        }
    }
    for placement in [
        CardCoverPlacement::BeforeHeader,
        CardCoverPlacement::AfterHeader,
        CardCoverPlacement::Left,
        CardCoverPlacement::Right,
    ] {
        view.update(cx, |p, cx| {
            p.header = true;
            p.body = true;
            p.footer = true;
            p.cover = Some(placement);
            cx.notify();
        });
        draw(cx);
        let cover = part(cx, "cover").bounds();
        let header = part(cx, "header").bounds();
        let body = part(cx, "body").bounds();
        assert!(cover.size.width > px(0.) && cover.size.height > px(0.));
        match placement {
            CardCoverPlacement::BeforeHeader => assert!(cover.bottom() <= header.top() + px(0.1)),
            CardCoverPlacement::AfterHeader => assert!(
                cover.top() >= header.bottom() - px(0.1) && cover.bottom() <= body.top() + px(0.1)
            ),
            CardCoverPlacement::Left => assert!(cover.right() <= header.left() + px(0.1)),
            CardCoverPlacement::Right => assert!(cover.left() >= header.right() - px(0.1)),
        }
    }
}

#[gpui_kit::test]
fn theme_density_zoom_and_selected_state_preserve_bounds(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        for accent in [Accent::Azure, Accent::Neutral] {
            cx.update(|_, cx| crate::set_theme(cx, crate::YueTheme::new(mode, accent)));
            for size in [CardSize::Small, CardSize::Medium, CardSize::Large] {
                for rem in [16., 20., 24.] {
                    view.update(cx, |p, cx| {
                        p.size = size;
                        p.width = 260.;
                        cx.notify();
                    });
                    cx.update(|w, _| w.set_rem_size(px(rem)));
                    draw(cx);
                    let before = get(cx, "card").bounds();
                    let body = part(cx, "body").bounds();
                    assert!(body.right() <= before.right() + px(0.1));
                    view.update(cx, |p, cx| {
                        p.selected = !p.selected;
                        cx.notify();
                    });
                    draw(cx);
                    assert_eq!(get(cx, "card").bounds(), before);
                }
            }
        }
    }
    assert_eq!(get(cx, "card").role(), None);
}

#[gpui_kit::test]
fn list_grid_stretches_surfaces_and_pins_same_row_footers(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    view.update(cx, |p, cx| {
        p.grid = true;
        cx.notify();
    });
    draw(cx);
    draw(cx);
    let a = get(cx, "card").bounds();
    let b = get(cx, "short").bounds();
    assert!((a.top() - b.top()).abs() < px(0.1));
    assert!(
        (a.size.height - b.size.height).abs() < px(0.1),
        "{a:?} vs {b:?}"
    );
    let af = part(cx, "footer").bounds();
    let bf = get(cx, (ElementId::from("short"), "footer")).bounds();
    assert!((af.bottom() - bf.bottom()).abs() < px(0.1));
    assert!(get(cx, "third").bounds().top() >= a.bottom());
    view.update(cx, |p, cx| {
        p.width = 215.;
        cx.notify();
    });
    draw(cx);
    draw(cx);
    assert!(get(cx, "short").bounds().top() >= get(cx, "card").bounds().bottom());
}

#[gpui_kit::test]
fn primary_command_pointer_keyboard_disabled_and_independent_slots(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    view.update(cx, |p, cx| {
        p.primary = true;
        cx.notify();
    });
    draw(cx);
    click(cx, "extra");
    click(cx, "secondary");
    assert_eq!(view.read_with(cx, |p, _| (p.clicks, p.secondary)), (0, 2));
    click(cx, (ElementId::from("card"), "body"));
    assert_eq!(view.read_with(cx, |p, _| p.clicks), 1);
    key(cx, "enter");
    key(cx, "space");
    assert_eq!(view.read_with(cx, |p, _| p.clicks), 3);
    click(cx, "primary");
    assert_eq!(view.read_with(cx, |p, _| p.clicks), 4);
    assert_eq!(get(cx, "primary").role(), Some(gpui_kit::Role::Button));
    for loading in [false, true] {
        view.update(cx, |p, cx| {
            p.disabled = !loading;
            p.loading = loading;
            cx.notify();
        });
        draw(cx);
        click(cx, (ElementId::from("card"), "body"));
        click(cx, "primary");
        assert_eq!(view.read_with(cx, |p, _| p.clicks), 4);
    }
    view.update(cx, |p, cx| {
        p.disabled = false;
        p.loading = false;
        p.custom = true;
        cx.notify();
    });
    draw(cx);
    click(cx, "custom");
    assert_eq!(view.read_with(cx, |p, _| (p.clicks, p.secondary)), (4, 3));
}

#[gpui_kit::test]
fn dragging_even_back_to_origin_and_right_click_do_not_activate(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    view.update(cx, |p, cx| {
        p.primary = true;
        cx.notify();
    });
    draw(cx);
    let start = part(cx, "body").bounds().center();
    cx.simulate_event(MouseDownEvent {
        position: start,
        button: MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
        first_mouse: false,
    });
    cx.simulate_mouse_move(
        start + point(px(20.), px(0.)),
        Some(MouseButton::Left),
        Default::default(),
    );
    cx.simulate_mouse_move(start, Some(MouseButton::Left), Default::default());
    cx.simulate_event(MouseUpEvent {
        position: start,
        button: MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
    });
    draw(cx);
    cx.simulate_event(MouseDownEvent {
        position: start,
        button: MouseButton::Right,
        modifiers: Default::default(),
        click_count: 1,
        first_mouse: false,
    });
    cx.simulate_event(MouseUpEvent {
        position: start,
        button: MouseButton::Right,
        modifiers: Default::default(),
        click_count: 1,
    });
    draw(cx);
    assert_eq!(view.read_with(cx, |p, _| p.clicks), 0);
}

#[test]
fn cover_rejects_nonfinite_and_contradictory_geometry() {
    assert!(std::panic::catch_unwind(|| YueCardCover::new(div()).aspect_ratio(f32::NAN)).is_err());
    assert!(std::panic::catch_unwind(|| YueCardCover::new(div()).height(px(-1.))).is_err());
    assert!(
        std::panic::catch_unwind(|| YueCardCover::new(div()).side_width(gpui_kit::relative(1.)))
            .is_err()
    );
}

#[gpui_kit::test]
fn cover_media_itself_has_correct_outer_corners(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    for position in [
        CardCoverPlacement::BeforeHeader,
        CardCoverPlacement::AfterHeader,
        CardCoverPlacement::Left,
        CardCoverPlacement::Right,
    ] {
        view.update(cx, |p, cx| {
            p.cover = Some(position);
            cx.notify();
        });
        draw(cx);
        cx.update(|w, _| {
            let color: gpui_kit::Hsla = gpui_kit::rgb(0x778899).into();
            let quad = w
                .painted_quads()
                .into_iter()
                .find(|q| q.background.as_solid() == Some(color))
                .expect("actual media surface");
            let corners = quad.corner_radii;
            let zero = gpui_kit::ScaledPixels(0.);
            match position {
                CardCoverPlacement::BeforeHeader => {
                    assert!(corners.top_left > zero && corners.top_right > zero);
                    assert_eq!(corners.bottom_left, zero);
                }
                CardCoverPlacement::AfterHeader => {
                    assert_eq!(corners.top_left, zero);
                    assert_eq!(corners.bottom_right, zero);
                }
                CardCoverPlacement::Left => {
                    assert!(corners.top_left > zero && corners.bottom_left > zero);
                    assert_eq!(corners.top_right, zero);
                }
                CardCoverPlacement::Right => {
                    assert!(corners.top_right > zero && corners.bottom_right > zero);
                    assert_eq!(corners.top_left, zero);
                }
            }
        });
    }
}

#[gpui_kit::test]
fn real_text_selection_survives_drag_without_primary_activation(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    view.update(cx, |p, cx| {
        p.primary = true;
        p.selectable = true;
        cx.notify();
    });
    draw(cx);
    let bounds = part(cx, "body").bounds();
    let start = bounds.origin + point(px(18.), px(22.));
    let end = start + point(px(110.), px(0.));
    cx.simulate_event(MouseDownEvent {
        position: start,
        button: MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
        first_mouse: false,
    });
    cx.simulate_mouse_move(end, Some(MouseButton::Left), Default::default());
    cx.simulate_event(MouseUpEvent {
        position: end,
        button: MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
    });
    draw(cx);
    assert!(
        cx.update(gpui_kit::base::TextSelection::has_selection),
        "the test must create a real selection"
    );
    assert_eq!(view.read_with(cx, |p, _| p.clicks), 0);
}

#[gpui_kit::test]
fn reduced_motion_settles_lift_and_body_only_preserves_full_inset(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    view.update(cx, |p, cx| {
        p.header = false;
        p.footer = false;
        cx.notify();
    });
    draw(cx);
    let card = get(cx, "card").bounds();
    let text = get(cx, "body-text").bounds();
    assert!((text.top() - card.top() - px(17.)).abs() < px(0.2));
    cx.update(|_, cx| cx.set_reduce_motion(true));
    view.update(cx, |p, cx| {
        p.lift = true;
        cx.notify();
    });
    draw(cx);
    let initial = get(cx, "card").bounds();
    cx.simulate_mouse_move(initial.center(), None, Default::default());
    draw(cx);
    let lifted = get(cx, "card").bounds();
    assert_eq!(lifted.origin, initial.origin);
    assert_eq!(lifted.size, initial.size);
    cx.simulate_mouse_move(point(px(0.), px(0.)), None, Default::default());
    draw(cx);
    assert_eq!(get(cx, "card").bounds(), initial);
}

#[gpui_kit::test]
fn adjacent_regions_share_one_divider_and_empty_actions_take_no_space(cx: &mut TestAppContext) {
    struct Boundaries;
    impl Render for Boundaries {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(px(320.))
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    YueCard::new("boundary")
                        .header(YueCardHeader::new("标题").divided(true))
                        .footer(YueCardFooter::new("页脚")),
                )
                .child(
                    YueCard::new("empty-actions")
                        .child("正文")
                        .footer(YueCardFooter::actions([])),
                )
                .child(
                    YueCard::new("cover-gap")
                        .cover(YueCardCover::new(div()).height(px(40.)))
                        .header(YueCardHeader::new("标题"))
                        .child(div().id("gap-text").test_support().child("正文")),
                )
        }
    }
    cx.update(|cx| crate::init(cx, crate::YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap());
    let (_, cx) = cx.add_window_view(|w, cx| {
        let view = cx.new(|_| Boundaries);
        Root::new(view, w, cx)
    });
    draw(cx);
    let card = get(cx, "boundary").bounds();
    let header = get(cx, (ElementId::from("boundary"), "header")).bounds();
    let footer = get(cx, (ElementId::from("boundary"), "footer")).bounds();
    assert!(
        (card.size.height - header.size.height - footer.size.height - px(3.)).abs() < px(0.1),
        "one divider plus two outer borders"
    );
    assert!(
        !cx.update(|w, _| snapshots(w))
            .iter()
            .any(|n| n.path().last()
                == Some(&ElementId::from((
                    ElementId::from("empty-actions"),
                    "footer"
                ))))
    );
    let header = get(cx, (ElementId::from("cover-gap"), "header")).bounds();
    assert!((get(cx, "gap-text").bounds().top() - header.bottom() - px(12.)).abs() < px(0.1));
}
