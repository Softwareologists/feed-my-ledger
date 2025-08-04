use std::path::Path;

use super::{ImportError, StatementImporter};
use crate::core::Record;
use chrono::NaiveDate;

pub struct OfxImporter;

impl OfxImporter {
    fn parse_internal(path: &Path) -> Result<Vec<Record>, ImportError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse_str(&content)
    }

    pub fn parse_str(input: &str) -> Result<Vec<Record>, ImportError> {
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
                let date = Self::extract_tag(block, "DTPOSTED").and_then(|s| {
                    let s = s.trim();
                    if s.len() >= 8 {
                        NaiveDate::parse_from_str(&s[..8], "%Y%m%d").ok()
                    } else {
                        None
                    }
                });
                let (debit, credit) = if amount < 0.0 {
                    ("expenses".to_string(), "bank".to_string())
                } else {
                    ("bank".to_string(), "income".to_string())
                };
                let mut rec = Record::new(
                    name.trim().to_string(),
                    debit.parse().unwrap(),
                    credit.parse().unwrap(),
                    amount.abs(),
                    "USD".into(),
                    None,
                    None,
                    vec![],
                )?;
                rec.transaction_description = Some(rec.description.clone());
                rec.transaction_date = date;
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

/// Parses an OFX file and sets all record currencies to the provided value.
pub fn parse_with_currency(path: &Path, currency: &str) -> Result<Vec<Record>, ImportError> {
    let mut records = OfxImporter::parse(path)?;
    for rec in &mut records {
        rec.currency = currency.to_string();
    }
    Ok(records)
}

pub fn parse_str(input: &str) -> Result<Vec<Record>, ImportError> {
    OfxImporter::parse_str(input)
}

#[cfg(feature = "bank-api")]
pub async fn download(url: &str) -> Result<Vec<Record>, ImportError> {
    use http_body_util::{BodyExt, Full};
    use hyper::body::Bytes;
    use hyper_util::client::legacy::Client;
    use hyper_util::rt::TokioExecutor;
    use yup_oauth2::hyper_rustls::HttpsConnectorBuilder;
    let https = HttpsConnectorBuilder::new()
        .with_native_roots()? // Use ? to unwrap Result
        .https_or_http()
        .enable_http1()
        .build();
    let client = Client::builder(TokioExecutor::new()).build::<_, Full<Bytes>>(https);
    let uri: hyper::Uri = url
        .parse::<hyper::Uri>()
        .map_err(|e| ImportError::Parse(e.to_string()))?;
    let req = hyper::Request::builder()
        .method(hyper::Method::GET)
        .uri(uri)
        .body(Full::new(Bytes::new()))
        .map_err(|e| ImportError::Parse(e.to_string()))?;
    let res = client
        .request(req)
        .await
        .map_err(|e| ImportError::Io(std::io::Error::other(e)))?;
    let bytes = res
        .into_body()
        .collect()
        .await
        .map_err(|e| ImportError::Io(std::io::Error::other(e)))?
        .to_bytes();
    let text = String::from_utf8(bytes.to_vec()).map_err(|e| ImportError::Parse(e.to_string()))?;
    parse_str(&text)
}
