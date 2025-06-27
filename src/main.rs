use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use rusty_ledger::cloud_adapters::{CloudSpreadsheetService, google_sheets4::GoogleSheets4Adapter};
use rusty_ledger::core::{Ledger, Query, Record};
use rusty_ledger::import;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;
use yup_oauth2::{self, InstalledFlowAuthenticator, InstalledFlowReturnMethod};

#[derive(Serialize, Deserialize, Default)]
struct GoogleSheetsConfig {
    credentials_path: String,
    spreadsheet_id: Option<String>,
    sheet_name: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct Config {
    google_sheets: GoogleSheetsConfig,
}

#[derive(Args, Debug, Default)]
struct CsvMapArgs {
    #[arg(long, help = "Column name for the description field")]
    map_description: Option<String>,
    #[arg(long, help = "Column name for the debit account field")]
    map_debit: Option<String>,
    #[arg(long, help = "Column name for the credit account field")]
    map_credit: Option<String>,
    #[arg(long, help = "Column name for the amount field")]
    map_amount: Option<String>,
    #[arg(long, help = "Column name for the currency field")]
    map_currency: Option<String>,
}

impl CsvMapArgs {
    fn into_mapping(self) -> Option<import::csv::CsvMapping> {
        if self.map_description.is_none()
            && self.map_debit.is_none()
            && self.map_credit.is_none()
            && self.map_amount.is_none()
            && self.map_currency.is_none()
        {
            return None;
        }
        Some(import::csv::CsvMapping {
            description: self
                .map_description
                .unwrap_or_else(|| "description".to_string()),
            debit_account: self
                .map_debit
                .unwrap_or_else(|| "debit_account".to_string()),
            credit_account: self
                .map_credit
                .unwrap_or_else(|| "credit_account".to_string()),
            amount: self.map_amount.unwrap_or_else(|| "amount".to_string()),
            currency: self.map_currency.unwrap_or_else(|| "currency".to_string()),
        })
    }
}

#[derive(Parser)]
#[command(name = "ledger", about = "Interact with a cloud ledger")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Perform OAuth login and store credentials
    Login,
    /// Add a new record to the ledger
    Add {
        #[arg(long)]
        description: String,
        #[arg(long)]
        debit: String,
        #[arg(long)]
        credit: String,
        #[arg(long)]
        amount: f64,
        #[arg(long)]
        currency: String,
    },
    /// List all rows in the active sheet
    List,
    /// Display a register of records
    Register {
        #[arg(long)]
        query: Option<String>,
    },
    /// Apply an adjustment referencing an existing record
    Adjust {
        #[arg(long)]
        id: String,
        #[arg(long)]
        description: String,
        #[arg(long)]
        debit: String,
        #[arg(long)]
        credit: String,
        #[arg(long)]
        amount: f64,
        #[arg(long)]
        currency: String,
    },
    /// Share the sheet with another user
    Share {
        #[arg(long)]
        email: String,
        #[arg(long, default_value = "read")]
        permission: String,
    },
    /// Import transactions from a file
    Import {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        format: Option<String>,
        #[command(flatten)]
        mapping: CsvMapArgs,
    },
    /// Display the balance for an account
    Balance {
        #[arg(long)]
        account: String,
        #[arg(long)]
        query: Option<String>,
    },
    /// Switch active sheet using a link or ID
    Switch {
        #[arg(long)]
        link: String,
    },
}

#[derive(Debug)]
enum CliError {
    MissingConfig,
    InvalidConfig(String),
    MissingCredentials,
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::MissingConfig => write!(f, "config.toml file not found"),
            CliError::InvalidConfig(msg) => write!(f, "invalid configuration: {msg}"),
            CliError::MissingCredentials => write!(f, "credentials json file was not found"),
        }
    }
}

impl std::error::Error for CliError {}

fn load_config(path: &PathBuf) -> Result<Config, CliError> {
    let data = fs::read_to_string(path).map_err(|_| CliError::MissingConfig)?;
    let cfg: Config = toml::from_str(&data).map_err(|e| CliError::InvalidConfig(e.to_string()))?;
    if cfg.google_sheets.credentials_path.is_empty() {
        return Err(CliError::InvalidConfig(
            "google_sheets.credentials_path is missing".to_string(),
        ));
    }
    Ok(cfg)
}

fn save_config(path: &PathBuf, cfg: &Config) {
    if let Ok(data) = toml::to_string(cfg) {
        let _ = fs::write(path, data);
    }
}

fn parse_sheet_id(input: &str) -> String {
    if let Some(start) = input.find("/d/") {
        let rest = &input[start + 3..];
        let end = rest.find('/').unwrap_or(rest.len());
        rest[..end].to_string()
    } else {
        input.to_string()
    }
}

fn record_from_row(row: &[String]) -> Option<Record> {
    if row.len() < 10 {
        return None;
    }

    let amount = row[5].parse::<f64>().ok()?;
    Some(Record {
        id: Uuid::nil(),
        timestamp: Utc::now(),
        description: row[2].clone(),
        debit_account: row[3].clone(),
        credit_account: row[4].clone(),
        amount,
        currency: row[6].clone(),
        reference_id: if row[7].is_empty() {
            None
        } else {
            Uuid::parse_str(&row[7]).ok()
        },
        external_reference: if row[8].is_empty() {
            None
        } else {
            Some(row[8].clone())
        },
        tags: if row[9].is_empty() {
            Vec::new()
        } else {
            row[9].split(',').map(|s| s.to_string()).collect()
        },
    })
}

