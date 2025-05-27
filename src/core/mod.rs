//! Core logic for the append-only immutable database.

/// Represents a record stored in the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub id: u64,
    pub data: String,
}

impl Record {
    /// Creates a new record with the provided id and data.
    pub fn new(id: u64, data: impl Into<String>) -> Self {
        Self { id, data: data.into() }
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
        ledger.append(Record::new(1, "first"));
        ledger.append(Record::new(2, "second"));

        let data: Vec<_> = ledger.records().map(|r| r.data.clone()).collect();
        assert_eq!(data, vec!["first".to_string(), "second".to_string()]);
    }
}
