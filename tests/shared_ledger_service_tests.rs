use std::cell::RefCell;
use std::rc::Rc;

use rusty_ledger::cloud_adapters::{CloudSpreadsheetService, GoogleSheetsAdapter};
use rusty_ledger::core::{Record, SharedLedger};

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
    let mut ledger = SharedLedger::new(adapter, "owner@example.com").unwrap();

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
