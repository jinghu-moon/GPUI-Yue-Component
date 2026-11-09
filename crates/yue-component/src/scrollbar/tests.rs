use super::*;
use gpui_kit::{
    Bounds, Context, Entity, InteractiveElement as _, Modifiers, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement as _, Render, ScrollHandle,
    StatefulInteractiveElement as _, Styled as _, TestAppContext, VisualTestContext, div, point,
    px, size,
};

struct Pane {
    handle: ScrollHandle,
    visibility: ScrollbarVisibility,
    content: gpui_kit::Size<Pixels>,
    raw_base: bool,
    mounted: bool,
    clicks: usize,
    focus: gpui_kit::FocusHandle,
}
impl Render for Pane {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = div()
            .id("viewport")
            .size_full()
            .overflow_scroll()
            .track_scroll(&self.handle)
            .child(
                div()
                    .w(self.content.width)
                    .h(self.content.height)
                    .flex_shrink_0()
                    .child(
                        crate::YueButton::new("content-button", "内容操作")
                            .track_focus(&self.focus)
                            .on_click(cx.listener(|p, _, _, cx| {
                                p.clicks += 1;
                                cx.notify();
                            })),
                    ),
            );
        let mut outer = div().relative().w(px(240.)).h(px(160.)).child(body);
        if self.mounted {
            outer = outer.child(if self.raw_base {
                Scrollbar::new(&self.handle)
                    .id("bar")
                    .mode(ScrollbarMode::Always)
                    .into_any_element()
            } else {
                YueScrollbar::both("bar", &self.handle)
                    .visibility(self.visibility)
                    .into_any_element()
            });
        }
        outer
    }
}
fn open(
    cx: &mut TestAppContext,
    visibility: ScrollbarVisibility,
) -> (Entity<Pane>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init(
            cx,
            YueTheme::new(crate::ThemeMode::Light, crate::Accent::Azure),
        )
        .unwrap();
        cx.set_reduce_motion(true);
    });
    let (view, cx) = cx.add_window_view(|w, cx| {
        w.activate_window();
        Pane {
            handle: ScrollHandle::default(),
            visibility,
            content: size(px(720.), px(960.)),
            raw_base: false,
            mounted: true,
            clicks: 0,
            focus: cx.focus_handle().tab_stop(true),
        }
    });
    draw(cx);
    draw(cx);
    (view, cx)
}
fn draw(cx: &mut VisualTestContext) {
    cx.update(|w, cx| {
        w.refresh();
        w.draw(cx).clear(cx);
    });
}
fn move_to(cx: &mut VisualTestContext, x: f32, y: f32, dragging: bool) {
    cx.simulate_event(MouseMoveEvent {
        position: point(px(x), px(y)),
        pressed_button: dragging.then_some(MouseButton::Left),
        modifiers: Modifiers::default(),
    });
    draw(cx);
}
fn down(cx: &mut VisualTestContext, x: f32, y: f32) {
    cx.simulate_event(MouseDownEvent {
        position: point(px(x), px(y)),
        button: MouseButton::Left,
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    });
    draw(cx);
}
fn up(cx: &mut VisualTestContext, x: f32, y: f32) {
    cx.simulate_event(MouseUpEvent {
        position: point(px(x), px(y)),
        button: MouseButton::Left,
        modifiers: Modifiers::default(),
        click_count: 1,
    });
    draw(cx);
}
fn thumbs(cx: &mut VisualTestContext) -> Vec<Bounds<Pixels>> {
    cx.update(|w, _| {
        let scale = w.scale_factor();
        w.painted_quads()
            .into_iter()
            .filter(|q| {
                !q.background.is_transparent()
                    && ((q.bounds.origin.x.0 / scale > 220.
                        && q.bounds.size.width.0 / scale <= 20.)
                        || (q.bounds.origin.y.0 / scale > 130.
                            && q.bounds.size.height.0 / scale <= 20.))
            })
            .map(|q| {
                Bounds::new(
                    point(
                        px(q.bounds.origin.x.0 / scale),
                        px(q.bounds.origin.y.0 / scale),
                    ),
                    size(
                        px(q.bounds.size.width.0 / scale),
                        px(q.bounds.size.height.0 / scale),
                    ),
                )
            })
            .collect()
    })
}
fn vertical(cx: &mut VisualTestContext) -> Bounds<Pixels> {
    thumbs(cx)
        .into_iter()
        .find(|b| b.size.height > b.size.width)
        .expect("painted vertical thumb")
}

