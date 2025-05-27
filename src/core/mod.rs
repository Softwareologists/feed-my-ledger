//! Core logic for the append-only immutable database.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Represents a record stored in the database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    /// Unique identifier for this record.
    pub id: Uuid,
    /// Time at which the record was created.
    pub timestamp: DateTime<Utc>,
    /// Arbitrary structured payload stored in the ledger.
    pub data: Value,
    /// Optional reference to another record when creating adjustments.
    pub reference: Option<Uuid>,
}

impl Record {
    /// Creates a new record with the provided data and optional reference.
    pub fn new(data: Value, reference: Option<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            data,
            reference,
        }
    }

    /// Serializes the record to a JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserializes a record from a JSON string.
    pub fn from_json(input: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(input)
    }
}

/// In-memory append-only store of records.
#[derive(Default)]
pub struct Ledger {
    records: Vec<Record>,
}

impl Ledger {
    /// Appends a record to the ledger.
    pub fn append(&mut self, record: Record) {
        self.records.push(record);
    }

    /// Returns an iterator over all records.
    pub fn records(&self) -> impl Iterator<Item = &Record> {
        self.records.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_and_iterate() {
        let mut ledger = Ledger::default();
        ledger.append(Record::new(serde_json::json!("first"), None));
        ledger.append(Record::new(serde_json::json!("second"), None));

        let data: Vec<_> = ledger.records().map(|r| r.data.clone()).collect();
        assert_eq!(
            data,
            vec![serde_json::json!("first"), serde_json::json!("second")]
        );
    }
}
