# rusty-ledger
Rust-based library that enables applications to interact with cloud-based spreadsheet services (e.g., Google Sheets) as immutable, append-only databases. It ensures that once data is committed, it cannot be edited or deleted. Adjustments are made by appending new records, akin to double-entry bookkeeping.

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

## Installation
Add the following to your Cargo.toml:
```toml
[dependencies]
rusty-ledger = "0.1.0"
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
`google-sheets4` crate:

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
    let mut service = GoogleSheets4Adapter::new(hub);
    let sheet_id = service.create_sheet("ledger")?;
    service.append_row(&sheet_id, vec!["hello".into()])?;
    Ok(())
}
```

# 🛠️ Configuration
Create a configuration file `config.toml` with the following content:
```toml
[google_sheets]
credentials_path = "path_to_credentials.json"
spreadsheet_id = "your_spreadsheet_id"
```

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
