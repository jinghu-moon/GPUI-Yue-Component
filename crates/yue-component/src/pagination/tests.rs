use super::*;
use crate::{Accent, ThemeMode, YueButton};
use gpui_kit::base::{
    Root,
    test_support::{ElementSnapshot, snapshots},
};
use gpui_kit::{
    AppContext as _, Context, Entity, KeyDownEvent, KeyUpEvent, Keystroke, ParentElement as _,
    Render, Role, Styled as _, TestAppContext, VisualTestContext, Window, div, px,
};

struct Probe {
    state: Entity<YuePaginationState>,
    events: Vec<PaginationChange>,
    accept: bool,
    simple: bool,
    width: f32,
    size: PaginationSize,
    hide: bool,
    static_dots: bool,
    editors: bool,
    _sub: gpui_kit::Subscription,
}
impl Probe {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let state =
            cx.new(|cx| YuePaginationState::new(PaginationValue::new(445, 1, 10).unwrap(), w, cx));
        let sub = cx.subscribe_in(&state, w, |p, state, e, w, cx| {
            p.events.push(*e);
            if p.accept {
                state.update(cx, |s, cx| s.set_value(e.value(), w, cx));
            }
            cx.notify();
        });
        Self {
            state,
            events: vec![],
            accept: true,
            simple: false,
            width: 760.,
            size: PaginationSize::Medium,
            hide: false,
            static_dots: false,
            editors: true,
            _sub: sub,
        }
    }
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_4()
            .w(px(self.width))
            .child(
                YuePagination::new("pager", &self.state)
                    .size(self.size)
                    .show_total(true)
                    .page_size_selector(self.editors)
                    .show_jumper(self.editors)
                    .first_last(true)
                    .ellipsis_jump(!self.static_dots)
                    .hide_on_single_page(self.hide)
                    .mode(if self.simple {
                        PaginationMode::Simple
                    } else {
                        PaginationMode::Pages
                    }),
            )
            .child(YueButton::new("outside", "外部操作"))
    }
}
fn open(cx: &mut TestAppContext) -> (Entity<Probe>, &mut VisualTestContext) {
    cx.update(|cx| crate::init(cx, crate::YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap());
    let mut saved = None;
    let (_, cx) = cx.add_window_view(|w, cx| {
        w.activate_window();
        let view = cx.new(|cx| Probe::new(w, cx));
        saved = Some(view.clone());
        Root::new(view, w, cx)
    });
    cx.simulate_resize(gpui_kit::size(px(1000.), px(800.)));
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
fn part(cx: &mut VisualTestContext, id: &str) -> ElementSnapshot {
    let key: ElementId = (ElementId::from("pager"), id.to_owned()).into();
    cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .find(|n| n.path().last() == Some(&key))
            .unwrap_or_else(|| panic!("missing {id}"))
    })
}
fn click(cx: &mut VisualTestContext, id: &str) {
    let p = part(cx, id).bounds().center();
    cx.simulate_click(p, Default::default());
    draw(cx);
}
fn key(cx: &mut VisualTestContext, key: &str) {
    let key = Keystroke::parse(key).unwrap();
    cx.simulate_event(KeyDownEvent {
        keystroke: key.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(KeyUpEvent { keystroke: key });
    draw(cx);
}
fn edit(cx: &mut VisualTestContext, text: &str) {
    click(cx, "jump");
    key(cx, "ctrl-a");
    cx.simulate_input(text);
    draw(cx);
}
fn state(view: &Entity<Probe>, cx: &VisualTestContext) -> Entity<YuePaginationState> {
    view.read_with(cx, |p, _| p.state.clone())
}
fn value(view: &Entity<Probe>, cx: &VisualTestContext) -> PaginationValue {
    state(view, cx).read_with(cx, |s, _| s.value())
}
fn update(view: &Entity<Probe>, cx: &mut VisualTestContext, v: PaginationValue) {
    let s = state(view, cx);
    cx.update(|w, cx| s.update(cx, |s, cx| s.set_value(v, w, cx)));
    draw(cx);
}

#[gpui_kit::test]
fn real_buttons_request_once_and_host_can_reject(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    assert_eq!(part(cx, "previous").disabled(), Some(true));
    assert_eq!(part(cx, "page-1").role(), Some(Role::Button));
    click(cx, "page-2");
    assert_eq!(value(&view, cx).page(), 2);
    key(cx, "enter");
    key(cx, "space");
    assert_eq!(
        view.read_with(cx, |p, _| p.events.len()),
        1,
        "current page must not request again"
    );
    key(cx, "tab");
    key(cx, "enter");
    assert_eq!(value(&view, cx).page(), 3);
    view.update(cx, |p, cx| {
        p.accept = false;
        cx.notify();
    });
    draw(cx);
    click(cx, "next");
    assert_eq!(value(&view, cx).page(), 3);
    assert_eq!(
        view.read_with(cx, |p, _| p.events.last().unwrap().value().page()),
        4
    );
    let count = view.read_with(cx, |p, _| p.events.len());
    update(&view, cx, PaginationValue::new(20, 99, 10).unwrap());
    assert_eq!(value(&view, cx).page(), 2);
    assert_eq!(view.read_with(cx, |p, _| p.events.len()), count);
}

#[gpui_kit::test]
fn jump_edits_confirm_cancel_validate_and_clamp(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    edit(cx, "12");
    assert_eq!(value(&view, cx).page(), 1);
    key(cx, "enter");
    assert_eq!(value(&view, cx).page(), 12);
    edit(cx, "-3");
    key(cx, "enter");
    assert_eq!(value(&view, cx).page(), 12);
    assert_eq!(part(cx, "jump-error").role(), Some(Role::Alert));
    key(cx, "escape");
    let s = state(&view, cx);
    assert_eq!(
        s.read_with(cx, |s, cx| s.input.read(cx).value()).as_ref(),
        "12"
    );
    edit(cx, "18");
    let outside = cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .find(|s| s.label() == Some("外部操作"))
            .unwrap()
            .bounds()
            .center()
    });
    cx.simulate_click(outside, Default::default());
    draw(cx);
    assert_eq!(value(&view, cx).page(), 12);
    assert_eq!(
        s.read_with(cx, |s, cx| s.input.read(cx).value()).as_ref(),
        "12"
    );
    edit(cx, "999");
    key(cx, "enter");
    assert_eq!(value(&view, cx).page(), 45);
    edit(cx, "100000000000000000000");
    key(cx, "enter");
    assert_eq!(value(&view, cx).page(), 45);
    assert!(part(cx, "jump-error").visible());
    assert_eq!(view.read_with(cx, |p, _| p.events.len()), 2);
}

