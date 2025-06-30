use std::str::FromStr;

use chrono::NaiveDate;

use super::{Ledger, Record};

#[derive(Debug, Default, Clone)]
pub struct Query {
    pub accounts: Vec<String>,
    pub tags: Vec<String>,
    pub start: Option<NaiveDate>,
    pub end: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    InvalidToken(String),
    InvalidDate(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidToken(t) => write!(f, "invalid token: {t}"),
            ParseError::InvalidDate(d) => write!(f, "invalid date: {d}"),
        }
    }
}

impl std::error::Error for ParseError {}

impl FromStr for Query {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut q = Query::default();
        for token in s.split_whitespace() {
            if let Some(rest) = token.strip_prefix("account:") {
                q.accounts.push(rest.to_string());
            } else if let Some(rest) = token.strip_prefix("tag:") {
                q.tags.push(rest.to_string());
            } else if let Some(rest) = token.strip_prefix("start:") {
                q.start = Some(parse_date(rest)?);
            } else if let Some(rest) = token.strip_prefix("end:") {
                q.end = Some(parse_date(rest)?);
            } else if let Some(rest) = token.strip_prefix("date:") {
                let parts: Vec<&str> = rest.split("..").collect();
                if parts.len() != 2 {
                    return Err(ParseError::InvalidToken(token.into()));
                }
                if !parts[0].is_empty() {
                    q.start = Some(parse_date(parts[0])?);
                }
                if !parts[1].is_empty() {
                    q.end = Some(parse_date(parts[1])?);
                }
            } else {
                return Err(ParseError::InvalidToken(token.into()));
            }
        }
        Ok(q)
    }
}

fn parse_date(s: &str) -> Result<NaiveDate, ParseError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| ParseError::InvalidDate(s.into()))
}

impl Query {
    pub fn matches(&self, rec: &Record) -> bool {
        if let Some(start) = self.start {
            if rec.timestamp.date_naive() < start {
                return false;
            }
        }
        if let Some(end) = self.end {
            if rec.timestamp.date_naive() > end {
                return false;
            }
        }
        if !self.accounts.is_empty()
            && !self.accounts.iter().any(|a| {
                rec.postings().any(|p| {
                    a == &p.debit_account.to_string() || a == &p.credit_account.to_string()
                })
            })
        {
            return false;
        }
        if !self.tags.is_empty() && !rec.tags.iter().any(|t| self.tags.contains(t)) {
            return false;
        }
        true
    }

    pub fn filter<'a>(&self, ledger: &'a Ledger) -> Vec<&'a Record> {
        ledger.records().filter(|r| self.matches(r)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use chrono::Utc;

    #[test]
    fn parse_simple_tokens() {
        let q = Query::from_str("account:cash tag:food start:2024-01-01 end:2024-01-31").unwrap();
        assert_eq!(q.accounts, vec!["cash"]);
        assert_eq!(q.tags, vec!["food"]);
        assert_eq!(q.start, Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()));
        assert_eq!(q.end, Some(NaiveDate::from_ymd_opt(2024, 1, 31).unwrap()));
    }

    #[test]
    fn filter_records_by_tag() {
        let mut ledger = Ledger::default();
        let mut rec = Record::new(
            "coffee".into(),
            "expenses".parse().unwrap(),
            "cash".parse().unwrap(),
            3.0,
            "USD".into(),
            None,
            None,
            vec!["food".into()],
        )
        .unwrap();
        rec.timestamp = Utc.with_ymd_and_hms(2024, 1, 5, 0, 0, 0).unwrap();
        ledger.commit(rec);
        let mut rec2 = Record::new(
            "rent".into(),
            "expenses".parse().unwrap(),
            "cash".parse().unwrap(),
            100.0,
            "USD".into(),
            None,
            None,
            vec!["rent".into()],
        )
        .unwrap();
        rec2.timestamp = Utc.with_ymd_and_hms(2024, 1, 10, 0, 0, 0).unwrap();
        ledger.commit(rec2);

        let q = Query::from_str("tag:food start:2024-01-01 end:2024-01-07").unwrap();
        let res = q.filter(&ledger);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].description, "coffee");
    }
}
