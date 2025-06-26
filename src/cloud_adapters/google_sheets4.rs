use crate::cloud_adapters::{CloudSpreadsheetService, SpreadsheetError};
use google_sheets4::{
    self as sheets4, Sheets,
    api::{Spreadsheet, SpreadsheetProperties, ValueRange},
    hyper_rustls, hyper_util,
};

use google_sheets4::common::Body;
use hyper_util::client::legacy::connect::HttpConnector;

/// Connector and client types used with the Sheets hub.
pub type HyperConnector = hyper_rustls::HttpsConnector<HttpConnector>;
pub type HyperClient = hyper_util::client::legacy::Client<HyperConnector, Body>;

/// Adapter backed by the real Google Sheets API.
pub struct GoogleSheets4Adapter {
    hub: Sheets<HyperConnector>,
    rt: tokio::runtime::Runtime,
    drive_base_url: String,
    sheet_name: String,
}

impl GoogleSheets4Adapter {
    /// Create a new adapter from an authenticated `Sheets` hub.
    pub fn new(hub: Sheets<HyperConnector>) -> Self {
        Self::with_drive_base_url_and_sheet_name(
            hub,
            "https://www.googleapis.com/drive/v3/",
            "Ledger",
        )
    }

    /// Create a new adapter with a custom Drive API base URL (useful for tests).
    pub fn with_drive_base_url(
        hub: Sheets<HyperConnector>,
        drive_base_url: impl Into<String>,
    ) -> Self {
        Self::with_drive_base_url_and_sheet_name(hub, drive_base_url, "Ledger")
    }

    /// Create a new adapter with a custom sheet name.
    pub fn with_sheet_name(hub: Sheets<HyperConnector>, sheet_name: impl Into<String>) -> Self {
        Self::with_drive_base_url_and_sheet_name(
            hub,
            "https://www.googleapis.com/drive/v3/",
            sheet_name,
        )
    }

    /// Create a new adapter with a custom Drive API base URL and sheet name.
    pub fn with_drive_base_url_and_sheet_name(
        hub: Sheets<HyperConnector>,
        drive_base_url: impl Into<String>,
        sheet_name: impl Into<String>,
    ) -> Self {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        Self {
            hub,
            rt,
            drive_base_url: drive_base_url.into(),
            sheet_name: sheet_name.into(),
        }
    }

    fn ensure_sheet(&self, sheet_id: &str) -> Result<(), SpreadsheetError> {
        use sheets4::api::{
            AddSheetRequest, BatchUpdateSpreadsheetRequest, Request as BatchRequest,
            SheetProperties,
        };
        let fut = self.hub.spreadsheets().get(sheet_id).doit();
        let res = self.rt.block_on(fut).map_err(Self::map_err)?;
        let exists = res
            .1
            .sheets
            .as_ref()
            .map(|sheets| {
                sheets.iter().any(|s| {
                    s.properties.as_ref().and_then(|p| p.title.as_deref())
                        == Some(self.sheet_name.as_str())
                })
            })
            .unwrap_or(false);
        if exists {
            return Ok(());
        }

        let add_sheet = BatchUpdateSpreadsheetRequest {
            requests: Some(vec![BatchRequest {
                add_sheet: Some(AddSheetRequest {
                    properties: Some(SheetProperties {
                        title: Some(self.sheet_name.clone()),
                        ..Default::default()
                    }),
                }),
                ..Default::default()
            }]),
            ..Default::default()
        };
        let fut = self
            .hub
            .spreadsheets()
            .batch_update(add_sheet, sheet_id)
            .doit();
        self.rt.block_on(fut).map_err(Self::map_err)?;
        Ok(())
    }

    fn map_err(err: sheets4::Error) -> SpreadsheetError {
        use sheets4::Error::*;
        match err {
            HttpError(_) | Io(_) | Failure(_) => SpreadsheetError::Transient(err.to_string()),
            _ => SpreadsheetError::Permanent(err.to_string()),
        }
    }
}