#[gpui_kit::test]
fn jump_tab_traverses_without_indenting_or_committing_the_draft(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    edit(cx, "29");
    key(cx, "tab");
    assert!(
        cx.update(|w, _| snapshots(w))
            .iter()
            .any(|n| { n.label() == Some("外部操作") && n.focused() == Some(true) })
    );
    let s = state(&view, cx);
    assert_eq!(
        s.read_with(cx, |s, cx| s.input.read(cx).value()).as_ref(),
        "1"
    );
    key(cx, "shift-tab");
    assert_eq!(part(cx, "jump").focused(), Some(true));
    edit(cx, "20");
    key(cx, "shift-tab");
    assert_eq!(part(cx, "last").focused(), Some(true));
    assert_eq!(value(&view, cx).page(), 1);
    assert_eq!(view.read_with(cx, |p, _| p.events.len()), 0);
}

#[gpui_kit::test]
fn select_requests_page_and_size_atomically_and_restores_rejected_value(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    update(&view, cx, PaginationValue::new(445, 12, 10).unwrap());
    let s = state(&view, cx);
    cx.update(|w, cx| {
        s.read(cx)
            .select
            .read(cx)
            .focus_handle()
            .clone()
            .focus(w, cx)
    });
    draw(cx);
    key(cx, "down");
    key(cx, "down");
    key(cx, "enter");
    assert_eq!(value(&view, cx).page_size(), 20);
    assert_eq!(value(&view, cx).page(), 1);
    assert_eq!(view.read_with(cx, |p, _| p.events.len()), 1);
    assert_eq!(
        view.read_with(cx, |p, _| p.events[0].source()),
        PaginationChangeSource::PageSize
    );
    view.update(cx, |p, cx| {
        p.accept = false;
        cx.notify();
    });
    draw(cx);
    cx.update(|w, cx| {
        s.read(cx)
            .select
            .read(cx)
            .focus_handle()
            .clone()
            .focus(w, cx)
    });
    key(cx, "down");
    key(cx, "down");
    key(cx, "enter");
    assert_eq!(value(&view, cx).page_size(), 20);
    assert_eq!(
        s.read_with(cx, |s, cx| s.select.read(cx).value().clone()),
        crate::SelectValue::Single(Some("20".into()))
    );
    assert_eq!(view.read_with(cx, |p, _| p.events.len()), 2);
}

