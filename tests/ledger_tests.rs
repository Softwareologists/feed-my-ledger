use rusty_ledger::core::{Ledger, LedgerError, Record};
use uuid::Uuid;

#[test]
fn records_are_appended() {
    let mut ledger = Ledger::default();
    ledger.commit(
        Record::new(
            "data".into(),
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
    assert_eq!(ledger.records().count(), 1);
}

#[test]
fn record_serialization_roundtrip() {
    let reference = Uuid::new_v4();
    let record = Record::new(
        "desc".into(),
        "cash".into(),
        "revenue".into(),
        10.0,
        "USD".into(),
        Some(reference),
        Some("INV-1".into()),
        vec!["tag".into()],
    )
    .unwrap();

    let json = record.to_json().unwrap();
    let parsed = Record::from_json(&json).unwrap();

    assert_eq!(record, parsed);
}

#[test]
fn record_creation_sets_fields() {
    let record = Record::new(
        "desc".into(),
        "cash".into(),
        "revenue".into(),
        5.0,
        "USD".into(),
        None,
        None,
        vec![],
    )
    .unwrap();

    assert!(record.timestamp <= chrono::Utc::now());
    assert!(record.reference_id.is_none());
}

#[test]
fn committed_record_can_be_retrieved() {
    let mut ledger = Ledger::default();
    let record = Record::new(
        "desc".into(),
        "cash".into(),
        "revenue".into(),
        3.0,
        "USD".into(),
        None,
        None,
        vec![],
    )
    .unwrap();
    let id = record.id;
    ledger.commit(record);

    let stored = ledger.get_record(id).unwrap();
    assert_eq!(stored.amount, 3.0);
}

#[test]
fn committed_records_are_immutable() {
    let mut ledger = Ledger::default();
    let record = Record::new(
        "desc".into(),
        "cash".into(),
        "revenue".into(),
        4.0,
        "USD".into(),
        None,
        None,
        vec![],
    )
    .unwrap();
    let id = record.id;
    ledger.commit(record);

    let err = ledger
        .modify_record(
            id,
            Record::new(
                "new".into(),
                "cash".into(),
                "revenue".into(),
                5.0,
                "USD".into(),
                None,
                None,
                vec![],
            )
            .unwrap(),
        )
        .unwrap_err();
    assert_eq!(err, LedgerError::ImmutableRecord);

    let err = ledger.delete_record(id).unwrap_err();
    assert_eq!(err, LedgerError::ImmutableRecord);
}
