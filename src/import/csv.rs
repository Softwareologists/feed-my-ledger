use std::path::Path;

use csv::{Reader, StringRecord};

use super::{ImportError, StatementImporter};
use crate::core::Record;

/// Mapping of CSV column names to [`Record`] fields.
#[derive(Debug, Clone)]
pub struct CsvMapping {
    pub description: String,
    pub debit_account: String,
    pub credit_account: String,
    pub amount: String,
    pub currency: String,
}

impl Default for CsvMapping {
    fn default() -> Self {
        Self {
            description: "description".into(),
            debit_account: "debit_account".into(),
            credit_account: "credit_account".into(),
            amount: "amount".into(),
            currency: "currency".into(),
        }
    }
}

pub struct CsvImporter;

impl CsvImporter {
    fn parse_internal(path: &Path, mapping: &CsvMapping) -> Result<Vec<Record>, ImportError> {
        let mut rdr = Reader::from_path(path).map_err(|e| ImportError::Parse(e.to_string()))?;
        let headers = rdr
            .headers()
            .map_err(|e| ImportError::Parse(e.to_string()))?
            .clone();
        let idx = |name: &str| {
            headers
                .iter()
                .position(|h| h == name)
                .ok_or_else(|| ImportError::Parse(format!("missing column {name}")))
        };
        let desc_idx = idx(&mapping.description)?;
        let debit_idx = idx(&mapping.debit_account)?;
        let credit_idx = idx(&mapping.credit_account)?;
        let amount_idx = idx(&mapping.amount)?;
        let currency_idx = idx(&mapping.currency)?;

        let mut records = Vec::new();
        for result in rdr.records() {
            let row: StringRecord = result.map_err(|e| ImportError::Parse(e.to_string()))?;
            let amount_val: f64 = row
                .get(amount_idx)
                .ok_or_else(|| ImportError::Parse("missing amount".into()))?
                .parse::<f64>()
                .map_err(|e: std::num::ParseFloatError| ImportError::Parse(e.to_string()))?;
            let debit_acc = row
                .get(debit_idx)
                .unwrap_or_default()
                .parse()
                .map_err(|_| ImportError::Parse("invalid account".into()))?;
            let credit_acc = row
                .get(credit_idx)
                .unwrap_or_default()
                .parse()
                .map_err(|_| ImportError::Parse("invalid account".into()))?;
            let mut rec = Record::new(
                row.get(desc_idx).unwrap_or_default().to_string(),
                debit_acc,
                credit_acc,
                amount_val,
                row.get(currency_idx).unwrap_or_default().to_string(),
                None,
                None,
                vec![],
            )?;
            rec.transaction_description = Some(rec.description.clone());
            records.push(rec);
        }
        Ok(records)
    }

    /// Parses a CSV file using the provided column mapping.
    pub fn parse_with_mapping(
        path: &Path,
        mapping: &CsvMapping,
    ) -> Result<Vec<Record>, ImportError> {
        Self::parse_internal(path, mapping)
    }
}

impl StatementImporter for CsvImporter {
    fn parse(path: &Path) -> Result<Vec<Record>, ImportError> {
        Self::parse_internal(path, &CsvMapping::default())
    }
}

pub fn parse(path: &Path) -> Result<Vec<Record>, ImportError> {
    CsvImporter::parse(path)
}

/// Convenience wrapper around [`CsvImporter::parse_with_mapping`].
pub fn parse_with_mapping(path: &Path, mapping: &CsvMapping) -> Result<Vec<Record>, ImportError> {
    CsvImporter::parse_with_mapping(path, mapping)
}

/// Writes the provided records to a CSV file using the given column mapping.
pub fn export_with_mapping(
    path: &Path,
    records: &[Record],
    mapping: &CsvMapping,
) -> Result<(), ImportError> {
    let mut wtr = csv::Writer::from_path(path).map_err(|e| ImportError::Parse(e.to_string()))?;
    wtr.write_record([
        mapping.description.as_str(),
        mapping.debit_account.as_str(),
        mapping.credit_account.as_str(),
        mapping.amount.as_str(),
        mapping.currency.as_str(),
    ])
    .map_err(|e| ImportError::Parse(e.to_string()))?;
    for rec in records {
        wtr.write_record([
            rec.description.as_str(),
            rec.debit_account.to_string().as_str(),
            rec.credit_account.to_string().as_str(),
            rec.amount.to_string().as_str(),
            rec.currency.as_str(),
        ])
        .map_err(|e| ImportError::Parse(e.to_string()))?;
    }
    wtr.flush().map_err(|e| ImportError::Parse(e.to_string()))?;
    Ok(())
}

/// Convenience wrapper around [`export_with_mapping`].
pub fn export(path: &Path, records: &[Record]) -> Result<(), ImportError> {
    export_with_mapping(path, records, &CsvMapping::default())
}