#[gpui_kit::test]
fn loading_and_disabled_block_every_input_and_keep_selection(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    let s = state(&view, cx);
    for loading in [false, true] {
        cx.update(|w, cx| {
            s.update(cx, |s, cx| {
                s.set_disabled(!loading, w, cx);
                s.set_loading(loading, w, cx);
            })
        });
        draw(cx);
        assert_eq!(part(cx, "next").disabled(), Some(true));
        click(cx, "next");
        click(cx, "page-2");
        click(cx, "jump");
        cx.simulate_input("9");
        key(cx, "enter");
        assert_eq!(value(&view, cx).page(), 1);
        assert_eq!(view.read_with(cx, |p, _| p.events.len()), 0);
        assert!(s.read_with(cx, |s, cx| s.select.read(cx).is_disabled()));
    }
}

#[gpui_kit::test]
fn ellipses_boundaries_and_disappearing_page_focus_work(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    click(cx, "forward");
    assert!(value(&view, cx).page() > 1);
    click(cx, "last");
    assert_eq!(value(&view, cx).page(), 45);
    assert_eq!(part(cx, "next").disabled(), Some(true));
    click(cx, "backward");
    assert!(value(&view, cx).page() < 45);
    click(cx, "page-1");
    update(&view, cx, PaginationValue::new(445, 40, 10).unwrap());
    assert_eq!(
        part(cx, "page-1").focused(),
        Some(true),
        "visible focused page retains identity"
    );
    click(cx, "page-40");
    update(&view, cx, PaginationValue::new(20, 1, 10).unwrap());
    assert_eq!(
        part(cx, "page-1").focused(),
        Some(true),
        "removed page focus follows current page"
    );
    view.update(cx, |p, cx| {
        p.static_dots = true;
        cx.notify();
    });
    update(&view, cx, PaginationValue::new(445, 20, 10).unwrap());
    assert_eq!(part(cx, "backward-ellipsis").role(), None);
}

#[gpui_kit::test]
fn composition_survives_themes_zoom_narrow_width_and_simple_empty_mode(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        for accent in [Accent::Azure, Accent::Neutral] {
            cx.update(|_, cx| crate::set_theme(cx, crate::YueTheme::new(mode, accent)));
            for rem in [16., 20., 24.] {
                for size in [
                    PaginationSize::Small,
                    PaginationSize::Medium,
                    PaginationSize::Large,
                ] {
                    view.update(cx, |p, cx| {
                        p.width = 290.;
                        p.size = size;
                        cx.notify();
                    });
                    cx.update(|w, _| w.set_rem_size(px(rem)));
                    draw(cx);
                    let bound = part(cx, "content").bounds();
                    let next = part(cx, "next").bounds();
                    let jump = part(cx, "jump").bounds();
                    assert!(
                        next.left() >= bound.left() && next.right() <= bound.right() + px(1.),
                        "{next:?} / {bound:?}"
                    );
                    assert!(jump.left() >= bound.left() && jump.right() <= bound.right() + px(1.));
                }
            }
        }
    }
    view.update(cx, |p, cx| {
        p.simple = true;
        cx.notify();
    });
    update(&view, cx, PaginationValue::new(0, 99, 10).unwrap());
    assert_eq!(value(&view, cx).item_range(), 0..0);
    assert_eq!(part(cx, "next").disabled(), Some(true));
    edit(cx, "2");
    key(cx, "enter");
    assert_eq!(view.read_with(cx, |p, _| p.events.len()), 0);
    view.update(cx, |p, cx| {
        p.hide = true;
        cx.notify();
    });
    draw(cx);
    assert!(
        !cx.update(|w, _| snapshots(w))
            .iter()
            .any(|n| n.path().last()
                == Some(&ElementId::from((ElementId::from("pager"), "content"))))
    );
}

