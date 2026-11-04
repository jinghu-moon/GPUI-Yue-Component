//! Exact external feedback must not turn per-item tolerances into scroll drift.
use super::*;
use gpui_kit::size;
use yue_component::{VirtualMasonryState, VirtualizerSizeUpdate};

#[test]
fn fractional_feedback_preserves_cumulative_geometry_and_frozen_snapshots() {
    let count = 8_192;
    let old_extent = px(20.);
    let new_extent = px(20. + 1. / 2_048.);
    for axis in [Axis::Vertical, Axis::Horizontal] {
        for measured in [false, true] {
            let sizing = if measured {
                VirtualizerSizing::measured(old_extent)
            } else {
                VirtualizerSizing::known(vec![old_extent; count])
            };
            let mut state = VirtualizerState::new(axis, source(0, count), sizing).unwrap();
            state.commit(&viewport(&state, 1, 0.)).unwrap();
            let updates = (0..count)
                .map(|ix| VirtualizerSizeUpdate::new(ix, old_extent))
                .collect::<Vec<_>>();
            state.update_sizes(&updates).unwrap();
            let offset = (count / 2) as f64 * f64::from(old_extent) + 5.;
            state.commit(&viewport(&state, 2, offset)).unwrap();
            let frozen = state.snapshot(&viewport(&state, 2, offset)).unwrap();
            let updates = (0..count)
                .rev()
                .map(|ix| VirtualizerSizeUpdate::new(ix, new_extent))
                .collect::<Vec<_>>();
            state.update_sizes(&updates).unwrap();
            let next = state.snapshot(&viewport(&state, 3, offset)).unwrap();
            assert_eq!(next.total_extent(), count as f64 * f64::from(new_extent));
            assert_eq!(frozen.total_extent(), count as f64 * f64::from(old_extent));
            for ix in [0, 127, 128, count / 2, count - 1] {
                assert_eq!(next.item(ix).unwrap().extent(), new_extent);
                assert_eq!(
                    next.item(ix).unwrap().start(),
                    ix as f64 * f64::from(new_extent)
                );
                assert_eq!(frozen.item(ix).unwrap().extent(), old_extent);
            }
            assert_eq!(
                state.validate_snapshot(&frozen),
                Err(VirtualizerError::StaleSnapshot)
            );
            state.update_sizes(&updates).unwrap();
            state.validate_snapshot(&next).unwrap();
            let (_, request) = state
                .prepare(&viewport(&state, 3, offset), Instant::now())
                .unwrap();
            assert_eq!(
                request.unwrap().offset(),
                (count / 2) as f64 * f64::from(new_extent) + 5.
            );
        }
    }
}

#[test]
fn fractional_hints_and_padded_feedback_are_idempotent_in_native_slot_units() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        for gap in [px(0.), px(0.1)] {
            let mut state =
                VirtualizerState::new(axis, source(0, 3), VirtualizerSizing::measured(px(20.)))
                    .unwrap();
            state
                .set_layout(VirtualizerLayout::default().gap(gap))
                .unwrap();
            state.commit(&viewport(&state, 1, 0.)).unwrap();
            let initial = [VirtualizerSizeUpdate::new(0usize, px(20.))];
            state.update_size_hints(&initial).unwrap();
            let old = state.snapshot(&viewport(&state, 1, 0.)).unwrap();
            let extent = px(20. + 1. / 2_048.);
            let changed = [VirtualizerSizeUpdate::new(0usize, extent)];
            state.update_size_hints(&changed).unwrap();
            assert_eq!(
                state.validate_snapshot(&old),
                Err(VirtualizerError::StaleSnapshot)
            );
            let next = state.snapshot(&viewport(&state, 1, 0.)).unwrap();
            assert_eq!(next.item(1).unwrap().start(), f64::from(extent + gap));
            assert!(!next.item(0).unwrap().is_measured());
            state.update_size_hints(&changed).unwrap();
            state.validate_snapshot(&next).unwrap();
            state.update_sizes(&changed).unwrap();
            let actual = state.snapshot(&viewport(&state, 1, 0.)).unwrap();
            assert!(actual.item(0).unwrap().is_measured());
            state.update_sizes(&changed).unwrap();
            state.validate_snapshot(&actual).unwrap();
        }
    }
}

#[test]
fn masonry_fractional_feedback_updates_singletons_and_batches_without_lane_drift() {
    let count = 4_096;
    let old_extent = px(20.);
    let new_extent = px(20. + 1. / 256.);
    let mut state =
        VirtualMasonryState::new(source(0, count), VirtualizerSizing::measured(old_extent), 3)
            .unwrap();
    let bounds = size(px(300.), px(100.));
    let updates = (0..count)
        .map(|ix| VirtualizerSizeUpdate::new(ix, old_extent))
        .collect::<Vec<_>>();
    state.update_sizes(&updates).unwrap();
    let frozen = state.snapshot(0., bounds).unwrap();
    let singleton = [VirtualizerSizeUpdate::new(0usize, new_extent)];
    state.update_sizes(&singleton).unwrap();
    let single = state.snapshot(0., bounds).unwrap();
    assert_eq!(single.item(0).unwrap().item().extent(), new_extent);
    assert_eq!(single.revision(), frozen.revision() + 1);
    state.update_sizes(&singleton).unwrap();
    assert_eq!(
        state.snapshot(0., bounds).unwrap().revision(),
        single.revision()
    );
    let updates = (1..count)
        .rev()
        .map(|ix| VirtualizerSizeUpdate::new(ix, new_extent))
        .collect::<Vec<_>>();
    state.update_sizes(&updates).unwrap();
    let next = state.snapshot(0., bounds).unwrap();
    assert_eq!(next.revision(), single.revision() + 1);
    assert_eq!(
        next.total_extent(),
        count.div_ceil(3) as f64 * f64::from(new_extent)
    );
    assert_eq!(
        frozen.total_extent(),
        count.div_ceil(3) as f64 * f64::from(old_extent)
    );
    for ix in [0, 127, 128, count - 1] {
        assert_eq!(next.item(ix).unwrap().item().extent(), new_extent);
        assert_eq!(
            next.item(ix).unwrap().lane(),
            frozen.item(ix).unwrap().lane()
        );
        assert_eq!(frozen.item(ix).unwrap().item().extent(), old_extent);
    }
    state.update_sizes(&updates).unwrap();
    assert_eq!(
        state.snapshot(0., bounds).unwrap().revision(),
        next.revision()
    );
}
