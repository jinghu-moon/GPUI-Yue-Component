use super::*;
use crate::{Accent, DialogFooter, ThemeMode, YueButton, YueDialog};
use gpui_base::test_support::{ElementSnapshot, snapshots};
use gpui_kit::base::{Root, TestSupportExt as _};
use gpui_kit::{
    AppContext as _, Context, InteractiveElement as _, Modifiers, MouseButton, MouseDownEvent,
    MouseUpEvent, Render, Styled as _, TestAppContext, VisualTestContext, div, point, size,
};
use std::{cell::RefCell, rc::Rc};

fn theme() -> YueTheme {
    YueTheme::new(ThemeMode::Light, Accent::Azure)
}
struct Demo {
    state: Entity<YueDrawerState>,
    child: Entity<YueDrawerState>,
    placement: DrawerPlacement,
    size: DrawerSize,
    modal: bool,
    resizable: bool,
    mask: bool,
    veto: bool,
    pending: bool,
    mounted: bool,
    background: usize,
    trigger: FocusHandle,
    inner: FocusHandle,
    ticket: Rc<RefCell<Option<DrawerTicket>>>,
}
impl Render for Demo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.clone();
        let mut root = div()
            .size_full()
            .child(
                YueButton::text("open", theme(), "Open")
                    .track_focus(&self.trigger)
                    .on_click(move |_, w, cx| {
                        state.update(cx, |s, cx| s.open(w, cx)).unwrap();
                    }),
            )
            .child(
                YueButton::text("background", theme(), "Background")
                    .on_click(cx.listener(|s, _, _, _| s.background += 1)),
            );
        if self.mounted {
            let veto = self.veto;
            let pending = self.pending;
            let ticket = self.ticket.clone();
            let child = self.child.clone();
            let parent = self.state.clone();
            root = root
                .child(
                    YueDrawer::new("drawer", theme(), &self.state)
                        .title("Details")
                        .placement(self.placement)
                        .size(self.size)
                        .modal(self.modal)
                        .resizable(self.resizable)
                        .resize_limits(gpui_kit::rems(10.), px(600.))
                        .overlay(self.mask)
                        .on_close_request(move |_, _, _| !veto)
                        .on_confirm(move |r, _, _| {
                            *ticket.borrow_mut() = Some(r.ticket());
                            if pending {
                                DrawerDecision::Pending
                            } else {
                                DrawerDecision::Close
                            }
                        })
                        .child(
                            YueButton::text("inside", theme(), "Inside").track_focus(&self.inner),
                        )
                        .child(YueButton::text("nested", theme(), "Nested").on_click(
                            move |_, w, cx| {
                                child
                                    .update(cx, |s, cx| s.open_child(&parent, w, cx))
                                    .unwrap();
                            },
                        ))
                        .children((0usize..60).map(|ix| {
                            div()
                                .id(("line", ix))
                                .test_support()
                                .flex_shrink_0()
                                .h(px(24.))
                                .child(format!("Row {ix}"))
                        })),
                )
                .child(
                    YueDialog::new("child", theme(), &self.child)
                        .title("Child decision")
                        .footer(DialogFooter::None),
                );
        }
        root
    }
}
fn setup(cx: &mut TestAppContext) -> (Entity<Demo>, &mut VisualTestContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::popup::init(cx);
        crate::dialog::init(cx);
        crate::set_theme(cx, theme());
        cx.set_reduce_motion(true);
    });
    let mut saved = None;
    let (_, cx) = cx.add_window_view(|w, cx| {
        let view = cx.new(|cx| Demo {
            state: cx.new(YueDrawerState::new),
            child: cx.new(YueDrawerState::new),
            placement: DrawerPlacement::Right,
            size: DrawerSize::Small,
            modal: true,
            resizable: false,
            mask: true,
            veto: false,
            pending: false,
            mounted: true,
            background: 0,
            trigger: cx.focus_handle(),
            inner: cx.focus_handle(),
            ticket: Rc::default(),
        });
        saved = Some(view.clone());
        Root::new(view, w, cx)
    });
    cx.update(|w, _| w.activate_window());
    draw(cx);
    (saved.unwrap(), cx)
}
fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| {
        w.simulate_next_frame(cx);
        w.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}
