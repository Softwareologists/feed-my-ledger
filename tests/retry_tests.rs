use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use feed_my_ledger::cloud_adapters::{CloudSpreadsheetService, RetryingService, SpreadsheetError};

struct FlakyAdapter {
    fail_times: usize,
    calls: Rc<RefCell<usize>>,
}

impl FlakyAdapter {
    fn new(fail_times: usize, calls: Rc<RefCell<usize>>) -> Self {
        Self { fail_times, calls }
    }
}

impl CloudSpreadsheetService for FlakyAdapter {
    fn create_sheet(&mut self, _title: &str) -> Result<String, SpreadsheetError> {
        let mut c = self.calls.borrow_mut();
        *c += 1;
        if *c <= self.fail_times {
            Err(SpreadsheetError::Transient("network".into()))
        } else {
            Ok(format!("sheet{c}"))
        }
    }

    fn append_row(
        &mut self,
        _sheet_id: &str,
        _values: Vec<String>,
    ) -> Result<(), SpreadsheetError> {
        unimplemented!()
    }

    fn read_row(&self, _sheet_id: &str, _index: usize) -> Result<Vec<String>, SpreadsheetError> {
        unimplemented!()
    }

    fn list_rows(&self, _sheet_id: &str) -> Result<Vec<Vec<String>>, SpreadsheetError> {
        unimplemented!()
    }

    fn share_sheet(&self, _sheet_id: &str, _email: &str) -> Result<(), SpreadsheetError> {
        unimplemented!()
    }
}

#[test]
fn retries_and_succeeds() {
    let calls = Rc::new(RefCell::new(0));
    let adapter = FlakyAdapter::new(2, Rc::clone(&calls));
    let mut retry = RetryingService::new(adapter, 3, Duration::from_millis(1));
    let id = retry.create_sheet("test").unwrap();
    assert_eq!(id, "sheet3");
    assert_eq!(*calls.borrow(), 3);
}

#[test]
fn gives_up_after_max_retries() {
    let calls = Rc::new(RefCell::new(0));
    let adapter = FlakyAdapter::new(5, Rc::clone(&calls));
    let mut retry = RetryingService::new(adapter, 3, Duration::from_millis(1));
    let err = retry.create_sheet("test").unwrap_err();
    assert!(matches!(err, SpreadsheetError::Transient(_)));
    assert_eq!(*calls.borrow(), 4);
}
