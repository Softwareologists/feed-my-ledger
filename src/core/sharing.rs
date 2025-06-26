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

pub struct SharedLedger<S: CloudSpreadsheetService> {
    ledger: Mutex<Ledger>,
    service: Mutex<S>,
    sheet_id: String,
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
            permissions: Mutex::new(permissions),
        })
    }

    /// Create a ledger bound to an existing spreadsheet.
    pub fn from_sheet(service: S, sheet_id: impl Into<String>, owner: &str) -> Self {
        let mut permissions = HashMap::new();
        permissions.insert(owner.to_string(), Permission::Write);
        Self {
            ledger: Mutex::new(Ledger::default()),
            service: Mutex::new(service),
            sheet_id: sheet_id.into(),
            permissions: Mutex::new(permissions),
        }
    }

    /// Return the underlying spreadsheet identifier.
    pub fn sheet_id(&self) -> &str {
        &self.sheet_id
    }

    pub fn share_with(&self, email: &str, permission: Permission) -> Result<(), AccessError> {
        let service = self.service.lock().unwrap();
        service
            .share_sheet(&self.sheet_id, email)
            .map_err(|_| AccessError::ShareFailed)?;
        let mut perms = self.permissions.lock().unwrap();
        perms.insert(email.to_string(), permission);
        Ok(())
    }

    fn check(&self, user: &str, required: Permission) -> Result<(), AccessError> {
        let perms = self.permissions.lock().unwrap();
        match perms.get(user) {
            Some(Permission::Write) => Ok(()),
            Some(Permission::Read) if required == Permission::Read => Ok(()),
            _ => Err(AccessError::Unauthorized),
        }
    }

    pub fn commit(&self, user: &str, record: Record) -> Result<(), AccessError> {
        self.check(user, Permission::Write)?;
        {
            let mut service = self.service.lock().unwrap();
            service
                .append_row(&self.sheet_id, record.to_row())
                .map_err(|_| AccessError::ShareFailed)?;
        }
        self.ledger.lock().unwrap().commit(record);
        Ok(())
    }

    pub fn get_record(&self, user: &str, id: Uuid) -> Result<Record, AccessError> {
        self.check(user, Permission::Read)?;
        self.ledger
            .lock()
            .unwrap()
            .get_record(id)
            .cloned()
            .map_err(AccessError::Ledger)
    }

    pub fn records(&self, user: &str) -> Result<Vec<Record>, AccessError> {
        self.check(user, Permission::Read)?;
        let ledger = self.ledger.lock().unwrap();
        Ok(ledger.records().cloned().collect())
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
            .unwrap()
            .apply_adjustment(original_id, adjustment)
            .map_err(AccessError::Ledger)
    }
}
