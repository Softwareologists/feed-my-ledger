//! Adapters for interacting with cloud spreadsheet services.

/// Trait representing a generic spreadsheet that supports appending rows.
pub trait CloudSpreadsheet {
    /// Appends a row of data to the spreadsheet.
    fn append_row(&self, values: &[String]) -> Result<(), String>;
}

/// Placeholder adapter for Google Sheets.
#[derive(Default)]
pub struct GoogleSheetsAdapter;

impl CloudSpreadsheet for GoogleSheetsAdapter {
    fn append_row(&self, _values: &[String]) -> Result<(), String> {
        // TODO: integrate with the Google Sheets API
        Ok(())
    }
}
