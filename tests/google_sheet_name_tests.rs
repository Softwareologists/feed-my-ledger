use feed_my_ledger::cloud_adapters::google_sheets4::TokenProvider;
use feed_my_ledger::cloud_adapters::{
    CloudSpreadsheetService, GoogleSheets4Adapter, SpreadsheetError,
};

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
async fn ensures_sheet_exists() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v4/spreadsheets/sheet123"))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("content-type", "application/json")
                .set_body_string("{\"spreadsheetId\":\"sheet123\",\"sheets\":[]}"),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v4/spreadsheets/sheet123:batchUpdate"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let mut adapter = GoogleSheets4Adapter::with_base_urls_and_sheet_name(
        StaticToken,
        format!("{}/", server.uri()),
        format!("{}/v4/", server.uri()),
        "Custom",
    );
    let result =
        tokio::task::spawn_blocking(move || adapter.append_row("sheet123", vec!["hello".into()]))
            .await
            .unwrap();
    if let Err(e) = result {
        println!("append_row error: {e:?}");
    }
    let requests = server.received_requests().await.unwrap();
    for req in &requests {
        println!("path: {}", req.url.path());
    }
    server.verify().await;
}
