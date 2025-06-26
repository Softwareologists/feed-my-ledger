use std::cell::RefCell;
use std::rc::Rc;

use rusty_ledger::cloud_adapters::{CloudSpreadsheetService, GoogleSheetsAdapter};
use rusty_ledger::core::{AccessError, Permission, Record, SharedLedger};

struct CountingAdapter {
    inner: GoogleSheetsAdapter,
    append_calls: Rc<RefCell<usize>>,
}

impl CountingAdapter {
    fn new(append_calls: Rc<RefCell<usize>>) -> Self {
        Self {
            inner: GoogleSheetsAdapter::new(),
            append_calls,
        }
    }
}

impl CloudSpreadsheetService for CountingAdapter {
    fn create_sheet(
        &mut self,
        title: &str,
    ) -> Result<String, rusty_ledger::cloud_adapters::SpreadsheetError> {
        self.inner.create_sheet(title)
    }

    fn append_row(
        &mut self,
        sheet_id: &str,
        values: Vec<String>,
    ) -> Result<(), rusty_ledger::cloud_adapters::SpreadsheetError> {
        *self.append_calls.borrow_mut() += 1;
        self.inner.append_row(sheet_id, values)
    }

    fn read_row(
        &self,
        sheet_id: &str,
        index: usize,
    ) -> Result<Vec<String>, rusty_ledger::cloud_adapters::SpreadsheetError> {
        self.inner.read_row(sheet_id, index)
    }

    fn list_rows(
        &self,
        sheet_id: &str,
    ) -> Result<Vec<Vec<String>>, rusty_ledger::cloud_adapters::SpreadsheetError> {
        self.inner.list_rows(sheet_id)
    }

    fn share_sheet(
        &self,
        sheet_id: &str,
        email: &str,
    ) -> Result<(), rusty_ledger::cloud_adapters::SpreadsheetError> {
        self.inner.share_sheet(sheet_id, email)
    }

    fn append_rows(
        &mut self,
        sheet_id: &str,
        rows: Vec<Vec<String>>,
    ) -> Result<(), rusty_ledger::cloud_adapters::SpreadsheetError> {
        *self.append_calls.borrow_mut() += rows.len();
        self.inner.append_rows(sheet_id, rows)
    }
}

#[test]
fn commit_invokes_append_row() {
    let counter = Rc::new(RefCell::new(0));
    let adapter = CountingAdapter::new(Rc::clone(&counter));
    let ledger = SharedLedger::new(adapter, "owner@example.com").unwrap();

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

    ledger.commit("owner@example.com", record).unwrap();

    assert_eq!(*counter.borrow(), 1);
}

#[derive(Default)]
struct FailingShare;

impl CloudSpreadsheetService for FailingShare {
    fn create_sheet(
        &mut self,
        _title: &str,
    ) -> Result<String, rusty_ledger::cloud_adapters::SpreadsheetError> {
        Ok("sheet1".into())
    }

    fn append_row(
        &mut self,
        _sheet_id: &str,
        _values: Vec<String>,
    ) -> Result<(), rusty_ledger::cloud_adapters::SpreadsheetError> {
        unimplemented!()
    }

    fn read_row(
        &self,
        _sheet_id: &str,
        _index: usize,
    ) -> Result<Vec<String>, rusty_ledger::cloud_adapters::SpreadsheetError> {
        unimplemented!()
    }

    fn list_rows(
        &self,
        _sheet_id: &str,
    ) -> Result<Vec<Vec<String>>, rusty_ledger::cloud_adapters::SpreadsheetError> {
        unimplemented!()
    }

    fn share_sheet(
        &self,
        _sheet_id: &str,
        _email: &str,
    ) -> Result<(), rusty_ledger::cloud_adapters::SpreadsheetError> {
        Err(rusty_ledger::cloud_adapters::SpreadsheetError::ShareFailed)
    }
}

#[test]
fn share_with_returns_access_error() {
    let adapter = FailingShare;
    let ledger = SharedLedger::new(adapter, "owner@example.com").unwrap();
    let err = ledger
        .share_with("user@example.com", Permission::Read)
        .unwrap_err();
    assert_eq!(err, AccessError::ShareFailed);
}

