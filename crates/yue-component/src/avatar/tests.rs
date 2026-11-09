use super::derive_fallback;

#[test]
fn fallback_text_uses_unicode_words_and_cjk_names() {
    assert_eq!(derive_fallback("Ada Lovelace").as_deref(), Some("AL"));
    assert_eq!(derive_fallback("huacnlee").as_deref(), Some("HU"));
    assert_eq!(derive_fallback("王伟").as_deref(), Some("王"));
    assert_eq!(
        derive_fallback("  Ada\u{00a0}Lovelace ").as_deref(),
        Some("AL")
    );
    assert_eq!(derive_fallback("  "), None);
}

#[cfg(feature = "headless-tests")]
mod visual {
    use crate::{Accent, AvatarSize, ThemeMode, YueAvatar, YueAvatarGroup, YueTheme};
    use gpui_kit::base::{Root, test_support::snapshots};
    use gpui_kit::{
        AppContext as _, Context, IntoElement, ParentElement as _, Render, Role, Styled as _,
        TestAppContext, VisualTestContext, Window, div,
    };

    struct Probe;

    impl Render for Probe {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let theme = crate::current_theme(cx);
            div()
                .flex()
                .gap_3()
                .child(YueAvatar::new("avatar-ada", theme).name("Ada Lovelace"))
                .child(
                    YueAvatar::new("avatar-failed", theme)
                        .name("Missing Image")
                        .src("missing-avatar.png"),
                )
                .child(
                    YueAvatar::new("avatar-cjk", theme)
                        .name("王伟")
                        .size(AvatarSize::Large),
                )
                .child(
                    YueAvatar::new("avatar-long", theme)
                        .fallback_text("A very long fallback label")
                        .size(AvatarSize::Medium),
                )
                .child(
                    YueAvatarGroup::new("group-team", theme)
                        .accessibility_label("设计团队")
                        .max(1)
                        .children([
                            YueAvatar::new("team-a", theme).name("Ada Lovelace"),
                            YueAvatar::new("team-b", theme).name("Grace Hopper"),
                        ]),
                )
        }
    }

    fn open(cx: &mut TestAppContext) -> &mut VisualTestContext {
        cx.update(|cx| crate::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap());
        let (_, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| Probe);
            Root::new(view, window, cx)
        });
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
        cx
    }

    #[gpui_kit::test]
    fn names_shapes_sizes_and_group_overflow_are_accessible(cx: &mut TestAppContext) {
        let cx = open(cx);
        cx.update(|window, _| {
            let nodes = snapshots(window);
            let labels = nodes.iter().map(|node| node.label()).collect::<Vec<_>>();
            let ada = nodes
                .iter()
                .find(|node| node.label() == Some("Ada Lovelace"))
                .unwrap_or_else(|| panic!("missing Ada label: {labels:?}"));
            let cjk = nodes
                .iter()
                .find(|node| node.label() == Some("王伟"))
                .unwrap_or_else(|| panic!("missing CJK label: {labels:?}"));
            let long = nodes
                .iter()
                .find(|node| node.label() == Some("头像"))
                .unwrap_or_else(|| panic!("missing fallback avatar: {labels:?}"));
            let group = nodes
                .iter()
                .find(|node| node.label() == Some("设计团队"))
                .unwrap_or_else(|| panic!("missing group label: {labels:?}"));
            assert_eq!(ada.role(), Some(Role::Image));
            assert_eq!(cjk.role(), Some(Role::Image));
            assert_eq!(group.role(), Some(Role::Group));
            assert_ne!(ada.bounds().size, cjk.bounds().size);
            assert_eq!(ada.bounds().size, long.bounds().size);
            assert_eq!(group.description(), Some("共 2 人，显示 1 人"));
            let fallback_id = gpui_kit::ElementId::from((
                gpui_kit::ElementId::from("avatar-failed"),
                "fallback-content",
            ));
            assert!(
                nodes
                    .iter()
                    .any(|node| node.path().last() == Some(&fallback_id))
            );
        });
    }

    struct GroupProbe {
        theme: YueTheme,
        size: AvatarSize,
    }

    impl Render for GroupProbe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().flex().child(
                YueAvatarGroup::new("overlap-group", self.theme)
                    .size(self.size)
                    .max(2)
                    .children([
                        YueAvatar::new("overlap-a", self.theme)
                            .name("Ada")
                            .size(AvatarSize::XLarge),
                        YueAvatar::new("overlap-b", self.theme)
                            .name("Grace")
                            .size(AvatarSize::Small),
                        YueAvatar::new("overlap-c", self.theme).name("林"),
                    ]),
            )
        }
    }

    #[gpui_kit::test]
    fn group_paints_separated_overlapping_rims_with_left_avatar_on_top(cx: &mut TestAppContext) {
        use gpui_kit::{Primitive, px};
        cx.update(|cx| crate::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap());
        let mut view = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let probe = cx.new(|_| GroupProbe {
                theme: YueTheme::new(ThemeMode::Light, Accent::Azure),
                size: AvatarSize::Medium,
            });
            view = Some(probe.clone());
            Root::new(probe, window, cx)
        });
        let view = view.unwrap();
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            let theme = YueTheme::new(mode, Accent::Azure);
            for size in [
                AvatarSize::Small,
                AvatarSize::Medium,
                AvatarSize::Large,
                AvatarSize::XLarge,
            ] {
                view.update(cx, |probe, cx| {
                    probe.theme = theme;
                    probe.size = size;
                    cx.notify();
                });
                for rem in [16., 24.] {
                    cx.update(|window, _| window.set_rem_size(px(rem)));
                    cx.run_until_parked();
                    cx.update(|window, cx| {
                        window.refresh();
                        window.draw(cx).clear(cx);
                        let nodes = snapshots(window);
                        let group = nodes
                            .iter()
                            .find(|node| node.role() == Some(Role::Group))
                            .unwrap();
                        let diameter = crate::tokens::pixels(theme, size.token()) * rem / 16.;
                        let width = f32::from(group.bounds().size.width);
                        assert!(
                            (width - diameter * (1. + 2. * 0.72)).abs() < 1.,
                            "group width {width}, diameter {diameter}"
                        );
                        assert!((f32::from(group.bounds().size.height) - diameter).abs() < 1.);
                        let surface = gpui_kit::Hsla::from(gpui_kit::rgb(theme.surface()));
                        let rims: Vec<_> = window
                            .painted_primitives()
                            .into_iter()
                            .filter_map(|primitive| match primitive {
                                Primitive::Quad(quad)
                                    if quad.border_color == surface
                                        && quad.border_widths.top.0 > 0. =>
                                {
                                    Some(quad)
                                }
                                _ => None,
                            })
                            .collect();
                        assert_eq!(
                            rims.len(),
                            3,
                            "each avatar, including +N, needs a visible separator"
                        );
                        let scale = window.scale_factor();
                        for rim in &rims {
                            assert!(
                                (rim.bounds.size.width.0 - diameter * scale).abs() < 1.,
                                "rim {:?}, diameter {diameter}, scale {scale}",
                                rim.bounds
                            );
                            assert!((rim.border_widths.top.0 - 2. * rem / 16. * scale).abs() < 1.);
                        }
                        for pair in rims.windows(2) {
                            let step = (pair[0].bounds.origin.x - pair[1].bounds.origin.x).0;
                            assert!(
                                (step - diameter * 0.72 * scale).abs() < 1.,
                                "left avatar must paint after right avatar at the overlap"
                            );
                        }
                    });
                }
            }
        }
    }
}
