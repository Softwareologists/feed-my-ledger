use feed_my_ledger::cloud_adapters::GoogleSheetsAdapter;
use feed_my_ledger::core::{Permission, Record, SharedLedger};

#[test]
fn cleared_status_persists() {
    let adapter = GoogleSheetsAdapter::new();
    let ledger = SharedLedger::new(adapter, "owner@example.com").unwrap();
    ledger
        .share_with("writer@example.com", Permission::Write)
        .unwrap();

    let record = Record::new(
        "desc".into(),
        "cash".parse().unwrap(),
        "revenue".parse().unwrap(),
        1.0,
        "USD".into(),
        None,
        None,
        vec![],
    )
    .unwrap();
    let id = record.id;
    ledger.commit("writer@example.com", record).unwrap();
    ledger.mark_cleared("writer@example.com", id).unwrap();

    let (adapter, sheet) = ledger.into_parts();
    let ledger2 = SharedLedger::from_sheet(adapter, &sheet, "owner@example.com").unwrap();
    let rec = ledger2.get_record("owner@example.com", id).unwrap();
    assert!(rec.cleared);
}
