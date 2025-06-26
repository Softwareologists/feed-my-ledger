use std::path::Path;

use quick_xml::Reader;
use quick_xml::events::Event;

use super::{ImportError, StatementImporter};
use crate::core::Record;

pub struct OfxImporter;

impl OfxImporter {
    fn parse_internal(path: &Path) -> Result<Vec<Record>, ImportError> {
        let content = std::fs::read_to_string(path)?;
        let mut reader = Reader::from_str(&content);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        let mut records = Vec::new();
        let mut in_stmt = false;
        let mut current_tag = String::new();
        let mut amount: Option<f64> = None;
        let mut name: Option<String> = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_uppercase();
                    if tag == "STMTTRN" {
                        in_stmt = true;
                        amount = None;
                        name = None;
                    } else if in_stmt {
                        current_tag = tag;
                    }
                }
                Ok(Event::End(e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_uppercase();
                    if tag == "STMTTRN" {
                        if let (Some(a), Some(n)) = (amount, name.clone()) {
                            let (debit, credit) = if a < 0.0 {
                                ("expenses".to_string(), "bank".to_string())
                            } else {
                                ("bank".to_string(), "income".to_string())
                            };
                            let rec = Record::new(
                                n,
                                debit,
                                credit,
                                a.abs(),
                                "USD".into(),
                                None,
                                None,
                                vec![],
                            )?;
                            records.push(rec);
                        }
                        in_stmt = false;
                    } else {
                        current_tag.clear();
                    }
                }
                Ok(Event::Text(t)) => {
                    if in_stmt {
                        match current_tag.as_str() {
                            "TRNAMT" => {
                                if let Ok(v) = t.unescape().unwrap_or_default().parse::<f64>() {
                                    amount = Some(v);
                                }
                            }
                            "NAME" => {
                                name = Some(t.unescape().unwrap_or_default().to_string());
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(ImportError::Parse(e.to_string())),
                _ => {}
            }
            buf.clear();
        }
        Ok(records)
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
