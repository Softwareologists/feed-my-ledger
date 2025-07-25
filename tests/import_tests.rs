use feed_my_ledger::import::{csv, json, ledger, ofx, qif};
use std::fs::write;

fn write_temp(name: &str, content: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(name);
    write(&path, content).unwrap();
    path
}

#[test]
fn csv_parsing() {
    let data = "description,debit_account,credit_account,amount,currency\nCoffee,expenses:food,cash,3.50,USD\n";
    let path = write_temp("test.csv", data);
    let records = csv::parse(&path).unwrap();
    assert_eq!(records.len(), 1);
    let r = &records[0];
    assert_eq!(r.description, "Coffee");
    assert_eq!(r.debit_account.to_string(), "expenses:food");
    assert_eq!(r.credit_account.to_string(), "cash");
    assert_eq!(r.amount, 3.50);
    let _ = std::fs::remove_file(path);
}

#[test]
fn qif_parsing() {
    let qif_content = "!Type:Bank\nD01/01/2024\nT-10.00\nPCoffee\nM\n^\n";
    let path = write_temp("test.qif", qif_content);
    let records = qif::parse(&path).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].description, "Coffee");
    assert_eq!(records[0].amount, 10.0);
    let _ = std::fs::remove_file(path);
}

#[test]
fn qif_memo_overrides_vendor() {
    let qif_content = "!Type:Bank\nD01/02/2024\nT5.00\nPVend\nMMemo text\n^\n";
    let path = write_temp("memo.qif", qif_content);
    let records = qif::parse(&path).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].description, "Memo text");
    assert_eq!(records[0].amount, 5.0);
    let _ = std::fs::remove_file(path);
}

#[test]
fn ofx_parsing() {
    let ofx_content = r#"<OFX><BANKMSGSRSV1><STMTTRNRS><STMTRS><BANKTRANLIST>
<STMTTRN><TRNAMT>-7.00</TRNAMT><NAME>Snack</NAME></STMTTRN>
</BANKTRANLIST></STMTRS></STMTTRNRS></BANKMSGSRSV1></OFX>"#;
    let path = write_temp("test.ofx", ofx_content);
    let records = ofx::parse(&path).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].description, "Snack");
    assert_eq!(records[0].amount, 7.0);
    let _ = std::fs::remove_file(path);
}

#[test]
fn csv_parsing_with_mapping() {
    let data = "desc,credit,debit,value,curr\nCoffee,cash,expenses:food,4.20,USD\n";
    let path = write_temp("test_map.csv", data);
    let mapping = csv::CsvMapping {
        description: "desc".into(),
        debit_account: "debit".into(),
        credit_account: "credit".into(),
        amount: "value".into(),
        currency: "curr".into(),
    };
    let records = csv::parse_with_mapping(&path, &mapping).unwrap();
    assert_eq!(records.len(), 1);
    let r = &records[0];
    assert_eq!(r.description, "Coffee");
    assert_eq!(r.debit_account.to_string(), "expenses:food");
    assert_eq!(r.credit_account.to_string(), "cash");
    assert_eq!(r.amount, 4.20);
    let _ = std::fs::remove_file(path);
}

#[test]
fn ledger_and_json_roundtrip() {
    let ledger_text = "2024-01-01 Coffee\n    expenses:food  5.00 USD\n    cash\n";
    let lpath = write_temp("test.ledger", ledger_text);
    let records = ledger::parse(&lpath).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].description, "Coffee");

    let jpath = write_temp("test.json", "");
    json::export(&jpath, &records).unwrap();
    let loaded = json::parse(&jpath).unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].description, "Coffee");
    let _ = std::fs::remove_file(lpath);
    let _ = std::fs::remove_file(jpath);
}

#[test]
fn csv_export_roundtrip() {
    let ledger_text = "2024-01-01 Coffee\n    expenses:food  5.00 USD\n    cash\n";
    let lpath = write_temp("roundtrip.ledger", ledger_text);
    let records = ledger::parse(&lpath).unwrap();
    let cpath = write_temp("roundtrip.csv", "");
    csv::export(&cpath, &records).unwrap();
    let loaded = csv::parse(&cpath).unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].description, "Coffee");
    assert_eq!(loaded[0].amount, 5.0);
    let _ = std::fs::remove_file(lpath);
    let _ = std::fs::remove_file(cpath);
}
