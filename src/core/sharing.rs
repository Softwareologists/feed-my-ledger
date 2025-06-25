use std::collections::HashMap;

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
    ledger: Ledger,
    service: S,
    sheet_id: String,
    permissions: HashMap<String, Permission>,
}

impl<S: CloudSpreadsheetService> SharedLedger<S> {
    pub fn new(mut service: S, owner: &str) -> Result<Self, SpreadsheetError> {
        let sheet_id = service.create_sheet("ledger")?;
        let mut permissions = HashMap::new();
        permissions.insert(owner.to_string(), Permission::Write);
        Ok(Self {
            ledger: Ledger::default(),
            service,
            sheet_id,
            permissions,
        })
    }

    pub fn share_with(&mut self, email: &str, permission: Permission) -> Result<(), AccessError> {
        self.service
            .share_sheet(&self.sheet_id, email)
            .map_err(|_| AccessError::ShareFailed)?;
        self.permissions.insert(email.to_string(), permission);
        Ok(())
    }

    fn check(&self, user: &str, required: Permission) -> Result<(), AccessError> {
        match self.permissions.get(user) {
            Some(Permission::Write) => Ok(()),
            Some(Permission::Read) if required == Permission::Read => Ok(()),
            _ => Err(AccessError::Unauthorized),
        }
    }

    pub fn commit(&mut self, user: &str, record: Record) -> Result<(), AccessError> {
        self.check(user, Permission::Write)?;
        self.service
            .append_row(&self.sheet_id, record.to_row())
            .map_err(|_| AccessError::ShareFailed)?;
        self.ledger.commit(record);
        Ok(())
    }

    pub fn get_record(&self, user: &str, id: Uuid) -> Result<&Record, AccessError> {
        self.check(user, Permission::Read)?;
        self.ledger.get_record(id).map_err(AccessError::Ledger)
    }

    pub fn records<'a>(
        &'a self,
        user: &str,
    ) -> Result<impl Iterator<Item = &'a Record>, AccessError> {
        self.check(user, Permission::Read)?;
        Ok(self.ledger.records())
    }

    pub fn apply_adjustment(
        &mut self,
        user: &str,
        original_id: Uuid,
        adjustment: Record,
    ) -> Result<(), AccessError> {
        self.check(user, Permission::Write)?;
        self.ledger
            .apply_adjustment(original_id, adjustment)
            .map_err(AccessError::Ledger)
    }
}
