use super::*;
use crate::{Accent, ThemeMode, YueButton, YueInput};
use gpui_kit::base::{
    Root, TestSupportExt as _,
    input::InputState,
    test_support::{ElementSnapshot, snapshots},
};
use gpui_kit::{
    AppContext as _, Context, Entity, Focusable, InteractiveElement as _, IntoElement, Modifiers,
    ParentElement as _, Render, Role, Styled as _, TestAppContext, VisualTestContext, Window, div,
    px, size,
};

#[test]
fn packing_preserves_order_and_original_spans() {
    use DescriptionsSpan::{Columns as C, Filled as F};
    let spans = [C(1), C(1), C(2), F, C(99)];
    let plain = model::pack(spans, 3, false);
    assert_eq!(
        plain
            .iter()
            .map(|c| (c.row, c.column, c.span))
            .collect::<Vec<_>>(),
        [(0, 0, 1), (0, 1, 1), (1, 0, 2), (1, 2, 1), (2, 0, 3)]
    );
    let bordered = model::pack(spans, 3, true);
    assert_eq!(bordered[1].span, 2);
    assert_eq!(
        model::pack(spans, 1, true)
            .iter()
            .map(|c| c.span)
            .collect::<Vec<_>>(),
        [1; 5]
    );
    assert_eq!(model::pack(spans, 3, false), plain);
    assert!(model::pack([], 3, true).is_empty());
}
#[test]
fn columns_threshold_zero_and_huge_width() {
    for (w, c) in [
        (0., 1),
        (439., 1),
        (440., 2),
        (659., 2),
        (660., 3),
        (f32::MAX, 4),
    ] {
        assert_eq!(model::columns(w, 220., 0., 4), c);
    }
    assert_eq!(model::columns(464., 220., 24., 4), 2);
    assert_eq!(model::columns(463., 220., 24., 4), 1);
}
#[test]
fn invalid_configuration_is_explicit_and_last_value_wins() {
    assert!(std::panic::catch_unwind(|| YueDescriptions::new("d").columns(0)).is_err());
    assert!(std::panic::catch_unwind(|| YueDescriptionItem::new(" ", "Name")).is_err());
    assert!(std::panic::catch_unwind(|| YueDescriptionItem::new("a", " ")).is_err());
    assert!(
        std::panic::catch_unwind(
            || YueDescriptionItem::new("a", "A").span(DescriptionsSpan::Columns(0))
        )
        .is_err()
    );
    assert!(
        std::panic::catch_unwind(
            || YueDescriptionItem::new("a", "A").text_overflow(DescriptionsTextOverflow::Lines(0))
        )
        .is_err()
    );
    assert!(
        std::panic::catch_unwind(|| YueDescriptions::new("d").min_item_width(px(f32::NAN)))
            .is_err()
    );
    assert!(std::panic::catch_unwind(|| YueDescriptions::new("d").label_width(px(-1.))).is_err());
    assert!(
        std::panic::catch_unwind(|| YueDescriptions::new("d").items([
            YueDescriptionItem::new("a", "A"),
            YueDescriptionItem::new("a", "B")
        ]))
        .is_err()
    );
    assert!(
        std::panic::catch_unwind(|| YueDescriptionItem::new("a", "A")
            .content(div())
            .copyable(true)
            .validate())
        .is_err()
    );
    assert!(
        std::panic::catch_unwind(|| YueDescriptionItem::new("a", "A")
            .content(div())
            .text_overflow(DescriptionsTextOverflow::Ellipsis)
            .validate())
        .is_err()
    );
    YueDescriptionItem::new("a", "A")
        .content(div())
        .text("")
        .text_overflow(DescriptionsTextOverflow::Ellipsis)
        .validate();
    YueDescriptionItem::new("a", "A")
        .content(div())
        .copy_text("")
        .validate();
}
struct Probe {
    width: f32,
    long: bool,
    vertical: DescriptionsVerticalAlign,
    horizontal: DescriptionsAlign,
    layout: DescriptionsLayout,
    size: DescriptionsSize,
    theme: YueTheme,
    bordered: bool,
    overflow: DescriptionsTextOverflow,
    copy: bool,
    visible: bool,
    payload: SharedString,
    input: Entity<InputState>,
    slot: bool,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut root = div()
            .size_full()
            .flex()
            .flex_col()
            .items_start()
            .child(YueButton::new("before", "Before"));
        if self.visible {
            root = root.child(
                YueDescriptions::new("details")
                    .theme(self.theme)
                    .width(px(self.width))
                    .columns(3)
                    .min_item_width(px(200.))
                    .label_width(px(80.))
                    .bordered(self.bordered)
                    .size(self.size)
                    .layout(self.layout)
                    .label_align(self.horizontal)
                    .value_align(self.horizontal)
                    .vertical_align(self.vertical)
                    .items([
                        YueDescriptionItem::new("number", "编号")
                            .text(if self.long {
                                self.payload.clone()
                            } else {
                                "42".into()
                            })
                            .copyable(self.copy)
                            .text_overflow(self.overflow),
                        if self.slot {
                            YueDescriptionItem::new("state", "状态")
                                .content(div().id("slot").test_support().w(px(32.)).h(px(20.)))
                        } else {
                            YueDescriptionItem::new("state", "状态").text("进行中")
                        },
                        YueDescriptionItem::new("owner", "负责人").content(
                            YueInput::new("editor", self.theme, &self.input)
                                .accessibility_label("负责人"),
                        ),
                    ]),
            );
        }
        root.child(YueButton::new("after", "After"))
    }
}
fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| {
        w.refresh();
        w.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}
