use super::*;
use yue_component::{TableOutline, TableOutlineSource, TablePage, TreeApply, TreeNode};
fn key(value: &str) -> TableRowKey {
    TableRowKey::try_new(value.to_owned()).unwrap()
}
#[test]
fn outline_reuses_base_tree_root_pages_keeps_paths_and_separates_detail_identity() {
    let source = source(1, &["a", "b", "c", "d", "e"]);
    let mut outline = TableOutline::new(source.clone());
    outline
        .set_tree(vec![
            TreeNode::new("a", "Parent")
                .children([TreeNode::new("b", "Match"), TreeNode::new("c", "Child")]),
            TreeNode::new("d", "Other").unloaded(),
        ])
        .unwrap();
    outline
        .set_details([(key("a"), key("detail-a")), (key("b"), key("detail-b"))])
        .unwrap();
    let closed = outline
        .snapshot(1, Some(TablePage::new(1, 1).unwrap()))
        .unwrap();
    assert_eq!(closed.len(), 1);
    assert_eq!(closed.root_total(), 2);
    outline.toggle_tree(&key("a")).unwrap();
    outline.toggle_detail(&key("b")).unwrap();
    let open = outline
        .snapshot(2, Some(TablePage::new(1, 1).unwrap()))
        .unwrap();
    assert_eq!((open.page_len(), open.record_len(), open.len()), (1, 3, 4));
    assert_eq!(
        (0..4)
            .map(|ix| open.row_key(ix).unwrap().as_str())
            .collect::<Vec<_>>(),
        ["a", "b", "detail-b", "c"]
    );
    assert_eq!(open.row(&key("b")).unwrap().depth(), 1);
    assert!(!open.contains_key(&key("detail-b")));
    assert!(std::ptr::eq(
        open.record(open.row(&key("b")).unwrap()).unwrap(),
        source.row(&key("b")).unwrap()
    ));
    let second = outline
        .snapshot(3, Some(TablePage::new(2, 1).unwrap()))
        .unwrap();
    assert_eq!(second.row_key(0).unwrap().as_str(), "d");
    outline.filter_tree("Match", None).unwrap();
    let filtered = outline.snapshot(4, None).unwrap();
    assert_eq!(
        (0..filtered.len())
            .map(|ix| filtered.row_key(ix).unwrap().as_str())
            .collect::<Vec<_>>(),
        ["a", "b", "detail-b"]
    );
    outline.filter_tree("", None).unwrap();
    assert!(outline.set_details([(key("a"), key("c"))]).is_err());
    assert!(
        outline
            .snapshot(5, None)
            .unwrap()
            .row(&key("detail-b"))
            .is_some()
    );
}
#[test]
fn outline_lazy_children_retry_and_replaced_owner_reject_old_completion() {
    let mut outline = TableOutline::new(source(1, &["a", "b"]));
    outline
        .set_tree(vec![TreeNode::new("a", "A").unloaded()])
        .unwrap();
    let first = outline.request_children(&key("a")).unwrap().unwrap();
    assert!(outline.request_children(&key("a")).unwrap().is_none());
    assert_eq!(
        outline.reject_children(&first, "failure".into()).unwrap(),
        TreeApply::Applied
    );
    let second = outline.request_children(&key("a")).unwrap().unwrap();
    assert_eq!(
        outline
            .apply_children(
                &first,
                vec![TreeNode::new("b", "B")],
                source(2, &["a", "b"])
            )
            .unwrap(),
        TreeApply::Stale
    );
    assert_eq!(
        outline
            .apply_children(
                &second,
                vec![TreeNode::new("b", "B")],
                source(2, &["a", "b"])
            )
            .unwrap(),
        TreeApply::Applied
    );
    outline.toggle_tree(&key("a")).unwrap();
    assert_eq!(outline.snapshot(3, None).unwrap().len(), 2);
    outline
        .set_tree(vec![TreeNode::new("a", "A").unloaded()])
        .unwrap();
    assert_eq!(
        outline.reject_children(&second, "late".into()).unwrap(),
        TreeApply::Stale
    );
}

struct OutlineDelegate;
impl TableDelegate for OutlineDelegate {
    type Source = TableOutlineSource<TableRows<Record>>;
}
#[gpui_kit::test]
fn root_paging_and_detail_selection_validate_as_distinct_domains(cx: &mut TestAppContext) {
    cx.update(|cx| {
        let mut outline = TableOutline::new(source(1, &["a", "b", "c"]));
        outline
            .set_tree(vec![
                TreeNode::new("a", "A").children([TreeNode::new("b", "B")]),
                TreeNode::new("c", "C"),
            ])
            .unwrap();
        outline.toggle_tree(&key("a")).unwrap();
        outline.set_details([(key("a"), key("detail"))]).unwrap();
        outline.toggle_detail(&key("a")).unwrap();
        let projection = Rc::new(
            outline
                .snapshot(1, Some(TablePage::new(1, 1).unwrap()))
                .unwrap(),
        );
        let delegate = cx.new(|_| OutlineDelegate);
        let state = cx.new(|cx| YueTableState::new(&delegate, cx));
        state.update(cx, |s, cx| {
            s.commit_view(
                TableView::new(
                    projection,
                    vec![TableColumn::text(
                        "k",
                        "Key",
                        |r: &yue_component::TableOutlineRow| r.record_key().as_str().to_owned(),
                    )],
                )
                .with_query(yue_component::TableQuery::paged(1, 1).unwrap())
                .with_result(
                    yue_component::TableDataMode::Remote,
                    yue_component::TableTotal::Known(2),
                )
                .selection_mode(TableSelectionMode::Multiple),
                None,
                cx,
            )
            .unwrap();
            assert!(s.request_toggle(&key("detail"), false, cx).is_err());
        });
    });
}

#[test]
fn sibling_sort_never_flattens_subtrees_and_stale_invalid_payload_is_ignored() {
    let mut outline = TableOutline::new(source(1, &["a", "b", "c", "d"]));
    outline
        .set_tree(vec![
            TreeNode::new("a", "A").children([TreeNode::new("b", "B"), TreeNode::new("c", "C")]),
            TreeNode::new("d", "D").unloaded(),
        ])
        .unwrap();
    outline.toggle_tree(&key("a")).unwrap();
    outline.sort_siblings(|a, b| b.key.cmp(a.key)).unwrap();
    let sorted = outline.snapshot(2, None).unwrap();
    assert_eq!(
        (0..sorted.len())
            .map(|i| sorted.row_key(i).unwrap().as_str())
            .collect::<Vec<_>>(),
        ["d", "a", "c", "b"]
    );
    let ticket = outline.request_children(&key("d")).unwrap().unwrap();
    outline.reject_children(&ticket, "retry".into()).unwrap();
    assert_eq!(
        outline
            .apply_children(
                &ticket,
                vec![TreeNode::new("missing", "invalid late payload")],
                source(1, &["a"])
            )
            .unwrap(),
        TreeApply::Stale
    );
    assert_eq!(outline.snapshot(3, None).unwrap().len(), 4);
}
