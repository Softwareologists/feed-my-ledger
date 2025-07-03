use feed_my_ledger::cloud_adapters::FileAdapter;
use feed_my_ledger::cloud_adapters::google_sheets4::TokenProvider;
use feed_my_ledger::cloud_adapters::{
    CloudSpreadsheetService, Excel365Adapter, GoogleSheets4Adapter, GoogleSheetsAdapter,
    SpreadsheetError,
};
use uuid::Uuid;

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

#[derive(Clone)]
struct StaticToken;

impl TokenProvider for StaticToken {
    fn token<'a>(
        &'a self,
        _scopes: &'a [&str],
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<String, SpreadsheetError>> + Send + 'a>,
    > {
        Box::pin(async { Ok("test-token".to_string()) })
    }
}

#[tokio::test]
async fn share_sheet_sends_request() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/files/sheet123/permissions"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let adapter =
        GoogleSheets4Adapter::with_drive_base_url(StaticToken, format!("{}/", server.uri()));
    tokio::task::spawn_blocking(move || {
        adapter.share_sheet("sheet123", "user@example.com").unwrap();
    })
    .await
    .unwrap();
    server.verify().await;
}

#[tokio::test]
async fn share_sheet_propagates_failure() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/files/bad/permissions"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let adapter =
        GoogleSheets4Adapter::with_drive_base_url(StaticToken, format!("{}/", server.uri()));
    let err = tokio::task::spawn_blocking(move || {
        adapter.share_sheet("bad", "user@example.com").unwrap_err()
    })
    .await
    .unwrap();
    assert_eq!(err, SpreadsheetError::ShareFailed);
    server.verify().await;
}

#[test]
fn excel365_adapter_is_service() {
    fn assert_impl<T: CloudSpreadsheetService>() {}
    assert_impl::<Excel365Adapter>();
}

#[tokio::test]
async fn excel_share_sheet_sends_request() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/me/drive/items/sheet123/invite"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let adapter = Excel365Adapter::with_base_url(StaticToken, format!("{}/", server.uri()));
    tokio::task::spawn_blocking(move || {
        adapter.share_sheet("sheet123", "user@example.com").unwrap();
    })
    .await
    .unwrap();
    server.verify().await;
}

#[tokio::test]
async fn excel_share_sheet_propagates_failure() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/me/drive/items/bad/invite"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let adapter = Excel365Adapter::with_base_url(StaticToken, format!("{}/", server.uri()));
    let err = tokio::task::spawn_blocking(move || {
        adapter.share_sheet("bad", "user@example.com").unwrap_err()
    })
    .await
    .unwrap();
    assert_eq!(err, SpreadsheetError::ShareFailed);
    server.verify().await;
}

#[test]
fn file_adapter_round_trip() {
    let dir = std::env::temp_dir().join(format!("ledger_{}", Uuid::new_v4()));
    std::fs::create_dir(&dir).unwrap();
    let mut adapter = FileAdapter::new(&dir);
    let id = adapter.create_sheet("test").unwrap();
    adapter
        .append_row(&id, vec!["a".into(), "b".into()])
        .unwrap();
    let rows = adapter.list_rows(&id).unwrap();
    assert_eq!(rows, vec![vec!["a", "b"]]);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn file_adapter_missing_sheet() {
    let adapter = FileAdapter::new(std::env::temp_dir());
    let err = adapter.read_row("missing", 0).unwrap_err();
    assert_eq!(err, SpreadsheetError::SheetNotFound);
}
