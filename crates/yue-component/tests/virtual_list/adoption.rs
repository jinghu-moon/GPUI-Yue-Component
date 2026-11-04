use super::*;

#[test]
fn sparse_measurement_export_preserves_order_constraints_and_frozen_geometry() {
    let list = ListState::new_with_uniform_height(10_000, ListAlignment::Top, px(0.), px(20.));
    assert!(list.geometry().measured_items().is_empty());
    let updates = [
        (9999, gpui_kit::size(px(240.), px(37.))),
        (0, gpui_kit::size(px(240.), px(21.))),
        (501, gpui_kit::size(px(240.), px(23.))),
    ];
    list.set_item_sizes(&updates, true);
    let frozen = list.geometry();
    let expected = vec![updates[1], updates[2], updates[0]];
    assert_eq!(frozen.measured_items(), expected);
    list.remeasure_items(500..502);
    assert_eq!(
        list.geometry().measured_items(),
        vec![updates[1], updates[0]]
    );
    assert_eq!(frozen.measured_items(), expected);
    list.set_item_sizes(&[(700, gpui_kit::size(px(300.), px(24.)))], false);
    assert_eq!(
        list.geometry().measured_items(),
        vec![updates[1], updates[0]]
    );
}
