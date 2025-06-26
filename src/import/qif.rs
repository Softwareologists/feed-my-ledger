use std::path::Path;

use qif::{DateFormat, QIF};

use super::{ImportError, StatementImporter};
use crate::core::Record;

pub struct QifImporter;

impl QifImporter {
    fn parse_internal(path: &Path) -> Result<Vec<Record>, ImportError> {
        let content = std::fs::read_to_string(path)?;
        let qif = QIF::from_str(&content, &DateFormat::MonthDayFullYear);
        let mut records = Vec::new();
        let sections = [
            qif.cash.as_ref(),
            qif.bank.as_ref(),
            qif.credit_card.as_ref(),
            qif.liability.as_ref(),
            qif.asset.as_ref(),
        ];
        for sec in sections.into_iter().flatten() {
            for tx in &sec.transactions {
                let desc = if !tx.memo.is_empty() {
                    tx.memo.clone()
                } else {
                    tx.vendor.clone()
                };
                let amt = tx.amount;
                let (debit, credit) = if amt < 0.0 {
                    ("expenses".to_string(), "bank".to_string())
                } else {
                    ("bank".to_string(), "income".to_string())
                };
                let rec = Record::new(
                    desc,
                    debit,
                    credit,
                    amt.abs(),
                    "USD".into(),
                    None,
                    None,
                    vec![],
                )?;
                records.push(rec);
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
