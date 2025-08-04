use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use feed_my_ledger::cloud_adapters::{
    CloudSpreadsheetService, FileAdapter, google_sheets4::GoogleSheets4Adapter,
};
use feed_my_ledger::core::{
    Account, Budget, BudgetBook, Ledger, Period, Posting, PriceDatabase, Query, Record,
    utils::generate_signature, verify_sheet,
};
use feed_my_ledger::import;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::{debug, info};
use uuid::Uuid;
use yup_oauth2::{self, InstalledFlowAuthenticator, InstalledFlowReturnMethod};

#[derive(Serialize, Deserialize, Default)]
struct GoogleSheetsConfig {
    credentials_path: String,
    spreadsheet_id: Option<String>,
    sheet_name: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct BudgetConfig {
    account: String,
    amount: f64,
    currency: String,
    period: String,
}

#[derive(Serialize, Deserialize, Default)]
struct ScheduleConfig {
    cron: String,
    description: String,
    debit: String,
    credit: String,
    amount: f64,
    currency: String,
}

#[derive(Serialize, Deserialize, Default)]
struct Config {
    /// The unique, non-empty name of this ledger instance (required).
    name: String,
    /// Optional password for row signature generation (never logged).
    password: Option<String>,
    google_sheets: GoogleSheetsConfig,
    #[serde(default)]
    budgets: Vec<BudgetConfig>,
    #[serde(default)]
    schedules: Vec<ScheduleConfig>,
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

#[derive(Deserialize)]
struct CliPosting {
    debit: String,
    credit: String,
    amount: f64,
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

#[derive(Parser, Debug)]
#[command(name = "ledger", about = "Interact with a cloud ledger")]
struct Cli {
    /// Directory for local CSV storage. When set, the CLI uses FileAdapter
    /// instead of a cloud service.
    #[arg(long)]
    local_dir: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum BudgetCommands {
    Add {
        #[arg(long)]
        account: String,
        #[arg(long)]
        amount: f64,
        #[arg(long)]
        currency: String,
        #[arg(long, default_value = "monthly")]
        period: String,
    },
    Report {
        #[arg(long)]
        account: String,
        #[arg(long)]
        year: i32,
        #[arg(long)]
        month: Option<u32>,
    },
}

#[derive(Subcommand, Debug)]
enum ScheduleCommands {
    Add {
        #[arg(long)]
        cron: String,
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
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(subcommand)]
    Budget(BudgetCommands),
    #[command(subcommand)]
    Schedule(ScheduleCommands),
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
        #[arg(long, help = "JSON array of additional postings")]
        splits: Option<String>,
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
        #[arg(long)]
        currency: Option<String>,
        #[command(flatten)]
        mapping: CsvMapArgs,
    },
    /// Export ledger data to a file
    Export {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        format: Option<String>,
    },
    #[cfg(feature = "bank-api")]
    /// Download and import OFX data from a URL
    Download {
        #[arg(long)]
        url: String,
    },
    /// Display the balance for an account
    Balance {
        #[arg(long)]
        account: String,
        #[arg(long)]
        query: Option<String>,
    },
    /// Import price data from a CSV file
    ImportPrices {
        #[arg(long)]
        file: PathBuf,
    },
    /// List loaded prices
    ListPrices,
    /// Switch active sheet using a link or ID
    Switch {
        #[arg(long)]
        link: String,
    },
    /// Reconcile ledger records with a statement file
    Reconcile {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        format: Option<String>,
    },
    /// Execute a Rhai script against the current ledger
    RunScript {
        #[arg(long)]
        file: PathBuf,
    },
    /// Verify stored rows against their hashes
    Verify,
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
    // Validate 'name' field: must be present and non-empty
    if cfg.name.trim().is_empty() {
        return Err(CliError::InvalidConfig(
            "'name' field is missing or empty in config.toml".to_string(),
        ));
    }
    // Optionally: enforce uniqueness of 'name' if multiple ledgers are supported (not implemented here)
    if cfg.google_sheets.credentials_path.is_empty() {
        return Err(CliError::InvalidConfig(
            "google_sheets.credentials_path is missing".to_string(),
        ));
    }
    // Never log or expose the password field
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
    if row.len() < 10 || row.first().map(|s| s.as_str()) == Some("status") {
        return None;
    }

