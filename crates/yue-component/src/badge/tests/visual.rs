use super::*;
use crate::{Accent, ThemeMode, YueButton};
use gpui_kit::base::{
    Root, TestSupportExt as _,
    test_support::{ElementSnapshot, snapshots},
};
use gpui_kit::{
    AppContext as _, Context, Entity, InteractiveElement as _, ParentElement as _, Render, Role,
    Styled as _, TestAppContext, VisualTestContext, Window, div, px,
};

struct Probe {
    count: u64,
    visible: bool,
    dot: bool,
    text: bool,
    animated: bool,
    disabled: bool,
    semantics: bool,
    clicks: usize,
    theme: YueTheme,
    size: BadgeSize,
    shape: BadgeShape,
    offset: Point<Rems>,
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let badge = |id| {
            let mut badge = YueBadge::new(id)
                .theme(self.theme)
                .count(self.count)
                .visible(self.visible)
                .animated(self.animated)
                .semantics(self.semantics)
                .size(self.size)
                .shape(self.shape)
                .offset(self.offset);
            if self.dot {
                badge = badge.dot();
            }
            if self.text {
                badge = badge.text("A very long label 中文 🐈");
            }
            badge
        };
        div()
            .flex()
            .items_start()
            .p_8()
            .gap_8()
            .child(
                badge("attached").child(
                    YueButton::new("host", "通知")
                        .disabled(self.disabled)
                        .accessibility_description(format!("{} 条未读消息", self.count))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.clicks += 1;
                            cx.notify();
                        })),
                ),
            )
            .child(div().id("neighbor").test_support().size(px(24.)))
            .child(badge("standalone"))
            .child(
                YueBadge::new("status")
                    .theme(self.theme)
                    .dot()
                    .tone(BadgeTone::Success)
                    .label("已同步"),
            )
    }
}

fn open(cx: &mut TestAppContext) -> (Entity<Probe>, &mut VisualTestContext) {
    let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
    cx.update(|cx| crate::init(cx, theme).unwrap());
    let mut saved = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| Probe {
            count: 8,
            visible: true,
            dot: false,
            text: false,
            animated: false,
            disabled: false,
            semantics: true,
            clicks: 0,
            theme,
            size: BadgeSize::Medium,
            shape: BadgeShape::Pill,
            offset: point(rems(0.), rems(0.)),
        });
        saved = Some(view.clone());
        Root::new(view, window, cx)
    });
    draw(cx);
    (saved.unwrap(), cx)
}

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
}

fn node(window: &Window, owner: &str, part: &str) -> ElementSnapshot {
    let id = ElementId::from((ElementId::from(owner.to_string()), part.to_string()));
    snapshots(window)
        .into_iter()
        .find(|n| n.path().last() == Some(&id))
        .unwrap_or_else(|| panic!("missing {owner}/{part}"))
}

#[gpui_kit::test]
fn attached_content_changes_preserve_host_and_neighbors_and_have_real_names(
    cx: &mut TestAppContext,
) {
    let (view, cx) = open(cx);
    let (host, neighbor) = cx.update(|w, _| {
        (
            observed(w, "host").bounds(),
            observed(w, "neighbor").bounds(),
        )
    });
    for (count, dot, text, visible) in [
        (128, false, false, true),
        (0, false, false, true),
        (8, true, false, true),
        (8, false, true, true),
        (8, false, false, false),
    ] {
        view.update(cx, |p, cx| {
            p.count = count;
            p.dot = dot;
            p.text = text;
            p.visible = visible;
            cx.notify();
        });
        draw(cx);
        cx.update(|w, _| {
            assert_eq!(observed(w, "host").bounds(), host);
            assert_eq!(observed(w, "neighbor").bounds(), neighbor);
            let shown = visible && (count > 0 || dot || text);
            if shown {
                let badge = node(w, "attached", "badge");
                assert_eq!(badge.role(), Some(Role::Caption));
                let mark = node(w, "attached", "mark").bounds();
                assert!((mark.center().x - host.right()).abs() < px(1.));
                assert!((mark.center().y - host.top()).abs() < px(1.));
                if count == 128 {
                    assert_eq!(badge.label(), Some("128"));
                }
                if text {
                    assert!(mark.size.width <= px(64.));
                }
                assert!(node(w, "standalone", "mark").bounds().size.height > px(0.));
            } else {
                assert!(!snapshots(w).iter().any(|n| n.path().last()
                    == Some(&ElementId::from((ElementId::from("attached"), "badge")))));
            }
            assert_eq!(observed(w, "host").role(), Some(Role::Button));
            assert_eq!(node(w, "status", "badge").label(), Some("已同步"));
        });
    }
}

