//! Native event paths, camera ownership, revisions and focus isolation.
use gpui_base::test_support::{ElementSnapshot, snapshots};
use gpui_kit::{
    AppContext as _, Context, Entity, Image, ImageFormat, IntoElement, Modifiers, MouseButton,
    MouseDownEvent, MouseUpEvent, ParentElement as _, Point, Render, Role, ScrollDelta,
    ScrollWheelEvent, Styled as _, TestAppContext, VisualTestContext, Window, div, point, px,
};
use std::{cell::RefCell, rc::Rc};
use yue_component::{
    Accent, ImagePreviewDocument, ImagePreviewError, ImagePreviewEvent, ImagePreviewFit,
    ImagePreviewImage, ImagePreviewLoad, ImagePreviewMode, ImagePreviewSnapshot,
    ImagePreviewToolbar, InputState, NeutralTone, ThemeMode, YueButton, YueImagePreview,
    YueImagePreviewState, YueInput, YueTheme,
};

struct Probe {
    state: Entity<YueImagePreviewState>,
    image: ImagePreviewImage,
    show: bool,
    disabled: bool,
    events: Rc<RefCell<Vec<ImagePreviewEvent>>>,
    _subscription: gpui_kit::Subscription,
    input: Entity<InputState>,
    toolbar: ImagePreviewToolbar,
    preview_size: [f32; 2],
}
impl Probe {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|cx| YueImagePreviewState::new("test-preview", w, cx));
        let image = Image::from_bytes(
            ImageFormat::Png,
            include_bytes!(
                "../../../examples/windows-showcase/assets/image-preview/transparent.png"
            )
            .to_vec(),
        )
        .to_image_data(cx.svg_renderer())
        .unwrap();
        let image = ImagePreviewImage::new(image, "透明测试图片").unwrap();
        state.update(cx, |s, cx| {
            s.set_document(
                document(&image, 1, ImagePreviewLoad::Ready(image.clone())),
                w,
                cx,
            )
            .unwrap()
        });
        let events = Rc::new(RefCell::new(vec![]));
        let out = events.clone();
        let subscription = cx.subscribe(&state, move |_, _, event, cx| {
            out.borrow_mut().push(event.clone());
            cx.notify();
        });
        Self {
            state,
            image,
            show: true,
            disabled: false,
            events,
            _subscription: subscription,
            input: cx.new(|cx| InputState::new(w, cx)),
            toolbar: ImagePreviewToolbar::Floating,
            preview_size: [600., 380.],
        }
    }
}
fn document(
    image: &ImagePreviewImage,
    revision: u64,
    result: ImagePreviewLoad,
) -> ImagePreviewDocument {
    ImagePreviewDocument::new("fixture", revision)
        .with_source(ImagePreviewLoad::Ready(image.clone()))
        .with_result(result)
        .full_frame(true)
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .p_4()
            .gap_3()
            .child(YueButton::new("before-preview", "之前"))
            .child(if self.show {
                div()
                    .w(px(self.preview_size[0]))
                    .h(px(self.preview_size[1]))
                    .child(
                        YueImagePreview::new(&self.state)
                            .animated(false)
                            .disabled(self.disabled)
                            .toolbar(self.toolbar),
                    )
                    .into_any_element()
            } else {
                div().h(px(380.)).into_any_element()
            })
            .child(
                YueInput::new(
                    "other-input",
                    YueTheme::new(ThemeMode::Light, Accent::Azure),
                    &self.input,
                )
                .accessibility_label("旁边的输入框"),
            )
            .child(YueButton::new("after-preview", "之后"))
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
        yue_component::init(cx, YueTheme::new(ThemeMode::Light, Accent::Azure)).unwrap()
    });
    let mut view = None;
    let (_, cx) = cx.add_window_view(|w, cx| {
        let p = cx.new(|cx| Probe::new(w, cx));
        view = Some(p.clone());
        gpui_kit::base::Root::new(p, w, cx)
    });
    cx.update(|w, _| w.activate_window());
    cx.simulate_scale_factor_change(1.);
    draw(cx);
    draw(cx);
    (view.unwrap(), cx)
}
fn node(cx: &mut VisualTestContext, label: &str) -> ElementSnapshot {
    cx.update(|w, _| {
        snapshots(w)
            .into_iter()
            .find(|n| n.label() == Some(label))
            .unwrap_or_else(|| panic!("missing {label}"))
    })
}
fn snap(view: &Entity<Probe>, cx: &VisualTestContext) -> ImagePreviewSnapshot {
    view.read_with(cx, |p, cx| p.state.read(cx).snapshot())
}
fn click(cx: &mut VisualTestContext, label: &str) {
    let p = node(cx, label).bounds().center();
    cx.simulate_click(p, Modifiers::default());
    draw(cx);
}
fn down(cx: &mut VisualTestContext, p: Point<gpui_kit::Pixels>) {
    cx.simulate_event(MouseDownEvent {
        position: p,
        button: MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    draw(cx);
}
fn up(cx: &mut VisualTestContext, p: Point<gpui_kit::Pixels>) {
    cx.simulate_event(MouseUpEvent {
        position: p,
        button: MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    draw(cx);
}
#[gpui_kit::test]
fn checker_paints_only_complete_squares_when_the_viewport_changes(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    let theme = YueTheme::new(ThemeMode::Light, Accent::Azure);
    let color: gpui_kit::Background = theme
        .token("--image-preview-checker-dark-b")
        .unwrap()
        .rgba()
        .unwrap()
        .into();
    for scale in [1., 1.25, 1.5, 2.] {
        cx.simulate_scale_factor_change(scale);
        for size in [[600., 380.], [437.5, 301.25], [680., 440.]] {
            view.update(cx, |v, cx| {
                v.preview_size = size;
                cx.notify();
            });
            draw(cx);
            cx.update(|w, _| {
                let tiles: Vec<_> = w
                    .painted_quads()
                    .into_iter()
                    .filter(|q| q.background == color)
                    .collect();
                assert!(tiles.len() > 10, "production checker must paint its tiles");
                let side = tiles[0].bounds.size.width.as_f32();
                for tile in tiles {
                    assert!(
                        (tile.bounds.size.width.as_f32() - side).abs() < 0.001,
                        "scale={scale}, size={size:?}, expected side={side}, tile={tile:?}"
                    );
                    assert!((tile.bounds.size.height.as_f32() - side).abs() < 0.001);
                    assert_eq!(
                        tile.bounds.intersect(&tile.content_mask.bounds),
                        tile.bounds,
                        "no clipped edge/corner tile"
                    );
                }
            });
        }
    }
}
#[gpui_kit::test]
fn comparison_grip_paints_a_visible_icon_inside_the_circle(cx: &mut TestAppContext) {
    let (_, cx) = open(cx);
    click(cx, "对比");
    let grip = node(cx, "对比分割线").bounds();
    cx.update(|w, _| {
        let bounds = grip.scale(w.scale_factor());
        assert!(
            w.painted_primitives().iter().any(|p| {
                matches!(p, gpui_kit::Primitive::MonochromeSprite(s)
                if s.color.a > 0. && s.bounds.size.width.as_f32() > 0.
                    && bounds.intersect(&s.bounds) == s.bounds)
            }),
            "grip must contain a painted, visible icon"
        );
    });
}
#[gpui_kit::test]
fn toolbar_zoom_and_actual_size_use_production_geometry(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    let first = snap(&view, cx);
    assert!(first.zoom() > 0. && first.zoom() < 1.);
    assert!(node(cx, "图片预览").bounds().size.height > px(100.));
    click(cx, "放大");
    assert!((snap(&view, cx).zoom() - first.zoom() * 1.25).abs() < 0.001);
    click(cx, "回到100%");
    assert_eq!(snap(&view, cx).zoom(), 1.);
    click(cx, "适应窗口");
    assert!((snap(&view, cx).zoom() - first.zoom()).abs() < 0.001);
}
#[gpui_kit::test]
fn mode_and_split_keyboard_keep_one_camera_and_restore_middle(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    let first = snap(&view, cx);
    click(cx, "对比");
    assert_eq!(snap(&view, cx).mode(), ImagePreviewMode::Compare);
    let grip = node(cx, "对比分割线");
    assert_eq!(grip.role(), Some(Role::Slider));
    click(cx, "对比分割线");
    cx.simulate_keystrokes("right");
    draw(cx);
    assert!((snap(&view, cx).split() - 0.52).abs() < 0.001);
    click(cx, "处理后");
    click(cx, "对比");
    assert!((snap(&view, cx).split() - 0.52).abs() < 0.001);
    assert_eq!(snap(&view, cx).center(), first.center());
    assert_eq!(snap(&view, cx).zoom(), first.zoom());
}
#[gpui_kit::test]
fn dragging_split_does_not_pan_and_releases_capture(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    click(cx, "对比");
    let before = snap(&view, cx);
    let grip = node(cx, "对比分割线").bounds().center();
    down(cx, grip);
    let p = grip + point(px(90.), px(30.));
    cx.simulate_mouse_move(p, Some(MouseButton::Left), Modifiers::default());
    draw(cx);
    assert!(snap(&view, cx).split() > before.split());
    assert_eq!(snap(&view, cx).center(), before.center());
    up(cx, p);
    assert!(cx.update(|w, _| w.captured_hitbox().is_none()));
}
#[gpui_kit::test]
fn fitted_image_can_be_dragged_in_every_mode_and_double_click_resets(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    for label in ["原图", "对比", "处理后"] {
        click(cx, label);
        click(cx, "适应窗口");
        let before = snap(&view, cx);
        assert_eq!(before.fit(), ImagePreviewFit::FitAll);
        // Away from the comparison divider and floating toolbar, on the image.
        let b = node(cx, "图片预览").bounds();
        let p = b.center() - point(px(90.), px(30.));
        down(cx, p);
        let delta = point(px(36.), px(24.));
        cx.simulate_mouse_move(p + delta, Some(MouseButton::Left), Modifiers::default());
        draw(cx);
        let after = snap(&view, cx);
        assert_ne!(
            after.center()[0],
            before.center()[0],
            "{label}: horizontal pan"
        );
        assert_ne!(
            after.center()[1],
            before.center()[1],
            "{label}: vertical pan"
        );
        assert_eq!(after.fit(), ImagePreviewFit::Manual);
        assert_eq!(after.zoom(), before.zoom());
        assert_eq!(after.split(), before.split());
        up(cx, p + delta);
        assert!(cx.update(|w, _| w.captured_hitbox().is_none()));
        cx.simulate_mouse_move(p, None, Modifiers::default());
        draw(cx);
        assert_eq!(
            snap(&view, cx),
            after,
            "released image must stop following pointer"
        );
        cx.simulate_event(MouseDownEvent {
            position: p,
            button: MouseButton::Left,
            click_count: 2,
            ..Default::default()
        });
        up(cx, p);
        assert_eq!(snap(&view, cx).fit(), ImagePreviewFit::FitAll);
        assert_eq!(snap(&view, cx).center(), before.center());
    }
}
#[gpui_kit::test]
fn image_drag_can_continue_outside_viewport(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    click(cx, "回到100%");
    click(cx, "放大");
    let before = snap(&view, cx);
    let p = node(cx, "图片预览").bounds().center();
    down(cx, p);
    let out = p + point(px(-800.), px(-500.));
    cx.simulate_mouse_move(out, Some(MouseButton::Left), Modifiers::default());
    draw(cx);
    up(cx, out);
    assert_ne!(snap(&view, cx).center(), before.center());
    assert!(cx.update(|w, _| w.captured_hitbox().is_none()));
}
#[gpui_kit::test]
fn wheel_changes_zoom_and_does_not_move_other_regions(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    let before = snap(&view, cx);
    let b = node(cx, "图片预览").bounds();
    let p = b.origin + point(px(80.), px(100.));
    cx.simulate_mouse_move(p, None, Modifiers::default());
    cx.simulate_event(ScrollWheelEvent {
        position: p,
        delta: ScrollDelta::Pixels(point(px(0.), px(-80.))),
        ..Default::default()
    });
    draw(cx);
    assert!(snap(&view, cx).zoom() > before.zoom());
    let after = snap(&view, cx);
    let p = node(cx, "之后").bounds().center();
    cx.simulate_mouse_move(p, None, Modifiers::default());
    cx.simulate_event(ScrollWheelEvent {
        position: p,
        delta: ScrollDelta::Lines(point(0., -3.)),
        ..Default::default()
    });
    draw(cx);
    assert_eq!(snap(&view, cx), after);
}
#[gpui_kit::test]
fn disabled_view_rejects_buttons_wheel_and_drag(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    view.update(cx, |p, cx| {
        p.disabled = true;
        cx.notify();
    });
    draw(cx);
    let before = snap(&view, cx);
    click(cx, "放大");
    let p = node(cx, "图片预览").bounds().center();
    down(cx, p);
    cx.simulate_mouse_move(
        p + point(px(60.), px(50.)),
        Some(MouseButton::Left),
        Modifiers::default(),
    );
    up(cx, p);
    assert_eq!(snap(&view, cx), before);
    assert!(view.read_with(cx, |p, _| p.events.borrow().is_empty()));
}
#[gpui_kit::test]
fn update_failure_retains_view_and_previous_revision(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    click(cx, "对比");
    click(cx, "回到100%");
    let before = snap(&view, cx);
    cx.update(|w, cx| {
        view.update(cx, |p, cx| {
            let d = document(&p.image, 2, ImagePreviewLoad::Loading);
            p.state
                .update(cx, |s, cx| s.set_document(d, w, cx).unwrap());
        })
    });
    draw(cx);
    let loading = snap(&view, cx);
    assert_eq!(loading.zoom(), before.zoom());
    assert_eq!(loading.result_revision(), Some(1));
    assert!(node(cx, "正在更新 · 当前为上一版").role() == Some(Role::Status));
    cx.update(|w, cx| {
        view.update(cx, |p, cx| {
            let d = document(&p.image, 2, ImagePreviewLoad::Failed("失败".into()));
            p.state
                .update(cx, |s, cx| s.set_document(d, w, cx).unwrap());
        })
    });
    draw(cx);
    assert_eq!(snap(&view, cx).result_revision(), Some(1));
    assert_eq!(snap(&view, cx).center(), before.center());
    click(cx, "重试");
    assert!(
        view.read_with(cx, |p, _| p.events.borrow().iter().any(|e| matches!(
            e,
            ImagePreviewEvent::RetryRequested {
                revision: 2,
                result: true,
                ..
            }
        )))
    );
}
#[gpui_kit::test]
fn stale_document_is_rejected_atomically_and_setters_do_not_echo(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    cx.update(|w, cx| {
        view.update(cx, |p, cx| {
            let d = document(&p.image, 3, ImagePreviewLoad::Ready(p.image.clone()));
            p.state
                .update(cx, |s, cx| s.set_document(d, w, cx).unwrap());
            let stale = document(&p.image, 2, ImagePreviewLoad::Empty);
            assert_eq!(
                p.state.update(cx, |s, cx| s.set_document(stale, w, cx)),
                Err(ImagePreviewError::StaleRevision)
            );
            p.state.update(cx, |s, cx| s.actual_size(w, cx).unwrap());
        })
    });
    draw(cx);
    assert_eq!(snap(&view, cx).revision(), 3);
    assert_eq!(snap(&view, cx).result_revision(), Some(3));
    assert!(view.read_with(cx, |p, _| p.events.borrow().is_empty()));
}
#[gpui_kit::test]
fn unmount_during_drag_releases_pointer_capture(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    let p = node(cx, "图片预览").bounds().center();
    down(cx, p);
    assert!(cx.update(|w, _| w.captured_hitbox().is_some()));
    view.update(cx, |p, cx| {
        p.show = false;
        cx.notify();
    });
    draw(cx);
    draw(cx);
    assert!(cx.update(|w, _| w.captured_hitbox().is_none()));
}
#[gpui_kit::test]
fn dpi_change_preserves_physical_zoom_and_footer_toolbar_does_not_overlap(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    click(cx, "回到100%");
    cx.simulate_scale_factor_change(1.5);
    draw(cx);
    assert_eq!(snap(&view, cx).zoom(), 1.);
    view.update(cx, |p, cx| {
        p.toolbar = ImagePreviewToolbar::Footer;
        cx.notify();
    });
    draw(cx);
    let preview = node(cx, "图片预览").bounds();
    let zoom = node(cx, "放大").bounds();
    assert!(zoom.top() >= preview.bottom());
}

#[gpui_kit::test]
fn keyboard_shortcuts_only_apply_when_the_viewport_itself_has_focus(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    let p = node(cx, "图片预览").bounds().center();
    cx.simulate_click(p, Modifiers::default());
    cx.simulate_keystrokes("1");
    draw(cx);
    assert_eq!(snap(&view, cx).zoom(), 1.);
    click(cx, "之后");
    let before = snap(&view, cx);
    cx.simulate_keystrokes("0");
    draw(cx);
    assert_eq!(snap(&view, cx), before);
    click(cx, "旁边的输入框");
    cx.simulate_keystrokes("0");
    draw(cx);
    assert_eq!(snap(&view, cx), before);
    assert_eq!(
        view.read_with(cx, |p, cx| p.input.read(cx).value().to_string()),
        "0"
    );
}

#[gpui_kit::test]
fn identical_document_does_not_interrupt_active_drag(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    click(cx, "回到100%");
    click(cx, "放大");
    let p = node(cx, "图片预览").bounds().center();
    down(cx, p);
    let capture = cx.update(|w, _| w.captured_hitbox());
    assert!(capture.is_some());
    cx.update(|w, cx| {
        view.update(cx, |p, cx| {
            let doc = document(&p.image, 1, ImagePreviewLoad::Ready(p.image.clone()));
            p.state
                .update(cx, |s, cx| s.set_document(doc, w, cx).unwrap());
        })
    });
    assert_eq!(cx.update(|w, _| w.captured_hitbox()), capture);
    let before = snap(&view, cx);
    cx.simulate_mouse_move(
        p + point(px(-60.), px(-50.)),
        Some(MouseButton::Left),
        Modifiers::default(),
    );
    draw(cx);
    assert_ne!(snap(&view, cx).center(), before.center());
    up(cx, p);
}

#[gpui_kit::test]
fn new_document_resets_view_while_updates_keep_the_manual_camera(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    click(cx, "对比");
    click(cx, "回到100%");
    assert_eq!(snap(&view, cx).fit(), ImagePreviewFit::Manual);
    cx.update(|w, cx| {
        view.update(cx, |p, cx| {
            let doc = ImagePreviewDocument::new("second-image", 1)
                .with_source(ImagePreviewLoad::Ready(p.image.clone()));
            p.state
                .update(cx, |s, cx| s.set_document(doc, w, cx).unwrap());
        })
    });
    draw(cx);
    let next = snap(&view, cx);
    assert_eq!(next.mode(), ImagePreviewMode::Source);
    assert_eq!(next.fit(), ImagePreviewFit::FitAll);
    assert_eq!(next.center(), [0.5, 0.5]);
    assert!(next.zoom() < 1.);
    assert_eq!(next.result_revision(), None);
}

#[gpui_kit::test]
fn half_resolution_result_converts_zoom_without_changing_view_center(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    cx.update(|w, cx| {
        view.update(cx, |p, cx| {
            let decode = |bytes: &[u8], label| {
                ImagePreviewImage::new(
                    Image::from_bytes(ImageFormat::Jpeg, bytes.to_vec())
                        .to_image_data(cx.svg_renderer())
                        .unwrap(),
                    label,
                )
                .unwrap()
            };
            let source = decode(
                include_bytes!(
                    "../../../examples/windows-showcase/assets/image-preview/desktop-q70.jpg"
                ),
                "原图",
            );
            let result = decode(
                include_bytes!(
                    "../../../examples/windows-showcase/assets/image-preview/desktop-half-q70.jpg"
                ),
                "半尺寸",
            );
            let doc = ImagePreviewDocument::new("half-resolution", 1)
                .with_source(ImagePreviewLoad::Ready(source))
                .with_result(ImagePreviewLoad::Ready(result))
                .full_frame(true);
            p.state
                .update(cx, |s, cx| s.set_document(doc, w, cx).unwrap());
        })
    });
    draw(cx);
    click(cx, "回到100%");
    let source = snap(&view, cx);
    assert_eq!(source.reference_size(), Some([3840, 2088]));
    click(cx, "对比");
    let result = snap(&view, cx);
    assert_eq!(result.reference_size(), Some([1920, 1044]));
    assert_eq!(result.zoom(), source.zoom() * 2.);
    assert_eq!(result.center(), source.center());
    click(cx, "原图");
    assert_eq!(snap(&view, cx).zoom(), source.zoom());
}

#[gpui_kit::test]
fn leaving_compare_transfers_removed_grip_focus_to_viewport(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    click(cx, "对比");
    click(cx, "对比分割线");
    cx.update(|w, cx| {
        view.update(cx, |p, cx| {
            p.state.update(cx, |s, cx| {
                s.set_mode(ImagePreviewMode::Result, w, cx).unwrap()
            });
        })
    });
    draw(cx);
    assert_eq!(node(cx, "图片预览").focused(), Some(true));
    cx.simulate_keystrokes("1");
    draw(cx);
    assert_eq!(snap(&view, cx).zoom(), 1.);
}

#[gpui_kit::test]
fn newly_declared_alignment_cannot_relabel_an_unaligned_previous_result(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    cx.update(|w, cx| {
        view.update(cx, |p, cx| {
            let unaligned =
                document(&p.image, 2, ImagePreviewLoad::Ready(p.image.clone())).full_frame(false);
            p.state.update(cx, |s, cx| {
                s.set_document(unaligned, w, cx).unwrap();
                assert!(!s.is_mode_available(ImagePreviewMode::Compare));
                s.set_mode(ImagePreviewMode::Result, w, cx).unwrap();
                let updating = document(&p.image, 3, ImagePreviewLoad::Loading).full_frame(true);
                s.set_document(updating, w, cx).unwrap();
                assert!(!s.is_mode_available(ImagePreviewMode::Compare));
                assert!(!s.is_mode_available(ImagePreviewMode::Result));
            });
        })
    });
    draw(cx);
    assert_eq!(snap(&view, cx).mode(), ImagePreviewMode::Source);
    assert_eq!(snap(&view, cx).result_revision(), None);
}

#[gpui_kit::test]
fn floating_toolbar_and_compare_grip_reuse_core_blur_footer_stays_solid(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    let filters = |cx: &mut VisualTestContext| {
        cx.update(|w, _| {
            w.painted_primitives()
                .into_iter()
                .filter_map(|p| {
                    if let gpui_kit::Primitive::BackdropBlur(b) = p {
                        Some(b)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        })
    };
    let initial = filters(cx);
    assert_eq!(
        initial.len(),
        1,
        "Floating toolbar owns one filter, not one per button"
    );
    assert_eq!(initial[0].sigma, gpui_kit::ScaledPixels(16.));
    click(cx, "对比");
    let compare = filters(cx);
    assert_eq!(compare.len(), 2);
    let grip = node(cx, "对比分割线").bounds();
    assert!(
        compare.iter().any(|b| {
            b.quad.bounds.size.width.0 == f32::from(grip.size.width)
                && b.quad.corner_radii.top_left.0 == f32::from(grip.size.width) / 2.
        }),
        "Circular handle uses its actual rounded output geometry"
    );
    view.update(cx, |p, cx| {
        p.toolbar = ImagePreviewToolbar::Footer;
        cx.notify();
    });
    draw(cx);
    assert_eq!(
        filters(cx).len(),
        1,
        "Only the comparison grip filters the image with a footer toolbar"
    );
    click(cx, "原图");
    assert!(filters(cx).is_empty());
}

#[test]
fn glass_tokens_keep_readable_foreground_over_the_brightest_background() {
    fn luminance(v: gpui_kit::Rgba) -> f32 {
        let linear = |c: f32| {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(v.r) + 0.7152 * linear(v.g) + 0.0722 * linear(v.b)
    }
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        for accent in [Accent::Azure, Accent::Neutral] {
            for tone in NeutralTone::ALL {
                let theme = YueTheme::new(mode, accent).with_neutral(tone);
                let tint = theme
                    .token("--image-preview-glass-background")
                    .unwrap()
                    .rgba()
                    .unwrap();
                let foreground = theme
                    .token("--image-preview-glass-color")
                    .unwrap()
                    .rgba()
                    .unwrap();
                let white_image = gpui_kit::Rgba {
                    r: tint.r * tint.a + 1. - tint.a,
                    g: tint.g * tint.a + 1. - tint.a,
                    b: tint.b * tint.a + 1. - tint.a,
                    a: 1.,
                };
                let contrast = (luminance(foreground) + 0.05) / (luminance(white_image) + 0.05);
                assert!(contrast >= 4.5, "{mode:?}/{accent:?}/{tone:?}: {contrast}");
            }
        }
    }
}
