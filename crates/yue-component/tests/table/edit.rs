use super::*;
use yue_component::{TableCellKey, TableColumnKey, TableEditMode, TableEditStage};
fn key(row: &str) -> TableCellKey {
    TableCellKey::new(
        TableRowKey::try_new(row.to_owned()).unwrap(),
        TableColumnKey::try_new("value").unwrap(),
    )
}
fn editable(source: Rc<TableRows<Record>>) -> TableView<TableRows<Record>> {
    TableView::new(
        source,
        vec![TableColumn::text("value", "Value", |r: &Record| r.value.to_string()).editable(true)],
    )
}
#[gpui_kit::test]
fn edit_versions_conflicts_cancel_restart_and_atomic_save(cx: &mut TestAppContext) {
    #[cfg(feature = "table-profile")]
    let capture = super::profile::start();
    let (probe, cx) = open(cx, 10, false);
    let _ = probe;
    let delegate = cx.new(|_| Delegate);
    let state = cx.new(|cx| YueTableState::new(&delegate, cx));
    cx.update(|w, cx| {
        state.update(cx, |s, cx| {
            s.commit_view(editable(source(1, &["a", "b"])), None, cx)
                .unwrap();
            let original = s.start_edit(key("a"), TableEditMode::Cell, w, cx).unwrap();
            let draft = s.edit_draft_changed(cx).unwrap();
            assert!(!s.is_current_edit(&original));
            assert_eq!(
                s.accept_edit_validation(&original, cx),
                Err(TableError::StaleRequest)
            );
            let save = s.request_edit_save(cx).unwrap();
            assert_ne!(draft, save);
            s.accept_edit_validation(&save, cx).unwrap();
            let accepted = s.view().unwrap().clone();
            assert_eq!(
                s.commit_edit(&save, editable(source(2, &["b"])), w, cx)
                    .unwrap_err(),
                TableError::StaleSource
            );
            assert!(Rc::ptr_eq(&accepted, s.view().unwrap()));
            assert_eq!(s.edit_session().unwrap().stage(), &TableEditStage::Saving);
            s.reject_edit(&save, "failed", cx).unwrap();
            // A second late failure must not overwrite this settled draft's error.
            assert_eq!(
                s.reject_edit(&save, "late", cx),
                Err(TableError::StaleRequest)
            );
            let retry = s.request_edit_save(cx).unwrap();
            assert_ne!(save, retry);
            assert_eq!(
                s.reject_edit(&save, "late retry", cx),
                Err(TableError::StaleRequest)
            );
            let save = retry;
            s.accept_edit_validation(&save, cx).unwrap();
            s.commit_edit(&save, editable(source(2, &["a", "b"])), w, cx)
                .unwrap();
            assert!(s.edit_session().is_none());
            assert!(!s.is_current_edit(&save));
            let next = s.start_edit(key("a"), TableEditMode::Row, w, cx).unwrap();
            assert_ne!(save, next);
            s.commit_view(editable(source(3, &["a", "b"])), None, cx)
                .unwrap();
            assert_eq!(s.edit_session().unwrap().stage(), &TableEditStage::Conflict);
            assert!(!s.is_current_edit(&next));
            assert_eq!(s.edit_draft_changed(cx), Err(TableError::StaleSource));
            s.cancel_edit(w, cx);
            let next = s.start_edit(key("a"), TableEditMode::Cell, w, cx).unwrap();
            s.commit_view(editable(source(4, &["b"])), None, cx)
                .unwrap();
            assert!(s.edit_session().is_none());
            assert_eq!(
                s.reject_edit(&next, "late deletion", cx),
                Err(TableError::StaleRequest)
            );
        })
    });
    #[cfg(feature = "table-profile")]
    super::profile::finish(capture);
}