fn open(cx: &mut TestAppContext) -> (Entity<Probe>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
        cx.set_reduce_motion(true);
    });
    let mut saved = None;
    let (_, cx) = cx.add_window_view(|w, cx| {
        let input = cx.new(|cx| InputState::new(w, cx).default_value("Alex"));
        let p = cx.new(|_| Probe {
            width: 720.,
            long: true,
            vertical: Default::default(),
            horizontal: Default::default(),
            layout: Default::default(),
            size: Default::default(),
            theme: YueTheme::new(ThemeMode::Light, Accent::Azure),
            bordered: true,
            overflow: Default::default(),
            copy: false,
            visible: true,
            payload: "First line\nSecond line\nThird line".into(),
            input,
            slot: false,
        });
        saved = Some(p.clone());
        Root::new(p, w, cx)
    });
    cx.simulate_resize(size(px(1200.), px(1000.)));
    cx.update(|w, _| w.activate_window());
    draw(cx);
    (saved.unwrap(), cx)
}
fn edit(p: &Entity<Probe>, cx: &mut VisualTestContext, f: impl FnOnce(&mut Probe)) {
    p.update(cx, |p, cx| {
        f(p);
        cx.notify();
    });
    draw(cx);
}
fn nodes(cx: &mut VisualTestContext) -> Vec<ElementSnapshot> {
    cx.update(|w, _| snapshots(w))
}
fn node(cx: &mut VisualTestContext, item: &str, id: &str) -> ElementSnapshot {
    nodes(cx)
        .into_iter()
        .find(|n| {
            n.path().contains(&ElementId::from(item.to_owned()))
                && n.path().last() == Some(&ElementId::from(id.to_owned()))
        })
        .unwrap_or_else(|| panic!("Missing {item}/{id}"))
}
fn near(a: f32, b: f32) {
    assert!((a - b).abs() <= 1.1, "{a} != {b}");
}
#[gpui_kit::test]
fn multiline_row_stretches_cells_and_centers_actual_content(cx: &mut TestAppContext) {
    let (p, cx) = open(cx);
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        for accent in [Accent::Azure, Accent::Neutral] {
            for density in [
                DescriptionsSize::Small,
                DescriptionsSize::Medium,
                DescriptionsSize::Large,
            ] {
                for rem in [12., 16., 24.] {
                    cx.update(|w, _| w.set_rem_size(px(rem)));
                    edit(&p, cx, |p| {
                        p.theme = YueTheme::new(mode, accent);
                        p.size = density;
                        p.horizontal = DescriptionsAlign::Center;
                    });
                    let long = node(cx, "number", "description-value").bounds();
                    let short = node(cx, "state", "description-value").bounds();
                    near(long.top().into(), short.top().into());
                    near(long.bottom().into(), short.bottom().into());
                    let text = node(cx, "state", "description-value-content").bounds();
                    near(text.center().y.into(), short.center().y.into());
                    let label = node(cx, "state", "description-label").bounds();
                    let label_text = node(cx, "state", "description-label-content").bounds();
                    near(label_text.center().y.into(), label.center().y.into());
                    near(label.top().into(), short.top().into());
                    near(label.bottom().into(), short.bottom().into());
                }
            }
        }
    }
    edit(&p, cx, |p| p.long = false);
    let small = node(cx, "state", "description-value").bounds().size.height;
    edit(&p, cx, |p| p.long = true);
    assert!(node(cx, "state", "description-value").bounds().size.height > small);
}
#[gpui_kit::test]
fn top_bottom_and_stacked_value_alignment(cx: &mut TestAppContext) {
    let (p, cx) = open(cx);
    for layout in [DescriptionsLayout::Horizontal, DescriptionsLayout::Vertical] {
        for v in [
            DescriptionsVerticalAlign::Top,
            DescriptionsVerticalAlign::Center,
            DescriptionsVerticalAlign::Bottom,
        ] {
            edit(&p, cx, |p| {
                p.layout = layout;
                p.vertical = v;
            });
            let b = node(cx, "state", "description-value").bounds();
            let c = node(cx, "state", "description-value-content").bounds();
            let pad = 12.;
            match v {
                DescriptionsVerticalAlign::Top => near(c.top().into(), f32::from(b.top()) + pad),
                DescriptionsVerticalAlign::Center => near(c.center().y.into(), b.center().y.into()),
                DescriptionsVerticalAlign::Bottom => {
                    near(c.bottom().into(), f32::from(b.bottom()) - pad)
                }
            }
        }
    }
}
#[gpui_kit::test]
fn reflow_keeps_real_editor_identity_text_and_focus(cx: &mut TestAppContext) {
    let (p, cx) = open(cx);
    let input = p.read_with(cx, |p, _| p.input.clone());
    cx.update(|w, cx| input.read(cx).focus_handle(cx).focus(w, cx));
    cx.simulate_input(" kept");
    draw(cx);
    let initial = input.read_with(cx, |i, _| i.value());
    let path = node(cx, "owner", "editor").path().to_vec();
    for width in [430., 160., 0., 720.] {
        edit(&p, cx, |p| p.width = width);
        assert_eq!(node(cx, "owner", "editor").path(), path.as_slice());
        assert_eq!(input.read_with(cx, |i, _| i.value()), initial);
        assert!(cx.update(|w, cx| input.read(cx).focus_handle(cx).is_focused(w)));
    }
    assert_eq!(
        nodes(cx)
            .iter()
            .filter(|n| n.role() == Some(Role::DescriptionList))
            .count(),
        1
    );
    assert_eq!(
        nodes(cx)
            .iter()
            .filter(|n| n.role() == Some(Role::Term))
            .count(),
        3
    );
    assert_eq!(
        nodes(cx)
            .iter()
            .filter(|n| n.role() == Some(Role::Definition))
            .count(),
        3
    );
}
#[gpui_kit::test]
fn copy_full_payload_feedback_revision_and_removal(cx: &mut TestAppContext) {
    let (p, cx) = open(cx);
    edit(&p, cx, |p| {
        p.copy = true;
        p.overflow = DescriptionsTextOverflow::Ellipsis;
    });
    let b = node(cx, "number", "description-copy").bounds();
    cx.simulate_click(b.center(), Modifiers::default());
    draw(cx);
    assert_eq!(
        cx.update(|_, cx| cx.read_from_clipboard().unwrap().text().unwrap()),
        "First line\nSecond line\nThird line"
    );
    assert_eq!(
        node(cx, "number", "description-copy").label(),
        Some("已复制")
    );
    edit(&p, cx, |p| p.payload = "New record".into());
    assert_eq!(
        node(cx, "number", "description-copy").label(),
        Some("复制完整内容")
    );
    let position = node(cx, "number", "description-copy").bounds().center();
    cx.simulate_click(position, Modifiers::default());
    draw(cx);
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(1300));
    draw(cx);
    assert_eq!(
        node(cx, "number", "description-copy").label(),
        Some("复制完整内容")
    );
    edit(&p, cx, |p| p.visible = false);
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(2));
    draw(cx);
}
#[gpui_kit::test]
fn continuous_identifiers_and_copy_stay_inside_value_bounds(cx: &mut TestAppContext) {
    let (p, cx) = open(cx);
    edit(&p, cx, |p| {
        p.payload = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".repeat(6).into();
        p.copy = true;
    });
    for width in [720., 480., 180.] {
        edit(&p, cx, |p| p.width = width);
        let b = node(cx, "number", "description-value").bounds();
        let text = node(cx, "number", "description-text").bounds();
        let button = node(cx, "number", "description-copy").bounds();
        assert!(text.right() <= button.left());
        assert!(button.right() <= b.right());
        assert!(text.left() >= b.left());
        assert!(text.size.height > px(24.));
    }
}

