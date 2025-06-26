use std::path::Path;

use crate::core::{Record, RecordError};

#[derive(Debug)]
pub enum ImportError {
    Io(std::io::Error),
    Parse(String),
    Record(RecordError),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::Io(e) => write!(f, "io error: {e}"),
            ImportError::Parse(e) => write!(f, "parse error: {e}"),
            ImportError::Record(e) => write!(f, "record error: {e}"),
        }
    }
}

impl std::error::Error for ImportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ImportError::Io(e) => Some(e),
            ImportError::Record(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ImportError {
    fn from(e: std::io::Error) -> Self {
        ImportError::Io(e)
    }
}

impl From<RecordError> for ImportError {
    fn from(e: RecordError) -> Self {
        ImportError::Record(e)
    }
}

pub trait StatementImporter {
    fn parse(path: &Path) -> Result<Vec<Record>, ImportError>;
}

pub mod csv;
pub mod ofx;
pub mod qif;
