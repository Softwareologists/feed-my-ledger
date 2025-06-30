use crate::cloud_adapters::{CloudSpreadsheetService, SpreadsheetError};
use csv::{ReaderBuilder, WriterBuilder};
use std::path::PathBuf;

/// Adapter that stores spreadsheet data in local CSV files.
pub struct FileAdapter {
    base_dir: PathBuf,
    next_id: usize,
}

impl FileAdapter {
    /// Create a new adapter rooted at `base_dir`.
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            next_id: 1,
        }
    }

    fn sheet_path(&self, id: &str) -> PathBuf {
        self.base_dir.join(format!("{id}.csv"))
    }
}

impl Default for FileAdapter {
    fn default() -> Self {
        Self::new(std::env::temp_dir())
    }
}

impl CloudSpreadsheetService for FileAdapter {
    fn create_sheet(&mut self, _title: &str) -> Result<String, SpreadsheetError> {
        let id = format!("sheet{}", self.next_id);
        self.next_id += 1;
        let path = self.sheet_path(&id);
        std::fs::File::create(&path).map_err(|e| SpreadsheetError::Permanent(e.to_string()))?;
        Ok(id)
    }

    fn append_row(&mut self, sheet_id: &str, values: Vec<String>) -> Result<(), SpreadsheetError> {
        self.append_rows(sheet_id, vec![values])
    }

    fn append_rows(
        &mut self,
        sheet_id: &str,
        rows: Vec<Vec<String>>,
    ) -> Result<(), SpreadsheetError> {
        let path = self.sheet_path(sheet_id);
        if !path.exists() {
            return Err(SpreadsheetError::SheetNotFound);
        }
        let file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
        let mut wtr = WriterBuilder::new().has_headers(false).from_writer(file);
        for row in rows {
            wtr.write_record(row)
                .map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
        }
        wtr.flush()
            .map_err(|e| SpreadsheetError::Transient(e.to_string()))
    }

    fn read_row(&self, sheet_id: &str, index: usize) -> Result<Vec<String>, SpreadsheetError> {
        let path = self.sheet_path(sheet_id);
        if !path.exists() {
            return Err(SpreadsheetError::SheetNotFound);
        }
        let file =
            std::fs::File::open(&path).map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
        let mut rdr = ReaderBuilder::new().has_headers(false).from_reader(file);
        for (i, record) in rdr.records().enumerate() {
            let rec = record.map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
            if i == index {
                return Ok(rec.iter().map(|s| s.to_string()).collect());
            }
        }
        Err(SpreadsheetError::RowNotFound)
    }

    fn list_rows(&self, sheet_id: &str) -> Result<Vec<Vec<String>>, SpreadsheetError> {
        let path = self.sheet_path(sheet_id);
        if !path.exists() {
            return Err(SpreadsheetError::SheetNotFound);
        }
        let file =
            std::fs::File::open(&path).map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
        let mut rdr = ReaderBuilder::new().has_headers(false).from_reader(file);
        let mut rows = Vec::new();
        for record in rdr.records() {
            let rec = record.map_err(|e| SpreadsheetError::Transient(e.to_string()))?;
            rows.push(rec.iter().map(|s| s.to_string()).collect());
        }
        Ok(rows)
    }

    fn share_sheet(&self, sheet_id: &str, _email: &str) -> Result<(), SpreadsheetError> {
        let path = self.sheet_path(sheet_id);
        if path.exists() {
            Ok(())
        } else {
            Err(SpreadsheetError::ShareFailed)
        }
    }
}
