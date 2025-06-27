use chrono::{NaiveDate, TimeZone, Utc};
use rusty_ledger::core::{Account, Ledger, LedgerError, PriceDatabase, Record, RecordError};
use uuid::Uuid;

#[test]
fn records_are_appended() {
    let mut ledger = Ledger::default();
    ledger.commit(
        Record::new(
            "data".into(),
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
    assert_eq!(ledger.records().count(), 1);
}

#[test]
fn record_serialization_roundtrip() {
    let reference = Uuid::new_v4();
    let record = Record::new(
        "desc".into(),
        "cash".parse().unwrap(),
        "revenue".parse().unwrap(),
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
        "cash".parse().unwrap(),
        "revenue".parse().unwrap(),
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
        "cash".parse().unwrap(),
        "revenue".parse().unwrap(),
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
        "cash".parse().unwrap(),
        "revenue".parse().unwrap(),
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
                "cash".parse().unwrap(),
                "revenue".parse().unwrap(),
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
        "cash".parse().unwrap(),
        "revenue".parse().unwrap(),
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
        "revenue".parse().unwrap(),
        "cash".parse().unwrap(),
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
        "cash".parse().unwrap(),
        "revenue".parse().unwrap(),
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
        "cash".parse().unwrap(),
        "revenue".parse().unwrap(),
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
        "cash".parse().unwrap(),
        "cash".parse().unwrap(),
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
        "cash".parse().unwrap(),
        "revenue".parse().unwrap(),
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
        "cash".parse().unwrap(),
        "revenue".parse().unwrap(),
        -1.0,
        "USD".into(),
        None,
        None,
        vec![],
    )
    .unwrap_err();
    assert_eq!(negative_err, RecordError::NonPositiveAmount);
}

#[test]
fn record_creation_validates_currency() {
    let valid = Record::new(
        "ok".into(),
        "cash".parse().unwrap(),
        "revenue".parse().unwrap(),
        1.0,
        "USD".into(),
        None,
        None,
        vec![],
    );
    assert!(valid.is_ok());

    let invalid = Record::new(
        "bad".into(),
        "cash".parse().unwrap(),
        "revenue".parse().unwrap(),
        1.0,
        "ZZZ".into(),
        None,
        None,
        vec![],
    )
    .unwrap_err();
    assert_eq!(invalid, RecordError::UnsupportedCurrency("ZZZ".into()));
}

#[test]
fn account_balance_after_commits() {
    let mut ledger = Ledger::default();
    ledger.commit(
        Record::new(
            "first".into(),
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
    ledger.commit(
        Record::new(
            "second".into(),
            "cash".parse().unwrap(),
            "revenue".parse().unwrap(),
            3.0,
            "USD".into(),
            None,
            None,
            vec![],
        )
        .unwrap(),
    );

    let prices = PriceDatabase::default();
    assert_eq!(ledger.account_balance("cash", "USD", &prices), 5.0);
    assert_eq!(ledger.account_balance("revenue", "USD", &prices), -5.0);
}

#[test]
fn account_balance_with_adjustments() {
    let mut ledger = Ledger::default();

    let original = Record::new(
        "orig".into(),
        "cash".parse().unwrap(),
        "revenue".parse().unwrap(),
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
        "revenue".parse().unwrap(),
        "cash".parse().unwrap(),
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
        "cash".parse().unwrap(),
        "revenue".parse().unwrap(),
        1.0,
        "USD".into(),
        None,
        None,
        vec![],
    )
    .unwrap();
    ledger.apply_adjustment(adj1_id, adj2).unwrap();

    let prices = PriceDatabase::default();
    assert_eq!(ledger.account_balance("cash", "USD", &prices), 9.0);
    assert_eq!(ledger.account_balance("revenue", "USD", &prices), -9.0);
}

#[test]
fn account_balance_converts_currencies() {
    let mut ledger = Ledger::default();
    let mut eur = Record::new(
        "eur".into(),
        "cash".parse().unwrap(),
        "rev".parse().unwrap(),
        10.0,
        "EUR".into(),
        None,
        None,
        vec![],
    )
    .unwrap();
    eur.timestamp = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    ledger.commit(eur);
    let mut usd = Record::new(
        "usd".into(),
        "cash".parse().unwrap(),
        "rev".parse().unwrap(),
        10.0,
        "USD".into(),
        None,
        None,
        vec![],
    )
    .unwrap();
    usd.timestamp = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    ledger.commit(usd);

    let mut prices = PriceDatabase::default();
    let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    prices.add_rate(date, "EUR", "USD", 2.0);
    prices.add_rate(date, "USD", "EUR", 0.5);

    assert_eq!(ledger.account_balance("cash", "USD", &prices), 30.0);
    assert_eq!(ledger.account_balance("cash", "EUR", &prices), 15.0);
}

#[test]
fn account_tree_balance_nested_accounts() {
    let mut ledger = Ledger::default();
    ledger.commit(
        Record::new(
            "check".into(),
            "Assets:Bank:Checking".parse().unwrap(),
            "income".parse().unwrap(),
            5.0,
            "USD".into(),
            None,
            None,
            vec![],
        )
        .unwrap(),
    );
    ledger.commit(
        Record::new(
            "save".into(),
            "Assets:Bank:Savings".parse().unwrap(),
            "income".parse().unwrap(),
            2.0,
            "USD".into(),
            None,
            None,
            vec![],
        )
        .unwrap(),
    );
    let prices = PriceDatabase::default();
    let parent: Account = "Assets:Bank".parse().unwrap();
    assert_eq!(ledger.account_tree_balance(&parent, "USD", &prices), 7.0);
}
