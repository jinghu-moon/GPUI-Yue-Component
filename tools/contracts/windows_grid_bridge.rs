//! Injected only into a disposable copy of the production-pinned Windows bridge.
//! Direct COM provider contracts: no live HWND, UIA client, or desktop automation.
use crate::{
    context::{ActionHandlerNoMut, Context},
    window_handle::WindowHandle,
};
use accesskit::{Action, ActionRequest, Node, NodeId, Role, Tree, TreeUpdate};
use std::sync::{Arc, Mutex};
use windows::{
    Win32::{Foundation::HWND, System::Variant::*, UI::Accessibility::*},
    core::Interface,
};

#[derive(Default)]
struct Actions(Mutex<Vec<ActionRequest>>);
impl ActionHandlerNoMut for Actions {
    fn do_action(&self, request: ActionRequest) {
        self.0.lock().unwrap().push(request);
    }
}

fn fixture() -> (
    Arc<Context>,
    Arc<Actions>,
    IRawElementProviderSimple,
    IRawElementProviderSimple,
) {
    let mut grid = Node::new(Role::Grid);
    grid.set_label("million-row grid");
    grid.set_row_count(1_000_000);
    grid.set_column_count(1_000);
    grid.set_children(vec![NodeId(2)]);
    let mut cell = Node::new(Role::Cell);
    cell.set_label("43 x 10");
    cell.set_description("row 43, column 10; spans 2 rows, 3 columns");
    cell.set_row_index(42);
    cell.set_column_index(9);
    cell.set_row_span(2);
    cell.set_column_span(3);
    cell.add_action(Action::Focus);
    cell.add_action(Action::Click);
    let tree = accesskit_consumer::Tree::new(
        TreeUpdate {
            nodes: vec![(NodeId(1), grid), (NodeId(2), cell)],
            tree: Some(Tree::new(NodeId(1))),
            tree_id: accesskit::TreeId::ROOT,
            focus: NodeId(2),
        },
        true,
    );
    let actions = Arc::new(Actions::default());
    let context = Context::new(WindowHandle(HWND::default()), tree, actions.clone(), false);
    let (grid_id, cell_id) = {
        let tree = context.read_tree();
        (
            tree.state().root_id(),
            tree.state().root().child_ids().next().unwrap(),
        )
    };
    let grid = context
        .get_or_create_platform_node(grid_id)
        .into_interface();
    let cell = context
        .get_or_create_platform_node(cell_id)
        .into_interface();
    (context, actions, grid, cell)
}

fn integer(node: &IRawElementProviderSimple, property: UIA_PROPERTY_ID) -> i32 {
    unsafe {
        let mut value = node.GetPropertyValue(property).unwrap();
        let result = VariantToInt32(&value).unwrap();
        VariantClear(&mut value).unwrap();
        result
    }
}

fn string(node: &IRawElementProviderSimple, property: UIA_PROPERTY_ID) -> String {
    unsafe {
        let mut value = node.GetPropertyValue(property).unwrap();
        let mut buffer = [0u16; 512];
        VariantToString(&value, &mut buffer).unwrap();
        VariantClear(&mut value).unwrap();
        String::from_utf16(&buffer[..buffer.iter().position(|c| *c == 0).unwrap()]).unwrap()
    }
}

#[test]
fn grid_roles_names_and_full_description_reach_com() {
    let (_context, _, grid, cell) = fixture();
    assert_eq!(
        integer(&grid, UIA_ControlTypePropertyId),
        UIA_DataGridControlTypeId.0
    );
    assert_eq!(
        integer(&cell, UIA_ControlTypePropertyId),
        UIA_DataItemControlTypeId.0
    );
    assert_eq!(string(&grid, UIA_NamePropertyId), "million-row grid");
    assert_eq!(
        string(&cell, UIA_FullDescriptionPropertyId),
        "row 43, column 10; spans 2 rows, 3 columns"
    );
    // description is FullDescription, not HelpText (which maps placeholder).
    assert_eq!(string(&cell, UIA_HelpTextPropertyId), "");
}

#[test]
fn pinned_bridge_inventory_keeps_missing_grid_and_realization_explicit() {
    let (_context, _, grid, cell) = fixture();
    for (node, patterns) in [
        (
            &grid,
            &[
                UIA_GridPatternId,
                UIA_TablePatternId,
                UIA_ItemContainerPatternId,
            ][..],
        ),
        (
            &cell,
            &[
                UIA_GridItemPatternId,
                UIA_TableItemPatternId,
                UIA_VirtualizedItemPatternId,
            ][..],
        ),
    ] {
        for pattern in patterns {
            assert!(
                unsafe { node.GetPatternProvider(*pattern) }.is_err(),
                "bridge capability changed: review actual Grid/GetItem/Realize contracts before updating this inventory"
            );
        }
    }
    // Positive control: an arbitrary unavailable/dead provider cannot pass.
    assert!(unsafe { cell.GetPatternProvider(UIA_InvokePatternId) }.is_ok());
    println!(
        "WINDOWS_GRID_CAPABILITIES,Grid=false,GridItem=false,Table=false,TableItem=false,ItemContainer=false,VirtualizedItem=false,Invoke=true"
    );
}

#[test]
fn cell_invoke_and_focus_forward_real_action_requests() {
    let (_context, actions, _, cell) = fixture();
    unsafe {
        let invoke: IInvokeProvider = cell
            .GetPatternProvider(UIA_InvokePatternId)
            .unwrap()
            .cast()
            .unwrap();
        invoke.Invoke().unwrap();
        let fragment: IRawElementProviderFragment = cell.cast().unwrap();
        fragment.SetFocus().unwrap();
    }
    let actions = actions.0.lock().unwrap();
    assert_eq!(actions.len(), 2);
    assert_eq!(actions[0].action, Action::Click);
    assert_eq!(actions[1].action, Action::Focus);
    assert!(
        actions
            .iter()
            .all(|a| a.target_node == NodeId(2) && a.data.is_none())
    );
}