#[gpui_kit::test]
fn slot_aligns_in_both_axes_and_stable_page_stops_frames(cx: &mut TestAppContext) {
    let (p, cx) = open(cx);
    edit(&p, cx, |p| p.slot = true);
    for h in [
        DescriptionsAlign::Left,
        DescriptionsAlign::Center,
        DescriptionsAlign::Right,
    ] {
        edit(&p, cx, |p| p.horizontal = h);
        let cell = node(cx, "state", "description-value").bounds();
        let slot = node(cx, "state", "slot").bounds();
        near(cell.center().y.into(), slot.center().y.into());
        match h {
            DescriptionsAlign::Left => near(slot.left().into(), f32::from(cell.left()) + 16.),
            DescriptionsAlign::Center => near(slot.center().x.into(), cell.center().x.into()),
            DescriptionsAlign::Right => near(slot.right().into(), f32::from(cell.right()) - 16.),
        }
    }
    cx.update(|w, cx| {
        w.simulate_next_frame(cx);
    });
    draw(cx);
    assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
}
#[gpui_kit::test]
fn copy_keyboard_repetition_and_focus_removal_are_scoped(cx: &mut TestAppContext) {
    let (p, cx) = open(cx);
    edit(&p, cx, |p| p.copy = true);
    let b = node(cx, "number", "description-copy").bounds();
    cx.simulate_click(b.center(), Modifiers::default());
    draw(cx);
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(800));
    draw(cx);
    let key = gpui_kit::Keystroke::parse("enter").unwrap();
    cx.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: key.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui_kit::KeyUpEvent { keystroke: key });
    draw(cx);
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(500));
    draw(cx);
    assert_eq!(
        node(cx, "number", "description-copy").label(),
        Some("已复制")
    );
    cx.update(|_, cx| {
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("sentinel".into()))
    });
    let key = gpui_kit::Keystroke::parse("space").unwrap();
    cx.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: key.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui_kit::KeyUpEvent { keystroke: key });
    draw(cx);
    assert_eq!(
        cx.update(|_, cx| cx.read_from_clipboard().unwrap().text().unwrap()),
        "First line\nSecond line\nThird line"
    );
    edit(&p, cx, |p| p.copy = false);
    assert!(
        !nodes(cx)
            .iter()
            .any(|n| n.path().last() == Some(&ElementId::from("description-copy")))
    );
    // A later update must not steal real editor focus.
    let input = p.read_with(cx, |p, _| p.input.clone());
    cx.update(|w, cx| input.read(cx).focus_handle(cx).focus(w, cx));
    edit(&p, cx, |p| p.payload = "Replacement".into());
    assert!(cx.update(|w, cx| input.read(cx).focus_handle(cx).is_focused(w)));
}

