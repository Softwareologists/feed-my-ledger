//! Core logic for the append-only immutable database.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Errors that can occur when creating a [`Record`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordError {
    /// The debit and credit accounts are identical.
    SameAccount,
    /// The amount provided is not positive.
    NonPositiveAmount,
}

impl std::fmt::Display for RecordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecordError::SameAccount => write!(f, "debit and credit accounts must differ"),
            RecordError::NonPositiveAmount => write!(f, "amount must be positive"),
        }
    }
}

impl std::error::Error for RecordError {}

/// Represents a record stored in the database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    /// Unique identifier for this record.
    pub id: Uuid,
    /// Time at which the record was created.
    pub timestamp: DateTime<Utc>,
    /// Description or memo for the transaction.
    pub description: String,
    /// Account that is debited.
    pub debit_account: String,
    /// Account that is credited.
    pub credit_account: String,
    /// Monetary amount of the transaction.
    pub amount: f64,
    /// Currency code for the amount (e.g., USD).
    pub currency: String,
    /// Optional reference to another record when creating adjustments.
    pub reference_id: Option<Uuid>,
    /// Optional external reference such as invoice or receipt number.
    pub external_reference: Option<String>,
    /// Tags for categorizing the transaction.
    pub tags: Vec<String>,
}

impl Record {
    /// Creates a new record after validating the accounts and amount.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        description: String,
        debit_account: String,
        credit_account: String,
        amount: f64,
        currency: String,
        reference_id: Option<Uuid>,
        external_reference: Option<String>,
        tags: Vec<String>,
    ) -> Result<Self, RecordError> {
        if debit_account == credit_account {
            return Err(RecordError::SameAccount);
        }
        if amount <= 0.0 {
            return Err(RecordError::NonPositiveAmount);
        }

        Ok(Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            description,
            debit_account,
            credit_account,
            amount,
            currency,
            reference_id,
            external_reference,
            tags,
        })
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
        ledger.append(
            Record::new(
                "first".into(),
                "cash".into(),
                "revenue".into(),
                1.0,
                "USD".into(),
                None,
                None,
                vec![],
            )
            .unwrap(),
        );
        ledger.append(
            Record::new(
                "second".into(),
                "cash".into(),
                "revenue".into(),
                2.0,
                "USD".into(),
                None,
                None,
                vec![],
            )
            .unwrap(),
        );

        let amounts: Vec<_> = ledger.records().map(|r| r.amount).collect();
        assert_eq!(amounts, vec![1.0, 2.0]);
    }
}
