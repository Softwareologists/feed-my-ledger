use rusty_ledger::cloud_adapters::google_sheets4::HyperConnector;
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

#[derive(Clone)]
struct StaticToken;

impl google_sheets4::common::GetToken for StaticToken {
    fn get_token<'a>(
        &'a self,
        _scopes: &'a [&str],
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async { Ok(Some("test-token".to_string())) })
    }
}

#[tokio::test]
async fn share_sheet_sends_request() {
    use google_sheets4::{Sheets, hyper_rustls, hyper_util};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/files/sheet123/permissions"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let connector: HyperConnector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .unwrap()
        .https_or_http()
        .enable_http1()
        .build();
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build(connector.clone());
    let hub = Sheets::new(client, StaticToken);

    let adapter = GoogleSheets4Adapter::with_drive_base_url(hub, format!("{}/", server.uri()));
    tokio::task::spawn_blocking(move || {
        adapter.share_sheet("sheet123", "user@example.com").unwrap();
    })
    .await
    .unwrap();
    server.verify().await;
}

#[tokio::test]
async fn share_sheet_propagates_failure() {
    use google_sheets4::{Sheets, hyper_rustls, hyper_util};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/files/bad/permissions"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let connector: HyperConnector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .unwrap()
        .https_or_http()
        .enable_http1()
        .build();
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build(connector.clone());
    let hub = Sheets::new(client, StaticToken);

    let adapter = GoogleSheets4Adapter::with_drive_base_url(hub, format!("{}/", server.uri()));
    let err = tokio::task::spawn_blocking(move || {
        adapter.share_sheet("bad", "user@example.com").unwrap_err()
    })
    .await
    .unwrap();
    assert_eq!(err, SpreadsheetError::ShareFailed);
    server.verify().await;
}
