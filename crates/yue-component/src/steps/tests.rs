use super::*;
use crate::{Accent, ThemeMode, YueButton};
use gpui_kit::base::{
    Root,
    test_support::{ElementSnapshot, snapshots},
};
use gpui_kit::{
    AppContext as _, Context, Entity, Modifiers, ParentElement as _, Render, Styled as _,
    TestAppContext, VisualTestContext, div, px, size,
};
struct Probe {
    keys: Vec<&'static str>,
    long: bool,
    value: StepsValue,
    accept: bool,
    disabled: bool,
    readonly: bool,
    blocked: Option<&'static str>,
    reverse: bool,
    orientation: StepsLayout,
    placement: StepsLabelPlacement,
    variant: StepsVariant,
    size: StepsSize,
    width: f32,
    extra: usize,
    events: Vec<StepsChange>,
    theme: YueTheme,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let items = self
            .keys
            .iter()
            .map(|&key| {
                YueStepItem::new(
                    key,
                    if self.long {
                        format!("{key} 标题很长，需要自然换行并保持节点与连接线位置稳定")
                    } else {
                        key.to_owned()
                    },
                )
                .description("说明文字可以换行，额外内容不会改变连接线中心。")
                .disabled(self.blocked == Some(key))
                .extra(
                    YueButton::new(format!("extra-{key}"), "Extra").on_click(cx.listener(
                        |p, _, _, cx| {
                            p.extra += 1;
                            cx.notify();
                        },
                    )),
                )
            })
            .collect::<Vec<_>>();
        div()
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .items_start()
            .gap_4()
            .child(YueButton::new("outside", "Outside"))
            .child(
                YueSteps::new("probe", self.value.clone())
                    .theme(self.theme)
                    .items(items)
                    .width(px(self.width))
                    .size(self.size)
                    .layout(self.orientation)
                    .label_placement(self.placement)
                    .variant(self.variant)
                    .reverse(self.reverse)
                    .disabled(self.disabled)
                    .readonly(self.readonly)
                    .on_change(cx.listener(|p, e: &StepsChange, _, cx| {
                        p.events.push(e.clone());
                        if p.accept {
                            p.value = e.key().clone().into();
                        }
                        cx.notify();
                    })),
            )
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
    let mut view = None;
    let (_, cx) = cx.add_window_view(|w, cx| {
        let p = cx.new(|_| Probe {
            keys: vec!["a", "b", "c"],
            long: false,
            value: "b".into(),
            accept: false,
            disabled: false,
            readonly: false,
            blocked: None,
            reverse: false,
            orientation: StepsLayout::Horizontal,
            placement: StepsLabelPlacement::Right,
            variant: StepsVariant::Default,
            size: StepsSize::Medium,
            width: 700.,
            extra: 0,
            events: vec![],
            theme: YueTheme::new(ThemeMode::Light, Accent::Azure),
        });
        view = Some(p.clone());
        Root::new(p, w, cx)
    });
    cx.simulate_resize(size(px(1000.), px(900.)));
    cx.update(|w, _| w.activate_window());
    draw(cx);
    (view.unwrap(), cx)
}
fn edit(view: &Entity<Probe>, cx: &mut VisualTestContext, f: impl FnOnce(&mut Probe)) {
    view.update(cx, |p, cx| {
        f(p);
        cx.notify();
    });
    draw(cx);
}
fn nodes(cx: &mut VisualTestContext, id: &str) -> Vec<ElementSnapshot> {
    cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .filter(|n| n.path().last() == Some(&ElementId::from(id.to_owned())))
            .collect()
    })
}
fn action(cx: &mut VisualTestContext, key: &str) -> ElementSnapshot {
    nodes(cx, "steps-action")
        .into_iter()
        .find(|n| n.label().is_some_and(|l| l.contains(&format!("，{key}，"))))
        .unwrap()
}
fn click(cx: &mut VisualTestContext, key: &str) {
    let b = action(cx, key).bounds();
    cx.simulate_click(b.center(), Modifiers::default());
    draw(cx);
}
fn key(cx: &mut VisualTestContext, key: &str) {
    let k = gpui_kit::Keystroke::parse(key).unwrap();
    cx.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: k.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui_kit::KeyUpEvent { keystroke: k });
    draw(cx);
}
#[gpui_kit::test]
fn controlled_requests_pointer_enter_space_and_same_value(cx: &mut TestAppContext) {
    let (p, cx) = open(cx);
    click(cx, "c");
    key(cx, "enter");
    key(cx, "space");
    p.read_with(cx, |p, _| {
        assert_eq!(p.events.len(), 3);
        assert_eq!(p.value, StepsValue::from("b"));
        for e in &p.events {
            assert_eq!(e.index(), 2);
            assert_eq!(e.key(), "c");
            assert_eq!(e.before(), &StepsValue::from("b"));
        }
    });
    click(cx, "b");
    assert_eq!(p.read_with(cx, |p, _| p.events.len()), 3);
    edit(&p, cx, |p| p.accept = true);
    click(cx, "c");
    key(cx, "enter");
    assert_eq!(p.read_with(cx, |p, _| p.events.len()), 4);
}
#[gpui_kit::test]
fn disabled_readonly_and_external_updates_are_silent_extra_remains_independent(
    cx: &mut TestAppContext,
) {
    let (p, cx) = open(cx);
    edit(&p, cx, |p| p.blocked = Some("c"));
    click(cx, "c");
    assert!(p.read_with(cx, |p, _| p.events.is_empty()));
    edit(&p, cx, |p| {
        p.blocked = None;
        p.disabled = true;
    });
    click(cx, "a");
    edit(&p, cx, |p| {
        p.disabled = false;
        p.readonly = true;
        p.value = StepsValue::Complete;
    });
    assert!(nodes(cx, "steps-action").is_empty());
    let b = nodes(cx, "extra-b")[0].bounds();
    cx.simulate_click(b.center(), Modifiers::default());
    draw(cx);
    p.read_with(cx, |p, _| {
        assert_eq!(p.extra, 1);
        assert!(p.events.is_empty());
    });
}
#[gpui_kit::test]
fn key_focus_survives_reorder_and_repairs_only_when_owned(cx: &mut TestAppContext) {
    let (p, cx) = open(cx);
    click(cx, "a");
    assert_eq!(action(cx, "a").focused(), Some(true));
    edit(&p, cx, |p| {
        p.keys = vec!["c", "b", "a"];
        p.reverse = true;
    });
    assert_eq!(action(cx, "a").focused(), Some(true));
    edit(&p, cx, |p| p.keys = vec!["c", "b"]);
    assert_eq!(action(cx, "b").focused(), Some(true));
    let b = nodes(cx, "outside")[0].bounds();
    cx.simulate_click(b.center(), Modifiers::default());
    draw(cx);
    edit(&p, cx, |p| p.disabled = true);
    assert_eq!(nodes(cx, "outside")[0].focused(), Some(true));
}
#[gpui_kit::test]
fn tab_visits_real_actions_and_extra_without_duplicate_stops(cx: &mut TestAppContext) {
    let (_, cx) = open(cx);
    let b = nodes(cx, "outside")[0].bounds();
    cx.simulate_click(b.center(), Modifiers::default());
    draw(cx);
    key(cx, "tab");
    assert_eq!(action(cx, "a").focused(), Some(true));
    key(cx, "tab");
    assert_eq!(nodes(cx, "extra-a")[0].focused(), Some(true));
    key(cx, "tab");
    assert_eq!(action(cx, "b").focused(), Some(true));
    key(cx, "shift-tab");
    assert_eq!(nodes(cx, "extra-a")[0].focused(), Some(true));
}
#[gpui_kit::test]
fn layout_markers_connect_through_centres_and_last_has_no_tail(cx: &mut TestAppContext) {
    let (p, cx) = open(cx);
    for orientation in [StepsLayout::Horizontal, StepsLayout::Vertical] {
        for placement in [StepsLabelPlacement::Right, StepsLabelPlacement::Below] {
            for reverse in [false, true] {
                edit(&p, cx, |p| {
                    p.long = true;
                    p.orientation = orientation;
                    p.placement = placement;
                    p.reverse = reverse;
                });
                let mut m = nodes(cx, "steps-marker");
                let mut l = nodes(cx, "steps-connector");
                if orientation == StepsLayout::Horizontal {
                    m.sort_by(|a, b| a.bounds().left().partial_cmp(&b.bounds().left()).unwrap());
                    l.sort_by(|a, b| a.bounds().left().partial_cmp(&b.bounds().left()).unwrap());
                } else {
                    m.sort_by(|a, b| a.bounds().top().partial_cmp(&b.bounds().top()).unwrap());
                    l.sort_by(|a, b| a.bounds().top().partial_cmp(&b.bounds().top()).unwrap());
                }
                assert_eq!(m.len(), 3);
                assert_eq!(l.len(), 2);
                for i in 0..2 {
                    if orientation == StepsLayout::Vertical {
                        assert!(
                            (l[i].bounds().center().x - m[i].bounds().center().x).abs() < px(0.1)
                        );
                        assert!(
                            (l[i].bounds().bottom() - m[i + 1].bounds().center().y).abs() < px(0.1)
                        );
                    } else {
                        assert!(
                            (l[i].bounds().center().y - m[i].bounds().center().y).abs() < px(0.1)
                        );
                        if placement == StepsLabelPlacement::Below {
                            assert!(
                                (l[i].bounds().left() - m[i].bounds().center().x).abs() < px(0.1)
                            );
                        } else {
                            assert!(l[i].bounds().left() >= m[i].bounds().right());
                        }
                    }
                }
            }
        }
    }
}
#[gpui_kit::test]
fn narrow_container_reveals_external_current_and_focus(cx: &mut TestAppContext) {
    let (p, cx) = open(cx);
    edit(&p, cx, |p| {
        p.width = 220.;
        p.value = "c".into();
    });
    let current = action(cx, "c").bounds();
    assert!(
        current.right() <= px(240.),
        "current {current:?}; list {:?}",
        nodes(cx, "steps-list")[0].bounds()
    );
    assert!(current.left() >= px(16.));
    edit(&p, cx, |p| {
        p.value = "a".into();
    });
    assert!(action(cx, "a").bounds().left() >= px(16.));
}

