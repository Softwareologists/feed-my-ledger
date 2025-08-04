use crate::cloud_adapters::{CloudSpreadsheetService, SpreadsheetError};
use crate::core::utils::hash_row;
use tracing::{debug, info};

/// Recomputes hashes for all ledger rows and returns the zero-based indices
/// of rows whose stored hash does not match the computed value.
pub fn verify_sheet(
    adapter: &dyn CloudSpreadsheetService,
    sheet_id: &str,
    signature: &str,
) -> Result<Vec<usize>, SpreadsheetError> {
    let rows = adapter.list_rows(sheet_id)?;
    info!(sheet_id, row_count = rows.len(), "Verifying sheet");
    let mut mismatched = Vec::new();
    for (idx, row) in rows.iter().enumerate() {
        if row.len() < 2 || row.first().map(|s| s.as_str()) == Some("status") {
            continue;
        }
        if let Some(stored_hash) = row.last() {
            let computed = hash_row(&row[..row.len() - 1], signature);
            if &computed != stored_hash {
                debug!(index = idx, "Row hash mismatch");
                mismatched.push(idx);
            }
        }
    }
    info!(mismatched = mismatched.len(), "Verification complete");
    Ok(mismatched)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud_adapters::GoogleSheetsAdapter;
    use crate::core::utils::generate_signature;
    use crate::core::{Account, Record};

    #[test]
    fn detect_no_tampering() {
        let mut adapter = GoogleSheetsAdapter::new();
        let sheet = adapter.create_sheet("test").unwrap();
        let sig = generate_signature("ledger", None).unwrap();
        let record = Record::new(
            "coffee".into(),
            "cash".parse::<Account>().unwrap(),
            "revenue".parse::<Account>().unwrap(),
            5.0,
            "USD".into(),
            None,
            None,
            vec![],
        )
        .unwrap();
        adapter
            .append_row(&sheet, record.to_row_hashed(&sig))
            .unwrap();
        let res = verify_sheet(&adapter, &sheet, &sig).unwrap();
        assert!(res.is_empty());
    }

    #[test]
    fn detect_tampering() {
        let mut adapter = GoogleSheetsAdapter::new();
        let sheet = adapter.create_sheet("test").unwrap();
        let sig = generate_signature("ledger", None).unwrap();
        let record = Record::new(
            "coffee".into(),
            "cash".parse::<Account>().unwrap(),
            "revenue".parse::<Account>().unwrap(),
            5.0,
            "USD".into(),
            None,
            None,
            vec![],
        )
        .unwrap();
        let mut row = record.to_row_hashed(&sig);
        adapter.append_row(&sheet, row.clone()).unwrap();
        // tamper second row by modifying description without updating hash
        row[2] = "tea".into();
        adapter.append_row(&sheet, row).unwrap();
        let res = verify_sheet(&adapter, &sheet, &sig).unwrap();
        assert_eq!(res, vec![1]);
    }
}
