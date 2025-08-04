//! Core logic for the append-only immutable database.

use chrono::{DateTime, Utc};
use iso_currency::Currency;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod sharing;
pub use sharing::{AccessError, Permission, SharedLedger};
pub mod prices;
pub use prices::PriceDatabase;
pub mod query;
pub mod utils;
pub mod verification;
pub use query::{ParseError as QueryParseError, Query};
pub use verification::verify_sheet;
pub mod account;
pub use account::Account;
pub mod budget;
pub mod scheduler;
pub use budget::{Budget, BudgetBook, Period};
pub use scheduler::{RecordTemplate, ScheduleEntry, Scheduler};

/// Represents a single debit/credit posting within a transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Posting {
    /// Account that is debited.
    pub debit_account: Account,
    /// Account that is credited.
    pub credit_account: Account,
    /// Monetary amount of the posting.
    pub amount: f64,
}

/// Errors that can occur when creating a [`Record`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordError {
    /// The debit and credit accounts are identical.
    SameAccount,
    /// The amount provided is not positive.
    NonPositiveAmount,
    /// The provided currency code is not supported.
    UnsupportedCurrency(String),
}

impl std::fmt::Display for RecordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecordError::SameAccount => {
                write!(f, "debit and credit accounts cannot be identical")
            }
            RecordError::NonPositiveAmount => {
                write!(f, "transaction amount must be greater than zero")
            }
            RecordError::UnsupportedCurrency(code) => {
                write!(f, "unsupported currency code: {code}")
            }
        }
    }
}

impl std::error::Error for RecordError {}

/// Represents a record stored in the database.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    /// Unique identifier for this record.
    pub id: Uuid,
    /// Time at which the record was created.
    pub timestamp: DateTime<Utc>,
    /// Description or memo for the transaction.
    pub description: String,
    /// Account that is debited.
    pub debit_account: Account,
    /// Account that is credited.
    pub credit_account: Account,
    /// Monetary amount of the transaction.
    pub amount: f64,
    /// Currency code for the amount (e.g., USD).
    pub currency: String,
    /// Additional postings that make up a split transaction.
    #[serde(default)]
    pub splits: Vec<Posting>,
    /// Optional reference to another record when creating adjustments.
    pub reference_id: Option<Uuid>,
    /// Optional external reference such as invoice or receipt number.
    pub external_reference: Option<String>,
    /// Tags for categorizing the transaction.
    pub tags: Vec<String>,
    /// Description from the original statement line, if available.
    #[serde(default)]
    pub transaction_description: Option<String>,
    /// Whether the record has been reconciled with a statement line.
    #[serde(default)]
    pub cleared: bool,
}

impl Record {
    /// Creates a new record after validating the accounts and amount.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        description: String,
        debit_account: Account,
        credit_account: Account,
        amount: f64,
        currency: String,
        reference_id: Option<Uuid>,
        external_reference: Option<String>,
        tags: Vec<String>,
    ) -> Result<Self, RecordError> {
        Self::new_split(
            description,
            vec![Posting {
                debit_account,
                credit_account,
                amount,
            }],
            currency,
            reference_id,
            external_reference,
            tags,
        )
    }

    /// Creates a record with multiple debit/credit postings.
    #[allow(clippy::too_many_arguments)]
    pub fn new_split(
        description: String,
        postings: Vec<Posting>,
        currency: String,
        reference_id: Option<Uuid>,
        external_reference: Option<String>,
        tags: Vec<String>,
    ) -> Result<Self, RecordError> {
        if postings.is_empty() {
            return Err(RecordError::NonPositiveAmount);
        }
        if Currency::from_code(&currency).is_none() {
            return Err(RecordError::UnsupportedCurrency(currency));
        }
        for p in &postings {
            if p.debit_account == p.credit_account {
                return Err(RecordError::SameAccount);
            }
            if p.amount <= 0.0 {
                return Err(RecordError::NonPositiveAmount);
            }
        }
        let mut iter = postings.into_iter();
        let first = iter.next().expect("postings.is_empty() checked above");
        Ok(Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            description,
            debit_account: first.debit_account,
            credit_account: first.credit_account,
            amount: first.amount,
            currency,
            reference_id,
            external_reference,
            tags,
            transaction_description: None,
            cleared: false,
            splits: iter.collect(),
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

    /// Returns an iterator over all postings, including splits.
    pub fn postings(&self) -> impl Iterator<Item = Posting> + '_ {
        let first = Posting {
            debit_account: self.debit_account.clone(),
            credit_account: self.credit_account.clone(),
            amount: self.amount,
        };
        std::iter::once(first).chain(self.splits.clone())
    }

    /// Converts the record into a row for spreadsheet storage.
    pub fn to_row(&self) -> Vec<String> {
        let splits = if self.splits.is_empty() {
            String::new()
        } else {
            serde_json::to_string(&self.splits).unwrap_or_default()
        };
        vec![
            self.id.to_string(),
            self.timestamp.to_rfc3339(),
            self.description.clone(),
            self.debit_account.to_string(),
            self.credit_account.to_string(),
            self.amount.to_string(),
            self.currency.clone(),
            self.reference_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            self.external_reference.clone().unwrap_or_default(),
            self.tags.join(","),
            splits,
            self.transaction_description.clone().unwrap_or_default(),
        ]
    }

    /// Converts the record into a row with an appended SHA-256 hash.
    ///
    /// The hash is computed using [`hash_row`] over the row values and the
    /// provided signature. This allows tamper detection when the row is stored
    /// externally.
    pub fn to_row_hashed(&self, signature: &str) -> Vec<String> {
        let mut row = self.to_row();
        let hash = crate::core::utils::hash_row(&row, signature);
        row.push(hash);
        row
    }

    /// Converts the cleared status into a row for spreadsheet storage.
    pub fn status_row(&self) -> Vec<String> {
        vec![
            "status".to_string(),
            self.id.to_string(),
            self.cleared.to_string(),
        ]
    }
}