impl CloudSpreadsheetService for GoogleSheets4Adapter {
    fn create_sheet(&mut self, title: &str) -> Result<String, SpreadsheetError> {
        let req = Spreadsheet {
            properties: Some(SpreadsheetProperties {
                title: Some(title.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let fut = self.hub.spreadsheets().create(req).doit();
        let res = self.rt.block_on(fut).map_err(Self::map_err)?;
        let id = res.1.spreadsheet_id.unwrap_or_default();
        self.ensure_sheet(&id)?;
        Ok(id)
    }

    fn append_row(&mut self, sheet_id: &str, values: Vec<String>) -> Result<(), SpreadsheetError> {
        self.ensure_sheet(sheet_id)?;
        let row = values.into_iter().map(serde_json::Value::String).collect();
        let req = ValueRange {
            values: Some(vec![row]),
            ..Default::default()
        };
        let fut = self
            .hub
            .spreadsheets()
            .values_append(req, sheet_id, &self.sheet_name)
            .value_input_option("USER_ENTERED")
            .doit();
        self.rt.block_on(fut).map_err(Self::map_err)?;
        Ok(())
    }

    fn read_row(&self, sheet_id: &str, index: usize) -> Result<Vec<String>, SpreadsheetError> {
        self.ensure_sheet(sheet_id)?;
        let range = format!("{}!A{}:Z{}", self.sheet_name, index + 1, index + 1);
        let fut = self.hub.spreadsheets().values_get(sheet_id, &range).doit();
        let res = self.rt.block_on(fut).map_err(Self::map_err)?;
        let rows = res.1.values.unwrap_or_default();
        let row = rows
            .into_iter()
            .next()
            .ok_or(SpreadsheetError::RowNotFound)?;
        Ok(row
            .into_iter()
            .map(|v| v.as_str().unwrap_or_default().to_string())
            .collect())
    }

    fn list_rows(&self, sheet_id: &str) -> Result<Vec<Vec<String>>, SpreadsheetError> {
        self.ensure_sheet(sheet_id)?;
        let fut = self
            .hub
            .spreadsheets()
            .values_get(sheet_id, &self.sheet_name)
            .doit();
        let res = self.rt.block_on(fut).map_err(Self::map_err)?;
        let rows = res.1.values.unwrap_or_default();
        Ok(rows
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|v| v.as_str().unwrap_or_default().to_string())
                    .collect()
            })
            .collect())
    }

    fn share_sheet(&self, sheet_id: &str, email: &str) -> Result<(), SpreadsheetError> {
        use google_sheets4::hyper::header::{
            AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE, USER_AGENT,
        };
        use google_sheets4::hyper::{Method, Request};
        use serde_json::json;

        let drive_url = format!("{}files/{}/permissions", self.drive_base_url, sheet_id);

        let fut = async {
            let token = self
                .hub
                .auth
                .get_token(&["https://www.googleapis.com/auth/drive"])
                .await
                .map_err(|_| SpreadsheetError::ShareFailed)?
                .ok_or(SpreadsheetError::ShareFailed)?;

            let body_json = json!({
                "type": "user",
                "role": "writer",
                "emailAddress": email,
            });
            let body =
                serde_json::to_vec(&body_json).expect("failed to serialize permission request");
            let req = Request::builder()
                .method(Method::POST)
                .uri(&drive_url)
                .header(USER_AGENT, "rusty-ledger")
                .header(AUTHORIZATION, format!("Bearer {}", token))
                .header(CONTENT_TYPE, "application/json")
                .header(CONTENT_LENGTH, body.len() as u64)
                .body(google_sheets4::common::to_body(Some(body)))
                .expect("failed to build permission request");

            match self.hub.client.request(req).await {
                Ok(res) if res.status().is_success() => Ok(()),
                _ => Err(SpreadsheetError::ShareFailed),
            }
        };

        self.rt.block_on(fut)
    }
}
