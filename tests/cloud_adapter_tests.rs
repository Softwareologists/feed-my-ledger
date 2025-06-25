use rusty_ledger::cloud_adapters::{
    CloudSpreadsheetService, GoogleSheets4Adapter, GoogleSheetsAdapter, SpreadsheetError,
};

#[test]
fn create_append_and_list_rows() {
    let mut adapter = GoogleSheetsAdapter::new();
    let id = adapter.create_sheet("test").unwrap();

    adapter
        .append_row(&id, vec!["a".into(), "b".into()])
        .unwrap();
    adapter
        .append_row(&id, vec!["c".into(), "d".into()])
        .unwrap();

    let rows = adapter.list_rows(&id).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], vec!["a", "b"]);
}

#[test]
fn reading_nonexistent_sheet_fails() {
    let adapter = GoogleSheetsAdapter::new();
    let err = adapter.read_row("missing", 0).unwrap_err();
    assert_eq!(err, SpreadsheetError::SheetNotFound);
}

#[test]
fn reading_nonexistent_row_fails() {
    let mut adapter = GoogleSheetsAdapter::new();
    let id = adapter.create_sheet("test").unwrap();

    let err = adapter.read_row(&id, 1).unwrap_err();
    assert_eq!(err, SpreadsheetError::RowNotFound);
}

#[test]
fn sharing_nonexistent_sheet_fails() {
    let adapter = GoogleSheetsAdapter::new();
    let err = adapter
        .share_sheet("missing", "user@example.com")
        .unwrap_err();
    assert_eq!(err, SpreadsheetError::ShareFailed);
}

#[test]
fn google_sheets4_adapter_is_service() {
    fn assert_impl<T: CloudSpreadsheetService>() {}
    assert_impl::<GoogleSheets4Adapter>();
}