#[gpui_kit::test]
fn floating_thumb_geometry_scales_without_painted_tracks(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, ScrollbarVisibility::Always);
    for rem in [16., 20.] {
        cx.update(|w, _| w.set_rem_size(px(rem)));
        view.update(cx, |p, cx| {
            p.content.height = px(10000.);
            cx.notify();
        });
        move_to(cx, 20., 20., false);
        draw(cx);
        let v = vertical(cx);
        assert_eq!(v.size.width, px(4. * rem / 16.));
        assert_eq!(v.size.height, px(32. * rem / 16.));
        // Paint bounds snap to physical pixels; allow half a physical pixel.
        let rounding = cx.update(|w, _| px(0.5 / w.scale_factor()));
        assert!(
            (v.right() - px(240. - 3. * rem / 16.)).abs() <= rounding,
            "rem={rem}, thumb={v:?}"
        );
        let before = v.right();
        move_to(cx, 232., 20., false);
        let hovered = vertical(cx);
        assert_eq!(hovered.size.width, px(8. * rem / 16.));
        assert_eq!(hovered.right(), before);
        // The only non-transparent paint is the two thumbs.
        let count = cx.update(|w, _| {
            w.painted_quads()
                .into_iter()
                .filter(|q| q.background != gpui_kit::transparent_black().into())
                .count()
        });
        view.update(cx, |p, cx| {
            p.mounted = false;
            cx.notify();
        });
        draw(cx);
        let content_count = cx.update(|w, _| {
            w.painted_quads()
                .into_iter()
                .filter(|q| q.background != gpui_kit::transparent_black().into())
                .count()
        });
        assert_eq!(
            count - content_count,
            2,
            "no visible track or corner may be painted"
        );
        view.update(cx, |p, cx| {
            p.mounted = true;
            cx.notify();
        });
        draw(cx);
    }
}

#[gpui_kit::test]
fn auto_reveals_at_edge_and_hover_reveals_inside_viewport(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, ScrollbarVisibility::Auto);
    assert!(thumbs(cx).is_empty());
    move_to(cx, 232., 20., false);
    assert_eq!(vertical(cx).size.width, px(8.));
    view.update(cx, |p, cx| {
        p.visibility = ScrollbarVisibility::Hover;
        cx.notify();
    });
    move_to(cx, 80., 60., false);
    assert_eq!(vertical(cx).size.width, px(4.));
}

#[gpui_kit::test]
fn transparent_edge_pages_and_drag_reaches_end_without_changing_other_axis(
    cx: &mut TestAppContext,
) {
    let (view, cx) = open(cx, ScrollbarVisibility::Always);
    let handle = view.read_with(cx, |v, _| v.handle.clone());
    move_to(cx, 232., 110., false);
    down(cx, 232., 110.);
    up(cx, 232., 110.);
    assert!(
        (handle.offset().y - px(-144.)).abs() < px(0.1),
        "page once by 90% of the visible height"
    );
    assert_eq!(handle.offset().x, px(0.));
    let b = vertical(cx);
    down(cx, f32::from(b.center().x), f32::from(b.top()) + 4.);
    move_to(cx, 232., 240., true);
    up(cx, 232., 240.);
    assert_eq!(handle.offset().y, px(-800.));
    assert_eq!(handle.offset().x, px(0.));
    assert!(cx.update(|w, _| w.captured_hitbox().is_none()));
}

