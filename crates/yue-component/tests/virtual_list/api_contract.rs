use super::*;
use gpui_kit::size;
use yue_component::{VirtualGrid, VirtualMasonryState};

#[test]
fn masonry_feedback_batches_preserve_atomicity_and_frozen_geometry_across_batch_shapes() {
    for count in [0, 1, 31, 32, 33, 256] {
        let mut state =
            VirtualMasonryState::new(source(0, 300), VirtualizerSizing::measured(px(20.)), 3)
                .unwrap();
        let frozen = state.snapshot(0., size(px(300.), px(200.))).unwrap();
        let mut updates = (0..count)
            .rev()
            .map(|ix| yue_component::VirtualizerSizeUpdate::new(ix, px(30.)))
            .collect::<Vec<_>>();
        state.update_sizes(&updates).unwrap();
        let next = state.snapshot(0., size(px(300.), px(200.))).unwrap();
        assert_eq!(next.revision(), frozen.revision() + u64::from(count > 0));
        for ix in 0..300 {
            assert_eq!(
                next.item(ix).unwrap().item().extent(),
                px(if ix < count { 30. } else { 20. })
            );
            assert_eq!(frozen.item(ix).unwrap().item().extent(), px(20.));
            assert_eq!(next.item(ix).unwrap().item().is_measured(), ix < count);
            assert!(!frozen.item(ix).unwrap().item().is_measured());
        }
        state.update_sizes(&updates).unwrap();
        assert_eq!(
            state
                .snapshot(0., size(px(300.), px(200.)))
                .unwrap()
                .revision(),
            next.revision()
        );
        updates.push(yue_component::VirtualizerSizeUpdate::new(300usize, px(40.)));
        assert_eq!(
            state.update_sizes(&updates),
            Err(VirtualizerError::MissingKey)
        );
        updates.pop();
        updates.push(yue_component::VirtualizerSizeUpdate::new(
            0usize,
            px(f32::NAN),
        ));
        assert!(state.update_sizes(&updates).is_err());
        updates.pop();
        if count > 0 {
            updates.push(yue_component::VirtualizerSizeUpdate::new(0usize, px(50.)));
            assert!(state.update_sizes(&updates).is_err());
        }
        assert_eq!(
            state
                .snapshot(0., size(px(300.), px(200.)))
                .unwrap()
                .revision(),
            next.revision()
        );
    }
}

#[test]
fn masonry_options_validate_before_replacing_the_active_command() {
    let mut state =
        VirtualMasonryState::new(source(0, 100), VirtualizerSizing::measured(px(20.)), 3).unwrap();
    let caps = state.capabilities();
    assert_eq!(caps.axis(), Axis::Vertical);
    assert!(!caps.supports_attached());
    assert!(caps.supports_follow_tail());
    assert!(!caps.supports_smooth());
    assert!(caps.supports_scroll_to_offset());
    let old = state
        .scroll_to_key(1usize.into(), VirtualizerAlign::End)
        .unwrap();
    let smooth = VirtualizerScrollOptions::default().motion(VirtualizerMotion::Smooth {
        duration: Duration::from_millis(100),
        max_distance: px(100.),
    });
    assert_eq!(
        state.scroll_to_key(
            2usize.into(),
            smooth.motion(VirtualizerMotion::Smooth {
                duration: Duration::from_millis(100),
                max_distance: px(f32::NAN),
            })
        ),
        Err(VirtualizerError::InvalidGeometry("smooth bounds"))
    );
    assert!(state.has_scroll_command());
    assert!(
        state.take_events().is_empty(),
        "failed request must not cancel old command"
    );
    let invalid = VirtualizerScrollOptions::default()
        .motion(VirtualizerMotion::Smooth {
            duration: Duration::ZERO,
            max_distance: px(100.),
        })
        .reduced_motion(true);
    assert_eq!(
        state.scroll_to_key(2usize.into(), invalid),
        Err(VirtualizerError::InvalidGeometry("smooth bounds"))
    );
    assert!(state.take_events().is_empty());
    assert_eq!(
        state.scroll_to_key(100usize.into(), VirtualizerScrollOptions::default()),
        Err(VirtualizerError::MissingKey)
    );
    assert!(state.take_events().is_empty());
    let next = state
        .scroll_to_key(2usize.into(), smooth.reduced_motion(true))
        .unwrap();
    assert_eq!(next, old + 1, "rejected commands must not consume IDs");
    assert_eq!(
        state.take_events(),
        vec![VirtualizerEvent::CommandFinished {
            id: old,
            outcome: VirtualizerCommandOutcome::Cancelled
        }]
    );
}