#[gpui_kit::test]
fn custom_page_size_keeps_actual_value_and_invalid_options_are_atomic(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    update(&view, cx, PaginationValue::new(445, 12, 17).unwrap());
    let s = state(&view, cx);
    assert_eq!(
        s.read_with(cx, |s, cx| s.select.read(cx).value().clone()),
        crate::SelectValue::Single(Some("17".into()))
    );
    cx.update(|w, cx| {
        s.update(cx, |s, cx| {
            for (sizes, error) in [
                (vec![], PaginationError::EmptyPageSizes),
                (vec![0], PaginationError::ZeroPageSize),
                (vec![10, 10], PaginationError::DuplicatePageSize),
                ((1..=33).collect(), PaginationError::TooManyPageSizes),
            ] {
                assert_eq!(s.set_page_sizes(sizes, w, cx), Err(error));
            }
            assert_eq!(s.value().page_size(), 17);
            s.set_page_sizes([10, 30], w, cx).unwrap();
            s.set_size_change_policy(PaginationSizeChangePolicy::KeepFirstItem);
        })
    });
    draw(cx);
    cx.update(|w, cx| {
        s.read(cx)
            .select
            .read(cx)
            .focus_handle()
            .clone()
            .focus(w, cx)
    });
    key(cx, "down");
    key(cx, "end");
    key(cx, "enter");
    assert_eq!(value(&view, cx).page_size(), 30);
    assert_eq!(value(&view, cx).page(), 7);
    assert!(value(&view, cx).item_range().contains(&187));
}

#[gpui_kit::test]
fn huge_totals_keep_exact_jump_values_and_a_bounded_page_window(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    update(&view, cx, PaginationValue::new(usize::MAX, 1, 1).unwrap());
    let target = usize::MAX - 2;
    edit(cx, &target.to_string());
    key(cx, "enter");
    assert_eq!(value(&view, cx).page(), target);
    let mounted = cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .filter(|n| {
                n.label()
                    .is_some_and(|s| s.starts_with("第 ") && s.contains(" 页"))
            })
            .count()
    });
    assert!(
        mounted > 0 && mounted <= 15,
        "mounted {mounted} numeric destinations"
    );
    assert_eq!(value(&view, cx).item_range(), target - 1..target);
    click(cx, "last");
    assert_eq!(value(&view, cx).page(), usize::MAX);
    assert_eq!(part(cx, "next").disabled(), Some(true));
}

#[gpui_kit::test]
fn hiding_editors_closes_the_select_and_releases_hidden_focus(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    let s = state(&view, cx);
    edit(cx, "29");
    view.update(cx, |p, cx| {
        p.editors = false;
        cx.notify();
    });
    draw(cx);
    assert_eq!(part(cx, "page-1").focused(), Some(true));
    assert_eq!(
        s.read_with(cx, |s, cx| s.input.read(cx).value()).as_ref(),
        "1"
    );
    view.update(cx, |p, cx| {
        p.editors = true;
        cx.notify();
    });
    draw(cx);
    cx.update(|w, cx| {
        s.read(cx)
            .select
            .read(cx)
            .focus_handle()
            .clone()
            .focus(w, cx)
    });
    key(cx, "down");
    assert!(s.read_with(cx, |s, cx| s.select.read(cx).is_open()));
    view.update(cx, |p, cx| {
        p.editors = false;
        cx.notify();
    });
    draw(cx);
    assert!(!s.read_with(cx, |s, cx| s.select.read(cx).is_open()));
    assert_eq!(part(cx, "page-1").focused(), Some(true));
    assert_eq!(view.read_with(cx, |p, _| p.events.len()), 0);
}
