use std::path::Path;

use csv::Reader;
use serde::Deserialize;

use super::{ImportError, StatementImporter};
use crate::core::Record;

#[derive(Deserialize)]
struct CsvRow {
    description: String,
    debit_account: String,
    credit_account: String,
    amount: f64,
    currency: String,
}

pub struct CsvImporter;

impl CsvImporter {
    fn parse_internal(path: &Path) -> Result<Vec<Record>, ImportError> {
        let mut rdr = Reader::from_path(path).map_err(|e| ImportError::Parse(e.to_string()))?;
        let mut records = Vec::new();
        for result in rdr.deserialize() {
            let row: CsvRow = result.map_err(|e| ImportError::Parse(e.to_string()))?;
            let rec = Record::new(
                row.description,
                row.debit_account,
                row.credit_account,
                row.amount,
                row.currency,
                None,
                None,
                vec![],
            )?;
            records.push(rec);
        }
        Ok(records)
    }
}

impl StatementImporter for CsvImporter {
    fn parse(path: &Path) -> Result<Vec<Record>, ImportError> {
        Self::parse_internal(path)
    }
}

pub fn parse(path: &Path) -> Result<Vec<Record>, ImportError> {
    CsvImporter::parse(path)
}