#[gpui_kit::test]
fn actual_container_threshold_narrow_fallback_and_restore(cx: &mut TestAppContext) {
    let (p, cx) = open(cx);
    edit(&p, cx, |p| p.width = 604.);
    near(
        node(cx, "number", "description-value")
            .bounds()
            .top()
            .into(),
        node(cx, "owner", "description-value").bounds().top().into(),
    );
    edit(&p, cx, |p| p.width = 603.);
    assert!(
        node(cx, "owner", "description-value").bounds().top()
            > node(cx, "number", "description-value").bounds().top()
    );
    edit(&p, cx, |p| p.width = 160.);
    let l = node(cx, "number", "description-label").bounds();
    let v = node(cx, "number", "description-value").bounds();
    assert!(v.top() >= l.bottom());
    near(l.left().into(), v.left().into());
    near(l.right().into(), v.right().into());
    edit(&p, cx, |p| p.width = 720.);
    let l = node(cx, "number", "description-label").bounds();
    let v = node(cx, "number", "description-value").bounds();
    near(l.right().into(), v.left().into());
    near(l.top().into(), v.top().into());
}

struct SpansProbe;
impl Render for SpansProbe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut root = div().size_full().flex().flex_col().items_start();
        for bordered in [false, true] {
            root = root.child(
                YueDescriptions::new(if bordered {
                    "bordered-spans"
                } else {
                    "plain-spans"
                })
                .width(px(720.))
                .columns(3)
                .min_item_width(px(200.))
                .bordered(bordered)
                .items([
                    YueDescriptionItem::new("a", "A").text("A"),
                    YueDescriptionItem::new("b", "B").text("B"),
                    YueDescriptionItem::new("c", "C")
                        .text("C")
                        .span(DescriptionsSpan::Columns(2)),
                    YueDescriptionItem::new("d", "D")
                        .text("D")
                        .span(DescriptionsSpan::Filled),
                    YueDescriptionItem::new("e", "E")
                        .text("E")
                        .span(DescriptionsSpan::Columns(99)),
                ]),
            );
        }
        root.child(YueDescriptions::new("empty").bordered(true))
    }
}
#[gpui_kit::test]
fn native_span_packing_matches_plan_and_borders_have_one_owner(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap();
    });
    let (_, cx) = cx.add_window_view(|w, cx| {
        let p = cx.new(|_| SpansProbe);
        Root::new(p, w, cx)
    });
    draw(cx);
    for (scope, gap) in [("bordered-spans", 1.), ("plain-spans", 24.)] {
        let all = nodes(cx);
        let bounds = |id: &str| {
            all.iter()
                .find(|n| {
                    n.path().contains(&ElementId::from(scope.to_owned()))
                        && n.path().last() == Some(&ElementId::from(id.to_owned()))
                })
                .unwrap()
                .bounds()
        };
        let (a, b, c, d, e) = (
            bounds("a"),
            bounds("b"),
            bounds("c"),
            bounds("d"),
            bounds("e"),
        );
        near(f32::from(b.left() - a.right()), gap);
        near(f32::from(d.left() - c.right()), gap);
        near(a.top().into(), b.top().into());
        near(c.top().into(), d.top().into());
        assert!(c.top() > a.top());
        assert!(e.top() > c.top());
        near(c.size.width.into(), f32::from(a.size.width) * 2. + gap);
        if scope == "bordered-spans" {
            near(b.size.width.into(), c.size.width.into());
            near(f32::from(c.top() - a.bottom()), gap);
        } else {
            near(a.size.width.into(), b.size.width.into());
        }
        near(e.size.width.into(), f32::from(a.size.width) * 3. + gap * 2.);
    }
    assert_eq!(node(cx, "empty", "empty").bounds().size.height, px(0.));
}

