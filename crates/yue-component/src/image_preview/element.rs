use super::*;
use crate::{
    ButtonSize, ButtonVariant, FeedbackSize, SegmentedDistribution, SegmentedSize, YueButton,
    YueSegmented, YueSegmentedOption, YueSpinner,
    tokens::{pixels, rems, rgba, shadow},
};
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::{
    InteractiveElement as _, MouseButton, Orientation, ParentElement as _, Role,
    StatefulInteractiveElement as _, Styled as _, div, prelude::FluentBuilder as _, px,
};
use std::time::Instant;

struct Mount {
    _observe: gpui_kit::Subscription,
    _release: gpui_kit::Subscription,
}
impl RenderOnce for YueImagePreview {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = self.theme.unwrap_or_else(|| crate::current_theme(cx));
        let state = self.state;
        let mounted = state.clone();
        let mount_id = state.read(cx).child_id("mount");
        w.use_keyed_state(mount_id, cx, move |w, cx| {
            let observe = cx.observe(&mounted, |_, _, cx| cx.notify());
            let weak = mounted.downgrade();
            let handle = w.window_handle();
            let release = cx.on_release(move |_: &mut Mount, cx| {
                let _ = handle.update(cx, |_, w, cx| {
                    let _ = weak.update(cx, |s, _| {
                        s.cancel_gesture(w);
                        s.finish_motion();
                        s.mounted = false;
                    });
                });
            });
            Mount {
                _observe: observe,
                _release: release,
            }
        });
        state.update(cx, |s, cx| {
            s.theme = theme;
            s.animated = self.animated;
            s.disabled = self.disabled;
            s.mounted = true;
            if self.disabled {
                s.cancel_gesture(w);
                s.finish_motion();
                if s.focus.is_focused(w) || s.grip_focus.is_focused(w) {
                    w.blur(cx);
                }
            }
            if !self.animated {
                s.finish_motion();
            }
        });
        let s = state.read(cx);
        let id = s.id.clone();
        let viewport_id = s.child_id("viewport");
        let mode_id = s.child_id("modes");
        let focus = s.focus.clone();
        let mode = s.mode;
        let source_available = s.mode_available(ImagePreviewMode::Source);
        let compare_available = s.mode_available(ImagePreviewMode::Compare);
        let result_available = s.mode_available(ImagePreviewMode::Result);
        let disabled = self.disabled;
        let available = s.reference().is_some();
        let (status, busy, error) = s.status();
        let snap = s.snapshot();
        let zoom = if let Some(image) = s.reference() {
            s.display_camera(Instant::now()).zoom(image.dimensions())
        } else {
            1.
        };
        let percentage = if zoom < 0.01 {
            format!("{:.3}%", zoom * 100.)
        } else if zoom < 0.1 {
            format!("{:.1}%", zoom * 100.)
        } else {
            format!("{:.0}%", zoom * 100.)
        };
        let source_size = s.source().map(|i| i.pixel_size());
        let result_size = s.result.as_ref().map(|i| i.pixel_size());
        let description = s
            .reference()
            .map(|i| i.description.clone())
            .unwrap_or_else(|| "图片预览".into());
        let focus_visible = focus.is_focused(w) && w.last_input_was_keyboard();
        let limits = s
            .reference()
            .map(|i| s.viewport.zoom_limits(i.dimensions()))
            .unwrap_or([0.05, 8.]);
        let toolbar = zoom_toolbar(
            &state,
            theme,
            self.toolbar == ImagePreviewToolbar::Floating,
            disabled || !available,
            percentage,
            [zoom > limits[0] + 0.000001, zoom < limits[1] - 0.000001],
            cx,
        );
        let mut viewport = div()
            .id(viewport_id)
            .test_support()
            .role(Role::Group)
            .aria_label("图片预览")
            .aria_description(description)
            .relative()
            .w_full()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .rounded(rems(theme, "--image-preview-radius"))
            .border_1()
            .border_color(gpui_kit::rgb(if focus_visible {
                theme.focus_ring()
            } else {
                theme.border_default()
            }))
            .when(!disabled, |d| d.track_focus(&focus))
            .key_context("YueImagePreview")
            .child(paint::viewport(
                &state,
                theme,
                self.toolbar_visible && self.toolbar == ImagePreviewToolbar::Floating,
                self.overlay,
            ));
        macro_rules! action {
            ($view:ident,$ty:ty,$op:expr) => {{
                let owner = state.clone();
                $view = $view.on_action(move |_: &$ty, w, cx| {
                    owner.update(cx, |s, cx| {
                        if s.disabled || !s.focus.is_focused(w) {
                            cx.propagate();
                            return;
                        }
                        ($op)(s, w, cx);
                    });
                });
            }};
        }
        action!(viewport, ZoomIn, |s: &mut YueImagePreviewState,
                                   w: &mut Window,
                                   cx: &mut gpui_kit::Context<
            YueImagePreviewState,
        >| s.user_zoom(
            1.25,
            s.viewport.midpoint(),
            true,
            ImagePreviewChangeSource::Keyboard,
            w,
            cx
        ));
        action!(viewport, ZoomOut, |s: &mut YueImagePreviewState,
                                    w: &mut Window,
                                    cx: &mut gpui_kit::Context<
            YueImagePreviewState,
        >| s.user_zoom(
            1. / 1.25,
            s.viewport.midpoint(),
            true,
            ImagePreviewChangeSource::Keyboard,
            w,
            cx
        ));
        action!(viewport, ActualSize, |s: &mut YueImagePreviewState,
                                       w: &mut Window,
                                       cx: &mut gpui_kit::Context<
            YueImagePreviewState,
        >| s.user_fit(
            ImagePreviewFit::Manual,
            true,
            ImagePreviewChangeSource::Keyboard,
            w,
            cx
        ));
        action!(viewport, FitAll, |s: &mut YueImagePreviewState,
                                   w: &mut Window,
                                   cx: &mut gpui_kit::Context<
            YueImagePreviewState,
        >| s.user_fit(
            ImagePreviewFit::FitAll,
            false,
            ImagePreviewChangeSource::Keyboard,
            w,
            cx
        ));
        action!(viewport, FitWidth, |s: &mut YueImagePreviewState,
                                     w: &mut Window,
                                     cx: &mut gpui_kit::Context<
            YueImagePreviewState,
        >| s.user_fit(
            ImagePreviewFit::FitWidth,
            false,
            ImagePreviewChangeSource::Keyboard,
            w,
            cx
        ));
        macro_rules! pan_action {
            ($ty:ty,$dx:expr,$dy:expr) => {
                action!(viewport, $ty, |s: &mut YueImagePreviewState,
                                        w: &mut Window,
                                        cx: &mut gpui_kit::Context<
                    YueImagePreviewState,
                >| {
                    let step =
                        pixels(s.theme, "--image-preview-pan-step") * f32::from(w.rem_size()) / 16.
                            * if w.modifiers().shift { 4. } else { 1. };
                    let direction = [$dx, $dy];
                    s.user_pan(direction.map(|v| v * step), w, cx);
                });
            };
        }
        pan_action!(PanLeft, 1., 0.);
        pan_action!(PanRight, -1., 0.);
        pan_action!(PanUp, 0., 1.);
        pan_action!(PanDown, 0., -1.);
        let owner = state.clone();
        viewport = viewport.on_key_down(move |e, w, cx| {
            if e.keystroke.key == "escape" && owner.read(cx).gesture.is_some() {
                owner.update(cx, |s, cx| {
                    s.cancel_gesture(w);
                    cx.notify();
                });
                cx.stop_propagation();
                w.prevent_default();
            }
        });
        if mode == ImagePreviewMode::Compare && !disabled {
            viewport = viewport.child(grip(&state, theme, w, cx));
        }
        if !status.is_empty() {
            let status_id = state.read(cx).child_id("status");
            let retry_id = state.read(cx).child_id("retry");
            let spinner_id = state.read(cx).child_id("spinner");
            let owner = state.clone();
            let notice = div()
                .id(status_id)
                .test_support()
                .role(Role::Status)
                .aria_label(status.clone())
                .aria_value(status.clone())
                .flex()
                .items_center()
                .gap_2()
                .px_3()
                .py_2()
                .rounded(rems(theme, "--image-preview-radius"))
                .bg(gpui_kit::rgb(theme.surface()))
                .text_color(gpui_kit::rgb(theme.text_primary()))
                .text_sm()
                .when(busy, |d| {
                    d.child(
                        YueSpinner::new(spinner_id)
                            .size(FeedbackSize::Small)
                            .decorative(true)
                            .animated(self.animated),
                    )
                })
                .child(status)
                .when(error, |d| {
                    d.child(
                        YueButton::new(retry_id, "重试")
                            .size(ButtonSize::Small)
                            .disabled(disabled)
                            .on_click(move |_, _, cx| {
                                owner.update(cx, |s, cx| s.retry(cx));
                            }),
                    )
                });
            let notice = gpui_kit::base::A11yLive::new(notice, gpui_kit::accesskit::Live::Polite)
                .atomic(true)
                .busy(busy);
            viewport = viewport.child(div().absolute().top_3().left_3().right_3().child(notice));
        }
        let mut footer = div()
            .flex()
            .items_center()
            .justify_between()
            .flex_wrap()
            .gap_2()
            .text_sm()
            .text_color(gpui_kit::rgb(theme.text_secondary()));
        if let Some([width, height]) = source_size {
            footer = footer.child(div().child(format!("原始 {width} × {height}")));
        }
        if let Some([width, height]) = result_size {
            footer = footer.child(div().child(format!(
                "输出 {width} × {height}{}",
                if snap.result_revision() != Some(snap.revision()) {
                    " · 上一版"
                } else {
                    ""
                }
            )));
        }
        if self.background_picker {
            let value = state.read(cx).background;
            let owner = state.clone();
            let bg = YueSegmented::new(
                state.read(cx).child_id("background"),
                theme,
                bg_value(value),
            )
            .accessibility_label("预览背景")
            .size(SegmentedSize::Small)
            .disabled(disabled)
            .options(
                [
                    ImagePreviewBackground::CheckerDark,
                    ImagePreviewBackground::CheckerLight,
                    ImagePreviewBackground::White,
                    ImagePreviewBackground::Black,
                ]
                .map(|v| YueSegmentedOption::new(bg_value(v), v.label())),
            )
            .on_change(move |v, _, cx| {
                let bg = match v.as_ref() {
                    "light" => ImagePreviewBackground::CheckerLight,
                    "white" => ImagePreviewBackground::White,
                    "black" => ImagePreviewBackground::Black,
                    _ => ImagePreviewBackground::CheckerDark,
                };
                owner.update(cx, |s, cx| s.user_background(bg, cx));
            });
            footer = footer.child(bg);
        }
        if let Some(renderer) = self.footer {
            footer = footer.child(renderer(w, cx));
        }
        let mut outer = div()
            .id(id)
            .test_support()
            .size_full()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(rems(theme, "--image-preview-gap"));
        if self.modes {
            let owner = state.clone();
            outer = outer.child(
                YueSegmented::new(mode_id, theme, mode.value())
                    .size(SegmentedSize::Small)
                    .accessibility_label("预览模式")
                    .distribution(SegmentedDistribution::Equal)
                    .disabled(disabled)
                    .options([
                        YueSegmentedOption::new("source", "原图").disabled(!source_available),
                        YueSegmentedOption::new("compare", "对比").disabled(!compare_available),
                        YueSegmentedOption::new("result", self.result_label)
                            .disabled(!result_available),
                    ])
                    .on_change(move |v, w, cx| {
                        let mode = match v.as_ref() {
                            "compare" => ImagePreviewMode::Compare,
                            "result" => ImagePreviewMode::Result,
                            _ => ImagePreviewMode::Source,
                        };
                        owner.update(cx, |s, cx| s.user_mode(mode, w, cx));
                    }),
            );
        }
        if self.toolbar_visible {
            if self.toolbar == ImagePreviewToolbar::Floating {
                viewport = viewport.child(div().absolute().bottom_3().right_3().child(toolbar));
            } else {
                footer = footer.child(toolbar);
            }
        }
        outer.child(viewport).child(footer)
    }
}
fn bg_value(bg: ImagePreviewBackground) -> &'static str {
    match bg {
        ImagePreviewBackground::CheckerDark => "dark",
        ImagePreviewBackground::CheckerLight => "light",
        ImagePreviewBackground::White => "white",
        ImagePreviewBackground::Black => "black",
    }
}
fn zoom_toolbar(
    state: &Entity<YueImagePreviewState>,
    theme: crate::YueTheme,
    floating: bool,
    disabled: bool,
    percentage: String,
    zoom_enabled: [bool; 2],
    cx: &App,
) -> AnyElement {
    let controls_theme = if floating {
        crate::YueTheme::new(crate::ThemeMode::Dark, theme.accent()).with_neutral(theme.neutral())
    } else {
        theme
    };
    let id = |name| state.read(cx).child_id(name);
    let out = state.clone();
    let actual = state.clone();
    let inside = state.clone();
    let fit = state.clone();
    div()
        .flex()
        .items_center()
        .gap_1()
        .font_features(crate::tabular_nums())
        .p_1()
        .rounded(rems(theme, "--image-preview-radius"))
        .bg(gpui_kit::rgb(theme.surface()))
        .border_1()
        .border_color(gpui_kit::rgb(theme.border_default()))
        .when(floating, |el| {
            el.backdrop_blur(rems(theme, "--image-preview-glass-blur"))
                .bg(rgba(theme, "--image-preview-glass-background"))
                .border_color(rgba(theme, "--image-preview-glass-border"))
                .text_color(rgba(theme, "--image-preview-glass-color"))
                .shadow(shadow(theme, "--image-preview-glass-shadow"))
        })
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_scroll_wheel(|_, w, cx| {
            cx.stop_propagation();
            w.prevent_default();
        })
        .child(
            YueButton::icon(
                id("zoom-out"),
                controls_theme,
                crate::outline::ZoomOut,
                "缩小",
            )
            .size(ButtonSize::Small)
            .variant(ButtonVariant::Ghost)
            .disabled(disabled || !zoom_enabled[0])
            .tooltip("缩小（−）")
            .on_click(move |_, w, cx| {
                out.update(cx, |s, cx| {
                    s.user_zoom(
                        1. / 1.25,
                        s.viewport.midpoint(),
                        true,
                        ImagePreviewChangeSource::Toolbar,
                        w,
                        cx,
                    )
                })
            }),
        )
        .child(
            YueButton::new(id("actual-size"), percentage)
                .theme(controls_theme)
                .accessibility_label("回到100%")
                .size(ButtonSize::Small)
                .variant(ButtonVariant::Ghost)
                .width(gpui_kit::rems(4.5))
                .disabled(disabled)
                .tooltip("100%：一个图像像素对应一个屏幕物理像素（1）")
                .on_click(move |_, w, cx| {
                    actual.update(cx, |s, cx| {
                        s.user_fit(
                            ImagePreviewFit::Manual,
                            true,
                            ImagePreviewChangeSource::Toolbar,
                            w,
                            cx,
                        )
                    })
                }),
        )
        .child(
            YueButton::icon(
                id("zoom-in"),
                controls_theme,
                crate::outline::ZoomIn,
                "放大",
            )
            .size(ButtonSize::Small)
            .variant(ButtonVariant::Ghost)
            .disabled(disabled || !zoom_enabled[1])
            .tooltip("放大（+）")
            .on_click(move |_, w, cx| {
                inside.update(cx, |s, cx| {
                    s.user_zoom(
                        1.25,
                        s.viewport.midpoint(),
                        true,
                        ImagePreviewChangeSource::Toolbar,
                        w,
                        cx,
                    )
                })
            }),
        )
        .child(
            YueButton::icon(
                id("fit-all"),
                controls_theme,
                crate::outline::ArrowsMaximize,
                "适应窗口",
            )
            .size(ButtonSize::Small)
            .variant(ButtonVariant::Ghost)
            .disabled(disabled)
            .tooltip("适应窗口（0）")
            .on_click(move |_, w, cx| {
                fit.update(cx, |s, cx| {
                    s.user_fit(
                        ImagePreviewFit::FitAll,
                        false,
                        ImagePreviewChangeSource::Toolbar,
                        w,
                        cx,
                    )
                })
            }),
        )
        .into_any_element()
}
fn grip(
    state: &Entity<YueImagePreviewState>,
    theme: crate::YueTheme,
    w: &Window,
    cx: &App,
) -> AnyElement {
    let s = state.read(cx);
    let split = s.display_split(Instant::now());
    let value = s.split;
    let diameter = rems(theme, "--image-preview-grip-size").to_pixels(w.rem_size());
    let width = px(s.viewport.size[0]);
    let x = (width * split - diameter / 2.)
        .max(px(0.))
        .min((width - diameter).max(px(0.)));
    let id = s.child_id("split");
    let focus = s.grip_focus.clone();
    let keyboard = focus.is_focused(w) && w.last_input_was_keyboard();
    let down = state.clone();
    let key = state.clone();
    let increment = state.clone();
    let decrement = state.clone();
    let set_value = state.clone();
    div()
        .id(id)
        .test_support()
        .role(Role::Slider)
        .aria_label("对比分割线")
        .aria_min_numeric_value(0.)
        .aria_max_numeric_value(100.)
        .aria_numeric_value((value * 100.) as f64)
        .aria_numeric_value_step(2.)
        .aria_orientation(Orientation::Horizontal)
        .aria_value(format!("原图占比{:.0}%", value * 100.))
        .absolute()
        .left(x)
        .top(px((s.viewport.size[1] - f32::from(diameter)) / 2.))
        .size(diameter)
        .rounded_full()
        .backdrop_blur(rems(theme, "--image-preview-glass-blur"))
        .shadow(shadow(theme, "--image-preview-glass-shadow"))
        .border_1()
        .border_color(if keyboard {
            gpui_kit::rgb(theme.focus_ring())
        } else {
            rgba(theme, "--image-preview-grip-border")
        })
        .bg(rgba(theme, "--image-preview-grip-background"))
        .text_color(rgba(theme, "--image-preview-glass-color"))
        .flex()
        .items_center()
        .justify_center()
        .cursor_ew_resize()
        .track_focus(&focus)
        .key_context("YueImagePreviewSplit")
        .on_a11y_action(gpui_kit::accesskit::Action::Increment, move |_, w, cx| {
            increment.update(cx, |s, cx| {
                s.cancel_gesture(w);
                s.user_split(s.split + 0.02, ImagePreviewChangeSource::Accessibility, cx);
            });
        })
        .on_a11y_action(gpui_kit::accesskit::Action::Decrement, move |_, w, cx| {
            decrement.update(cx, |s, cx| {
                s.cancel_gesture(w);
                s.user_split(s.split - 0.02, ImagePreviewChangeSource::Accessibility, cx);
            });
        })
        .on_a11y_action(gpui_kit::accesskit::Action::SetValue, move |data, w, cx| {
            if let Some(gpui_kit::accesskit::ActionData::NumericValue(value)) = data {
                set_value.update(cx, |s, cx| {
                    if value.is_finite() {
                        s.cancel_gesture(w);
                        s.user_split(
                            (*value / 100.) as f32,
                            ImagePreviewChangeSource::Accessibility,
                            cx,
                        );
                    }
                });
            }
        })
        .child(
            crate::Icon::new(crate::outline::ArrowBarBoth)
                .size(crate::IconSize::Medium.rems(theme))
                .color(rgba(theme, "--image-preview-glass-color").into()),
        )
        .on_mouse_down(MouseButton::Left, move |e, w, cx| {
            down.update(cx, |s, cx| {
                if e.click_count == 2 {
                    s.cancel_gesture(w);
                    s.user_split(0.5, ImagePreviewChangeSource::Pointer, cx);
                } else if let Some(hit) = s.hitbox {
                    s.begin([e.position.x.into(), e.position.y.into()], true, hit, w, cx);
                }
            });
            cx.stop_propagation();
            w.prevent_default();
        })
        .on_key_down(move |e, w, cx| {
            if e.keystroke.modifiers.control
                || e.keystroke.modifiers.alt
                || e.keystroke.modifiers.platform
            {
                return;
            }
            let current = key.read(cx).split;
            let step = if e.keystroke.modifiers.shift {
                0.1
            } else {
                0.02
            };
            let next = match e.keystroke.key.as_str() {
                "left" => current - step,
                "right" => current + step,
                "home" => 0.,
                "end" => 1.,
                _ => return,
            };
            key.update(cx, |s, cx| {
                s.user_split(next, ImagePreviewChangeSource::Keyboard, cx)
            });
            cx.stop_propagation();
            w.prevent_default();
        })
        .into_any_element()
}
