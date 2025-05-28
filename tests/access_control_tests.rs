use rusty_ledger::cloud_adapters::GoogleSheetsAdapter;
use rusty_ledger::core::{AccessError, Permission, Record, SharedLedger};

#[test]
fn reader_cannot_write() {
    let adapter = GoogleSheetsAdapter::new();
    let mut ledger = SharedLedger::new(adapter, "owner@example.com").unwrap();
    ledger
        .share_with("reader@example.com", Permission::Read)
        .unwrap();

    let record = Record::new(
        "desc".into(),
        "cash".into(),
        "revenue".into(),
        1.0,
        "USD".into(),
        None,
        None,
        vec![],
    )
    .unwrap();

    let err = ledger.commit("reader@example.com", record).unwrap_err();
    assert_eq!(err, AccessError::Unauthorized);
}

#[test]
fn writer_can_write() {
    let adapter = GoogleSheetsAdapter::new();
    let mut ledger = SharedLedger::new(adapter, "owner@example.com").unwrap();
    ledger
        .share_with("writer@example.com", Permission::Write)
        .unwrap();

    let record = Record::new(
        "desc".into(),
        "cash".into(),
        "revenue".into(),
        2.0,
        "USD".into(),
        None,
        None,
        vec![],
    )
    .unwrap();

    ledger.commit("writer@example.com", record).unwrap();

    assert_eq!(ledger.records("writer@example.com").unwrap().count(), 1);
}

#[test]
fn access_is_required_for_reads() {
    let adapter = GoogleSheetsAdapter::new();
    let mut ledger = SharedLedger::new(adapter, "owner@example.com").unwrap();

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
    ledger.commit("owner@example.com", record).unwrap();

    let err = ledger.get_record("unknown@example.com", id).unwrap_err();
    assert_eq!(err, AccessError::Unauthorized);
}
