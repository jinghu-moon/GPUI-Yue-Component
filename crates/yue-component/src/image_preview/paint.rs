use super::{
    geometry::{CheckerGrid, Viewport},
    *,
};
use crate::{
    YueTheme,
    tokens::{pixels, rgba},
};
use gpui_kit::{
    AnyElement, Bounds, ContentMask, CursorStyle, HitboxBehavior, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ScrollWheelEvent, Styled as _, canvas, fill,
    point, px, size,
};
use std::time::Instant;

pub(super) fn viewport(
    state: &Entity<YueImagePreviewState>,
    theme: YueTheme,
    floating: bool,
    overlay: Option<OverlayRenderer>,
) -> AnyElement {
    let measure = state.clone();
    let paint = state.clone();
    canvas(
        move |b, w, cx| {
            let hit = w.insert_hitbox(b, HitboxBehavior::Normal);
            let rem = f32::from(w.rem_size()) / 16.;
            let vp = Viewport {
                origin: [b.origin.x.into(), b.origin.y.into()],
                size: [b.size.width.into(), b.size.height.into()],
                scale: w.scale_factor(),
                inset: pixels(theme, "--image-preview-inset") * rem,
                bottom: if floating {
                    pixels(theme, "--image-preview-toolbar-clearance") * rem
                } else {
                    0.
                },
            };
            let changed = measure.update(cx, |s, _| {
                s.bind_hitbox(hit.id, w);
                s.measure(vp)
            });
            if changed {
                let weak = measure.downgrade();
                cx.defer(move |cx| {
                    let _ = weak.update(cx, |_, cx| cx.notify());
                });
            }
            hit
        },
        move |b, hit, w, cx| {
            let now = Instant::now();
            if cx.reduce_motion() {
                paint.update(cx, |s, _| s.finish_motion());
            }
            let s = paint.read(cx);
            let camera = s.display_camera(now);
            let compare = s.is_comparing(now);
            let split = s.display_split(now);
            let vp = s.viewport;
            let disabled = s.disabled;
            let reference = s.reference().cloned();
            let source = s.source().cloned();
            let result = s.result.clone();
            let background = s.background;
            let moving = s.gesture.is_some();
            let active = s.is_animating(now) && w.is_visible() && !cx.reduce_motion();
            let radius =
                px(pixels(theme, "--image-preview-radius") * f32::from(w.rem_size()) / 16.);
            let mut errors = Vec::new();
            w.with_content_mask(Some(ContentMask { bounds: b }), |w| {
                checker(b, background, theme, w);
                if let Some(image) = reference {
                    let d = image.dimensions();
                    let o = camera.top_left(d, vp);
                    let sz = camera.display_size(d, vp.scale);
                    let image_bounds =
                        Bounds::new(point(px(o[0]), px(o[1])), size(px(sz[0]), px(sz[1])));
                    let corners = image_corners(b, image_bounds, radius);
                    let mut draw_image = |image: ImagePreviewImage, w: &mut Window| {
                        if paint.read(cx).render_failed(image.image.id) {
                            return;
                        }
                        let id = image.image.id;
                        if let Err(error) =
                            w.paint_image(b, image_bounds, corners, image.image, 0, false)
                        {
                            errors.push((id, error.to_string()));
                        }
                    };
                    if compare {
                        let edge = px((f32::from(b.left() + b.size.width * split)
                            * w.scale_factor())
                        .round()
                            / w.scale_factor());
                        if let Some(source) = source {
                            let clip = Bounds::new(
                                b.origin,
                                size((edge - b.left()).max(px(0.)), b.size.height),
                            );
                            w.with_content_mask(Some(ContentMask { bounds: clip }), |w| {
                                draw_image(source, w);
                            });
                        }
                        if let Some(result) = result {
                            let clip = Bounds::new(
                                point(edge, b.top()),
                                size((b.right() - edge).max(px(0.)), b.size.height),
                            );
                            w.with_content_mask(Some(ContentMask { bounds: clip }), |w| {
                                draw_image(result, w);
                            });
                        }
                        let width = px(pixels(theme, "--image-preview-divider-width")
                            * f32::from(w.rem_size())
                            / 16.);
                        if split > 0. && split < 1. {
                            w.paint_quad(fill(
                                Bounds::new(
                                    point(edge - width / 2., b.top()),
                                    size(width, b.size.height),
                                ),
                                rgba(theme, "--image-preview-divider-color"),
                            ));
                        }
                    } else {
                        draw_image(image, w);
                    }
                    if let Some(overlay) = &overlay {
                        overlay(
                            ImagePreviewOverlay {
                                viewport: b,
                                image_bounds,
                            },
                            w,
                            cx,
                        );
                    }
                }
            });
            if !errors.is_empty() {
                let weak = paint.downgrade();
                cx.defer(move |cx| {
                    let _ = weak.update(cx, |s, cx| {
                        let mut changed = false;
                        for (id, error) in errors {
                            changed |= s.record_render_error(id, error);
                        }
                        if changed {
                            cx.notify();
                        }
                    });
                });
            }
            if !disabled {
                let near = compare && near_split(w.mouse_position().x.into(), vp, split, theme, w);
                w.set_cursor_style(
                    if near {
                        CursorStyle::ResizeLeftRight
                    } else if moving {
                        CursorStyle::ClosedHand
                    } else {
                        CursorStyle::OpenHand
                    },
                    &hit,
                );
            }
            if active {
                let weak = paint.downgrade();
                w.on_next_frame(move |_, cx| {
                    let _ = weak.update(cx, |s, cx| {
                        if s.mounted {
                            cx.notify();
                        }
                    });
                });
            }
            let weak = paint.downgrade();
            let h = hit.clone();
            w.on_mouse_event(move |e: &MouseDownEvent, phase, w, cx| {
                if !phase.bubble() || e.button != MouseButton::Left || !h.is_hovered(w) {
                    return;
                }
                if let Some(state) = weak.upgrade() {
                    let s = state.read(cx);
                    if s.disabled {
                        return;
                    }
                    let split = s.is_comparing(Instant::now())
                        && near_split(
                            e.position.x.into(),
                            s.viewport,
                            s.display_split(Instant::now()),
                            theme,
                            w,
                        );
                    if e.click_count == 2 {
                        state.update(cx, |s, cx| {
                            s.cancel_gesture(w);
                            if split {
                                s.user_split(0.5, ImagePreviewChangeSource::Pointer, cx);
                            } else {
                                s.user_fit(
                                    ImagePreviewFit::FitAll,
                                    false,
                                    ImagePreviewChangeSource::Pointer,
                                    w,
                                    cx,
                                );
                            }
                        });
                    } else {
                        state.update(cx, |s, cx| {
                            s.begin(
                                [e.position.x.into(), e.position.y.into()],
                                split,
                                h.id,
                                w,
                                cx,
                            )
                        });
                    }
                    cx.stop_propagation();
                    w.prevent_default();
                }
            });
            let weak = paint.downgrade();
            w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                if !phase.bubble() {
                    return;
                }
                if let Some(state) = weak.upgrade()
                    && state.read(cx).gesture.is_some()
                {
                    state.update(cx, |s, cx| {
                        if e.pressed_button == Some(MouseButton::Left) {
                            s.drag([e.position.x.into(), e.position.y.into()], w, cx);
                        } else {
                            s.cancel_gesture(w);
                            cx.notify();
                        }
                    });
                    cx.stop_propagation();
                }
            });
            let weak = paint.downgrade();
            w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                if phase.bubble()
                    && e.button == MouseButton::Left
                    && let Some(state) = weak.upgrade()
                    && state.read(cx).gesture.is_some()
                {
                    state.update(cx, |s, cx| {
                        s.cancel_gesture(w);
                        cx.notify();
                    });
                    cx.stop_propagation();
                }
            });
            let weak = paint.downgrade();
            let h = hit;
            w.on_mouse_event(move |e: &ScrollWheelEvent, phase, w, cx| {
                if !phase.bubble() || !h.is_hovered(w) {
                    return;
                }
                if let Some(state) = weak.upgrade() {
                    if state.read(cx).disabled || state.read(cx).reference().is_none() {
                        return;
                    }
                    let delta = e.delta.pixel_delta(w.rem_size() * 3.);
                    // Bound one event without losing high precision fractional scrolling.
                    let factor = (-f32::from(delta.y) * 0.0015).clamp(-2., 2.).exp();
                    state.update(cx, |s, cx| {
                        s.cancel_gesture(w);
                        s.user_zoom(
                            factor,
                            [e.position.x.into(), e.position.y.into()],
                            false,
                            ImagePreviewChangeSource::Pointer,
                            w,
                            cx,
                        );
                    });
                    cx.stop_propagation();
                    w.prevent_default();
                }
            });
        },
    )
    .absolute()
    .inset_0()
    .into_any_element()
}
fn near_split(x: f32, vp: Viewport, split: f32, theme: YueTheme, w: &Window) -> bool {
    let half = pixels(theme, "--image-preview-divider-hit-width") * f32::from(w.rem_size()) / 32.;
    (x - vp.origin[0] - vp.size[0] * split).abs() <= half
}
// GPUI applies radii to the visible sprite rectangle. Do not round a small
// image's own corners when it does not reach the viewport's rounded corners.
pub(super) fn image_corners(
    viewport: Bounds<gpui_kit::Pixels>,
    image: Bounds<gpui_kit::Pixels>,
    radius: gpui_kit::Pixels,
) -> gpui_kit::Corners<gpui_kit::Pixels> {
    let visible = viewport.intersect(&image);
    let corner = |edge: bool| if edge { radius } else { px(0.) };
    gpui_kit::Corners {
        top_left: corner(visible.left() == viewport.left() && visible.top() == viewport.top()),
        top_right: corner(visible.right() == viewport.right() && visible.top() == viewport.top()),
        bottom_left: corner(
            visible.left() == viewport.left() && visible.bottom() == viewport.bottom(),
        ),
        bottom_right: corner(
            visible.right() == viewport.right() && visible.bottom() == viewport.bottom(),
        ),
    }
}
fn checker(
    b: Bounds<gpui_kit::Pixels>,
    bg: ImagePreviewBackground,
    theme: YueTheme,
    w: &mut Window,
) {
    let (a, c) = match bg {
        ImagePreviewBackground::CheckerDark => (
            "--image-preview-checker-dark-a",
            Some("--image-preview-checker-dark-b"),
        ),
        ImagePreviewBackground::CheckerLight => (
            "--image-preview-checker-light-a",
            Some("--image-preview-checker-light-b"),
        ),
        ImagePreviewBackground::White => ("--image-preview-white", None),
        ImagePreviewBackground::Black => ("--image-preview-black", None),
    };
    let radius = px(pixels(theme, "--image-preview-radius") * f32::from(w.rem_size()) / 16.);
    w.paint_quad(fill(b, rgba(theme, a)).corner_radii(radius));
    if let Some(c) = c
        && let Some(grid) = CheckerGrid::new(
            [b.size.width.into(), b.size.height.into()],
            radius.into(),
            pixels(theme, "--image-preview-checker-size") * f32::from(w.rem_size()) / 16.,
            w.scale_factor(),
        )
    {
        let step = px(grid.side);
        let origin = w.pixel_snap_point(b.origin + point(px(grid.origin[0]), px(grid.origin[1])));
        let color = rgba(theme, c);
        for y in 0..grid.rows {
            for x in 0..grid.columns {
                if (x + y) % 2 == 0 {
                    let tile = Bounds::new(
                        origin + point(step * x as f32, step * y as f32),
                        size(step, step),
                    );
                    w.paint_quad(fill(tile, color));
                }
            }
        }
    }
}