async fn adapter_from_config(
    cfg: &GoogleSheetsConfig,
) -> Result<GoogleSheets4Adapter, Box<dyn std::error::Error>> {
    if !std::path::Path::new(&cfg.credentials_path).exists() {
        return Err(Box::new(CliError::MissingCredentials));
    }
    let secret = yup_oauth2::read_application_secret(&cfg.credentials_path)
        .await
        .map_err(|e| {
            Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error>
        })?;
    let auth = InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::Interactive)
        .persist_tokens_to_disk("tokens.json")
        .build()
        .await?;

    let adapter = match &cfg.sheet_name {
        Some(name) => GoogleSheets4Adapter::with_sheet_name(auth, name.clone()),
        None => GoogleSheets4Adapter::new(auth),
    };
    Ok(adapter)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    let cli = Cli::parse();
    let config_path = PathBuf::from("config.toml");
    let mut cfg =
        load_config(&config_path).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    if matches!(cli.command, Commands::Login) {
        rt.block_on(rusty_ledger::cloud_adapters::auth::initial_oauth_login(
            &cfg.google_sheets.credentials_path,
            "tokens.json",
        ))?;
        println!("Login successful");
        return Ok(());
    }

    if let Commands::Switch { link } = &cli.command {
        let id = parse_sheet_id(link);
        cfg.google_sheets.spreadsheet_id = Some(id.clone());
        save_config(&config_path, &cfg);
        println!("Active sheet set to {id}");
        return Ok(());
    }

    let mut adapter = rt.block_on(adapter_from_config(&cfg.google_sheets))?;
    let sheet_id = match &cfg.google_sheets.spreadsheet_id {
        Some(id) => id.clone(),
        None => {
            let id = adapter.create_sheet("ledger")?;
            cfg.google_sheets.spreadsheet_id = Some(id.clone());
            save_config(&config_path, &cfg);
            id
        }
    };

    match cli.command {
        Commands::Add {
            description,
            debit,
            credit,
            amount,
            currency,
        } => {
            let record = Record::new(
                description,
                debit,
                credit,
                amount,
                currency,
                None,
                None,
                vec![],
            )?;
            adapter.append_row(&sheet_id, record.to_row())?;
        }
        Commands::List => {
            let rows = adapter.list_rows(&sheet_id)?;
            for row in rows {
                println!("{}", row.join(" | "));
            }
        }
        Commands::Register { query } => {
            let rows = adapter.list_rows(&sheet_id)?;
            let mut ledger = Ledger::default();
            for row in rows {
                if let Some(rec) = record_from_row(&row) {
                    ledger.commit(rec);
                }
            }
            let q = match query {
                Some(expr) => Query::from_str(&expr)?,
                None => Query::default(),
            };
            for rec in q.filter(&ledger) {
                println!(
                    "{} | {} | {} | {} | {}",
                    rec.timestamp.to_rfc3339(),
                    rec.debit_account,
                    rec.credit_account,
                    rec.amount,
                    rec.description
                );
            }
        }
        Commands::Adjust {
            id,
            description,
            debit,
            credit,
            amount,
            currency,
        } => {
            let reference = uuid::Uuid::parse_str(&id)?;
            let mut record = Record::new(
                description,
                debit,
                credit,
                amount,
                currency,
                None,
                None,
                vec![],
            )?;
            record.reference_id = Some(reference);
            adapter.append_row(&sheet_id, record.to_row())?;
        }
        Commands::Share { email, .. } => {
            adapter
                .share_sheet(&sheet_id, &email)
                .map_err(|e| format!("{e}"))?;
            println!("Shared with {email}");
        }
        Commands::Import {
            file,
            format,
            mapping,
        } => {
            let fmt = format
                .or_else(|| {
                    file.extension()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                })
                .ok_or_else(|| "could not determine file format".to_string())?;
            let records = match fmt.to_lowercase().as_str() {
                "csv" => {
                    if let Some(map) = mapping.into_mapping() {
                        import::csv::parse_with_mapping(&file, &map)
                    } else {
                        import::csv::parse(&file)
                    }
                }
                "qif" => import::qif::parse(&file),
                "ofx" => import::ofx::parse(&file),
                other => return Err(format!("unsupported format: {other}").into()),
            }?;
            for rec in records {
                adapter.append_row(&sheet_id, rec.to_row())?;
            }
        }
        Commands::Balance { account, query } => {
            let rows = adapter.list_rows(&sheet_id)?;
            let mut ledger = Ledger::default();
            for row in rows {
                if let Some(rec) = record_from_row(&row) {
                    ledger.commit(rec);
                }
            }
            let mut q = match query {
                Some(expr) => Query::from_str(&expr)?,
                None => Query::default(),
            };
            q.accounts.push(account.clone());
            let mut balance = 0.0;
            for rec in q.filter(&ledger) {
                if rec.debit_account == account {
                    balance += rec.amount;
                }
                if rec.credit_account == account {
                    balance -= rec.amount;
                }
            }
            println!("{balance}");
        }
        Commands::Switch { .. } | Commands::Login => unreachable!(),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::CsvMapArgs;

    #[test]
    fn mapping_conversion_none() {
        let args = CsvMapArgs::default();
        assert!(args.into_mapping().is_none());
    }

    #[test]
    fn mapping_conversion_values() {
        let args = CsvMapArgs {
            map_description: Some("desc".into()),
            map_debit: Some("debit".into()),
            map_credit: Some("credit".into()),
            map_amount: Some("amount".into()),
            map_currency: Some("curr".into()),
        };
        let mapping = args.into_mapping().unwrap();
        assert_eq!(mapping.description, "desc");
        assert_eq!(mapping.debit_account, "debit");
        assert_eq!(mapping.credit_account, "credit");
        assert_eq!(mapping.amount, "amount");
        assert_eq!(mapping.currency, "curr");
    }
}