#[gpui_kit::test]
fn narrow_below_titles_wrap_inside_each_cell_without_overlapping(cx: &mut TestAppContext) {
    let (p, cx) = open(cx);
    for rem in [16., 24.] {
        cx.update(|w, _| w.set_rem_size(px(rem)));
        edit(&p, cx, |p| {
            p.width = 320.;
            p.long = true;
            p.readonly = true;
            p.placement = StepsLabelPlacement::Below;
        });
        let titles = nodes(cx, "steps-title");
        let markers = nodes(cx, "steps-marker");
        for title in titles {
            let marker = markers
                .iter()
                .find(|n| n.path()[..n.path().len() - 1] == title.path()[..title.path().len() - 1])
                .unwrap();
            assert!(
                title.bounds().size.width <= px(160. * rem / 16.),
                "title exceeds its cell: {:?}",
                title.bounds()
            );
            assert!(
                title.bounds().size.height > marker.bounds().size.height,
                "long title did not wrap: {:?}",
                title.bounds()
            );
        }
    }
}

#[gpui_kit::test]
fn narrow_below_scroll_reaches_last_step_and_redraw_keeps_user_position(cx: &mut TestAppContext) {
    let (p, cx) = open(cx);
    edit(&p, cx, |p| {
        p.width = 320.;
        p.long = true;
        p.readonly = true;
        p.value = "a".into();
        p.placement = StepsLabelPlacement::Below;
    });
    let viewport = nodes(cx, "steps-list")[0].bounds();
    cx.simulate_event(gpui_kit::ScrollWheelEvent {
        position: viewport.center(),
        delta: gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(-1000.), px(0.))),
        touch_phase: gpui_kit::TouchPhase::Moved,
        ..Default::default()
    });
    draw(cx);
    let last = cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .find(|n| n.label().is_some_and(|l| l.contains("第 3/3 步")))
            .unwrap()
    });
    assert!(last.bounds().right() <= viewport.right() + px(0.1));
    assert!(last.bounds().left() >= viewport.left());
    let before = last.bounds();
    draw(cx);
    let after = cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .find(|n| n.label().is_some_and(|l| l.contains("第 3/3 步")))
            .unwrap()
            .bounds()
    });
    assert_eq!(before, after);
}