    let amount = row[5].parse::<f64>().ok()?;
    let splits_col = if row.len() > 10 { &row[10] } else { "" };
    let tx_desc = if row.len() > 11 { &row[11] } else { "" };
    Some(Record {
        id: Uuid::nil(),
        timestamp: Utc::now(),
        description: row[2].clone(),
        debit_account: row[3].parse().ok()?,
        credit_account: row[4].parse().ok()?,
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
        transaction_description: if tx_desc.is_empty() {
            None
        } else {
            Some(tx_desc.to_string())
        },
        cleared: false,
        splits: if !splits_col.is_empty() {
            serde_json::from_str(splits_col).ok()?
        } else {
            Vec::new()
        },
    })
}

fn status_from_row(row: &[String]) -> Option<(Uuid, bool)> {
    if row.len() >= 3 && row.first().map(|s| s.as_str()) == Some("status") {
        let id = Uuid::parse_str(&row[1]).ok()?;
        let cleared = row[2].parse::<bool>().ok()?;
        Some((id, cleared))
    } else {
        None
    }
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

fn import_with_progress(
    adapter: &mut dyn CloudSpreadsheetService,
    sheet_id: &str,
    file: &Path,
    format: Option<String>,
    mapping: CsvMapArgs,
    currency: Option<String>,
    signature: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let fmt = format
        .or_else(|| {
            file.extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .ok_or_else(|| "could not determine file format".to_string())?;
    let mapping = mapping.into_mapping();
    let records = match fmt.to_lowercase().as_str() {
        "csv" => {
            if let Some(cur) = currency.as_deref() {
                if let Some(ref map) = mapping {
                    import::csv::parse_with_mapping_and_currency(file, map, cur)
                } else {
                    import::csv::parse_with_currency(file, cur)
                }
            } else if let Some(ref map) = mapping {
                import::csv::parse_with_mapping(file, map)
            } else {
                import::csv::parse(file)
            }
        }
        "qif" => match currency.as_deref() {
            Some(cur) => import::qif::parse_with_currency(file, cur),
            None => import::qif::parse(file),
        },
        "ofx" => match currency.as_deref() {
            Some(cur) => import::ofx::parse_with_currency(file, cur),
            None => import::ofx::parse(file),
        },
        "ledger" => match currency.as_deref() {
            Some(cur) => import::ledger::parse_with_currency(file, cur),
            None => import::ledger::parse(file),
        },
        "json" => match currency.as_deref() {
            Some(cur) => import::json::parse_with_currency(file, cur),
            None => import::json::parse(file),
        },
        other => return Err(format!("unsupported format: {other}").into()),
    }?;

    let pb = indicatif::ProgressBar::new(records.len() as u64);
    for rec in records {
        adapter.append_row(sheet_id, rec.to_row_hashed(signature))?;
        pb.inc(1);
    }
    pb.finish_with_message("done");

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stdout)
        .init();
    info!("Starting feed-my-ledger");
    let rt = tokio::runtime::Runtime::new()?;
    let cli = Cli::parse();
    debug!(?cli, "Parsed CLI arguments");
    let Cli { local_dir, command } = cli;
    let config_path = PathBuf::from("config.toml");
    let mut cfg =
        load_config(&config_path).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    let signature = generate_signature(&cfg.name, cfg.password.as_deref())
        .map_err(|e| Box::new(CliError::InvalidConfig(e)) as Box<dyn std::error::Error>)?;

    if matches!(command, Commands::Login) {
        rt.block_on(feed_my_ledger::cloud_adapters::auth::initial_oauth_login(
            &cfg.google_sheets.credentials_path,
            "tokens.json",
        ))?;
        println!("Login successful");
        return Ok(());
    }

    if let Commands::Switch { link } = &command {
        let id = parse_sheet_id(link);
        cfg.google_sheets.spreadsheet_id = Some(id.clone());
        save_config(&config_path, &cfg);
        println!("Active sheet set to {id}");
        return Ok(());
    }

    let mut adapter: Box<dyn CloudSpreadsheetService> = if let Some(dir) = &local_dir {
        std::fs::create_dir_all(dir)?;
        Box::new(FileAdapter::new(dir))
    } else {
        Box::new(rt.block_on(adapter_from_config(&cfg.google_sheets))?)
    };
    let sheet_id = match &cfg.google_sheets.spreadsheet_id {
        Some(id) => id.clone(),
        None => {
            let id = adapter.create_sheet("ledger")?;
            cfg.google_sheets.spreadsheet_id = Some(id.clone());
            save_config(&config_path, &cfg);
            id
        }
    };

    info!(?command, "Dispatching command");
    match command {
        Commands::Budget(BudgetCommands::Add {
            account,
            amount,
            currency,
            period,
        }) => {
            cfg.budgets.push(BudgetConfig {
                account,
                amount,
                currency,
                period,
            });
            save_config(&config_path, &cfg);
            println!("Budget added");
        }
        Commands::Budget(BudgetCommands::Report {
            account,
            year,
            month,
        }) => {
            let rows = adapter.list_rows(&sheet_id)?;
            let mut ledger = Ledger::default();
            for row in rows {
                if let Some(rec) = record_from_row(&row) {
                    ledger.commit(rec);
                }
            }
            let prices = if Path::new("prices.csv").exists() {
                PriceDatabase::from_csv(Path::new("prices.csv"))?
            } else {
                PriceDatabase::default()
            };
            let mut book = BudgetBook::default();
            for b in &cfg.budgets {
                book.add(
                    Budget {
                        account: b.account.parse()?,
                        amount: b.amount,
                        currency: b.currency.clone(),
                        period: if b.period.to_lowercase() == "yearly" {
                            Period::Yearly
                        } else {
                            Period::Monthly
                        },
                    },
                    Some(year),
                    month,
                );
            }
            let acc: Account = account.parse()?;
            let diff = if let Some(m) = month {
                book.compare_month(&ledger, &prices, &acc, year, m)
            } else {
                book.compare_year(&ledger, &prices, &acc, year)
            };
            if let Some(d) = diff {
                println!("{d}");
            }
        }
        Commands::Schedule(ScheduleCommands::Add {
            cron,
            description,
            debit,
            credit,
            amount,
            currency,
        }) => {
            cfg.schedules.push(ScheduleConfig {
                cron,
                description,
                debit,
                credit,
                amount,
                currency,
            });
            save_config(&config_path, &cfg);
            println!("Schedule added");
        }
        Commands::Add {
            description,
            debit,
            credit,
            amount,
            currency,
            splits,
        } => {
            let mut postings = vec![Posting {
                debit_account: debit.parse()?,
                credit_account: credit.parse()?,
                amount,
            }];
            if let Some(data) = splits {
                let extra: Vec<CliPosting> = serde_json::from_str(&data)?;
                for p in extra {
                    postings.push(Posting {
                        debit_account: p.debit.parse()?,
                        credit_account: p.credit.parse()?,
                        amount: p.amount,
                    });
                }
            }
            let record = Record::new_split(description, postings, currency, None, None, vec![])?;
            adapter.append_row(&sheet_id, record.to_row_hashed(&signature))?;
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
                debit.parse()?,
                credit.parse()?,
                amount,
                currency,
                None,
                None,
                vec![],
            )?;
            record.reference_id = Some(reference);
            adapter.append_row(&sheet_id, record.to_row_hashed(&signature))?;
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
            currency,
            mapping,
        } => {
            import_with_progress(
                &mut *adapter,
                &sheet_id,
                &file,
                format,
                mapping,
                currency,
                &signature,
            )?;
        }
        Commands::Export { file, format } => {
            let rows = adapter.list_rows(&sheet_id)?;
            let mut records = Vec::new();
            for row in rows {
                if let Some(rec) = record_from_row(&row) {
                    records.push(rec);
                }
            }
            let fmt = format
                .or_else(|| {
                    file.extension()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                })
                .ok_or_else(|| "could not determine file format".to_string())?;
            match fmt.to_lowercase().as_str() {
                "csv" => import::csv::export(&file, &records)?,
                "ledger" => import::ledger::export(&file, &records)?,
                "json" => import::json::export(&file, &records)?,
                other => return Err(format!("unsupported format: {other}").into()),
            }
        }
        #[cfg(feature = "bank-api")]
        Commands::Download { url } => {
            let records = rt.block_on(import::ofx::download(&url))?;
            for rec in records {
                adapter.append_row(&sheet_id, rec.to_row_hashed(&signature))?;
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
            let account_parsed: Account = account.parse()?;
            let mut balance = 0.0;
            for rec in q.filter(&ledger) {
                if rec.debit_account.starts_with(&account_parsed) {
                    balance += rec.amount;
                }
                if rec.credit_account.starts_with(&account_parsed) {
                    balance -= rec.amount;
                }
            }
            println!("{balance}");
        }
        Commands::ImportPrices { file } => {
            let db = PriceDatabase::from_csv(&file)?;
            db.to_csv(Path::new("prices.csv"))?;
            println!("Imported {} prices", db.all_rates().len());
        }
        Commands::ListPrices => {
            let path = Path::new("prices.csv");
            if path.exists() {
                let db = PriceDatabase::from_csv(path)?;
                for (date, from, to, rate) in db.all_rates() {
                    println!("{date} {from}->{to} {rate}");
                }
            }
        }
        Commands::Reconcile { file, format } => {
            let fmt = format
                .or_else(|| {
                    file.extension()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                })
                .ok_or_else(|| "could not determine file format".to_string())?;
            let statements = match fmt.to_lowercase().as_str() {
                "csv" => import::csv::parse(&file),
                "qif" => import::qif::parse(&file),
                "ofx" => import::ofx::parse(&file),
                "ledger" => import::ledger::parse(&file),
                "json" => import::json::parse(&file),
                other => return Err(format!("unsupported format: {other}").into()),
            }?;
            let rows = adapter.list_rows(&sheet_id)?;
            let mut ledger = Ledger::default();
            let mut statuses: HashMap<Uuid, bool> = HashMap::new();
            for row in rows {
                if let Some(rec) = record_from_row(&row) {
                    ledger.commit(rec);
                } else if let Some((id, cleared)) = status_from_row(&row) {
                    statuses.insert(id, cleared);
                }
            }
            for rec in ledger.records() {
                let mut matched = false;
                for stmt in &statements {
                    if stmt.description == rec.description
                        && (stmt.amount - rec.amount).abs() < f64::EPSILON
                    {
                        matched = true;
                        break;
                    }
                }
                if statuses.get(&rec.id).copied() != Some(matched) {
                    adapter.append_row(
                        &sheet_id,
                        vec!["status".into(), rec.id.to_string(), matched.to_string()],
                    )?;
                }
            }
        }
        Commands::RunScript { file } => {
            let rows = adapter.list_rows(&sheet_id)?;
            let mut ledger = Ledger::default();
            for row in rows {
                if let Some(rec) = record_from_row(&row) {
                    ledger.commit(rec);
                }
            }
            let script = std::fs::read_to_string(file)?;
            let result = feed_my_ledger::script::run_script(&script, &ledger)?;
            println!("{result}");
        }
        Commands::Verify => {
            let mismatched = verify_sheet(&*adapter, &sheet_id, &signature)?;
            if mismatched.is_empty() {
                println!("All rows verified");
            } else {
                println!("Tampered rows: {mismatched:?}");
                return Err("tampering detected".into());
            }
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