#[gpui_kit::test]
fn axes_hide_independently_and_single_axis_recovers_full_length(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, ScrollbarVisibility::Always);
    view.update(cx, |p, cx| {
        p.content.height = px(480.);
        cx.notify();
    });
    draw(cx);
    let both = vertical(cx);
    view.update(cx, |p, cx| {
        p.content.width = px(100.);
        cx.notify();
    });
    draw(cx);
    assert_eq!(thumbs(cx).len(), 1);
    assert!(vertical(cx).size.height > both.size.height);
    view.update(cx, |p, cx| {
        p.content.height = px(100.);
        cx.notify();
    });
    draw(cx);
    assert!(thumbs(cx).is_empty());
    view.update(cx, |p, cx| {
        p.content.width = px(720.);
        cx.notify();
    });
    draw(cx);
    assert_eq!(thumbs(cx).len(), 1);
    assert!(thumbs(cx)[0].size.width > thumbs(cx)[0].size.height);
}

#[gpui_kit::test]
fn internal_base_consumers_use_window_resolved_yue_presentation(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, ScrollbarVisibility::Always);
    view.update(cx, |p, cx| {
        p.raw_base = true;
        cx.notify();
    });
    cx.update(|w, _| w.set_rem_size(px(20.)));
    draw(cx);
    assert_eq!(vertical(cx).size.width, px(5.));
    cx.update(|_, cx| {
        crate::set_theme(
            cx,
            YueTheme::new(crate::ThemeMode::Dark, crate::Accent::Azure),
        )
    });
    draw(cx);
    assert_eq!(vertical(cx).size.width, px(5.));
}

#[gpui_kit::test]
fn drag_cleans_capture_on_shrink_deactivation_and_unmount(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, ScrollbarVisibility::Always);
    for cancellation in 0..5 {
        cx.simulate_visibility_change(gpui_kit::WindowVisibility::Visible);
        view.update(cx, |p, cx| {
            p.content.height = px(960.);
            p.mounted = true;
            cx.notify();
        });
        cx.update(|w, _| w.activate_window());
        draw(cx);
        let b = vertical(cx);
        move_to(cx, f32::from(b.center().x), f32::from(b.top()) + 5., false);
        down(cx, f32::from(b.center().x), f32::from(b.top()) + 5.);
        assert!(cx.update(|w, _| w.captured_hitbox().is_some()));
        draw(cx);
        assert!(
            cx.update(|w, _| w.captured_hitbox().is_some()),
            "capture survives a new frame"
        );
        match cancellation {
            0 => view.update(cx, |p, cx| {
                p.content.height = px(100.);
                cx.notify();
            }),
            1 => cx.deactivate_window(),
            2 => view.update(cx, |p, cx| {
                p.mounted = false;
                cx.notify();
            }),
            3 => cx.update(|w, _| w.release_pointer()),
            _ => cx.simulate_visibility_change(gpui_kit::WindowVisibility::Hidden),
        }
        draw(cx);
        assert!(
            cx.update(|w, _| w.captured_hitbox().is_none()),
            "cancellation {cancellation}"
        );
        up(cx, 232., 20.);
    }
}

#[gpui_kit::test]
fn right_button_does_not_page_or_start_drag(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, ScrollbarVisibility::Always);
    move_to(cx, 232., 110., false);
    cx.simulate_event(MouseDownEvent {
        position: point(px(232.), px(110.)),
        button: MouseButton::Right,
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    });
    draw(cx);
    assert_eq!(
        view.read_with(cx, |v, _| v.handle.offset()),
        point(px(0.), px(0.))
    );
    assert!(cx.update(|w, _| w.captured_hitbox().is_none()));
}

#[gpui_kit::test]
fn width_reverses_continuously_and_auto_fades_after_idle(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, ScrollbarVisibility::Always);
    cx.update(|_, cx| cx.set_reduce_motion(false));
    move_to(cx, 232., 20., false);
    cx.background_executor
        .advance_clock(std::time::Duration::from_millis(40));
    draw(cx);
    let partial = vertical(cx).size.width;
    assert!(
        partial > px(4.) && partial < px(8.),
        "intermediate width {partial:?}"
    );
    move_to(cx, 80., 60., false);
    assert_eq!(
        vertical(cx).size.width,
        partial,
        "reverse starts at current width"
    );
    cx.background_executor
        .advance_clock(std::time::Duration::from_millis(140));
    draw(cx);
    assert_eq!(vertical(cx).size.width, px(4.));
    view.update(cx, |p, cx| {
        p.visibility = ScrollbarVisibility::Auto;
        p.handle.set_offset(point(px(0.), px(-100.)));
        cx.notify();
    });
    draw(cx);
    cx.background_executor
        .advance_clock(std::time::Duration::from_millis(799));
    draw(cx);
    assert!(!thumbs(cx).is_empty(), "held until idle expires");
    cx.background_executor
        .advance_clock(std::time::Duration::from_millis(1));
    draw(cx);
    cx.background_executor
        .advance_clock(std::time::Duration::from_millis(140));
    draw(cx);
    assert!(
        thumbs(cx).is_empty(),
        "fade completes using the controlled clock"
    );
}