fn observed(window: &Window, id: &str) -> ElementSnapshot {
    gpui_kit::base::test_support::find(window, &[], &ElementId::from(id.to_owned())).unwrap()
}

#[gpui_kit::test]
fn overlapped_clicks_and_keyboard_keep_the_hosts_command_and_focus(cx: &mut TestAppContext) {
    use gpui_kit::Modifiers;
    let (view, cx) = open(cx);
    let bounds = cx.update(|w, _| observed(w, "host").bounds());
    cx.simulate_click(
        point(bounds.right() - px(2.), bounds.top() + px(2.)),
        Modifiers::default(),
    );
    draw(cx);
    assert_eq!(
        view.read_with(cx, |p, _| p.clicks),
        1,
        "badge overlap must not block the host"
    );
    cx.simulate_click(
        point(bounds.right() + px(2.), bounds.top() - px(2.)),
        Modifiers::default(),
    );
    draw(cx);
    assert_eq!(
        view.read_with(cx, |p, _| p.clicks),
        1,
        "overhang must not expand the host hit area"
    );
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|w, _| {
        assert_eq!(observed(w, "host").focused(), Some(true));
        assert_ne!(node(w, "attached", "badge").focused(), Some(true));
    });
    press(cx, "space");
    draw(cx);
    assert_eq!(view.read_with(cx, |p, _| p.clicks), 2);
    view.update(cx, |p, cx| {
        p.disabled = true;
        p.semantics = false;
        cx.notify();
    });
    draw(cx);
    cx.simulate_click(
        point(bounds.right() - px(2.), bounds.top() + px(2.)),
        Modifiers::default(),
    );
    press(cx, "space");
    draw(cx);
    assert_eq!(view.read_with(cx, |p, _| p.clicks), 2);
    cx.update(|w, _| {
        assert_eq!(node(w, "attached", "badge").role(), None);
        assert_eq!(observed(w, "host").role(), Some(Role::Button));
        assert_eq!(observed(w, "host").description(), Some("8 条未读消息"));
    });
}

#[gpui_kit::test]
fn density_shape_offset_and_theme_are_applied_to_painted_marks(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        for density in [BadgeSize::Small, BadgeSize::Medium] {
            for shape in [BadgeShape::Pill, BadgeShape::Rounded] {
                for rem in [16., 24.] {
                    let theme = YueTheme::new(mode, Accent::Azure);
                    view.update(cx, |p, cx| {
                        p.theme = theme;
                        p.size = density;
                        p.shape = shape;
                        p.offset = point(rems(-0.25), rems(0.25));
                        cx.notify();
                    });
                    cx.update(|w, _| w.set_rem_size(px(rem)));
                    draw(cx);
                    cx.update(|w, _| {
                        let mark = node(w, "attached", "mark").bounds();
                        let host = observed(w, "host").bounds();
                        let height = if density == BadgeSize::Small {
                            16.
                        } else {
                            20.
                        } * rem
                            / 16.;
                        assert!((mark.size.height - px(height)).abs() < px(1.));
                        assert!((mark.center().x - host.right() + px(rem * 0.25)).abs() < px(1.));
                        assert!((mark.center().y - host.top() - px(rem * 0.25)).abs() < px(1.));
                        let background =
                            crate::tokens::rgba(theme, crate::theme::keys::BADGE_DANGER_BACKGROUND);
                        let quads = w.painted_quads();
                        let q = quads
                            .iter()
                            .find(|q| {
                                q.bounds == mark.scale(w.scale_factor())
                                    && q.background.as_solid() == Some(background.into())
                            })
                            .expect("actual badge background must be painted");
                        let radius = q.corner_radii.top_left.0 / w.scale_factor();
                        let expected = if shape == BadgeShape::Pill {
                            height / 2.
                        } else {
                            2. * rem / 16.
                        };
                        assert!((radius - expected).abs() < 1.);
                    });
                }
            }
        }
    }
}

fn settle(cx: &mut VisualTestContext, ms: u64) {
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(ms));
    draw(cx);
}

fn press(cx: &mut VisualTestContext, key: &str) {
    let keystroke = gpui_kit::Keystroke::parse(key).unwrap();
    cx.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui_kit::KeyUpEvent { keystroke });
}

