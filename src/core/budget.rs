use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(test)]
use super::Record;
use super::{Account, Ledger, PriceDatabase};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Period {
    Monthly,
    Yearly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub account: Account,
    pub amount: f64,
    pub currency: String,
    pub period: Period,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct BudgetBook {
    monthly: HashMap<(Account, i32, u32), Budget>,
    yearly: HashMap<(Account, i32), Budget>,
}

impl BudgetBook {
    pub fn add(&mut self, budget: Budget, year: Option<i32>, month: Option<u32>) {
        match budget.period {
            Period::Monthly => {
                let y = year.unwrap_or_else(|| Utc::now().year());
                let m = month.unwrap_or_else(|| Utc::now().month());
                self.monthly.insert((budget.account.clone(), y, m), budget);
            }
            Period::Yearly => {
                let y = year.unwrap_or_else(|| Utc::now().year());
                self.yearly.insert((budget.account.clone(), y), budget);
            }
        }
    }

    pub fn compare_month(
        &self,
        ledger: &Ledger,
        prices: &PriceDatabase,
        account: &Account,
        year: i32,
        month: u32,
    ) -> Option<f64> {
        let b = self.monthly.get(&(account.clone(), year, month))?;
        let start = NaiveDate::from_ymd_opt(year, month, 1)?;
        let (next_y, next_m) = if month == 12 {
            (year + 1, 1)
        } else {
            (year, month + 1)
        };
        let end = NaiveDate::from_ymd_opt(next_y, next_m, 1)?.pred_opt()?;
        let actual = account_sum(ledger, account, start, end, &b.currency, prices);
        Some(b.amount - actual)
    }

    pub fn compare_year(
        &self,
        ledger: &Ledger,
        prices: &PriceDatabase,
        account: &Account,
        year: i32,
    ) -> Option<f64> {
        let b = self.yearly.get(&(account.clone(), year))?;
        let start = NaiveDate::from_ymd_opt(year, 1, 1)?;
        let end = NaiveDate::from_ymd_opt(year, 12, 31)?;
        let actual = account_sum(ledger, account, start, end, &b.currency, prices);
        Some(b.amount - actual)
    }
}

use chrono::Utc;

fn account_sum(
    ledger: &Ledger,
    account: &Account,
    start: NaiveDate,
    end: NaiveDate,
    target: &str,
    prices: &PriceDatabase,
) -> f64 {
    ledger.records().fold(0.0, |mut acc, r| {
        let date = r.timestamp.date_naive();
        if date < start || date > end {
            return acc;
        }
        for p in r.postings() {
            let mut amount = p.amount;
            if r.currency != target {
                if let Some(rate) = prices.get_rate(date, &r.currency, target) {
                    amount *= rate;
                } else {
                    continue;
                }
            }
            if p.debit_account.starts_with(account) {
                acc += amount;
            }
            if p.credit_account.starts_with(account) {
                acc -= amount;
            }
        }
        acc
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn monthly_comparison() {
        let mut ledger = Ledger::default();
        let mut rec = Record::new(
            "groceries".into(),
            "expenses:food".parse().unwrap(),
            "cash".parse().unwrap(),
            80.0,
            "USD".into(),
            None,
            None,
            vec![],
        )
        .unwrap();
        rec.timestamp = Utc.with_ymd_and_hms(2024, 1, 5, 0, 0, 0).unwrap();
        ledger.commit(rec);
        let mut book = BudgetBook::default();
        book.add(
            Budget {
                account: "expenses:food".parse().unwrap(),
                amount: 100.0,
                currency: "USD".into(),
                period: Period::Monthly,
            },
            Some(2024),
            Some(1),
        );
        let diff = book
            .compare_month(
                &ledger,
                &PriceDatabase::default(),
                &"expenses:food".parse().unwrap(),
                2024,
                1,
            )
            .unwrap();
        assert_eq!(diff, 20.0);
    }

    #[test]
    fn yearly_comparison() {
        let mut ledger = Ledger::default();
        for m in 1..=2 {
            let mut rec = Record::new(
                "expense".into(),
                "expenses".parse().unwrap(),
                "cash".parse().unwrap(),
                50.0,
                "USD".into(),
                None,
                None,
                vec![],
            )
            .unwrap();
            rec.timestamp = Utc.with_ymd_and_hms(2024, m, 10, 0, 0, 0).unwrap();
            ledger.commit(rec);
        }
        let mut book = BudgetBook::default();
        book.add(
            Budget {
                account: "expenses".parse().unwrap(),
                amount: 150.0,
                currency: "USD".into(),
                period: Period::Yearly,
            },
            Some(2024),
            None,
        );
        let diff = book
            .compare_year(
                &ledger,
                &PriceDatabase::default(),
                &"expenses".parse().unwrap(),
                2024,
            )
            .unwrap();
        assert_eq!(diff, 50.0);
    }
}
