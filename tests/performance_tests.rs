use std::cell::RefCell;
use std::rc::Rc;

use rusty_ledger::cloud_adapters::{
    CloudSpreadsheetService, GoogleSheetsAdapter,
    buffered::{BatchingCacheService, EvictionPolicy},
};

struct CountingAdapter {
    inner: GoogleSheetsAdapter,
    append_calls: Rc<RefCell<usize>>,
    read_calls: Rc<RefCell<usize>>,
}

impl CountingAdapter {
    fn new(append_calls: Rc<RefCell<usize>>, read_calls: Rc<RefCell<usize>>) -> Self {
        Self {
            inner: GoogleSheetsAdapter::new(),
            append_calls,
            read_calls,
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
        *self.read_calls.borrow_mut() += 1;
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
fn batches_writes_until_capacity() {
    let append_calls = Rc::new(RefCell::new(0));
    let read_calls = Rc::new(RefCell::new(0));
    let adapter = CountingAdapter::new(Rc::clone(&append_calls), Rc::clone(&read_calls));
    let mut service = BatchingCacheService::new(adapter, 2, EvictionPolicy::None);
    let sheet = service.create_sheet("test").unwrap();

    service.append_row(&sheet, vec!["a".into()]).unwrap();
    // not flushed yet
    assert_eq!(*append_calls.borrow(), 0);

    service.append_row(&sheet, vec!["b".into()]).unwrap();
    // batch size reached -> two rows appended
    assert_eq!(*append_calls.borrow(), 2);

    service.append_row(&sheet, vec!["c".into()]).unwrap();
    // pending one row
    assert_eq!(*append_calls.borrow(), 2);

    service.flush().unwrap();
    assert_eq!(*append_calls.borrow(), 3);
}

#[test]
fn cache_respects_lru_policy() {
    let append_calls = Rc::new(RefCell::new(0));
    let read_calls = Rc::new(RefCell::new(0));
    let adapter = CountingAdapter::new(Rc::clone(&append_calls), Rc::clone(&read_calls));
    let mut service = BatchingCacheService::new(adapter, 1, EvictionPolicy::Lru(1));
    let sheet = service.create_sheet("test").unwrap();

    service.append_row(&sheet, vec!["a".into()]).unwrap();
    service.append_row(&sheet, vec!["b".into()]).unwrap();
    service.flush().unwrap();

    let r1 = service.read_row(&sheet, 0).unwrap();
    assert_eq!(r1, vec!["a"]);
    assert_eq!(*read_calls.borrow(), 1);

    // cached
    let r1_again = service.read_row(&sheet, 0).unwrap();
    assert_eq!(r1_again, vec!["a"]);
    assert_eq!(*read_calls.borrow(), 1);

    // read second row -> evicts first (capacity 1)
    let r2 = service.read_row(&sheet, 1).unwrap();
    assert_eq!(r2, vec!["b"]);
    assert_eq!(*read_calls.borrow(), 2);

    // first row should be fetched again
    let r1_third = service.read_row(&sheet, 0).unwrap();
    assert_eq!(r1_third, vec!["a"]);
    assert_eq!(*read_calls.borrow(), 3);
}