#[gpui_kit::test]
fn optional_presence_hides_old_semantics_and_reverses_without_a_new_mount(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    view.update(cx, |p, cx| {
        p.animated = true;
        cx.notify();
    });
    draw(cx);
    cx.update(|w, _| assert!(node(w, "attached", "badge").visible()));
    view.update(cx, |p, cx| {
        p.visible = false;
        cx.notify();
    });
    draw(cx);
    cx.update(|w, _| {
        assert_eq!(node(w, "attached", "badge").role(), None);
        assert_eq!(observed(w, "host").role(), Some(Role::Button));
    });
    settle(cx, 40);
    let fading = cx.update(|w, _| mark_alpha(w, "attached"));
    assert!(
        fading > 0. && fading < 1.,
        "visibility transition must change the painted opacity"
    );
    view.update(cx, |p, cx| {
        p.visible = true;
        p.count = 128;
        cx.notify();
    });
    draw(cx);
    cx.update(|w, _| {
        assert_eq!(node(w, "attached", "badge").label(), Some("128"));
        assert!(
            (mark_alpha(w, "attached") - fading).abs() < 0.01,
            "reversal must continue at the current opacity"
        );
    });
    settle(cx, 250);
    view.update(cx, |p, cx| {
        p.visible = false;
        cx.notify();
    });
    draw(cx);
    settle(cx, 150);
    cx.update(|w, _| {
        assert!(
            !snapshots(w).iter().any(|n| n.path().last()
                == Some(&ElementId::from((ElementId::from("attached"), "badge"))))
        );
        assert_eq!(observed(w, "host").role(), Some(Role::Button));
    });
    cx.update(|_, cx| cx.set_reduce_motion(true));
    view.update(cx, |p, cx| {
        p.visible = true;
        cx.notify();
    });
    draw(cx);
    cx.update(|w, _| assert_eq!(node(w, "attached", "badge").label(), Some("128")));
    view.update(cx, |p, cx| {
        p.visible = false;
        cx.notify();
    });
    draw(cx);
    cx.update(|w, _| {
        assert!(
            !snapshots(w)
                .iter()
                .any(|n| n.role() == Some(Role::Caption) && n.label() == Some("128"))
        )
    });
}

struct ClipProbe;

impl Render for ClipProbe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().p_8().child(
            div()
                .id("clip")
                .test_support()
                .size(px(40.))
                .overflow_hidden()
                .child(YueBadge::new("clipped").count(8).child(div().size(px(40.)))),
        )
    }
}

#[gpui_kit::test]
fn attached_marks_follow_parent_clip_and_separator_scales_with_display(cx: &mut TestAppContext) {
    let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
    cx.update(|cx| crate::init(cx, theme).unwrap());
    let (_, cx) = cx.add_window_view(|w, cx| {
        let view = cx.new(|_| ClipProbe);
        Root::new(view, w, cx)
    });
    for scale in [1., 1.5, 2.] {
        cx.simulate_scale_factor_change(scale);
        draw(cx);
        cx.update(|w, cx| {
            let clip = observed(w, "clip").bounds().scale(scale);
            let mark = node(w, "clipped", "mark").bounds().scale(scale);
            assert!(mark.top() < clip.top());
            let bg = crate::tokens::rgba(theme, crate::theme::keys::BADGE_DANGER_BACKGROUND);
            let quad = w
                .painted_quads()
                .into_iter()
                .find(|q| q.bounds == mark && q.background.as_solid() == Some(bg.into()))
                .unwrap();
            assert!(quad.content_mask.bounds.top() >= clip.top());
            assert!(quad.content_mask.bounds.right() <= clip.right());
            let separator = gpui_kit::Hsla::from(crate::tokens::rgba(
                theme,
                crate::theme::keys::BADGE_SEPARATOR_COLOR,
            ));
            let shadow = w
                .painted_primitives()
                .into_iter()
                .find_map(|p| match p {
                    gpui_kit::Primitive::Shadow(shadow)
                        if shadow.color == separator && shadow.element_bounds == mark =>
                    {
                        Some(shadow)
                    }
                    _ => None,
                })
                .expect("attached separator must paint outside the badge without affecting layout");
            assert_eq!(shadow.blur_radius.0, 0.);
            let spread = f32::from(w.rem_size()) / 16. * scale;
            assert!(
                (shadow.bounds.size.width.0 - mark.size.width.0 - 2. * spread).abs() <= 2.,
                "separator bounds must cover rounded physical pixel edges"
            );
            assert!(shadow.bounds.left() < mark.left() && shadow.bounds.right() > mark.right());
            assert_eq!(
                w.simulate_next_frame(cx),
                0,
                "static badges must not keep requesting frames"
            );
        });
    }
}

fn mark_alpha(window: &Window, owner: &str) -> f32 {
    let bounds = node(window, owner, "mark")
        .bounds()
        .scale(window.scale_factor());
    window
        .painted_quads()
        .iter()
        .find(|q| q.bounds == bounds && q.background.as_solid().is_some())
        .unwrap()
        .background
        .as_solid()
        .unwrap()
        .a
}