#[gpui_kit::test]
fn arrow_description_has_the_same_outer_bottom_inset_as_marker_top(cx: &mut TestAppContext) {
    let (p, cx) = open(cx);
    for rem in [16., 24.] {
        cx.update(|w, _| w.set_rem_size(px(rem)));
        for size in [StepsSize::Small, StepsSize::Medium, StepsSize::Large] {
            edit(&p, cx, |p| {
                p.variant = StepsVariant::Arrow;
                p.readonly = true;
                p.size = size;
            });
            let cells = cx.update(|w, _| {
                snapshots(w)
                    .into_iter()
                    .filter(|n| n.role() == Some(gpui_kit::Role::ListItem))
                    .collect::<Vec<_>>()
            });
            for cell in cells {
                let marker = nodes(cx, "steps-marker")
                    .into_iter()
                    .find(|n| n.path().starts_with(cell.path()))
                    .unwrap();
                let body = nodes(cx, "steps-body")
                    .into_iter()
                    .find(|n| n.path().starts_with(cell.path()))
                    .unwrap();
                let top = marker.bounds().top() - cell.bounds().top();
                let bottom = cell.bounds().bottom() - body.bounds().bottom();
                assert!(top > px(0.));
                assert!(
                    (top - bottom).abs() < px(0.1),
                    "unbalanced arrow insets: top {top:?}, bottom {bottom:?}"
                );
            }
        }
    }
}
#[gpui_kit::test]
fn empty_single_pending_complete_and_variants(cx: &mut TestAppContext) {
    let (p, cx) = open(cx);
    for variant in [
        StepsVariant::Default,
        StepsVariant::Dot,
        StepsVariant::Navigation,
        StepsVariant::Arrow,
    ] {
        edit(&p, cx, |p| {
            p.variant = variant;
            p.value = StepsValue::Pending;
        });
        assert!(action(cx, "a").label().unwrap().contains("未开始"));
        click(cx, "c");
        edit(&p, cx, |p| {
            p.value = StepsValue::Complete;
        });
        assert!(action(cx, "c").label().unwrap().contains("已完成"));
    }
    edit(&p, cx, |p| {
        p.keys = vec!["b"];
        p.value = "b".into();
    });
    assert_eq!(nodes(cx, "steps-marker").len(), 1);
    assert!(nodes(cx, "steps-connector").is_empty());
    edit(&p, cx, |p| {
        p.keys.clear();
        p.value = StepsValue::Pending;
    });
    assert!(nodes(cx, "steps-marker").is_empty());
}
#[gpui_kit::test]
fn four_themes_three_sizes_and_rem_scaling_keep_geometry(cx: &mut TestAppContext) {
    let (p, cx) = open(cx);
    for rem in [12., 16., 24.] {
        cx.update(|w, _| w.set_rem_size(px(rem)));
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            for accent in [Accent::Azure, Accent::Neutral] {
                for size in [StepsSize::Small, StepsSize::Medium, StepsSize::Large] {
                    edit(&p, cx, |p| {
                        p.theme = YueTheme::new(mode, accent);
                        p.size = size;
                    });
                    let d = nodes(cx, "steps-marker")[0].bounds().size.width;
                    let expected = match size {
                        StepsSize::Small => 20.,
                        StepsSize::Medium => 24.,
                        StepsSize::Large => 32.,
                    } * rem
                        / 16.;
                    assert!(
                        (d - px(expected)).abs() < px(0.1),
                        "{mode:?}/{accent:?}/{size:?}/{rem}: {d:?}"
                    );
                }
            }
        }
    }
}
#[test]
#[should_panic(expected = "unique")]
fn duplicate_keys_fail_early() {
    let _ = YueSteps::new("duplicates", StepsValue::Pending)
        .items([YueStepItem::new("a", "A"), YueStepItem::new("a", "B")]);
}

