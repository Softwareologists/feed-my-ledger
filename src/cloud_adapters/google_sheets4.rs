use crate::cloud_adapters::{CloudSpreadsheetService, SpreadsheetError};
use http_body_util::BodyExt;
use http_body_util::Full;
use hyper::Method;
use hyper::Request;
use hyper::body::Bytes;
use hyper::header;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use serde_json::json;
use std::future::Future;
use std::pin::Pin;
use yup_oauth2::hyper_rustls::HttpsConnectorBuilder;

/// Asynchronous token retrieval interface used by the adapter.
pub trait TokenProvider: Send + Sync + 'static {
    fn token<'a>(
        &'a self,
        scopes: &'a [&str],
    ) -> Pin<Box<dyn Future<Output = Result<String, SpreadsheetError>> + Send + 'a>>;
}

impl TokenProvider for yup_oauth2::authenticator::DefaultAuthenticator {
    fn token<'a>(
        &'a self,
        scopes: &'a [&str],
    ) -> Pin<Box<dyn Future<Output = Result<String, SpreadsheetError>> + Send + 'a>> {
        Box::pin(async move {
            self.token(scopes)
                .await
                .map_err(|e| SpreadsheetError::Transient(e.to_string()))?
                .token()
                .map(|t| t.to_string())
                .ok_or_else(|| SpreadsheetError::Transient("missing token".into()))
        })
    }
}

/// Adapter backed by the Google Sheets REST API.
pub struct GoogleSheets4Adapter {
    client: Client<yup_oauth2::hyper_rustls::HttpsConnector<HttpConnector>, Full<Bytes>>,
    auth: Box<dyn TokenProvider>,
    rt: tokio::runtime::Runtime,
    drive_base_url: String,
    sheets_base_url: String,
    sheet_name: String,
}

impl GoogleSheets4Adapter {
    /// Create a new adapter using default API endpoints.
    pub fn new<A: TokenProvider>(auth: A) -> Self {
        Self::with_base_urls_and_sheet_name(
            auth,
            "https://www.googleapis.com/drive/v3/",
            "https://sheets.googleapis.com/v4/",
            "Ledger",
        )
    }

    /// Create an adapter with a custom Drive base URL.
    pub fn with_drive_base_url<A: TokenProvider>(
        auth: A,
        drive_base_url: impl Into<String>,
    ) -> Self {
        Self::with_base_urls_and_sheet_name(
            auth,
            drive_base_url,
            "https://sheets.googleapis.com/v4/",
            "Ledger",
        )
    }

    /// Create an adapter with a custom sheet name.
    pub fn with_sheet_name<A: TokenProvider>(auth: A, sheet_name: impl Into<String>) -> Self {
        Self::with_base_urls_and_sheet_name(
            auth,
            "https://www.googleapis.com/drive/v3/",
            "https://sheets.googleapis.com/v4/",
            sheet_name,
        )
    }

    /// Create an adapter with custom base URLs and sheet name.
    pub fn with_base_urls_and_sheet_name<A: TokenProvider>(
        auth: A,
        drive_base_url: impl Into<String>,
        sheets_base_url: impl Into<String>,
        sheet_name: impl Into<String>,
    ) -> Self {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        let https = HttpsConnectorBuilder::new()
            .with_native_roots()
            .expect("native roots")
            .https_or_http()
            .enable_http1()
            .build();
        let client = Client::builder(TokioExecutor::new()).build::<_, Full<Bytes>>(https);
        Self {
            client,
            auth: Box::new(auth),
            rt,
            drive_base_url: drive_base_url.into(),
            sheets_base_url: sheets_base_url.into(),
            sheet_name: sheet_name.into(),
        }
    }

    async fn get_token(&self, scopes: &[&str]) -> Result<String, SpreadsheetError> {
        self.auth.token(scopes).await
    }

    async fn ensure_sheet(&self, sheet_id: &str) -> Result<(), SpreadsheetError> {
        let token = self
            .get_token(&["https://www.googleapis.com/auth/spreadsheets"])
            .await?;
        let url = format!("{}spreadsheets/{}", self.sheets_base_url, sheet_id);
        let req = Request::builder()
            .method(Method::GET)
            .uri(&url)
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Full::new(Bytes::new()))
            .map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
        let res = self
            .client
            .request(req)
            .await
            .map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
        let exists = if res.status().is_success() {
            let bytes = res
                .into_body()
                .collect()
                .await
                .map_err(|e| SpreadsheetError::Transient(e.to_string()))?
                .to_bytes();
            let body: serde_json::Value = serde_json::from_slice(&bytes[..])
                .map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
            body["sheets"].as_array().is_some_and(|sheets| {
                sheets
                    .iter()
                    .any(|s| s["properties"]["title"].as_str() == Some(self.sheet_name.as_str()))
            })
        } else {
            false
        };
        if exists {
            return Ok(());
        }

        let update_url = format!(
            "{}spreadsheets/{}:batchUpdate",
            self.sheets_base_url, sheet_id
        );
        let body_json = json!({
            "requests": [{"addSheet": {"properties": {"title": self.sheet_name}}}]
        });
        let req = Request::builder()
            .method(Method::POST)
            .uri(update_url)
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::from(Bytes::from(body_json.to_string())))
            .map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
        let res = self
            .client
            .request(req)
            .await
            .map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
        if res.status().is_success() {
            Ok(())
        } else {
            Err(SpreadsheetError::Transient("batch update failed".into()))
        }
    }
}

