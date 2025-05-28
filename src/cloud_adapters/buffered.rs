use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

use super::{CloudSpreadsheetService, SpreadsheetError};

/// Policy used to evict cached entries.
pub enum EvictionPolicy {
    /// No eviction, cache grows without bounds.
    None,
    /// Least recently used policy with a maximum number of entries.
    Lru(usize),
}

/// Wrapper that batches writes and caches read operations.
pub struct BatchingCacheService<S> {
    inner: S,
    batch_size: usize,
    batches: RefCell<HashMap<String, Vec<Vec<String>>>>,
    cache_policy: EvictionPolicy,
    cache: RefCell<HashMap<(String, usize), Vec<String>>>, // (sheet_id, row)
    order: RefCell<VecDeque<(String, usize)>>,
}

impl<S: CloudSpreadsheetService> BatchingCacheService<S> {
    /// Create a new wrapper with the given batch size and eviction policy.
    pub fn new(inner: S, batch_size: usize, cache_policy: EvictionPolicy) -> Self {
        Self {
            inner,
            batch_size: batch_size.max(1),
            batches: RefCell::new(HashMap::new()),
            cache_policy,
            cache: RefCell::new(HashMap::new()),
            order: RefCell::new(VecDeque::new()),
        }
    }

    /// Flush pending writes for a specific sheet.
    fn flush_sheet(&mut self, sheet_id: &str) -> Result<(), SpreadsheetError> {
        if let Some(rows) = self.batches.borrow_mut().remove(sheet_id) {
            if !rows.is_empty() {
                self.inner.append_rows(sheet_id, rows)?;
            }
        }
        Ok(())
    }

    /// Flush all pending writes.
    pub fn flush(&mut self) -> Result<(), SpreadsheetError> {
        let keys: Vec<String> = self.batches.borrow().keys().cloned().collect();
        for key in keys {
            self.flush_sheet(&key)?;
        }
        Ok(())
    }

    fn cache_insert(&self, sheet_id: &str, index: usize, row: Vec<String>) {
        match self.cache_policy {
            EvictionPolicy::None => {
                self.cache
                    .borrow_mut()
                    .insert((sheet_id.to_string(), index), row);
            }
            EvictionPolicy::Lru(cap) => {
                let key = (sheet_id.to_string(), index);
                let mut cache = self.cache.borrow_mut();
                let mut order = self.order.borrow_mut();
                if cache.contains_key(&key) {
                    order.retain(|k| k != &key);
                }
                cache.insert(key.clone(), row);
                order.push_back(key.clone());
                if order.len() > cap {
                    if let Some(old) = order.pop_front() {
                        cache.remove(&old);
                    }
                }
            }
        }
    }

    fn cache_get(&self, sheet_id: &str, index: usize) -> Option<Vec<String>> {
        let key = (sheet_id.to_string(), index);
        let mut cache = self.cache.borrow_mut();
        if let Some(val) = cache.get(&key).cloned() {
            if let EvictionPolicy::Lru(_cap) = self.cache_policy {
                let mut order = self.order.borrow_mut();
                order.retain(|k| k != &key);
                order.push_back(key);
            }
            Some(val)
        } else {
            None
        }
    }
}

impl<S: CloudSpreadsheetService> Drop for BatchingCacheService<S> {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

impl<S: CloudSpreadsheetService> CloudSpreadsheetService for BatchingCacheService<S> {
    fn create_sheet(&mut self, title: &str) -> Result<String, SpreadsheetError> {
        self.inner.create_sheet(title)
    }

    fn append_row(&mut self, sheet_id: &str, values: Vec<String>) -> Result<(), SpreadsheetError> {
        let mut batches = self.batches.borrow_mut();
        let batch = batches.entry(sheet_id.to_string()).or_default();
        batch.push(values);
        if batch.len() >= self.batch_size {
            let rows = batches.remove(sheet_id).unwrap();
            drop(batches);
            self.inner.append_rows(sheet_id, rows)?;
        }
        Ok(())
    }

    fn read_row(&self, sheet_id: &str, index: usize) -> Result<Vec<String>, SpreadsheetError> {
        if let Some(cached) = self.cache_get(sheet_id, index) {
            return Ok(cached);
        }
        let row = self.inner.read_row(sheet_id, index)?;
        self.cache_insert(sheet_id, index, row.clone());
        Ok(row)
    }

    fn list_rows(&self, sheet_id: &str) -> Result<Vec<Vec<String>>, SpreadsheetError> {
        self.inner.list_rows(sheet_id)
    }

    fn share_sheet(&self, sheet_id: &str, email: &str) -> Result<(), SpreadsheetError> {
        self.inner.share_sheet(sheet_id, email)
    }

    fn append_rows(
        &mut self,
        sheet_id: &str,
        rows: Vec<Vec<String>>,
    ) -> Result<(), SpreadsheetError> {
        for row in rows {
            self.append_row(sheet_id, row)?;
        }
        Ok(())
    }
}