#[derive(Default)]
struct FailingCreate;

impl CloudSpreadsheetService for FailingCreate {
    fn create_sheet(
        &mut self,
        _title: &str,
    ) -> Result<String, rusty_ledger::cloud_adapters::SpreadsheetError> {
        Err(rusty_ledger::cloud_adapters::SpreadsheetError::Permanent(
            "boom".into(),
        ))
    }

    fn append_row(
        &mut self,
        _sheet_id: &str,
        _values: Vec<String>,
    ) -> Result<(), rusty_ledger::cloud_adapters::SpreadsheetError> {
        unimplemented!()
    }

    fn read_row(
        &self,
        _sheet_id: &str,
        _index: usize,
    ) -> Result<Vec<String>, rusty_ledger::cloud_adapters::SpreadsheetError> {
        unimplemented!()
    }

    fn list_rows(
        &self,
        _sheet_id: &str,
    ) -> Result<Vec<Vec<String>>, rusty_ledger::cloud_adapters::SpreadsheetError> {
        unimplemented!()
    }

    fn share_sheet(
        &self,
        _sheet_id: &str,
        _email: &str,
    ) -> Result<(), rusty_ledger::cloud_adapters::SpreadsheetError> {
        unimplemented!()
    }
}

#[test]
fn new_propagates_spreadsheet_error() {
    let adapter = FailingCreate;
    let res = SharedLedger::new(adapter, "owner@example.com");
    let err = match res {
        Ok(_) => panic!("expected error"),
        Err(e) => e,
    };
    assert_eq!(
        err,
        rusty_ledger::cloud_adapters::SpreadsheetError::Permanent("boom".into())
    );
}

#[test]
fn from_sheet_loads_existing_rows() {
    let mut adapter = GoogleSheetsAdapter::new();
    let sheet = adapter.create_sheet("ledger").unwrap();
    let record = Record::new(
        "desc".into(),
        "cash".into(),
        "revenue".into(),
        2.0,
        "USD".into(),
        None,
        None,
        vec!["tag".into()],
    )
    .unwrap();
    adapter.append_row(&sheet, record.to_row()).unwrap();

    let ledger = SharedLedger::from_sheet(adapter, &sheet, "owner@example.com").unwrap();
    let records = ledger.records("owner@example.com").unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0], record);
}

#[derive(Default)]
struct FailingList;

impl CloudSpreadsheetService for FailingList {
    fn create_sheet(
        &mut self,
        _title: &str,
    ) -> Result<String, rusty_ledger::cloud_adapters::SpreadsheetError> {
        Ok("sheet1".into())
    }

    fn append_row(
        &mut self,
        _sheet_id: &str,
        _values: Vec<String>,
    ) -> Result<(), rusty_ledger::cloud_adapters::SpreadsheetError> {
        unimplemented!()
    }

    fn read_row(
        &self,
        _sheet_id: &str,
        _index: usize,
    ) -> Result<Vec<String>, rusty_ledger::cloud_adapters::SpreadsheetError> {
        unimplemented!()
    }

    fn list_rows(
        &self,
        _sheet_id: &str,
    ) -> Result<Vec<Vec<String>>, rusty_ledger::cloud_adapters::SpreadsheetError> {
        Err(rusty_ledger::cloud_adapters::SpreadsheetError::SheetNotFound)
    }

    fn share_sheet(
        &self,
        _sheet_id: &str,
        _email: &str,
    ) -> Result<(), rusty_ledger::cloud_adapters::SpreadsheetError> {
        unimplemented!()
    }
}

#[test]
fn from_sheet_propagates_errors() {
    let adapter = FailingList;
    let res = SharedLedger::from_sheet(adapter, "missing", "owner@example.com");
    let err = res.err().unwrap();
    assert_eq!(
        err,
        rusty_ledger::cloud_adapters::SpreadsheetError::SheetNotFound
    );
}
