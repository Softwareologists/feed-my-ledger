use chrono::{DateTime, Utc};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::{Account, Record};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordTemplate {
    pub description: String,
    pub debit: Account,
    pub credit: Account,
    pub amount: f64,
    pub currency: String,
}

impl RecordTemplate {
    pub fn to_record(&self, timestamp: DateTime<Utc>) -> Result<Record, super::RecordError> {
        let mut rec = Record::new(
            self.description.clone(),
            self.debit.clone(),
            self.credit.clone(),
            self.amount,
            self.currency.clone(),
            None,
            None,
            vec![],
        )?;
        rec.timestamp = timestamp;
        Ok(rec)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleEntry {
    pub cron: String,
    pub template: RecordTemplate,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Scheduler {
    pub entries: Vec<ScheduleEntry>,
}

impl Scheduler {
    pub fn generate(&self, since: DateTime<Utc>, until: DateTime<Utc>) -> Vec<Record> {
        let mut out = Vec::new();
        for entry in &self.entries {
            if let Ok(schedule) = Schedule::from_str(&entry.cron) {
                for datetime in schedule.after(&since).take_while(|d| *d <= until) {
                    if let Ok(rec) = entry.template.to_record(datetime) {
                        out.push(rec);
                    }
                }
            }
        }
        out
    }
}
