use super::google_sheets4::TokenProvider;
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
use yup_oauth2::hyper_rustls::HttpsConnectorBuilder;

/// Adapter backed by the Microsoft Graph API for Excel 365.
pub struct Excel365Adapter {
    client: Client<yup_oauth2::hyper_rustls::HttpsConnector<HttpConnector>, Full<Bytes>>,
    auth: Box<dyn TokenProvider>,
    rt: tokio::runtime::Runtime,
    drive_base_url: String,
    sheets_base_url: String,
    sheet_name: String,
}

impl Excel365Adapter {
    /// Create a new adapter using the default Graph endpoint.
    pub fn new<A: TokenProvider>(auth: A) -> Self {
        Self::with_base_url_and_sheet_name(auth, "https://graph.microsoft.com/v1.0/", "Ledger")
    }

    /// Create an adapter with a custom Graph base URL.
    pub fn with_base_url<A: TokenProvider>(auth: A, graph_base_url: impl Into<String>) -> Self {
        Self::with_base_url_and_sheet_name(auth, graph_base_url, "Ledger")
    }

    /// Create an adapter with a custom sheet name.
    pub fn with_sheet_name<A: TokenProvider>(auth: A, sheet_name: impl Into<String>) -> Self {
        Self::with_base_url_and_sheet_name(auth, "https://graph.microsoft.com/v1.0/", sheet_name)
    }

    /// Create an adapter with custom base URL and sheet name.
    pub fn with_base_url_and_sheet_name<A: TokenProvider>(
        auth: A,
        graph_base_url: impl Into<String>,
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
        let graph_base_url = graph_base_url.into();
        Self {
            client,
            auth: Box::new(auth),
            rt,
            drive_base_url: graph_base_url.clone(),
            sheets_base_url: graph_base_url,
            sheet_name: sheet_name.into(),
        }
    }

    async fn get_token(&self, scopes: &[&str]) -> Result<String, SpreadsheetError> {
        self.auth.token(scopes).await
    }

    async fn ensure_sheet(&self, sheet_id: &str) -> Result<(), SpreadsheetError> {
        let token = self
            .get_token(&["https://graph.microsoft.com/.default"])
            .await?;
        let url = format!(
            "{}me/drive/items/{}/workbook/worksheets",
            self.sheets_base_url, sheet_id
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
        let exists = if res.status().is_success() {
            let bytes = res
                .into_body()
                .collect()
                .await
                .map_err(|e| SpreadsheetError::Transient(e.to_string()))?
                .to_bytes();
            let body: serde_json::Value = serde_json::from_slice(&bytes[..])
                .map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
            body["value"].as_array().is_some_and(|sheets| {
                sheets
                    .iter()
                    .any(|s| s["name"].as_str() == Some(self.sheet_name.as_str()))
            })
        } else {
            false
        };
        if exists {
            return Ok(());
        }
        let add_url = format!(
            "{}me/drive/items/{}/workbook/worksheets/add",
            self.sheets_base_url, sheet_id
        );
        let body_json = json!({ "name": self.sheet_name });
        let req = Request::builder()
            .method(Method::POST)
            .uri(add_url)
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
            Err(SpreadsheetError::Transient(
                "worksheet creation failed".into(),
            ))
        }
    }
}

impl CloudSpreadsheetService for Excel365Adapter {
    fn create_sheet(&mut self, title: &str) -> Result<String, SpreadsheetError> {
        self.rt.block_on(async {
            let token = self
                .get_token(&["https://graph.microsoft.com/.default"])
                .await?;
            let url = format!("{}me/drive/root/children", self.drive_base_url);
            let body_json = json!({
                "name": format!("{}.xlsx", title),
                "file": {}
            });
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
            let id = body["id"].as_str().unwrap_or_default().to_string();
            self.ensure_sheet(&id).await?;
            Ok(id)
        })
    }

    fn append_row(&mut self, sheet_id: &str, values: Vec<String>) -> Result<(), SpreadsheetError> {
        self.rt.block_on(async {
            self.ensure_sheet(sheet_id).await?;
            let token = self
                .get_token(&["https://graph.microsoft.com/.default"])
                .await?;
            let url = format!(
                "{}me/drive/items/{}/workbook/worksheets/{}/tables/Table1/rows/add",
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
                .get_token(&["https://graph.microsoft.com/.default"])
                .await?;
            let url = format!(
                "{}me/drive/items/{}/workbook/worksheets/{}/range(address='A{}:Z{}')",
                self.sheets_base_url,
                sheet_id,
                self.sheet_name,
                index + 1,
                index + 1
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
                .cloned()
                .ok_or(SpreadsheetError::RowNotFound)?;
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
                .get_token(&["https://graph.microsoft.com/.default"])
                .await?;
            let url = format!(
                "{}me/drive/items/{}/workbook/worksheets/{}/usedRange(valuesOnly=true)",
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
                .get_token(&["https://graph.microsoft.com/.default"])
                .await?;
            let url = format!("{}me/drive/items/{}/invite", self.drive_base_url, sheet_id);
            let body_json = json!({
                "requireSignIn": true,
                "sendInvitation": true,
                "roles": ["write"],
                "recipients": [{"email": email}]
            });
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
