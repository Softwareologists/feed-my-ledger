# Rusty Ledger (rusty-ledger) 

[![Release](https://github.com/Softwareologists/rusty-ledger/actions/workflows/release.yml/badge.svg)](https://github.com/Softwareologists/rusty-ledger/actions/workflows/release.yml)
[![CI](https://github.com/Softwareologists/rusty-ledger/actions/workflows/ci.yml/badge.svg)](https://github.com/Softwareologists/rusty-ledger/actions/workflows/ci.yml)

Rust-based library that enables applications to interact with cloud-based spreadsheet services (e.g., Google Sheets) as immutable, append-only databases. It ensures that once data is committed, it cannot be edited or deleted. Adjustments are made by appending new records, akin to double-entry bookkeeping.

![rusty-ledger](https://github.com/user-attachments/assets/6c630732-3bc5-43ac-bcb7-ade199cefcc2)

# 📦 Features
- Immutable Data Entries: Once data is committed, it becomes read-only.
- Append-Only Adjustments: Modifications are handled by appending new records that reference the original entries.
- Cloud Service Integration: Supports integration with services like Google Sheets.
- User Authentication: Users authenticate via OAuth2 to link their cloud accounts.
- Data Sharing: Users can share their data with others, controlling access permissions.
- Resilient API Calls: Automatically retries transient errors with exponential backoff.

# 🚀 Getting Started
## Prerequisites
- Rust (version 1.74 or higher)
- Google Cloud account with Sheets API enabled
- OAuth2 credentials for Google Sheets API

### Creating credentials.json
1. Visit the [Google Cloud Console](https://console.cloud.google.com/) and create
   or select a project.
2. Enable the **Google Sheets API** for that project.
3. Navigate to **APIs & Services > Credentials** and choose **Create
   credentials > OAuth client ID**. Configure the consent screen if prompted and
   select **Desktop app**.
4. Download the resulting JSON file and save it as `credentials.json` in the
   project root or another location of your choice.
5. Reference this path in the `credentials_path` field of `config.toml`.

## Installation
Add the following to your Cargo.toml:
```toml
[dependencies]
rusty-ledger = "1.0.0"
```

## Usage
```rust
use rusty_ledger::core::{Ledger, Record};

fn main() {
    let mut ledger = Ledger::default();
    let record = Record::new(
        "Sample transaction".into(),
        "cash".into(),
        "revenue".into(),
        100.0,
        "USD".into(),
        None,
        None,
        vec!["example".into()],
    )
    .unwrap();
    ledger.append(record);
}
```

To work with a live Google Sheet, construct a `GoogleSheets4Adapter` using the
`google-sheets4` crate. You may optionally specify the worksheet name when
creating the adapter; otherwise, it defaults to `Ledger`:

```rust,no_run
use rusty_ledger::cloud_adapters::{GoogleSheets4Adapter, HyperConnector};
use google_sheets4::{hyper_rustls, hyper_util, yup_oauth2, Sheets};

async fn example() -> Result<(), Box<dyn std::error::Error>> {
    let secret = yup_oauth2::read_application_secret("client_secret.json").await?;
    let auth = yup_oauth2::InstalledFlowAuthenticator::builder(
        secret,
        yup_oauth2::InstalledFlowReturnMethod::Interactive,
    )
    .build()
    .await?;

    let connector: HyperConnector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .https_or_http()
        .enable_http1()
        .build();
    let client = hyper_util::client::legacy::Client::builder(
        hyper_util::rt::TokioExecutor::new(),
    )
    .build(connector.clone());
    let hub = Sheets::new(client, auth);
    let mut service = GoogleSheets4Adapter::with_sheet_name(hub, "Custom");
    let sheet_id = service.create_sheet("ledger")?;
    service.append_row(&sheet_id, vec!["hello".into()])?;
    Ok(())
}
```

### Command Line Interface

The crate ships with a small CLI for local experimentation. To add a record and
view the stored data:

```bash
$ cargo run --bin ledger -- add \
    --description "Coffee" \
    --debit cash --credit expenses \
    --amount 3.5 --currency USD
$ cargo run --bin ledger -- list
```

Before issuing API commands for the first time, authorize the application:

```bash
$ cargo run --bin ledger -- login
```

Adjustments reference an existing record by ID:

```bash
$ cargo run --bin ledger -- adjust \
    --id <RECORD_ID> --description "Refund" \
    --debit expenses --credit cash \
    --amount 3.5 --currency USD
```

Share the active sheet:

```bash
$ cargo run --bin ledger -- share --email someone@example.com
```

Switch to a different sheet by URL:

```bash
$ cargo run --bin ledger -- switch --link "https://docs.google.com/spreadsheets/d/<ID>/edit"
```

# 🛠️ Configuration
Rusty Ledger looks for a `config.toml` file in the same directory as the
binary. This file stores your OAuth credentials and the spreadsheet ID used by
the CLI.

1. Create the file in your project root:
   ```bash
   $ touch config.toml
   ```

2. Determine your spreadsheet ID. Open the sheet in your browser and copy the
   portion of the URL between `/d/` and `/edit`, for example
   `https://docs.google.com/spreadsheets/d/<ID>/edit`.
3. Add the following contents, replacing the placeholder values:
   ```toml
   [google_sheets]
   credentials_path = "path_to_credentials.json"
   spreadsheet_id = "<ID>"
   # optional: defaults to "Ledger"
   sheet_name = "Custom"
   ```
4. Save the file. The CLI reads this configuration on startup and will use the
   specified `sheet_name` for all ledger operations.

# 🧪 Running Tests
```bash
cargo test
```

# 📄 Documentation
Comprehensive documentation is available in the docs directory, covering:
- Module architecture
- Data model specification
- Public API usage
- Authentication integration
- Instructions for extending cloud service support

# 🤝 Contributing
Contributions are welcome! Please read the [CONTRIBUTING](CONTRIBUTING.md) for guidelines on how to contribute to this project

# 📄 License
This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