#[test]
fn thumb_contrast_clears_three_to_one_on_owned_surfaces() {
    let luminance = |c: gpui_kit::Rgba| {
        let linear = |v: f32| {
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
    };
    let mut minimum = f32::MAX;
    for mode in [crate::ThemeMode::Light, crate::ThemeMode::Dark] {
        for accent in [crate::Accent::Azure, crate::Accent::Neutral] {
            for tone in crate::NeutralTone::ALL {
                let theme = YueTheme::new(mode, accent).with_neutral(tone);
                for background in ["--page", "--surface", "--popover-background"] {
                    let bg = luminance(rgba(theme, background));
                    let mut previous = 0.;
                    for foreground in [
                        "--scrollbar-thumb-color",
                        "--scrollbar-thumb-color-hover",
                        "--scrollbar-thumb-color-active",
                    ] {
                        let fg = luminance(rgba(theme, foreground));
                        let ratio = (fg.max(bg) + 0.05) / (fg.min(bg) + 0.05);
                        minimum = minimum.min(ratio);
                        assert!(
                            ratio >= 3. && ratio >= previous,
                            "{mode:?}/{accent:?}/{tone:?}: {foreground} on {background}: {ratio}"
                        );
                        previous = ratio;
                    }
                }
            }
        }
    }
    eprintln!("minimum scrollbar contrast: {minimum:.3}:1");
}

#[gpui_kit::test]
fn content_click_focus_and_wheel_survive_the_overlay(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, ScrollbarVisibility::Always);
    cx.simulate_click(point(px(30.), px(15.)), Modifiers::default());
    draw(cx);
    assert_eq!(view.read_with(cx, |p, _| p.clicks), 1);
    let focus = view.read_with(cx, |p, _| p.focus.clone());
    assert!(cx.update(|w, _| focus.is_focused(w)));
    move_to(cx, 232., 110., false);
    down(cx, 232., 110.);
    up(cx, 232., 110.);
    assert!(
        cx.update(|w, _| focus.is_focused(w)),
        "scrollbar must preserve content focus"
    );
    let handle = view.read_with(cx, |p, _| p.handle.clone());
    let before = handle.offset();
    cx.simulate_event(gpui_kit::ScrollWheelEvent {
        position: point(px(232.), px(60.)),
        delta: gpui_kit::ScrollDelta::Pixels(point(px(0.), px(-40.))),
        touch_phase: gpui_kit::TouchPhase::Moved,
        ..Default::default()
    });
    draw(cx);
    assert!(
        handle.offset().y < before.y,
        "wheel over the thumb remains owned by content"
    );
    assert_eq!(handle.offset().x, before.x);
}

#[gpui_kit::test]
fn per_window_resolution_keeps_different_rem_sizes_independent(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init(
            cx,
            YueTheme::new(crate::ThemeMode::Light, crate::Accent::Azure),
        )
        .unwrap();
        cx.set_reduce_motion(true);
    });
    let mut windows = Vec::new();
    for rem in [16., 20.] {
        let (_, window) = cx.add_window_view(|w, cx| {
            w.set_rem_size(px(rem));
            Pane {
                handle: Default::default(),
                visibility: ScrollbarVisibility::Always,
                content: size(px(720.), px(960.)),
                raw_base: true,
                mounted: true,
                clicks: 0,
                focus: cx.focus_handle(),
            }
        });
        windows.push(window.update(|w, _| w.window_handle()));
    }
    for _ in 0..2 {
        for (i, window) in windows.iter().enumerate() {
            window
                .update(cx, |_, w, cx| {
                    w.refresh();
                    w.draw(cx).clear(cx);
                    let color: gpui_kit::Background =
                        rgba(crate::current_theme(cx), "--scrollbar-thumb-color").into();
                    let quad = w
                        .painted_quads()
                        .into_iter()
                        .find(|q| {
                            q.background == color && q.bounds.size.height > q.bounds.size.width
                        })
                        .unwrap();
                    assert_eq!(
                        quad.bounds.size.width.0 / w.scale_factor(),
                        if i == 0 { 4. } else { 5. }
                    );
                })
                .unwrap();
        }
    }
}

