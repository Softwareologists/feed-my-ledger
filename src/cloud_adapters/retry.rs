use std::cell::RefCell;
use std::thread::sleep;
use std::time::Duration;

use super::{CloudSpreadsheetService, SpreadsheetError};

/// Wrapper that adds retry logic with exponential backoff to a spreadsheet service.
///
/// Transient errors are retried with exponential backoff until `max_retries`
/// is reached. The delay starts at `base_delay` and doubles after each failed
/// attempt.
pub struct RetryingService<S> {
    inner: RefCell<S>,
    max_retries: u32,
    base_delay: Duration,
}

impl<S> RetryingService<S> {
    /// Create a new `RetryingService` wrapping `inner`.
    pub fn new(inner: S, max_retries: u32, base_delay: Duration) -> Self {
        Self {
            inner: RefCell::new(inner),
            max_retries,
            base_delay,
        }
    }

    fn with_retry<T, F>(&self, mut op: F) -> Result<T, SpreadsheetError>
    where
        F: FnMut(&mut S) -> Result<T, SpreadsheetError>,
    {
        let mut attempt = 0;
        loop {
            let result = op(&mut self.inner.borrow_mut());
            match result {
                Ok(val) => return Ok(val),
                Err(e) if e.is_retryable() && attempt < self.max_retries => {
                    let factor = 2f64.powi(attempt as i32);
                    let delay = self.base_delay.mul_f64(factor);
                    sleep(delay);
                    attempt += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

impl<S: CloudSpreadsheetService> CloudSpreadsheetService for RetryingService<S> {
    fn create_sheet(&mut self, title: &str) -> Result<String, SpreadsheetError> {
        self.with_retry(|inner| inner.create_sheet(title))
    }

    fn append_row(&mut self, sheet_id: &str, values: Vec<String>) -> Result<(), SpreadsheetError> {
        self.with_retry(|inner| inner.append_row(sheet_id, values.clone()))
    }

    fn read_row(&self, sheet_id: &str, index: usize) -> Result<Vec<String>, SpreadsheetError> {
        self.with_retry(|inner| inner.read_row(sheet_id, index))
    }

    fn list_rows(&self, sheet_id: &str) -> Result<Vec<Vec<String>>, SpreadsheetError> {
        self.with_retry(|inner| inner.list_rows(sheet_id))
    }

    fn share_sheet(&self, sheet_id: &str, email: &str) -> Result<(), SpreadsheetError> {
        self.with_retry(|inner| inner.share_sheet(sheet_id, email))
    }
}