#[gpui_kit::test]
fn masonry_options_and_legacy_align_reach_the_same_target(cx: &mut TestAppContext) {
    use yue_component::VirtualMasonry;
    let mut offsets = Vec::new();
    for legacy in [true, false] {
        let state = cx.new(|_| {
            VirtualMasonryState::new(source(0, 100), VirtualizerSizing::fixed(px(20.)), 3).unwrap()
        });
        let (view, cx) = cx.add_window_view(|_, _| LayoutHarness {
            state: state.clone(),
        });
        cx.simulate_resize(size(px(300.), px(200.)));
        cx.update(|w, cx| w.draw(cx).clear(cx));
        state.update(cx, |s, cx| {
            if legacy {
                s.scroll_to_key(80usize.into(), VirtualizerAlign::Center)
                    .unwrap();
            } else {
                s.scroll_to_key(
                    80usize.into(),
                    VirtualizerScrollOptions::default().align(VirtualizerAlign::Center),
                )
                .unwrap();
            }
            cx.notify();
        });
        cx.update(|w, cx| w.draw(cx).clear(cx));
        offsets.push(state.read_with(cx, |s, _| s.scroll_handle().precise_offset().y));
        assert!(!state.read_with(cx, |s, _| s.has_scroll_command()));
        drop(view);
    }
    assert_eq!(offsets[0], offsets[1]);
    assert!(offsets[0] < -100.);
    struct LayoutHarness {
        state: Entity<VirtualMasonryState>,
    }
    impl Render for LayoutHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            VirtualMasonry::new("api-masonry", &self.state, |_, _, _| div().size_full())
        }
    }
}

#[test]
fn grid_preflight_rejects_unsupported_axes_without_source_reads_or_binding_changes() {
    let b = VirtualizerBinding::new();
    let rows = VirtualizerState::new_attached(
        Axis::Vertical,
        source(0, 10),
        VirtualizerSizing::fixed(px(20.)),
        &b,
    )
    .unwrap();
    let columns = VirtualizerState::new_attached(
        Axis::Horizontal,
        source(0, 10),
        VirtualizerSizing::fixed(px(30.)),
        &b,
    )
    .unwrap();
    assert_eq!(VirtualGrid::validate_axes(&rows, &columns), Ok(()));
    assert_eq!(
        VirtualGrid::validate_axes(&columns, &rows),
        Err(VirtualizerError::InvalidGeometry("grid axes"))
    );
    let owned = fixed(10);
    assert_eq!(
        VirtualGrid::validate_axes(&owned, &columns),
        Err(VirtualizerError::Unsupported("grid requires attached axes"))
    );
    let other = VirtualizerBinding::new();
    let unrelated = VirtualizerState::new_attached(
        Axis::Horizontal,
        source(0, 10),
        VirtualizerSizing::fixed(px(30.)),
        &other,
    )
    .unwrap();
    assert_eq!(
        VirtualGrid::validate_axes(&rows, &unrelated),
        Err(VirtualizerError::BindingConflict)
    );
    let measured_binding = VirtualizerBinding::new();
    let measured_source = source(0, 10);
    let dynamic = VirtualizerState::new_attached(
        Axis::Vertical,
        measured_source.clone(),
        VirtualizerSizing::measured(px(20.)),
        &measured_binding,
    )
    .unwrap();
    let fixed_columns = VirtualizerState::new_attached(
        Axis::Horizontal,
        source(0, 10),
        VirtualizerSizing::fixed(px(30.)),
        &measured_binding,
    )
    .unwrap();
    let before = measured_source.calls.get();
    assert_eq!(
        VirtualGrid::validate_axes(&dynamic, &fixed_columns),
        Err(VirtualizerError::Unsupported(
            "grid requires explicit row/column dimensions"
        ))
    );
    assert_eq!(measured_source.calls.get(), before);
    assert_eq!(VirtualGrid::validate_axes(&rows, &columns), Ok(()));
}