fn node(cx: &mut VisualTestContext, label: &str) -> ElementSnapshot {
    cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .find(|s| s.label() == Some(label))
            .unwrap_or_else(|| panic!("missing {label}"))
    })
}
fn click(cx: &mut VisualTestContext, label: &str) {
    let p = node(cx, label).bounds().center();
    cx.simulate_click(p, Modifiers::default());
    draw(cx);
}
fn edit(v: &Entity<Demo>, cx: &mut VisualTestContext, f: impl FnOnce(&mut Demo)) {
    v.update(cx, |s, cx| {
        f(s);
        cx.notify();
    });
    draw(cx);
}
fn state(v: &Entity<Demo>, cx: &mut VisualTestContext) -> Entity<YueDrawerState> {
    v.read_with(cx, |s, _| s.state.clone())
}
fn press(cx: &mut VisualTestContext, key: &str) {
    let keystroke = gpui_kit::Keystroke::parse(key).unwrap();
    cx.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui_kit::KeyUpEvent { keystroke });
    draw(cx);
}

#[gpui_kit::test]
fn four_edges_fill_cross_axis_and_clamp_on_resize(cx: &mut TestAppContext) {
    let (v, cx) = setup(cx);
    for placement in [
        DrawerPlacement::Left,
        DrawerPlacement::Right,
        DrawerPlacement::Top,
        DrawerPlacement::Bottom,
    ] {
        edit(&v, cx, |s| s.placement = placement);
        click(cx, "Open");
        draw(cx);
        let b = node(cx, "Details").bounds();
        let viewport = cx.update(|w, _| w.viewport_size());
        assert_eq!(
            b.size,
            if placement.horizontal() {
                size(px(320.), viewport.height)
            } else {
                size(viewport.width, px(320.))
            }
        );
        assert_eq!(b.origin, placement.geometry(viewport, px(320.), 1.).0);
        press(cx, "escape");
        draw(cx);
    }
    edit(&v, cx, |s| s.size = DrawerSize::Full);
    click(cx, "Open");
    draw(cx);
    cx.simulate_resize(size(px(260.), px(220.)));
    draw(cx);
    assert_eq!(node(cx, "Details").bounds().size, size(px(260.), px(220.)));
}
#[gpui_kit::test]
fn nonmodal_background_click_and_focus_are_available(cx: &mut TestAppContext) {
    let (v, cx) = setup(cx);
    edit(&v, cx, |s| s.modal = false);
    click(cx, "Open");
    draw(cx);
    assert!(cx.update(|w, cx| gpui_kit::base::active_modal_scope(w, cx).is_none()));
    click(cx, "Background");
    assert_eq!(v.read_with(cx, |s, _| s.background), 1);
    let outside = v.read_with(cx, |s, _| s.trigger.clone());
    cx.update(|w, cx| outside.focus(w, cx));
    draw(cx);
    let focus = cx.update(|w, cx| w.focused(cx));
    let state = state(&v, cx);
    cx.update(|w, cx| state.update(cx, |s, cx| s.close(w, cx)));
    draw(cx);
    assert_eq!(cx.update(|w, cx| w.focused(cx)), focus);
}
#[gpui_kit::test]
fn hidden_mask_still_modal_and_guard_covers_escape(cx: &mut TestAppContext) {
    let (v, cx) = setup(cx);
    edit(&v, cx, |s| {
        s.mask = false;
        s.veto = true;
    });
    click(cx, "Open");
    draw(cx);
    click(cx, "Background");
    assert_eq!(v.read_with(cx, |s, _| s.background), 0);
    press(cx, "escape");
    assert!(state(&v, cx).read_with(cx, |s, _| s.is_open()));
    edit(&v, cx, |s| s.veto = false);
    press(cx, "escape");
    draw(cx);
    assert!(!state(&v, cx).read_with(cx, |s, _| s.is_open()));
}
#[gpui_kit::test]
fn pending_is_one_use_and_stale_ticket_cannot_close_reopened_session(cx: &mut TestAppContext) {
    let (v, cx) = setup(cx);
    edit(&v, cx, |s| s.pending = true);
    click(cx, "Open");
    draw(cx);
    click(cx, "OK");
    let ticket = v.read_with(cx, |s, _| s.ticket.borrow().clone().unwrap());
    press(cx, "escape");
    assert!(state(&v, cx).read_with(cx, |s, _| s.is_pending()));
    assert!(cx.update(|w, cx| ticket.complete(DrawerOutcome::KeepOpen, w, cx)));
    draw(cx);
    press(cx, "escape");
    draw(cx);
    click(cx, "Open");
    draw(cx);
    assert!(!cx.update(|w, cx| ticket.complete(DrawerOutcome::Close, w, cx)));
    assert!(state(&v, cx).read_with(cx, |s, _| s.is_open()));
}
#[gpui_kit::test]
fn drawer_dialog_nested_escape_only_closes_top(cx: &mut TestAppContext) {
    let (v, cx) = setup(cx);
    click(cx, "Open");
    draw(cx);
    click(cx, "Nested");
    draw(cx);
    press(cx, "escape");
    draw(cx);
    assert!(state(&v, cx).read_with(cx, |s, _| s.is_open()));
    assert!(!v.read_with(cx, |s, cx| s.child.read(cx).is_open()));
    press(cx, "escape");
    draw(cx);
    assert!(!state(&v, cx).read_with(cx, |s, _| s.is_open()));
}
#[gpui_kit::test]
fn overlay_requires_complete_background_press(cx: &mut TestAppContext) {
    let (v, cx) = setup(cx);
    click(cx, "Open");
    draw(cx);
    let inside = node(cx, "Inside").bounds().center();
    let outside = point(px(10.), px(500.));
    cx.simulate_event(MouseDownEvent {
        button: MouseButton::Left,
        position: inside,
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    });
    cx.simulate_event(MouseUpEvent {
        button: MouseButton::Left,
        position: outside,
        modifiers: Modifiers::default(),
        click_count: 1,
    });
    draw(cx);
    assert!(state(&v, cx).read_with(cx, |s, _| s.is_open()));
    cx.simulate_click(outside, Modifiers::default());
    draw(cx);
    assert!(!state(&v, cx).read_with(cx, |s, _| s.is_open()));
}
#[gpui_kit::test]
fn default_enter_does_not_submit_and_unmount_releases_barrier(cx: &mut TestAppContext) {
    let (v, cx) = setup(cx);
    click(cx, "Open");
    draw(cx);
    click(cx, "Inside");
    press(cx, "enter");
    assert!(state(&v, cx).read_with(cx, |s, _| s.is_open()));
    edit(&v, cx, |s| s.mounted = false);
    draw(cx);
    draw(cx);
    assert!(!state(&v, cx).read_with(cx, |s, _| s.is_open()));
    click(cx, "Background");
    assert_eq!(v.read_with(cx, |s, _| s.background), 1);
}