fn painted_vertical(cx: &mut VisualTestContext) -> (Bounds<Pixels>, gpui_kit::Hsla) {
    cx.update(|w, _| {
        let scale = w.scale_factor();
        let q = w
            .painted_quads()
            .into_iter()
            .find(|q| {
                q.bounds.origin.x.0 / scale > 220.
                    && q.bounds.size.height > q.bounds.size.width
                    && !q.background.is_transparent()
            })
            .expect("visible vertical thumb");
        (
            Bounds::new(
                point(
                    px(q.bounds.origin.x.0 / scale),
                    px(q.bounds.origin.y.0 / scale),
                ),
                size(
                    px(q.bounds.size.width.0 / scale),
                    px(q.bounds.size.height.0 / scale),
                ),
            ),
            q.background.as_solid().unwrap(),
        )
    })
}
fn own_motion_frame(cx: &mut VisualTestContext, ms: u64) -> usize {
    cx.background_executor
        .advance_clock(std::time::Duration::from_millis(ms));
    let callbacks = cx.update(|w, cx| w.simulate_next_frame(cx));
    cx.run_until_parked();
    cx.update(|w, cx| w.draw(cx).clear(cx));
    callbacks
}
#[gpui_kit::test]
fn demo_hover_and_press_colors_animate_using_their_own_frames(cx: &mut TestAppContext) {
    let (_, cx) = open(cx, ScrollbarVisibility::Always);
    cx.update(|_, cx| cx.set_reduce_motion(false));
    let (rest, rest_color) = painted_vertical(cx);
    move_to(cx, 232., 20., false);
    assert_eq!(
        painted_vertical(cx).1,
        rest_color,
        "hover color starts from the current painted color"
    );
    assert!(
        own_motion_frame(cx, 70) > 0,
        "hover must request its next frame"
    );
    let (middle, middle_color) = painted_vertical(cx);
    assert!(middle.size.width > rest.size.width && middle.size.width < px(8.));
    assert_ne!(middle_color, rest_color);
    let factor = gpui_kit::base::animation::cubic_bezier(0.2, 0., 0.2, 1.)(0.5);
    let theme = cx.update(|_, cx| crate::current_theme(cx));
    let from: gpui_kit::Rgba = rest_color.into();
    let expected = rgba(theme, "--scrollbar-thumb-color-hover");
    let actual: gpui_kit::Rgba = middle_color.into();
    assert!(
        (actual.r - (from.r + (expected.r - from.r) * factor)).abs() < 0.0001,
        "70ms color matches the HTML cubic-bezier at 50% elapsed time"
    );
    own_motion_frame(cx, 70);
    let (hover, hover_color) = painted_vertical(cx);
    assert_eq!(hover.size.width, px(8.));
    assert_eq!(hover.right(), rest.right());
    down(cx, 232., 20.);
    assert_eq!(
        painted_vertical(cx).1,
        hover_color,
        "pressed color must not jump"
    );
    assert!(
        own_motion_frame(cx, 70) > 0,
        "color-only change must request frames"
    );
    let pressed_middle = painted_vertical(cx).1;
    assert_ne!(pressed_middle, hover_color);
    up(cx, 232., 20.);
    assert_eq!(
        painted_vertical(cx).1,
        pressed_middle,
        "release reverses from sampled color"
    );
    own_motion_frame(cx, 140);
    assert_eq!(painted_vertical(cx).1, hover_color);
    own_motion_frame(cx, 140);
    assert_eq!(
        own_motion_frame(cx, 0),
        0,
        "settled scrollbar stops requesting frames"
    );
    cx.update(|_, cx| cx.set_reduce_motion(true));
    move_to(cx, 80., 60., false);
    assert_eq!(painted_vertical(cx).0.size.width, px(4.));
    assert_eq!(
        painted_vertical(cx).1,
        rest_color,
        "reduced motion snaps color as well as width"
    );
}
#[gpui_kit::test]
fn demo_fade_changes_opacity_without_sliding_the_thumb(cx: &mut TestAppContext) {
    let (_, cx) = open(cx, ScrollbarVisibility::Auto);
    cx.update(|_, cx| cx.set_reduce_motion(false));
    move_to(cx, 232., 20., false);
    own_motion_frame(cx, 140);
    move_to(cx, 80., 60., false);
    own_motion_frame(cx, 140);
    let before = painted_vertical(cx).0;
    own_motion_frame(cx, 660);
    assert!(own_motion_frame(cx, 70) > 0);
    let (during, color) = painted_vertical(cx);
    assert_eq!(
        during.right(),
        before.right(),
        "fade stays on the outer baseline"
    );
    assert!(color.a > 0. && color.a < 1.);
    let factor = gpui_kit::base::animation::cubic_bezier(0.2, 0., 0.2, 1.)(0.5);
    assert!(
        (color.a - (1. - factor)).abs() < 0.0001,
        "fade uses the same HTML curve"
    );
    own_motion_frame(cx, 70);
    assert!(thumbs(cx).is_empty());
}

