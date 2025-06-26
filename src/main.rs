use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use google_sheets4::{Sheets, hyper_rustls, hyper_util};
use rusty_ledger::cloud_adapters::{
    CloudSpreadsheetService,
    google_sheets4::{GoogleSheets4Adapter, HyperClient, HyperConnector},
};
use rusty_ledger::core::Record;
use serde::{Deserialize, Serialize};
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

    let connector: HyperConnector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()?
        .https_or_http()
        .enable_http1()
        .build();
    let client: HyperClient =
        hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
            .build(connector.clone());
    let hub = Sheets::new(client, auth);
    let adapter = match &cfg.sheet_name {
        Some(name) => GoogleSheets4Adapter::with_sheet_name(hub, name.clone()),
        None => GoogleSheets4Adapter::new(hub),
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
        Commands::Switch { .. } | Commands::Login => unreachable!(),
    }

    Ok(())
}
