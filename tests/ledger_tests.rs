use rusty_ledger::core::{Ledger, LedgerError, Record, RecordError};
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

#[test]
fn adjustment_chaining() {
    let mut ledger = Ledger::default();

    let original = Record::new(
        "orig".into(),
        "cash".into(),
        "revenue".into(),
        10.0,
        "USD".into(),
        None,
        None,
        vec![],
    )
    .unwrap();
    let orig_id = original.id;
    ledger.commit(original);

    let adj1 = Record::new(
        "adj1".into(),
        "revenue".into(),
        "cash".into(),
        2.0,
        "USD".into(),
        None,
        None,
        vec![],
    )
    .unwrap();
    let adj1_id = adj1.id;
    ledger.apply_adjustment(orig_id, adj1).unwrap();

    let adj2 = Record::new(
        "adj2".into(),
        "cash".into(),
        "revenue".into(),
        1.0,
        "USD".into(),
        None,
        None,
        vec![],
    )
    .unwrap();
    let adj2_id = adj2.id;
    ledger.apply_adjustment(adj1_id, adj2).unwrap();

    let history = ledger.adjustment_history(orig_id);
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].reference_id, Some(orig_id));
    assert_eq!(history[1].reference_id, Some(adj1_id));

    let history_adj1 = ledger.adjustment_history(adj1_id);
    assert_eq!(history_adj1.len(), 1);
    assert_eq!(history_adj1[0].id, adj2_id);
}

#[test]
fn adjustment_requires_existing_record() {
    let mut ledger = Ledger::default();
    let adj = Record::new(
        "adj".into(),
        "cash".into(),
        "revenue".into(),
        1.0,
        "USD".into(),
        None,
        None,
        vec![],
    )
    .unwrap();

    let missing = Uuid::new_v4();
    let err = ledger.apply_adjustment(missing, adj).unwrap_err();
    assert_eq!(err, LedgerError::RecordNotFound);
}

#[test]
fn record_creation_rejects_identical_accounts() {
    let err = Record::new(
        "desc".into(),
        "cash".into(),
        "cash".into(),
        1.0,
        "USD".into(),
        None,
        None,
        vec![],
    )
    .unwrap_err();
    assert_eq!(err, RecordError::SameAccount);
}

#[test]
fn record_creation_rejects_nonpositive_amounts() {
    let zero_err = Record::new(
        "zero".into(),
        "cash".into(),
        "revenue".into(),
        0.0,
        "USD".into(),
        None,
        None,
        vec![],
    )
    .unwrap_err();
    assert_eq!(zero_err, RecordError::NonPositiveAmount);

    let negative_err = Record::new(
        "neg".into(),
        "cash".into(),
        "revenue".into(),
        -1.0,
        "USD".into(),
        None,
        None,
        vec![],
    )
    .unwrap_err();
    assert_eq!(negative_err, RecordError::NonPositiveAmount);
}
