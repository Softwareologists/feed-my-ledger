use std::path::Path;

use super::{ImportError, StatementImporter};
use crate::core::Record;
use chrono::{DateTime, Local, NaiveDate, TimeZone};

pub struct QifImporter;

impl QifImporter {
    fn parse_internal(path: &Path, date_format: Option<&str>) -> Result<Vec<Record>, ImportError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse_str(&content, date_format)
    }

    fn parse_str(input: &str, date_format: Option<&str>) -> Result<Vec<Record>, ImportError> {
        let mut records = Vec::new();
        let mut amount: Option<f64> = None;
        let mut memo: Option<String> = None;
        let mut vendor: Option<String> = None;
        let mut date: Option<DateTime<Local>> = None;

        for line in input.lines() {
            if line.starts_with('!') {
                continue;
            } else if let Some(rest) = line.strip_prefix('D') {
                let s = rest.trim();
                let parsed = if let Some(fmt) = date_format {
                    NaiveDate::parse_from_str(s, fmt)
                } else {
                    NaiveDate::parse_from_str(s, "%Y-%m-%d")
                        .or_else(|_| NaiveDate::parse_from_str(s, "%m/%d/%Y"))
                };

                let final_result = parsed
                    .map_err(|e| e.to_string()) // Unify error type to String for the next step
                    .and_then(|naive_date| {
                        // This closure runs only if parsing was successful.
                        // It converts the NaiveDate to a DateTime<Local> at midnight.
                        let naive_datetime = naive_date.and_hms_opt(0, 0, 0).unwrap();
                        Local.from_local_datetime(&naive_datetime)
                            .single()
                            .ok_or_else(|| format!("Could not convert date '{}' to a unique local time", s))
                    });
                if let Ok(d) = final_result {
                    date = Some(d);
                }
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
                    let (debit, credit) = if a < 0.0 {
                        (
                            "bank".to_string(),
                            vendor.or(Option::from("UNK".to_string())).unwrap(),
                        )
                    } else {
                        (
                            vendor.or(Option::from("UNK".to_string())).unwrap(),
                            "bank".to_string(),
                        )
                    };
                    let mut rec = Record::new(
                        memo.or(Option::from("".to_string())).unwrap(),
                        debit.parse().unwrap(),
                        credit.parse().unwrap(),
                        a,
                        "USD".into(),
                        None,
                        None,
                        vec![],
                    )?;
                    rec.transaction_date = date;
                    records.push(rec);
                }
                amount = None;
                memo = None;
                vendor = None;
                date = None;
            }
        }
        Ok(records)
    }
}

impl StatementImporter for QifImporter {
    fn parse(path: &Path) -> Result<Vec<Record>, ImportError> {
        Self::parse_internal(path, None)
    }
}

pub fn parse(path: &Path) -> Result<Vec<Record>, ImportError> {
    QifImporter::parse(path)
}

/// Parses an QIF file and sets all record currencies to the provided value.
pub fn parse_with_currency(path: &Path, currency: &str) -> Result<Vec<Record>, ImportError> {
    let mut records = QifImporter::parse(path)?;
    for rec in &mut records {
        rec.currency = currency.to_string();
    }
    Ok(records)
}

pub fn parse_with_date_format(path: &Path, fmt: &str) -> Result<Vec<Record>, ImportError> {
    QifImporter::parse_internal(path, Some(fmt))
}

pub fn parse_str(input: &str) -> Result<Vec<Record>, ImportError> {
    QifImporter::parse_str(input, None)
}

pub fn parse_str_with_date_format(input: &str, fmt: &str) -> Result<Vec<Record>, ImportError> {
    QifImporter::parse_str(input, Some(fmt))
}
