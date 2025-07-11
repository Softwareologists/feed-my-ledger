use std::collections::HashMap;
use std::sync::Mutex;

use uuid::Uuid;

use crate::cloud_adapters::{CloudSpreadsheetService, SpreadsheetError};

use super::{Ledger, LedgerError, Record};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    Read,
    Write,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessError {
    Unauthorized,
    Ledger(LedgerError),
    ShareFailed,
}

impl std::fmt::Display for AccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccessError::Unauthorized => {
                write!(f, "user does not have sufficient permissions")
            }
            AccessError::Ledger(e) => write!(f, "ledger error: {e}"),
            AccessError::ShareFailed => write!(f, "failed to share the spreadsheet"),
        }
    }
}

impl std::error::Error for AccessError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AccessError::Ledger(e) => Some(e),
            _ => None,
        }
    }
}

pub struct SharedLedger<S: CloudSpreadsheetService> {
    ledger: Mutex<Ledger>,
    service: Mutex<S>,
    sheet_id: String,
    statuses: Mutex<HashMap<Uuid, bool>>,
    permissions: Mutex<HashMap<String, Permission>>,
}

impl<S: CloudSpreadsheetService> SharedLedger<S> {
    pub fn new(mut service: S, owner: &str) -> Result<Self, SpreadsheetError> {
        let sheet_id = service.create_sheet("ledger")?;
        let mut permissions = HashMap::new();
        permissions.insert(owner.to_string(), Permission::Write);
        Ok(Self {
            ledger: Mutex::new(Ledger::default()),
            service: Mutex::new(service),
            sheet_id,
            statuses: Mutex::new(HashMap::new()),
            permissions: Mutex::new(permissions),
        })
    }

    /// Create a ledger bound to an existing spreadsheet.
    pub fn from_sheet(
        service: S,
        sheet_id: impl Into<String>,
        owner: &str,
    ) -> Result<Self, SpreadsheetError> {
        let sheet_id = sheet_id.into();
        let mut ledger = Ledger::default();
        let mut statuses = HashMap::new();
        Self::load_existing_rows(&service, &mut ledger, &mut statuses, &sheet_id)?;

        let mut permissions = HashMap::new();
        permissions.insert(owner.to_string(), Permission::Write);
        Ok(Self {
            ledger: Mutex::new(ledger),
            service: Mutex::new(service),
            sheet_id,
            statuses: Mutex::new(statuses),
            permissions: Mutex::new(permissions),
        })
    }

    fn load_existing_rows(
        service: &S,
        ledger: &mut Ledger,
        statuses: &mut HashMap<Uuid, bool>,
        sheet_id: &str,
    ) -> Result<(), SpreadsheetError> {
        let rows = service.list_rows(sheet_id)?;
        for row in rows {
            if row.first().map(|s| s.as_str()) == Some("status") {
                if row.len() >= 3 {
                    if let Ok(id) = uuid::Uuid::parse_str(&row[1]) {
                        if let Ok(c) = row[2].parse::<bool>() {
                            statuses.insert(id, c);
                        }
                    }
                }
                continue;
            }
            let rec = Self::record_from_row(&row)?;
            ledger.commit(rec);
        }
        Ok(())
    }

    fn record_from_row(row: &[String]) -> Result<Record, SpreadsheetError> {
        if row.len() < 10 {
            return Err(SpreadsheetError::Permanent("invalid row".into()));
        }

        let id = uuid::Uuid::parse_str(&row[0])
            .map_err(|e| SpreadsheetError::Permanent(e.to_string()))?;
        let timestamp = chrono::DateTime::parse_from_rfc3339(&row[1])
            .map_err(|e| SpreadsheetError::Permanent(e.to_string()))?
            .with_timezone(&chrono::Utc);
        let amount = row[5]
            .parse::<f64>()
            .map_err(|e| SpreadsheetError::Permanent(e.to_string()))?;
        let reference_id = if row[7].is_empty() {
            None
        } else {
            Some(
                uuid::Uuid::parse_str(&row[7])
                    .map_err(|e| SpreadsheetError::Permanent(e.to_string()))?,
            )
        };
        let external_reference = if row[8].is_empty() {
            None
        } else {
            Some(row[8].clone())
        };
        let tags = if row[9].is_empty() {
            Vec::new()
        } else {
            row[9].split(',').map(|s| s.to_string()).collect()
        };
        let splits_col = if row.len() > 10 { &row[10] } else { "" };
        let splits = if !splits_col.is_empty() {
            serde_json::from_str(splits_col)
                .map_err(|e| SpreadsheetError::Permanent(e.to_string()))?
        } else {
            Vec::new()
        };

        Ok(Record {
            id,
            timestamp,
            description: row[2].clone(),
            debit_account: row[3]
                .parse()
                .map_err(|e| SpreadsheetError::Permanent(format!("invalid account: {e}")))?,
            credit_account: row[4]
                .parse()
                .map_err(|e| SpreadsheetError::Permanent(format!("invalid account: {e}")))?,
            amount,
            currency: row[6].clone(),
            reference_id,
            external_reference,
            tags,
            cleared: false,
            splits,
        })
    }