fn part(cx: &mut VisualTestContext, id: &'static str) -> gpui_kit::Bounds<Pixels> {
    cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .find(|n| n.path().last() == Some(&id.into()))
            .unwrap_or_else(|| panic!("missing {id}"))
            .bounds()
    })
}
#[gpui_kit::test]
fn resize_keyboard_limits_and_viewport_preserve_preferred_size(cx: &mut TestAppContext) {
    let (v, cx) = setup(cx);
    edit(&v, cx, |s| s.resizable = true);
    click(cx, "Open");
    draw(cx);
    click(cx, "Adjust drawer size");
    press(cx, "home");
    assert_eq!(node(cx, "Details").bounds().size.width, px(160.));
    press(cx, "left");
    assert!(node(cx, "Details").bounds().size.width > px(160.));
    press(cx, "end");
    assert_eq!(node(cx, "Details").bounds().size.width, px(600.));
    cx.simulate_resize(size(px(360.), px(500.)));
    draw(cx);
    assert_eq!(node(cx, "Details").bounds().size.width, px(360.));
    cx.simulate_resize(size(px(800.), px(600.)));
    draw(cx);
    assert_eq!(node(cx, "Details").bounds().size.width, px(600.));
}
#[gpui_kit::test]
fn resize_pointer_keeps_tracking_outside_the_surface(cx: &mut TestAppContext) {
    let (v, cx) = setup(cx);
    edit(&v, cx, |s| s.resizable = true);
    click(cx, "Open");
    draw(cx);
    let start = node(cx, "Adjust drawer size").bounds().center();
    cx.simulate_event(MouseDownEvent {
        button: MouseButton::Left,
        position: start,
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    });
    cx.simulate_mouse_move(
        start + point(px(-4.), px(0.)),
        Some(MouseButton::Left),
        Modifiers::default(),
    );
    draw(cx);
    cx.simulate_mouse_move(
        start + point(px(-140.), px(0.)),
        Some(MouseButton::Left),
        Modifiers::default(),
    );
    draw(cx);
    assert!(
        node(cx, "Details").bounds().size.width > px(440.),
        "resize stopped at old panel boundary"
    );
    cx.simulate_event(MouseUpEvent {
        button: MouseButton::Left,
        position: start + point(px(-140.), px(0.)),
        modifiers: Modifiers::default(),
        click_count: 1,
    });
    draw(cx);
    let after = node(cx, "Details").bounds();
    cx.simulate_mouse_move(start, None, Modifiers::default());
    draw(cx);
    assert_eq!(node(cx, "Details").bounds(), after);
}
#[gpui_kit::test]
fn long_body_scroll_does_not_move_chrome(cx: &mut TestAppContext) {
    let (_, cx) = setup(cx);
    click(cx, "Open");
    draw(cx);
    let header = part(cx, "dialog-header");
    let footer = part(cx, "dialog-footer");
    let first = part(cx, "dialog-body");
    cx.simulate_event(gpui_kit::ScrollWheelEvent {
        position: first.center(),
        delta: gpui_kit::ScrollDelta::Pixels(point(px(0.), px(-300.))),
        touch_phase: gpui_kit::TouchPhase::Moved,
        ..Default::default()
    });
    draw(cx);
    assert_eq!(header, part(cx, "dialog-header"));
    assert_eq!(footer, part(cx, "dialog-footer"));
    let line = cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .find(|n| n.path().last() == Some(&("line", 0usize).into()))
            .unwrap()
            .bounds()
    });
    assert!(line.top() < first.top());
}
#[gpui_kit::test]
fn interrupted_slide_reverses_without_jump_and_closes_once(cx: &mut TestAppContext) {
    let (v, cx) = setup(cx);
    cx.update(|_, cx| cx.set_reduce_motion(false));
    click(cx, "Open");
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(100));
    draw(cx);
    let middle = node(cx, "Details").bounds().left();
    let width = cx.update(|w, _| w.viewport_size().width);
    assert!(
        middle > width - px(320.) && middle < width,
        "midpoint {middle:?}, viewport {width:?}"
    );
    let state = state(&v, cx);
    cx.update(|w, cx| state.update(cx, |s, cx| s.close(w, cx)));
    draw(cx);
    let reversed = node(cx, "Details").bounds().left();
    assert!((reversed - middle).abs() < px(2.));
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(400));
    draw(cx);
    draw(cx);
    assert!(!cx.update(|w, _| snapshots(w).iter().any(|n| n.label() == Some("Details"))));
}

