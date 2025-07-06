use feed_my_ledger::core::{Record, utils::{generate_signature, hash_row}};

#[test]
fn hash_changes_on_field_or_signature() {
    let record = Record::new(
        "desc".into(),
        "cash".parse().unwrap(),
        "revenue".parse().unwrap(),
        1.0,
        "USD".into(),
        None,
        None,
        vec![],
    ).unwrap();
    let sig1 = generate_signature("ledger", None).unwrap();
    let sig2 = generate_signature("ledger2", Some("pw")).unwrap();
    let row = record.to_row();
    let h1 = hash_row(&row, &sig1);
    let mut row_changed = row.clone();
    row_changed[2] = "other".into();
    let h2 = hash_row(&row_changed, &sig1);
    assert_ne!(h1, h2);
    let h3 = hash_row(&row, &sig2);
    assert_ne!(h1, h3);
}

#[test]
fn hash_column_ignored() {
    let record = Record::new(
        "desc".into(),
        "cash".parse().unwrap(),
        "revenue".parse().unwrap(),
        1.0,
        "USD".into(),
        None,
        None,
        vec![],
    ).unwrap();
    let sig = generate_signature("ledger", None).unwrap();
    let mut row = record.to_row_hashed(&sig);
    let orig_hash = row.last().cloned().unwrap();
    let len = row.len();
    row[len - 1] = "junk".into();
    let recomputed = hash_row(&row[..len - 1], &sig);
    assert_eq!(orig_hash, recomputed);
}

#[test]
fn to_row_hashed_appends_hash() {
    let record = Record::new(
        "desc".into(),
        "cash".parse().unwrap(),
        "revenue".parse().unwrap(),
        1.0,
        "USD".into(),
        None,
        None,
        vec![],
    ).unwrap();
    let sig = generate_signature("ledger", None).unwrap();
    let row = record.to_row_hashed(&sig);
    assert_eq!(row.len(), record.to_row().len() + 1);
    assert!(!row.last().unwrap().is_empty());
}
