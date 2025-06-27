use std::path::Path;

use super::{ImportError, StatementImporter};
use crate::core::Record;

pub struct OfxImporter;

impl OfxImporter {
    fn parse_internal(path: &Path) -> Result<Vec<Record>, ImportError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse_str(&content)
    }

    fn parse_str(input: &str) -> Result<Vec<Record>, ImportError> {
        let mut records = Vec::new();
        let mut remaining = input;
        while let Some(start) = remaining.find("<STMTTRN>") {
            remaining = &remaining[start + "<STMTTRN>".len()..];
            let end = match remaining.find("</STMTTRN>") {
                Some(idx) => idx,
                None => break,
            };
            let block = &remaining[..end];
            remaining = &remaining[end + "</STMTTRN>".len()..];

            if let Some(amt_str) = Self::extract_tag(block, "TRNAMT") {
                let amount: f64 = amt_str
                    .trim()
                    .parse()
                    .map_err(|e: std::num::ParseFloatError| ImportError::Parse(e.to_string()))?;
                let name = Self::extract_tag(block, "NAME").unwrap_or_default();
                let (debit, credit) = if amount < 0.0 {
                    ("expenses".to_string(), "bank".to_string())
                } else {
                    ("bank".to_string(), "income".to_string())
                };
                let rec = Record::new(
                    name.trim().to_string(),
                    debit.parse().unwrap(),
                    credit.parse().unwrap(),
                    amount.abs(),
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

    fn extract_tag(block: &str, tag: &str) -> Option<String> {
        let start_tag = format!("<{tag}>");
        let end_tag = format!("</{tag}>");
        let start = block.find(&start_tag)? + start_tag.len();
        let rest = &block[start..];
        let end = rest.find(&end_tag)?;
        Some(rest[..end].to_string())
    }
}

impl StatementImporter for OfxImporter {
    fn parse(path: &Path) -> Result<Vec<Record>, ImportError> {
        Self::parse_internal(path)
    }
}

pub fn parse(path: &Path) -> Result<Vec<Record>, ImportError> {
    OfxImporter::parse(path)
}
