use rusty_ledger::core::{Ledger, Record};
use uuid::Uuid;

#[test]
fn records_are_appended() {
    let mut ledger = Ledger::default();
    ledger.append(Record::new(serde_json::json!("data"), None));
    assert_eq!(ledger.records().count(), 1);
}

#[test]
fn record_serialization_roundtrip() {
    let reference = Uuid::new_v4();
    let record = Record::new(serde_json::json!({"amount": 10}), Some(reference));

    let json = record.to_json().unwrap();
    let parsed = Record::from_json(&json).unwrap();

    assert_eq!(record, parsed);
}

#[test]
fn record_creation_sets_fields() {
    let record = Record::new(serde_json::json!({"k": "v"}), None);

    assert!(record.timestamp <= chrono::Utc::now());
    assert!(record.reference.is_none());
}
