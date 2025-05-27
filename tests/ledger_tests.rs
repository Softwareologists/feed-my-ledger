use rusty_ledger::core::{Ledger, Record};

#[test]
fn records_are_appended() {
    let mut ledger = Ledger::default();
    ledger.append(Record::new(1, "data"));
    assert_eq!(ledger.records().count(), 1);
}
