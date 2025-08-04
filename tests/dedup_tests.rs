use std::str::FromStr;

use feed_my_ledger::{
    cloud_adapters::{CloudSpreadsheetService, GoogleSheetsAdapter},
    core::{Account, Record},
    import::dedup::filter_new_records,
};

#[test]
fn filter_new_records_skips_duplicates() {
    let mut adapter = GoogleSheetsAdapter::new();
    let sheet_id = adapter.create_sheet("test").unwrap();
    let signature = "";

    let header: Vec<String> = vec![
        "id", "timestamp", "description", "debit_account", "credit_account", "amount",
        "currency", "reference_id", "external_reference", "tags", "splits",
        "transaction_description", "hash",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    adapter.append_row(&sheet_id, header).unwrap();

    let r1 = Record::new(
        "Coffee".to_string(),
        Account::from_str("expenses:food").unwrap(),
        Account::from_str("cash").unwrap(),
        3.5,
        "USD".to_string(),
        None,
        None,
        vec![],
    )
    .unwrap();
    let r2 = Record::new(
        "Tea".to_string(),
        Account::from_str("expenses:food").unwrap(),
        Account::from_str("cash").unwrap(),
        2.0,
        "USD".to_string(),
        None,
        None,
        vec![],
    )
    .unwrap();

    let existing = r1.to_row_hashed(signature);
    adapter.append_row(&sheet_id, existing).unwrap();

    let rows =
        filter_new_records(&adapter, &sheet_id, vec![r1.clone(), r2.clone()], signature).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], r2.id.to_string());
}
