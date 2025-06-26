use clap::{Parser, Subcommand};
use rusty_ledger::cloud_adapters::GoogleSheetsAdapter;
use rusty_ledger::core::{Record, SharedLedger};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "ledger", about = "Interact with a local ledger")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
    /// List all records
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
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let ledger = SharedLedger::new(GoogleSheetsAdapter::new(), "cli")?;

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
            ledger
                .commit("cli", record)
                .map_err(|e| format!("{:?}", e))?;
        }
        Commands::List => {
            for record in ledger.records("cli").map_err(|e| format!("{:?}", e))? {
                println!(
                    "{} | {} -> {} {} {} ({})",
                    record.description,
                    record.debit_account,
                    record.credit_account,
                    record.amount,
                    record.currency,
                    record.id
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
            let original = Uuid::parse_str(&id)?;
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
            ledger
                .apply_adjustment("cli", original, record)
                .map_err(|e| format!("{:?}", e))?;
        }
    }

    Ok(())
}
