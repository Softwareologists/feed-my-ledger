//! Adapters for interacting with cloud spreadsheet services.

pub mod auth;
pub mod retry;
pub use retry::RetryingService;
pub mod buffered;
pub use buffered::{BatchingCacheService, EvictionPolicy};
pub mod google_sheets4;
pub use google_sheets4::GoogleSheets4Adapter;
pub mod excel_365;
pub use excel_365::Excel365Adapter;

use std::collections::HashMap;

/// Represents errors that can occur when interacting with a spreadsheet
/// service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpreadsheetError {
    /// The requested sheet does not exist.
    SheetNotFound,
    /// The requested row does not exist.
    RowNotFound,
    /// Failed to share the sheet with the given recipient.
    ShareFailed,
    /// A temporary error that may succeed when retried.
    Transient(String),
    /// A non-recoverable error returned by the service.
    Permanent(String),
    /// An unspecified error occurred.
    Unknown,
}

impl std::fmt::Display for SpreadsheetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpreadsheetError::SheetNotFound => {
                write!(f, "spreadsheet not found: check the provided ID")
            }
            SpreadsheetError::RowNotFound => {
                write!(f, "row not found at the specified index")
            }
            SpreadsheetError::ShareFailed => {
                write!(f, "failed to share spreadsheet with the recipient")
            }
            SpreadsheetError::Transient(msg) => {
                write!(f, "temporary service error: {msg}. Please retry")
            }
            SpreadsheetError::Permanent(msg) => write!(f, "service error: {msg}"),
            SpreadsheetError::Unknown => write!(f, "an unknown error occurred"),
        }
    }
}

impl std::error::Error for SpreadsheetError {}

impl SpreadsheetError {
    /// Returns `true` if the error can be retried.
    pub fn is_retryable(&self) -> bool {
        matches!(self, SpreadsheetError::Transient(_))
    }
}

/// Abstraction over cloud spreadsheet services.
pub trait CloudSpreadsheetService {
    /// Creates a new spreadsheet and returns its ID.
    fn create_sheet(&mut self, title: &str) -> Result<String, SpreadsheetError>;
    /// Appends a row of data to the given spreadsheet.
    fn append_row(&mut self, sheet_id: &str, values: Vec<String>) -> Result<(), SpreadsheetError>;
    /// Appends multiple rows of data to the given spreadsheet. The default
    /// implementation calls [`append_row`] for each row.
    fn append_rows(
        &mut self,
        sheet_id: &str,
        rows: Vec<Vec<String>>,
    ) -> Result<(), SpreadsheetError> {
        for row in rows {
            self.append_row(sheet_id, row)?;
        }
        Ok(())
    }
    /// Reads a specific row from the spreadsheet.
    fn read_row(&self, sheet_id: &str, index: usize) -> Result<Vec<String>, SpreadsheetError>;
    /// Lists all rows from the spreadsheet.
    fn list_rows(&self, sheet_id: &str) -> Result<Vec<Vec<String>>, SpreadsheetError>;
    /// Shares the spreadsheet with the given email.
    fn share_sheet(&self, sheet_id: &str, email: &str) -> Result<(), SpreadsheetError>;
}

/// Mock adapter simulating Google Sheets behaviour.
#[derive(Default)]
pub struct GoogleSheetsAdapter {
    sheets: HashMap<String, Vec<Vec<String>>>,
    next_id: usize,
}

impl GoogleSheetsAdapter {
    /// Creates a new mock adapter instance.
    pub fn new() -> Self {
        Self {
            sheets: HashMap::new(),
            next_id: 1,
        }
    }
}

impl CloudSpreadsheetService for GoogleSheetsAdapter {
    fn create_sheet(&mut self, _title: &str) -> Result<String, SpreadsheetError> {
        let id = format!("sheet{}", self.next_id);
        self.next_id += 1;
        self.sheets.insert(id.clone(), Vec::new());
        Ok(id)
    }

    fn append_row(&mut self, sheet_id: &str, values: Vec<String>) -> Result<(), SpreadsheetError> {
        match self.sheets.get_mut(sheet_id) {
            Some(rows) => {
                rows.push(values);
                Ok(())
            }
            None => Err(SpreadsheetError::SheetNotFound),
        }
    }

    fn append_rows(
        &mut self,
        sheet_id: &str,
        rows: Vec<Vec<String>>,
    ) -> Result<(), SpreadsheetError> {
        match self.sheets.get_mut(sheet_id) {
            Some(dest) => {
                dest.extend(rows);
                Ok(())
            }
            None => Err(SpreadsheetError::SheetNotFound),
        }
    }

    fn read_row(&self, sheet_id: &str, index: usize) -> Result<Vec<String>, SpreadsheetError> {
        match self.sheets.get(sheet_id) {
            Some(rows) => rows
                .get(index)
                .cloned()
                .ok_or(SpreadsheetError::RowNotFound),
            None => Err(SpreadsheetError::SheetNotFound),
        }
    }

    fn list_rows(&self, sheet_id: &str) -> Result<Vec<Vec<String>>, SpreadsheetError> {
        match self.sheets.get(sheet_id) {
            Some(rows) => Ok(rows.clone()),
            None => Err(SpreadsheetError::SheetNotFound),
        }
    }

    fn share_sheet(&self, sheet_id: &str, _email: &str) -> Result<(), SpreadsheetError> {
        if self.sheets.contains_key(sheet_id) {
            Ok(())
        } else {
            Err(SpreadsheetError::ShareFailed)
        }
    }
}
