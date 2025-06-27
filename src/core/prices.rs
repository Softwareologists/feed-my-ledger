use chrono::NaiveDate;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

#[derive(Default)]
pub struct PriceDatabase {
    rates: BTreeMap<NaiveDate, HashMap<(String, String), f64>>,
}

impl PriceDatabase {
    pub fn add_rate(&mut self, date: NaiveDate, from: &str, to: &str, rate: f64) {
        self.rates
            .entry(date)
            .or_default()
            .insert((from.to_string(), to.to_string()), rate);
    }

    pub fn get_rate(&self, date: NaiveDate, from: &str, to: &str) -> Option<f64> {
        let pair = (from.to_string(), to.to_string());
        for (_, map) in self.rates.range(..=date).rev() {
            if let Some(rate) = map.get(&pair) {
                return Some(*rate);
            }
        }
        None
    }

    pub fn from_csv(path: &Path) -> Result<Self, std::io::Error> {
        let mut db = PriceDatabase::default();
        let content = std::fs::read_to_string(path)?;
        for line in content.lines().skip(1) {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() != 4 {
                continue;
            }
            let date = NaiveDate::parse_from_str(parts[0], "%Y-%m-%d")
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad date"))?;
            let rate: f64 = parts[3]
                .parse()
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad rate"))?;
            db.add_rate(date, parts[1], parts[2], rate);
        }
        Ok(db)
    }

    pub fn to_csv(&self, path: &Path) -> Result<(), std::io::Error> {
        let mut lines = vec!["date,from,to,rate".to_string()];
        for (date, from, to, rate) in self.all_rates() {
            lines.push(format!("{},{},{},{}", date, from, to, rate));
        }
        std::fs::write(path, lines.join("\n"))
    }

    pub fn all_rates(&self) -> Vec<(NaiveDate, String, String, f64)> {
        let mut res = Vec::new();
        for (date, map) in &self.rates {
            for ((from, to), rate) in map {
                res.push((*date, from.clone(), to.clone(), *rate));
            }
        }
        res.sort_by_key(|(d, _, _, _)| *d);
        res
    }
}
