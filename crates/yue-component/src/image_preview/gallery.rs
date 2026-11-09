//! A compact, transient filmstrip; the viewport retains its full height.
use super::*;
use crate::{
    motion::{MotionDuration, MotionEase},
    tokens::{rems, rgba, shadow},
};
use gpui_kit::base::{Presence, TestSupportExt as _, Transition};
use gpui_kit::{
    InteractiveElement as _, ParentElement as _, Role, ScrollHandle,
    StatefulInteractiveElement as _, Styled as _, canvas, div, prelude::FluentBuilder as _, px,
};

struct FilmstripScroll {
    handle: ScrollHandle,
    active: Option<SharedString>,
    visible: bool,
    pitch: f32,
}

pub(super) fn filmstrip(
    state: &Entity<YueImagePreviewState>,
    theme: crate::YueTheme,
    window: &mut Window,
    cx: &mut App,
) -> Option<AnyElement> {
    let s = state.read(cx);
    let visible = s.filmstrip_visible();
    let animated = s.animated && !s.disabled;
    let presence = Presence::new(s.child_id("filmstrip-presence"), visible)
        .transition(
            Transition::new(if animated {
                if visible {
                    MotionDuration::Enter.duration(theme)
                } else {
                    MotionDuration::Exit.duration(theme)
                }
            } else {
                std::time::Duration::ZERO
            })
            .ease(MotionEase::Enter.easing(theme)),
        )
        .sample(window, cx);
    if !presence.should_render() {
        return None;
    }
    let s = state.read(cx);
    let index = s.gallery_index();
    let items = s.gallery().to_vec();
    let active = items.get(index).map(|item| item.id.clone());
    let interactive = visible && !s.disabled;
    let id = s.child_id("filmstrip");
    let scroll_id = s.child_id("filmstrip-scroll");
    let rem = f32::from(window.rem_size());
    // Match Taffy's device-pixel snapping; fractional-DPI gaps otherwise
    // accumulate enough drift to put later thumbnails outside the frame.
    let pitch = f32::from(window.pixel_snap(px(rem * 3.25)) + window.pixel_snap(px(rem * 0.125)));
    let count = items.len();
    let scroll = window.use_keyed_state(scroll_id, cx, |_, _| FilmstripScroll {
        handle: ScrollHandle::new(),
        active: None,
        visible: false,
        pitch,
    });
    let handle = scroll.update(cx, |scroll, _| {
        if visible && (!scroll.visible || scroll.active != active || scroll.pitch != pitch) {
            scroll
                .handle
                .set_offset(gpui_kit::point(px(-(index as f32) * pitch), px(0.)));
        }
        scroll.active = active;
        scroll.visible = visible;
        scroll.pitch = pitch;
        scroll.handle.clone()
    });
    let owner = state.clone();
    let wheel_scroll = handle.clone();
    let wheel_owner = state.clone();
    let wheel_state = scroll.clone();
    let strip = div()
        .id(id)
        .test_support()
        .aria_label("图片胶片条")
        .when(interactive, |strip| strip.role(Role::Toolbar).occlude())
        .absolute()
        .left_3()
        .right_3()
        .bottom(gpui_kit::rems(0.75 - (1. - presence.progress) * 0.75))
        .opacity(presence.progress)
        .flex()
        .flex_col()
        .p_1()
        .rounded(rems(theme, "--image-preview-radius"))
        .border_1()
        .border_color(rgba(theme, "--image-preview-glass-border"))
        .bg(rgba(theme, "--image-preview-glass-background"))
        .backdrop_blur(rems(theme, "--image-preview-glass-blur"))
        .shadow(shadow(theme, "--image-preview-glass-shadow"))
        .when(interactive, |strip| {
            strip.on_hover(move |hovered, w, cx| {
                owner.update(cx, |s, cx| s.update_filmstrip_pointer(*hovered, w, cx));
            })
        });
    let mut row = div()
        .id(state.read(cx).child_id("filmstrip-viewport"))
        .w_full()
        .min_w_0()
        .flex()
        .items_center()
        .gap(gpui_kit::rems(0.125))
        // The two half-viewport insets let even the first/last item align
        // with the fixed central frame. Half-item negative margins below
        // leave the exact scroll range (count - 1) * pitch.
        .px(gpui_kit::relative(0.5))
        .overflow_hidden()
        .track_scroll(&handle)
        .when(interactive, |row| {
            row.overflow_x_scroll().on_scroll_wheel(move |_, w, cx| {
                // GPUI's scroll owner already maps vertical wheel input
                // onto its horizontal axis. Do not apply the delta twice.
                let x = wheel_scroll
                    .offset()
                    .x
                    .clamp(-px(count.saturating_sub(1) as f32 * pitch), px(0.));
                wheel_scroll.set_offset(gpui_kit::point(x, px(0.)));
                let next = ((-f32::from(x) / pitch).round() as usize).min(count - 1);
                // The selected ID changes, but the wheel's continuous offset
                // must not snap back to the item center on the next render.
                let active = wheel_owner.read(cx).gallery()[next].id.clone();
                wheel_state.update(cx, |scroll, _| scroll.active = Some(active));
                wheel_owner.update(cx, |s, cx| {
                    let _ = s.select_gallery_item(next, w, cx);
                });
                cx.stop_propagation();
                w.prevent_default();
            })
        });
    for (position, document) in items.into_iter().enumerate() {
        let selected = position == index;
        let owner = state.clone();
        let keyboard = state.clone();
        let click_scroll = handle.clone();
        let key_scroll = handle.clone();
        let image = document
            .thumbnail
            .or_else(|| document.source.ready().cloned());
        let item_id = gpui_kit::ElementId::NamedChild(
            std::sync::Arc::new(state.read(cx).id.clone()),
            document.id.clone(),
        );
        let label = format!(
            "第 {} 张：{}{}",
            position + 1,
            document.id,
            if selected { "，当前图片" } else { "" }
        );
        let item = div()
            .id(item_id)
            .test_support()
            .aria_label(label)
            .flex()
            .items_center()
            .justify_center()
            .w(gpui_kit::rems(3.25))
            .h(gpui_kit::rems(3.25))
            .flex_shrink_0()
            .when(position == 0, |item| item.ml(gpui_kit::rems(-1.625)))
            .when(position + 1 == count, |item| {
                item.mr(gpui_kit::rems(-1.625))
            })
            .bg(gpui_kit::rgb(theme.surface_component()))
            .rounded_sm()
            .overflow_hidden()
            .when(interactive, |item| {
                item.role(Role::Button)
                    .tab_stop(true)
                    .aria_selected(selected)
                    .on_click(move |_, w, cx| {
                        click_scroll
                            .set_offset(gpui_kit::point(px(-(position as f32) * pitch), px(0.)));
                        owner.update(cx, |s, cx| {
                            let _ = s.select_gallery_item(position, w, cx);
                            cx.notify();
                        });
                    })
                    .on_key_down(move |event, w, cx| {
                        if event.keystroke.key == "enter" || event.keystroke.key == "space" {
                            key_scroll.set_offset(gpui_kit::point(
                                px(-(position as f32) * pitch),
                                px(0.),
                            ));
                            keyboard.update(cx, |s, cx| {
                                let _ = s.select_gallery_item(position, w, cx);
                                cx.notify();
                            });
                            cx.stop_propagation();
                        }
                    })
            })
            .when_some(image, |item, image| item.child(thumbnail(image)));
        row = row.child(item);
    }
    Some(
        strip
            .child(row)
            .child(
                div()
                    .id(state.read(cx).child_id("filmstrip-selection"))
                    .test_support()
                    .aria_label("缩略图固定选中框")
                    .absolute()
                    .left(gpui_kit::relative(0.5))
                    .ml(gpui_kit::rems(-1.625))
                    .top_1()
                    .w(gpui_kit::rems(3.25))
                    .h(gpui_kit::rems(3.25))
                    .border_2()
                    .border_color(gpui_kit::rgb(theme.accent_solid()))
                    .rounded_sm(),
            )
            .into_any_element(),
    )
}

fn thumbnail(image: ImagePreviewImage) -> AnyElement {
    canvas(
        |bounds, _, _| bounds,
        move |bounds, _, window, _| {
            let [width, height] = image.dimensions();
            let scale =
                (f32::from(bounds.size.width) / width).max(f32::from(bounds.size.height) / height);
            let size = gpui_kit::size(px(width * scale), px(height * scale));
            let origin = bounds.origin
                + gpui_kit::point(
                    (bounds.size.width - size.width) / 2.,
                    (bounds.size.height - size.height) / 2.,
                );
            let _ = image.paint(
                bounds,
                gpui_kit::Bounds::new(origin, size),
                px(0.),
                ImagePreviewSampling::Smooth,
                window,
            );
        },
    )
    .size_full()
    .into_any_element()
}
