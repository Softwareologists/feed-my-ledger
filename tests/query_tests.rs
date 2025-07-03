use chrono::{TimeZone, Utc};
use feed_my_ledger::core::{Ledger, Query, Record};
use std::str::FromStr;

#[test]
fn parse_basic() {
    let q = Query::from_str("account:cash tag:food start:2024-01-01 end:2024-01-31").unwrap();
    assert_eq!(q.accounts, vec!["cash"]);
    assert_eq!(q.tags, vec!["food"]);
    assert_eq!(q.start.unwrap().to_string(), "2024-01-01".to_string());
    assert_eq!(q.end.unwrap().to_string(), "2024-01-31".to_string());
}

#[test]
fn filter_by_tag_and_date() {
    let mut ledger = Ledger::default();
    let mut rec1 = Record::new(
        "coffee".into(),
        "expenses".parse().unwrap(),
        "cash".parse().unwrap(),
        3.0,
        "USD".into(),
        None,
        None,
        vec!["food".into()],
    )
    .unwrap();
    rec1.timestamp = Utc.with_ymd_and_hms(2024, 1, 5, 0, 0, 0).unwrap();
    ledger.commit(rec1);

    let mut rec2 = Record::new(
        "rent".into(),
        "expenses".parse().unwrap(),
        "cash".parse().unwrap(),
        100.0,
        "USD".into(),
        None,
        None,
        vec!["rent".into()],
    )
    .unwrap();
    rec2.timestamp = Utc.with_ymd_and_hms(2024, 1, 10, 0, 0, 0).unwrap();
    ledger.commit(rec2);

    let q = Query::from_str("tag:food start:2024-01-01 end:2024-01-07").unwrap();
    let res = q.filter(&ledger);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].description, "coffee");
}
