use super::*;

#[test]
fn contiguous_and_sparse_working_items_match_the_sorted_union_on_both_axes() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        for known in [false, true] {
            let mut state = VirtualizerState::new(
                axis,
                source(0, 100_000),
                if known {
                    VirtualizerSizing::known(vec![px(20.); 100_000])
                } else {
                    VirtualizerSizing::fixed(px(20.))
                },
            )
            .unwrap();
            for keys in [
                vec![],
                vec![0usize.into(), 99_999usize.into(), 51usize.into()],
            ] {
                state.set_retained_keys(keys).unwrap();
                for offset in [0., 1000., 1_999_900.] {
                    let viewport = VirtualizerViewport::new(
                        state.binding().unwrap(),
                        1,
                        offset,
                        px(100.),
                        px(240.),
                    );
                    let snapshot = state.snapshot(&viewport).unwrap();
                    let actual: Vec<_> = snapshot
                        .working_items()
                        .map(|item| item.unwrap().index())
                        .collect();
                    assert_eq!(actual, snapshot.working_indices());
                    assert!(actual.windows(2).all(|pair| pair[0] < pair[1]));
                    assert!(actual.len() <= 9);
                }
            }
        }
    }
}

#[gpui_kit::test]
fn masonry_final_frame_metadata_is_prepared_once(cx: &mut TestAppContext) {
    struct Harness {
        state: Entity<yue_component::VirtualMasonryState>,
        builds: Rc<Cell<usize>>,
    }
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let builds = self.builds.clone();
            yue_component::VirtualMasonry::new("metadata-once", &self.state, move |_, _, _| {
                builds.set(builds.get() + 1);
                div().h(px(40.))
            })
            .w(px(300.))
            .h(px(100.))
        }
    }
    let source = source(0, 10_000);
    let builds = Rc::new(Cell::new(0));
    let (_, cx) = cx.add_window_view(|_, cx| Harness {
        state: cx.new(|_| {
            yue_component::VirtualMasonryState::new(
                source.clone(),
                VirtualizerSizing::measured(px(40.)),
                3,
            )
            .unwrap()
        }),
        builds: builds.clone(),
    });
    for _ in 0..3 {
        cx.update(|w, cx| {
            w.refresh();
            w.draw(cx).clear(cx)
        });
    }
    source.calls.set(0);
    builds.set(0);
    cx.update(|w, cx| {
        w.refresh();
        w.draw(cx).clear(cx)
    });
    assert_eq!(builds.get(), 9);
    assert_eq!(source.calls.get(), builds.get() * 4);
}

#[test]
fn masonry_noop_batches_validate_duplicates_and_fail_atomically() {
    use yue_component::{VirtualMasonryState, VirtualizerSizeUpdate};
    let mut state =
        VirtualMasonryState::new(source(0, 100), VirtualizerSizing::measured(px(40.)), 3).unwrap();
    state
        .update_sizes(&[VirtualizerSizeUpdate::new(0usize, px(40.))])
        .unwrap();
    let before = state
        .snapshot(0., gpui_kit::size(px(300.), px(100.)))
        .unwrap();
    state
        .update_sizes(&[VirtualizerSizeUpdate::new(0usize, px(40.))])
        .unwrap();
    for batch in [
        vec![
            VirtualizerSizeUpdate::new(0usize, px(40.)),
            VirtualizerSizeUpdate::new(0usize, px(40.)),
        ],
        vec![
            VirtualizerSizeUpdate::new(0usize, px(80.)),
            VirtualizerSizeUpdate::new(1usize, px(0.)),
        ],
    ] {
        assert!(state.update_sizes(&batch).is_err());
    }
    let after = state
        .snapshot(0., gpui_kit::size(px(300.), px(100.)))
        .unwrap();
    assert_eq!(before.revision(), after.revision());
    assert_eq!(after.item(0).unwrap().item().extent(), px(40.));
}
