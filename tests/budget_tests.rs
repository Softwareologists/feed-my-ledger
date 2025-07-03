use chrono::{TimeZone, Utc};
use feed_my_ledger::core::{Budget, BudgetBook, Ledger, Period, PriceDatabase, Record};

#[test]
fn monthly_budget_diff() {
    let mut ledger = Ledger::default();
    let mut rec = Record::new(
        "coffee".into(),
        "expenses:food".parse().unwrap(),
        "cash".parse().unwrap(),
        30.0,
        "USD".into(),
        None,
        None,
        vec![],
    )
    .unwrap();
    rec.timestamp = Utc.with_ymd_and_hms(2024, 5, 2, 0, 0, 0).unwrap();
    ledger.commit(rec);
    let mut book = BudgetBook::default();
    book.add(
        Budget {
            account: "expenses:food".parse().unwrap(),
            amount: 50.0,
            currency: "USD".into(),
            period: Period::Monthly,
        },
        Some(2024),
        Some(5),
    );
    let diff = book
        .compare_month(
            &ledger,
            &PriceDatabase::default(),
            &"expenses:food".parse().unwrap(),
            2024,
            5,
        )
        .unwrap();
    assert_eq!(diff, 20.0);
}

#[test]
fn yearly_budget_diff() {
    let mut ledger = Ledger::default();
    for m in 1..=3 {
        let mut rec = Record::new(
            "expense".into(),
            "expenses".parse().unwrap(),
            "cash".parse().unwrap(),
            40.0,
            "USD".into(),
            None,
            None,
            vec![],
        )
        .unwrap();
        rec.timestamp = Utc.with_ymd_and_hms(2025, m, 1, 0, 0, 0).unwrap();
        ledger.commit(rec);
    }
    let mut book = BudgetBook::default();
    book.add(
        Budget {
            account: "expenses".parse().unwrap(),
            amount: 150.0,
            currency: "USD".into(),
            period: Period::Yearly,
        },
        Some(2025),
        None,
    );
    let diff = book
        .compare_year(
            &ledger,
            &PriceDatabase::default(),
            &"expenses".parse().unwrap(),
            2025,
        )
        .unwrap();
    assert_eq!(diff, 30.0);
}