impl CloudSpreadsheetService for GoogleSheets4Adapter {
    fn create_sheet(&mut self, title: &str) -> Result<String, SpreadsheetError> {
        self.rt.block_on(async {
            let token = self
                .get_token(&["https://www.googleapis.com/auth/spreadsheets"])
                .await?;
            let url = format!("{}spreadsheets", self.sheets_base_url);
            let body_json = json!({"properties": {"title": title}});
            let req = Request::builder()
                .method(Method::POST)
                .uri(&url)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::from(Bytes::from(body_json.to_string())))
                .map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
            let res = self
                .client
                .request(req)
                .await
                .map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
            if !res.status().is_success() {
                return Err(SpreadsheetError::Transient("create failed".into()));
            }
            let bytes = res
                .into_body()
                .collect()
                .await
                .map_err(|e| SpreadsheetError::Transient(e.to_string()))?
                .to_bytes();
            let body: serde_json::Value = serde_json::from_slice(&bytes[..])
                .map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
            let id = body["spreadsheetId"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            self.ensure_sheet(&id).await?;
            Ok(id)
        })
    }

    fn append_row(&mut self, sheet_id: &str, values: Vec<String>) -> Result<(), SpreadsheetError> {
        self.rt.block_on(async {
            self.ensure_sheet(sheet_id).await?;
            let token = self
                .get_token(&["https://www.googleapis.com/auth/spreadsheets"])
                .await?;
            let url = format!(
                "{}spreadsheets/{}/values/{}:append?valueInputOption=USER_ENTERED",
                self.sheets_base_url, sheet_id, self.sheet_name
            );
            let row: Vec<serde_json::Value> =
                values.into_iter().map(serde_json::Value::String).collect();
            let body_json = json!({"values": [row]});
            let req = Request::builder()
                .method(Method::POST)
                .uri(&url)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::from(Bytes::from(body_json.to_string())))
                .map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
            let res = self
                .client
                .request(req)
                .await
                .map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
            if res.status().is_success() {
                Ok(())
            } else {
                Err(SpreadsheetError::Transient("append failed".into()))
            }
        })
    }

    fn read_row(&self, sheet_id: &str, index: usize) -> Result<Vec<String>, SpreadsheetError> {
        self.rt.block_on(async {
            self.ensure_sheet(sheet_id).await?;
            let token = self
                .get_token(&["https://www.googleapis.com/auth/spreadsheets"])
                .await?;
            let range = format!("{}!A{}:Z{}", self.sheet_name, index + 1, index + 1);
            let url = format!(
                "{}spreadsheets/{}/values/{}",
                self.sheets_base_url, sheet_id, range
            );
            let req = Request::builder()
                .method(Method::GET)
                .uri(&url)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Full::new(Bytes::new()))
                .map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
            let res = self
                .client
                .request(req)
                .await
                .map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
            if !res.status().is_success() {
                return Err(SpreadsheetError::RowNotFound);
            }
            let bytes = res
                .into_body()
                .collect()
                .await
                .map_err(|e| SpreadsheetError::Transient(e.to_string()))?
                .to_bytes();
            let body: serde_json::Value = serde_json::from_slice(&bytes[..])
                .map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
            let row = body["values"]
                .as_array()
                .and_then(|arr| arr.first())
                .cloned();
            let row = row.ok_or(SpreadsheetError::RowNotFound)?;
            Ok(row
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|v| v.as_str().unwrap_or_default().to_string())
                .collect())
        })
    }

    fn list_rows(&self, sheet_id: &str) -> Result<Vec<Vec<String>>, SpreadsheetError> {
        self.rt.block_on(async {
            self.ensure_sheet(sheet_id).await?;
            let token = self
                .get_token(&["https://www.googleapis.com/auth/spreadsheets"])
                .await?;
            let url = format!(
                "{}spreadsheets/{}/values/{}",
                self.sheets_base_url, sheet_id, self.sheet_name
            );
            let req = Request::builder()
                .method(Method::GET)
                .uri(&url)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Full::new(Bytes::new()))
                .map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
            let res = self
                .client
                .request(req)
                .await
                .map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
            if !res.status().is_success() {
                return Err(SpreadsheetError::Transient("list failed".into()));
            }
            let bytes = res
                .into_body()
                .collect()
                .await
                .map_err(|e| SpreadsheetError::Transient(e.to_string()))?
                .to_bytes();
            let body: serde_json::Value = serde_json::from_slice(&bytes[..])
                .map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
            let rows = body["values"].as_array().cloned().unwrap_or_default();
            Ok(rows
                .into_iter()
                .map(|row| {
                    row.as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .map(|v| v.as_str().unwrap_or_default().to_string())
                        .collect()
                })
                .collect())
        })
    }

    fn share_sheet(&self, sheet_id: &str, email: &str) -> Result<(), SpreadsheetError> {
        self.rt.block_on(async {
            let token = self
                .get_token(&["https://www.googleapis.com/auth/drive"])
                .await?;
            let url = format!("{}files/{}/permissions", self.drive_base_url, sheet_id);
            let body_json = json!({"type": "user", "role": "writer", "emailAddress": email});
            let req = Request::builder()
                .method(Method::POST)
                .uri(&url)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::from(Bytes::from(body_json.to_string())))
                .map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
            let res = self
                .client
                .request(req)
                .await
                .map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
            if res.status().is_success() {
                Ok(())
            } else {
                Err(SpreadsheetError::ShareFailed)
            }
        })
    }
}
