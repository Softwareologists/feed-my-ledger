use std::path::Path;

use super::{ImportError, StatementImporter};
use crate::core::Record;

pub struct LedgerImporter;

impl LedgerImporter {
    fn parse_internal(path: &Path) -> Result<Vec<Record>, ImportError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse_str(&content)
    }

    pub fn parse_str(input: &str) -> Result<Vec<Record>, ImportError> {
        let mut records = Vec::new();
        let mut lines = input.lines().peekable();
        while let Some(header) = lines.next() {
            if header.trim().is_empty() {
                continue;
            }
            let parts: Vec<&str> = header.trim().splitn(2, ' ').collect();
            let description = parts.get(1).map(|s| s.trim()).unwrap_or("").to_string();
            let debit_line = lines
                .next()
                .ok_or_else(|| ImportError::Parse("missing debit line".into()))?;
            let credit_line = lines
                .next()
                .ok_or_else(|| ImportError::Parse("missing credit line".into()))?;
            let mut debit_parts = debit_line.split_whitespace();
            let debit_account = debit_parts
                .next()
                .ok_or_else(|| ImportError::Parse("missing debit account".into()))?
                .parse()
                .map_err(|_| ImportError::Parse("invalid account".into()))?;
            let amount: f64 = debit_parts
                .next()
                .ok_or_else(|| ImportError::Parse("missing amount".into()))?
                .parse()
                .map_err(|e: std::num::ParseFloatError| ImportError::Parse(e.to_string()))?;
            let currency = debit_parts
                .next()
                .ok_or_else(|| ImportError::Parse("missing currency".into()))?
                .to_string();
            let credit_account = credit_line
                .trim()
                .parse()
                .map_err(|_| ImportError::Parse("invalid account".into()))?;
            let rec = Record::new(
                description,
                debit_account,
                credit_account,
                amount,
                currency,
                None,
                None,
                vec![],
            )?;
            records.push(rec);
            while let Some(l) = lines.peek() {
                if l.trim().is_empty() {
                    lines.next();
                } else {
                    break;
                }
            }
        }
        Ok(records)
    }

    fn export_internal(records: &[Record]) -> String {
        let mut out = String::new();
        for r in records {
            let date = r.timestamp.format("%Y-%m-%d");
            out.push_str(&format!("{date} {}\n", r.description));
            out.push_str(&format!(
                "    {}  {} {}\n",
                r.debit_account, r.amount, r.currency
            ));
            out.push_str(&format!("    {}\n\n", r.credit_account));
        }
        out
    }

    fn write(path: &Path, records: &[Record]) -> Result<(), ImportError> {
        let data = Self::export_internal(records);
        std::fs::write(path, data)?;
        Ok(())
    }
}

impl StatementImporter for LedgerImporter {
    fn parse(path: &Path) -> Result<Vec<Record>, ImportError> {
        Self::parse_internal(path)
    }
}

pub fn parse(path: &Path) -> Result<Vec<Record>, ImportError> {
    LedgerImporter::parse(path)
}

pub fn parse_str(input: &str) -> Result<Vec<Record>, ImportError> {
    LedgerImporter::parse_str(input)
}

pub fn export(path: &Path, records: &[Record]) -> Result<(), ImportError> {
    LedgerImporter::write(path, records)
}
