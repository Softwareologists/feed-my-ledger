//! Adapters for interacting with cloud spreadsheet services.

pub mod auth;

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
    /// An unspecified error occurred.
    Unknown,
}

impl std::fmt::Display for SpreadsheetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpreadsheetError::SheetNotFound => write!(f, "sheet not found"),
            SpreadsheetError::RowNotFound => write!(f, "row not found"),
            SpreadsheetError::ShareFailed => write!(f, "sharing failed"),
            SpreadsheetError::Unknown => write!(f, "unknown error"),
        }
    }
}

impl std::error::Error for SpreadsheetError {}

/// Abstraction over cloud spreadsheet services.
pub trait CloudSpreadsheetService {
    /// Creates a new spreadsheet and returns its ID.
    fn create_sheet(&mut self, title: &str) -> Result<String, SpreadsheetError>;
    /// Appends a row of data to the given spreadsheet.
    fn append_row(&mut self, sheet_id: &str, values: Vec<String>) -> Result<(), SpreadsheetError>;
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
