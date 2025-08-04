use std::collections::HashSet;

use crate::cloud_adapters::{CloudSpreadsheetService, SpreadsheetError};
use crate::core::Record;

/// Filter out records already present in the target sheet.
///
/// Existing rows are identified by their hash in the last column. Records whose
/// hashed rows match existing hashes are discarded. The remaining records are
/// converted to rows ready for appending.
pub fn filter_new_records(
    adapter: &dyn CloudSpreadsheetService,
    sheet_id: &str,
    records: Vec<Record>,
    signature: &str,
) -> Result<Vec<Vec<String>>, SpreadsheetError> {
    let existing: HashSet<String> = adapter
        .list_rows(sheet_id)?
        .into_iter()
        .skip(1)
        .filter_map(|row| row.last().cloned())
        .collect();

    let mut rows = Vec::new();
    for record in records {
        let row = record.to_row_hashed(signature);
        if let Some(hash) = row.last() {
            if existing.contains(hash) {
                continue;
            }
        }
        rows.push(row);
    }
    Ok(rows)
}
