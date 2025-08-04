use std::path::Path;

use super::{ImportError, StatementImporter};
use crate::core::Record;

pub struct QifImporter;

impl QifImporter {
    fn parse_internal(path: &Path) -> Result<Vec<Record>, ImportError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse_str(&content)
    }

    fn parse_str(input: &str) -> Result<Vec<Record>, ImportError> {
        let mut records = Vec::new();
        let mut amount: Option<f64> = None;
        let mut memo: Option<String> = None;
        let mut vendor: Option<String> = None;

        for line in input.lines() {
            if line.starts_with('!') {
                continue;
            } else if line.starts_with('D') {
                // date line - ignored
            } else if let Some(rest) = line.strip_prefix('T') {
                let val = rest.trim().replace(',', "");
                let parsed = val
                    .parse::<f64>()
                    .map_err(|e| ImportError::Parse(e.to_string()))?;
                amount = Some(parsed);
            } else if let Some(rest) = line.strip_prefix('P') {
                vendor = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix('M') {
                memo = Some(rest.trim().to_string());
            } else if line.starts_with('^') {
                if let Some(a) = amount {
                    let desc = match &memo {
                        Some(m) if !m.is_empty() => m.clone(),
                        _ => vendor.clone().unwrap_or_default(),
                    };
                    let (debit, credit) = if a < 0.0 {
                        ("expenses".to_string(), "bank".to_string())
                    } else {
                        ("bank".to_string(), "income".to_string())
                    };
                    let mut rec = Record::new(
                        desc,
                        debit.parse().unwrap(),
                        credit.parse().unwrap(),
                        a.abs(),
                        "USD".into(),
                        None,
                        None,
                        vec![],
                    )?;
                    rec.transaction_description = Some(rec.description.clone());
                    records.push(rec);
                }
                amount = None;
                memo = None;
                vendor = None;
            }
        }
        Ok(records)
    }
}

impl StatementImporter for QifImporter {
    fn parse(path: &Path) -> Result<Vec<Record>, ImportError> {
        Self::parse_internal(path)
    }
}

pub fn parse(path: &Path) -> Result<Vec<Record>, ImportError> {
    QifImporter::parse(path)
}

/// Parses a QIF file and sets all record currencies to the provided value.
pub fn parse_with_currency(path: &Path, currency: &str) -> Result<Vec<Record>, ImportError> {
    let mut records = QifImporter::parse(path)?;
    for rec in &mut records {
        rec.currency = currency.to_string();
    }
    Ok(records)
}