struct LocalDemo {
    state: Entity<YueDrawerState>,
    placement: DrawerPlacement,
    push: bool,
}
impl Render for LocalDemo {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.clone();
        div()
            .size_full()
            .p(px(30.))
            .child(
                YueButton::text("local-open", theme(), "Open local").on_click(move |_, w, cx| {
                    state.update(cx, |s, cx| s.open(w, cx)).unwrap();
                }),
            )
            .child(
                YueDrawerHost::new(
                    "local-host",
                    YueDrawer::new("local-drawer", theme(), &self.state)
                        .title("Local details")
                        .extent(px(200.))
                        .placement(self.placement)
                        .resizable(true)
                        .resize_limits(px(100.), px(400.)),
                )
                .push(self.push)
                .w(px(600.))
                .h(px(360.))
                .child(
                    div()
                        .id("document")
                        .test_support()
                        .size_full()
                        .child("Document"),
                ),
            )
    }
}
#[gpui_kit::test]
fn local_host_stays_in_its_bounds_and_pushes_each_edge(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::popup::init(cx);
        crate::dialog::init(cx);
        crate::set_theme(cx, theme());
        cx.set_reduce_motion(true);
    });
    let mut saved = None;
    let (_, cx) = cx.add_window_view(|w, cx| {
        let view = cx.new(|cx| LocalDemo {
            state: cx.new(YueDrawerState::new),
            placement: DrawerPlacement::Right,
            push: true,
        });
        saved = Some(view.clone());
        Root::new(view, w, cx)
    });
    let v = saved.unwrap();
    cx.update(|w, _| w.activate_window());
    draw(cx);
    draw(cx);
    for placement in [
        DrawerPlacement::Right,
        DrawerPlacement::Left,
        DrawerPlacement::Top,
        DrawerPlacement::Bottom,
    ] {
        v.update(cx, |s, cx| {
            s.placement = placement;
            cx.notify();
        });
        draw(cx);
        click(cx, "Open local");
        draw(cx);
        draw(cx);
        let host = part(cx, "local-host");
        let panel = node(cx, "Local details").bounds();
        let (origin, size) = placement.geometry(host.size, px(200.), 1.);
        assert_eq!(panel.origin, host.origin + origin);
        assert_eq!(panel.size, size);
        let document = part(cx, "document");
        assert_eq!(
            document.size,
            if placement.horizontal() {
                size_2(px(400.), px(360.))
            } else {
                size_2(px(600.), px(160.))
            }
        );
        assert!(cx.update(|w, cx| gpui_kit::base::active_modal_scope(w, cx).is_none()));
        let state = v.read_with(cx, |s, _| s.state.clone());
        cx.update(|w, cx| state.update(cx, |s, cx| s.close(w, cx)));
        draw(cx);
        draw(cx);
        assert_eq!(part(cx, "document").size, host.size);
    }
}
fn size_2(w: Pixels, h: Pixels) -> Size<Pixels> {
    gpui_kit::size(w, h)
}