/// Errors that can occur when interacting with the [`Ledger`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    /// The requested record was not found.
    RecordNotFound,
    /// Records are immutable once committed and cannot be modified or deleted.
    ImmutableRecord,
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LedgerError::RecordNotFound => {
                write!(f, "record not found in ledger")
            }
            LedgerError::ImmutableRecord => {
                write!(f, "records are immutable and cannot be modified")
            }
        }
    }
}

impl std::error::Error for LedgerError {}

/// In-memory append-only store of records.
#[derive(Default)]
pub struct Ledger {
    records: Vec<Record>,
}

impl Ledger {
    /// Commits a record to the ledger.
    pub fn commit(&mut self, record: Record) {
        self.records.push(record);
    }

    /// Appends a record to the ledger.
    #[deprecated(note = "use `commit` instead")]
    pub fn append(&mut self, record: Record) {
        self.commit(record);
    }

    /// Returns an iterator over all records.
    pub fn records(&self) -> impl Iterator<Item = &Record> {
        self.records.iter()
    }

    /// Retrieves a record by its unique identifier.
    pub fn get_record(&self, id: Uuid) -> Result<&Record, LedgerError> {
        self.records
            .iter()
            .find(|r| r.id == id)
            .ok_or(LedgerError::RecordNotFound)
    }

    /// Applies an adjustment to an existing record by creating a new record
    /// referencing the original. The provided `adjustment` record will have its
    /// `reference_id` field overwritten with `original_id`.
    pub fn apply_adjustment(
        &mut self,
        original_id: Uuid,
        mut adjustment: Record,
    ) -> Result<(), LedgerError> {
        // Ensure the original record exists before creating the adjustment.
        self.get_record(original_id)?;
        adjustment.reference_id = Some(original_id);
        self.commit(adjustment);
        Ok(())
    }

    /// Returns all adjustments referencing the provided record ID, following
    /// the chain of adjustments recursively. The results are ordered by
    /// timestamp from oldest to newest.
    pub fn adjustment_history(&self, id: Uuid) -> Vec<&Record> {
        let mut history = Vec::new();
        let mut queue = vec![id];

        while let Some(current) = queue.pop() {
            for r in self
                .records
                .iter()
                .filter(|r| r.reference_id == Some(current))
            {
                history.push(r);
                queue.push(r.id);
            }
        }

        history.sort_by_key(|r| r.timestamp);
        history
    }

    /// Attempts to modify an existing record. Always fails because records are immutable.
    pub fn modify_record(&mut self, _id: Uuid, _record: Record) -> Result<(), LedgerError> {
        Err(LedgerError::ImmutableRecord)
    }

    /// Attempts to delete an existing record. Always fails because records are immutable.
    pub fn delete_record(&mut self, _id: Uuid) -> Result<(), LedgerError> {
        Err(LedgerError::ImmutableRecord)
    }

    /// Calculates the balance for the specified account by summing debits and
    /// credits. Debits increase the balance while credits decrease it.
    pub fn account_balance(&self, account: &str, target: &str, prices: &PriceDatabase) -> f64 {
        self.records.iter().fold(0.0, |mut acc, r| {
            for p in r.postings() {
                let mut amount = p.amount;
                if r.currency != target {
                    if let Some(rate) =
                        prices.get_rate(r.timestamp.date_naive(), &r.currency, target)
                    {
                        amount *= rate;
                    } else {
                        continue;
                    }
                }
                if p.debit_account.to_string() == account {
                    acc += amount;
                }
                if p.credit_account.to_string() == account {
                    acc -= amount;
                }
            }
            acc
        })
    }

    /// Calculates the balance for an account and all of its subaccounts.
    pub fn account_tree_balance(
        &self,
        account: &Account,
        target: &str,
        prices: &PriceDatabase,
    ) -> f64 {
        self.records.iter().fold(0.0, |mut acc, r| {
            for p in r.postings() {
                let mut amount = p.amount;
                if r.currency != target {
                    if let Some(rate) =
                        prices.get_rate(r.timestamp.date_naive(), &r.currency, target)
                    {
                        amount *= rate;
                    } else {
                        continue;
                    }
                }
                if p.debit_account.starts_with(account) {
                    acc += amount;
                }
                if p.credit_account.starts_with(account) {
                    acc -= amount;
                }
            }
            acc
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_and_iterate() {
        let mut ledger = Ledger::default();
        ledger.commit(
            Record::new(
                "first".into(),
                "cash".parse().unwrap(),
                "revenue".parse().unwrap(),
                1.0,
                "USD".into(),
                None,
                None,
                vec![],
            )
            .unwrap(),
        );
        ledger.commit(
            Record::new(
                "second".into(),
                "cash".parse().unwrap(),
                "revenue".parse().unwrap(),
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
