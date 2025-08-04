use std::path::Path;

use super::{ImportError, StatementImporter};
use crate::core::Record;

pub struct JsonImporter;

impl JsonImporter {
    fn parse_internal(path: &Path) -> Result<Vec<Record>, ImportError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse_str(&content)
    }

    pub fn parse_str(input: &str) -> Result<Vec<Record>, ImportError> {
        let mut records: Vec<Record> =
            serde_json::from_str(input).map_err(|e| ImportError::Parse(e.to_string()))?;
        for rec in &mut records {
            if rec.transaction_description.is_none() {
                rec.transaction_description = Some(rec.description.clone());
            }
        }
        Ok(records)
    }

    fn write(path: &Path, records: &[Record]) -> Result<(), ImportError> {
        let data =
            serde_json::to_string_pretty(records).map_err(|e| ImportError::Parse(e.to_string()))?;
        std::fs::write(path, data)?;
        Ok(())
    }
}

impl StatementImporter for JsonImporter {
    fn parse(path: &Path) -> Result<Vec<Record>, ImportError> {
        Self::parse_internal(path)
    }
}

pub fn parse(path: &Path) -> Result<Vec<Record>, ImportError> {
    JsonImporter::parse(path)
}

/// Parses a JSON file and sets all record currencies to the provided value.
pub fn parse_with_currency(path: &Path, currency: &str) -> Result<Vec<Record>, ImportError> {
    let mut records = JsonImporter::parse(path)?;
    for rec in &mut records {
        rec.currency = currency.to_string();
    }
    Ok(records)
}

pub fn parse_str(input: &str) -> Result<Vec<Record>, ImportError> {
    JsonImporter::parse_str(input)
}

pub fn export(path: &Path, records: &[Record]) -> Result<(), ImportError> {
    JsonImporter::write(path, records)
}