    /// Return the underlying spreadsheet identifier.
    pub fn sheet_id(&self) -> &str {
        &self.sheet_id
    }

    pub fn share_with(&self, email: &str, permission: Permission) -> Result<(), AccessError> {
        let service = self.service.lock().expect("service mutex poisoned");
        service
            .share_sheet(&self.sheet_id, email)
            .map_err(|_| AccessError::ShareFailed)?;
        let mut perms = self.permissions.lock().expect("permissions mutex poisoned");
        perms.insert(email.to_string(), permission);
        Ok(())
    }

    fn check(&self, user: &str, required: Permission) -> Result<(), AccessError> {
        let perms = self.permissions.lock().expect("permissions mutex poisoned");
        match perms.get(user) {
            Some(Permission::Write) => Ok(()),
            Some(Permission::Read) if required == Permission::Read => Ok(()),
            _ => Err(AccessError::Unauthorized),
        }
    }

    pub fn commit(&self, user: &str, record: Record) -> Result<(), AccessError> {
        self.check(user, Permission::Write)?;
        {
            let mut service = self.service.lock().expect("service mutex poisoned");
            let sig = crate::core::utils::generate_signature(user, None)
                .map_err(|_| AccessError::ShareFailed)?;
            service
                .append_row(&self.sheet_id, record.to_row_hashed(&sig))
                .map_err(|_| AccessError::ShareFailed)?;
        }
        self.ledger
            .lock()
            .expect("ledger mutex poisoned")
            .commit(record.clone());
        self.statuses
            .lock()
            .expect("statuses mutex poisoned")
            .insert(record.id, record.cleared);
        Ok(())
    }

    pub fn get_record(&self, user: &str, id: Uuid) -> Result<Record, AccessError> {
        self.check(user, Permission::Read)?;
        let mut record = self
            .ledger
            .lock()
            .expect("ledger mutex poisoned")
            .get_record(id)
            .cloned()
            .map_err(AccessError::Ledger)?;
        let statuses = self.statuses.lock().expect("statuses mutex poisoned");
        record.cleared = *statuses.get(&id).unwrap_or(&false);
        Ok(record)
    }

    pub fn records(&self, user: &str) -> Result<Vec<Record>, AccessError> {
        self.check(user, Permission::Read)?;
        let ledger = self.ledger.lock().expect("ledger mutex poisoned");
        let statuses = self.statuses.lock().expect("statuses mutex poisoned");
        Ok(ledger
            .records()
            .map(|r| {
                let mut rec = r.clone();
                rec.cleared = *statuses.get(&rec.id).unwrap_or(&false);
                rec
            })
            .collect())
    }

    pub fn apply_adjustment(
        &self,
        user: &str,
        original_id: Uuid,
        adjustment: Record,
    ) -> Result<(), AccessError> {
        self.check(user, Permission::Write)?;
        self.ledger
            .lock()
            .expect("ledger mutex poisoned")
            .apply_adjustment(original_id, adjustment)
            .map_err(AccessError::Ledger)
    }

    pub fn set_cleared(&self, user: &str, id: Uuid, cleared: bool) -> Result<(), AccessError> {
        self.check(user, Permission::Write)?;
        {
            let mut service = self.service.lock().expect("service mutex poisoned");
            service
                .append_row(
                    &self.sheet_id,
                    vec!["status".into(), id.to_string(), cleared.to_string()],
                )
                .map_err(|_| AccessError::ShareFailed)?;
        }
        self.statuses
            .lock()
            .expect("statuses mutex poisoned")
            .insert(id, cleared);
        Ok(())
    }

    pub fn mark_cleared(&self, user: &str, id: Uuid) -> Result<(), AccessError> {
        self.set_cleared(user, id, true)
    }

    pub fn mark_pending(&self, user: &str, id: Uuid) -> Result<(), AccessError> {
        self.set_cleared(user, id, false)
    }

    pub fn into_parts(self) -> (S, String) {
        (
            self.service.into_inner().expect("service mutex poisoned"),
            self.sheet_id,
        )
    }
}
