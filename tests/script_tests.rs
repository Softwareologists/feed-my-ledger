use feed_my_ledger::core::{Ledger, Record};
use feed_my_ledger::script::run_script;

#[test]
fn totals_cash_debits() {
    let mut ledger = Ledger::default();
    ledger.commit(
        Record::new(
            "coffee".into(),
            "cash".parse().unwrap(),
            "expenses".parse().unwrap(),
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
            "snack".into(),
            "cash".parse().unwrap(),
            "expenses".parse().unwrap(),
            3.0,
            "USD".into(),
            None,
            None,
            vec![],
        )
        .unwrap(),
    );
    let script = r#"
let total = 0.0;
for r in records {
    if r.debit == "cash" {
        total += r.amount;
    }
}
total
"#;
    let result = run_script(script, &ledger).unwrap();
    assert_eq!(result.cast::<f64>(), 8.0);
}
