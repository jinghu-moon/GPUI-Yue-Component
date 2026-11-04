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