#[gpui_kit::test]
fn drag_bursts_present_the_latest_pointer_on_the_next_display_frame(cx: &mut TestAppContext) {
    let (view, cx) = open(cx, ScrollbarVisibility::Always);
    let handle = view.read_with(cx, |p, _| p.handle.clone());
    // Keep interactive content outside the pointer's path so this observes
    // drag scheduling independently of content hover/focus notifications.
    handle.set_offset(point(px(0.), px(-100.)));
    draw(cx);
    let b = vertical(cx);
    let at = point(b.center().x, b.top() + px(4.));
    move_to(cx, f32::from(at.x), f32::from(at.y), false);
    down(cx, f32::from(at.x), f32::from(at.y));
    own_motion_frame(cx, 0);
    let notifications = std::rc::Rc::new(std::cell::Cell::new(0));
    let observed = notifications.clone();
    let _subscription =
        cx.update(|_, cx| cx.observe(&view, move |_, _| observed.set(observed.get() + 1)));
    let mut latest = handle.offset();
    for i in 1..=32 {
        cx.simulate_event(MouseMoveEvent {
            position: at + point(px(0.), px(i as f32 * 2.)),
            pressed_button: Some(MouseButton::Left),
            modifiers: Modifiers::default(),
        });
        assert!(
            handle.offset().y < latest.y,
            "every pointer sample writes the current owner immediately"
        );
        latest = handle.offset();
    }
    cx.run_until_parked();
    assert_eq!(
        notifications.get(),
        0,
        "coalesce the burst before presentation"
    );
    assert_eq!(
        own_motion_frame(cx, 0),
        1,
        "the pending drag must request a display frame without waiting for a timer"
    );
    assert_eq!(
        notifications.get(),
        1,
        "one owner invalidation per burst/frame"
    );
    assert_eq!(
        handle.offset(),
        latest,
        "presentation consumes the newest pointer position"
    );
    assert!(
        (vertical(cx).top() + px(4.) - (at.y + px(64.))).abs() < px(0.01),
        "the painted thumb follows the latest pointer sample"
    );
    assert_eq!(
        own_motion_frame(cx, 0),
        0,
        "stationary held pointer does not redraw continuously"
    );
    cx.simulate_event(MouseMoveEvent {
        position: at + point(px(0.), px(70.)),
        pressed_button: Some(MouseButton::Left),
        modifiers: Modifiers::default(),
    });
    up(cx, f32::from(at.x), f32::from(at.y) + 72.);
    let released = handle.offset();
    let after_release = notifications.get();
    own_motion_frame(cx, 0);
    assert_eq!(
        notifications.get(),
        after_release,
        "release invalidates a pending drag callback"
    );
    assert_eq!(handle.offset(), released);
    assert!(cx.update(|w, _| w.captured_hitbox().is_none()));
}