#[test]
#[should_panic(expected = "current key must exist")]
fn unknown_current_fails_instead_of_silently_selecting_another_item() {
    YueSteps::new("unknown", "missing")
        .items([YueStepItem::new("a", "A")])
        .validate();
}
#[test]
#[should_panic(expected = "Horizontal + Right")]
fn unsupported_variant_layout_fails_explicitly() {
    YueSteps::new("unsupported", StepsValue::Pending)
        .variant(StepsVariant::Arrow)
        .layout(StepsLayout::Vertical)
        .validate();
}
#[test]
#[should_panic(expected = "Horizontal + Right")]
fn unsupported_navigation_labels_fail_explicitly() {
    YueSteps::new("unsupported", StepsValue::Pending)
        .variant(StepsVariant::Navigation)
        .label_placement(StepsLabelPlacement::Below)
        .validate();
}
#[gpui_kit::test]
fn disabled_focused_step_releases_focus_and_reverse_keeps_logical_index(cx: &mut TestAppContext) {
    let (p, cx) = open(cx);
    click(cx, "a");
    edit(&p, cx, |p| {
        p.value = StepsValue::Pending;
        p.blocked = Some("a");
    });
    assert_eq!(action(cx, "b").focused(), Some(true));
    edit(&p, cx, |p| p.disabled = true);
    assert!(
        !nodes(cx, "steps-action")
            .iter()
            .any(|n| n.focused() == Some(true))
    );
    edit(&p, cx, |p| {
        p.disabled = false;
        p.reverse = true;
    });
    click(cx, "c");
    p.read_with(cx, |p, _| {
        assert_eq!(p.events.last().unwrap().index(), 2);
        assert_eq!(p.events.last().unwrap().key(), "c");
    });
}
