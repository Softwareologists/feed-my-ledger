use std::sync::Arc;
use std::thread;

use rusty_ledger::cloud_adapters::GoogleSheetsAdapter;
use rusty_ledger::core::{Permission, Record, SharedLedger};

#[test]
fn concurrent_commits() {
    let adapter = GoogleSheetsAdapter::new();
    let ledger = SharedLedger::new(adapter, "owner@example.com").unwrap();
    ledger
        .share_with("writer@example.com", Permission::Write)
        .unwrap();

    let ledger = Arc::new(ledger);
    let mut handles = Vec::new();

    for _ in 0..10 {
        let ledger_cloned = Arc::clone(&ledger);
        handles.push(thread::spawn(move || {
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
            ledger_cloned.commit("writer@example.com", record).unwrap();
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(ledger.records("writer@example.com").unwrap().len(), 10);
}