// Inspect actual paint, not only boxes: transparent cells must not erase the
// host surface, while composed avatars retain their own circular surface.
struct TransparentProbe(YueTheme, bool);
impl Render for TransparentProbe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(720.)).child(
            crate::YueCard::new("transparent-card").theme(self.0).child(
                YueDescriptions::new("transparent-details")
                    .theme(self.0)
                    .bordered(self.1)
                    .columns(2)
                    .items([
                        YueDescriptionItem::new("avatar-item", "负责人")
                            .content(crate::YueAvatar::new("avatar", self.0).name("林悦")),
                        YueDescriptionItem::new("text-item", "说明").text("First\nSecond"),
                        YueDescriptionItem::new("last-item", "编号").text("42"),
                    ]),
            ),
        )
    }
}
#[gpui_kit::test]
fn transparent_cells_preserve_avatar_surface_and_single_separators(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap());
    let mut saved = None;
    let (_, cx) = cx.add_window_view(|w, cx| {
        let p = cx.new(|_| TransparentProbe(YueTheme::new(ThemeMode::Light, Accent::Azure), true));
        saved = Some(p.clone());
        Root::new(p, w, cx)
    });
    cx.simulate_resize(size(px(1000.), px(1000.)));
    let p = saved.unwrap();
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        for accent in [Accent::Azure, Accent::Neutral] {
            for bordered in [true, false] {
                let theme = YueTheme::new(mode, accent);
                p.update(cx, |p, cx| {
                    p.0 = theme;
                    p.1 = bordered;
                    cx.notify();
                });
                draw(cx);
                cx.update(|w, _| {
                    let quads = w.painted_quads();
                    let scale = w.scale_factor();
                    let nodes = snapshots(w);
                    for n in nodes.iter().filter(|n| {
                        [
                            "descriptions-grid",
                            "description-label",
                            "description-value",
                        ]
                        .iter()
                        .any(|id| n.path().last() == Some(&ElementId::from(*id)))
                    }) {
                        let b = n.bounds();
                        assert!(
                            !quads.iter().any(|q| {
                                (q.bounds.origin.x.0 - f32::from(b.left()) * scale).abs() < 0.1
                                    && (q.bounds.origin.y.0 - f32::from(b.top()) * scale).abs()
                                        < 0.1
                                    && (q.bounds.size.width.0 - f32::from(b.size.width) * scale)
                                        .abs()
                                        < 0.1
                                    && (q.bounds.size.height.0 - f32::from(b.size.height) * scale)
                                        .abs()
                                        < 0.1
                                    && q.background.as_solid().is_some_and(|c| c.a > 0.)
                            }),
                            "cell must not paint a background: {:?}",
                            n.path()
                        );
                    }
                    let avatar = nodes
                        .iter()
                        .find(|n| n.role() == Some(Role::Image))
                        .unwrap()
                        .bounds();
                    let color =
                        gpui_kit::Hsla::from(crate::tokens::rgba(theme, "--avatar-background"));
                    let host =
                        gpui_kit::Hsla::from(crate::tokens::rgba(theme, "--card-background"));
                    assert_ne!(color, host, "avatar must remain distinct on its host");
                    assert!(quads.iter().any(|q| q.background.as_solid() == Some(color)
                        && (q.bounds.origin.x.0 - f32::from(avatar.left()) * scale).abs() < 0.1
                        && (q.bounds.origin.y.0 - f32::from(avatar.top()) * scale).abs() < 0.1
                        && q.corner_radii.top_left.0 > 0.));
                    let lines = nodes
                        .iter()
                        .filter(|n| {
                            n.path().last() == Some(&ElementId::from("description-separator-right"))
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(lines.len(), usize::from(bordered));
                    for line in lines {
                        near(f32::from(line.bounds().size.width), 1.);
                    }
                });
            }
        }
    }
}
