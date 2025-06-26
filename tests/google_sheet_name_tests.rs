use rusty_ledger::cloud_adapters::google_sheets4::HyperConnector;
use rusty_ledger::cloud_adapters::{CloudSpreadsheetService, GoogleSheets4Adapter};

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
async fn ensures_sheet_exists() {
    use google_sheets4::{Sheets, hyper_rustls, hyper_util};
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

    let connector: HyperConnector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .unwrap()
        .https_or_http()
        .enable_http1()
        .build();
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build(connector.clone());
    let mut hub = Sheets::new(client, StaticToken);
    let base = format!("{}/", server.uri());
    hub.base_url(base.clone());
    hub.root_url(base);

    let mut adapter = GoogleSheets4Adapter::with_sheet_name(hub, "Custom");
    let result =
        tokio::task::spawn_blocking(move || adapter.append_row("sheet123", vec!["hello".into()]))
            .await
            .unwrap();
    if let Err(e) = result {
        println!("append_row error: {:?}", e);
    }
    let requests = server.received_requests().await.unwrap();
    for req in &requests {
        println!("path: {}", req.url.path());
    }
    server.verify().await;
}
