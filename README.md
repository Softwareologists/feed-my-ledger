# FeedMyLedger (feed-my-ledger)

[![Release](https://github.com/Softwareologists/feed-my-ledger/actions/workflows/release.yml/badge.svg)](https://github.com/Softwareologists/feed-my-ledger/actions/workflows/release.yml)
[![CI](https://github.com/Softwareologists/feed-my-ledger/actions/workflows/ci.yml/badge.svg)](https://github.com/Softwareologists/feed-my-ledger/actions/workflows/ci.yml)

Rust-based library that enables applications to interact with cloud-based spreadsheet services (e.g., Google Sheets) as immutable, append-only databases. It ensures that once data is committed, it cannot be edited or deleted. Adjustments are made by appending new records, akin to double-entry bookkeeping.

![feed-my-ledger-logo](https://github.com/user-attachments/assets/fae6a921-00cc-471d-9335-fcdbb99362a0)

# 📦 Features
- Immutable Data Entries: Once data is committed, it becomes read-only.
- Append-Only Adjustments: Modifications are handled by appending new records that reference the original entries.
- Cloud Service Integration: Supports integration with services like Google Sheets and Microsoft Excel 365.
- Local File Storage: Save ledger data to CSV files using the `FileAdapter`.
- User Authentication: Users authenticate via OAuth2 to link their cloud accounts.
- Data Sharing: Users can share their data with others, controlling access permissions.
- Resilient API Calls: Automatically retries transient errors with exponential backoff.
- Ledger Verification: Detects tampering by recomputing row hashes.

# 🚀 Getting Started
## Prerequisites
- Rust (version 1.74 or higher)
- Google Cloud account with Sheets API enabled
- OAuth2 credentials for Google Sheets API
- Microsoft account with Excel 365 access

## Installation
Add the following to your Cargo.toml:
```toml
[dependencies]
feed-my-ledger = "2.0.0"
```

## Usage
```rust
use feed_my_ledger::core::{Ledger, Record};

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

To work with a live Google Sheet, construct a `GoogleSheets4Adapter` that
communicates with the official Google Sheets REST API. This approach avoids
extra third‑party wrappers and keeps the dependency surface minimal. You may
optionally specify the worksheet name when creating the adapter; otherwise, it
defaults to `Ledger`:

```rust,no_run
use feed_my_ledger::cloud_adapters::GoogleSheets4Adapter;
use yup_oauth2::{self, InstalledFlowAuthenticator, InstalledFlowReturnMethod};

async fn example() -> Result<(), Box<dyn std::error::Error>> {
    let secret = yup_oauth2::read_application_secret("client_secret.json").await?;
    let auth = yup_oauth2::InstalledFlowAuthenticator::builder(
        secret,
        yup_oauth2::InstalledFlowReturnMethod::Interactive,
    )
    .build()
    .await?;

    let mut service = GoogleSheets4Adapter::with_sheet_name(auth, "Custom");
    let sheet_id = service.create_sheet("ledger")?;
    service.append_row(&sheet_id, vec!["hello".into()])?;
    Ok(())
}
```

### Command Line Interface

The crate ships with a small CLI for local experimentation. To add a record and
view the stored data:

```bash
$ cargo run --bin feed-my-ledger -- add \
    --description "Coffee" \
    --debit cash --credit expenses \
    --amount 3.5 --currency USD
$ cargo run --bin feed-my-ledger -- list
```

Pass `--local-dir <DIR>` to store rows in local CSV files instead of a cloud
service:

```bash
$ cargo run --bin feed-my-ledger -- --local-dir ledger_data add \
    --description "Coffee" \
    --debit cash --credit expenses \
    --amount 3.5 --currency USD
$ cargo run --bin feed-my-ledger -- --local-dir ledger_data list
```

Before issuing API commands for the first time, authorize the application:

```bash
$ cargo run --bin feed-my-ledger -- login
```

Adjustments reference an existing record by ID:

```bash
$ cargo run --bin feed-my-ledger -- adjust \
    --id <RECORD_ID> --description "Refund" \
    --debit expenses --credit cash \
    --amount 3.5 --currency USD
```

Share the active sheet:

```bash
$ cargo run --bin feed-my-ledger -- share --email someone@example.com
```

Switch to a different sheet by URL:

```bash
$ cargo run --bin feed-my-ledger -- switch --link "https://docs.google.com/spreadsheets/d/<ID>/edit"
```

Import statements from existing files. Supported formats are **csv**, **qif**, **ofx**, **ledger**, and **json**:

```bash
$ cargo run --bin feed-my-ledger -- import --format csv --file transactions.csv \
    --map-description desc --map-debit debit --map-credit credit \
    --map-amount value --map-currency curr
```
Mapping flags override the default column names when importing CSV files.

If your CSV does not include a currency column, you can provide a default value:

```bash
$ cargo run --bin feed-my-ledger -- import --format csv --file transactions.csv --currency USD
```
All imported rows will use the supplied currency.

For QIF or OFX files with non-standard transaction date formats, provide a custom
`--date-format`:

```bash
$ cargo run --bin feed-my-ledger -- import --format qif --file statement.qif \
    --date-format "%Y/%m/%d"
```

Ledger text and JSON formats can also be imported:

```bash
$ cargo run --bin feed-my-ledger -- import --format ledger --file statement.ledger
$ cargo run --bin feed-my-ledger -- import --format json --file data.json
```

When compiled with the `bank-api` feature, you can download statements directly:

```bash
$ cargo run --bin feed-my-ledger -- download --url "https://bank.example.com/statement.ofx"
```

Verify ledger integrity:

```bash
$ cargo run --bin feed-my-ledger -- verify
```

# 🛠️ Configuration
FeedMyLedger looks for a `config.toml` file in the same directory as the
binary. This file stores your OAuth credentials and the spreadsheet ID used by
the CLI. When using `--local-dir`, only the sheet ID is persisted and no OAuth
credentials are required.

1. Create the file in your project root:
   ```bash
   $ touch config.toml
   ```

2. Determine your spreadsheet ID. Open the sheet in your browser and copy the
   portion of the URL between `/d/` and `/edit`, for example
   `https://docs.google.com/spreadsheets/d/<ID>/edit`.

3. Create credentials.json

   1. Visit the [Google Cloud Console](https://console.cloud.google.com/) and create
      or select a project.

   2. Enable the **Google Sheets API** for that project.

   3. Navigate to **APIs & Services > Credentials** and choose **Create
      credentials > OAuth client ID**. Configure the consent screen if prompted and
      select **Desktop app**.

   4. Download the resulting JSON file and save it as `credentials.json` in the
      project root or another location of your choice.

   5. Reference this path in the `credentials_path` field of `config.toml`.

5. Add the following contents, replacing the placeholder values:
   ```toml
   [google_sheets]
   credentials_path = "path_to_credentials.json"
   spreadsheet_id = "<ID>"
   # optional: defaults to "Ledger"
   sheet_name = "Custom"

    [[budgets]]
    account = "expenses:food"
    amount = 200.0
    currency = "USD"
    period = "monthly"

    [[schedules]]
    cron = "0 0 1 * *"
    description = "rent"
    debit = "expenses:rent"
    credit = "cash"
    amount = 1000.0
    currency = "USD"
   ```

6. Save the file. The CLI reads this configuration on startup and will use the
   specified `sheet_name` for all ledger operations.

### Excel 365 Setup

To connect FeedMyLedger to Microsoft Excel 365 you must register an application
in Azure and provide workbook credentials.

1. Open the [Azure Portal](https://portal.azure.com/) and create a new
   application under **Azure Active Directory > App registrations**.
2. Add the **Files.ReadWrite** delegated permission for Microsoft Graph and
   grant consent.
3. Generate a client secret under **Certificates & secrets** and note the
   secret value as well as the **Application (client) ID** and **Directory
   (tenant) ID**.
4. Create or select the workbook you want to use and copy its ID from the share
   link or via the Graph Explorer.
5. Store these details in your `config.toml`:
   ```toml
   [excel_365]
   tenant_id = "<TENANT_ID>"
   client_id = "<CLIENT_ID>"
   client_secret = "<CLIENT_SECRET>"
   workbook_id = "<WORKBOOK_ID>"
   # optional: defaults to "Ledger"
   sheet_name = "Ledger"
   ```
6. Load this configuration when creating an `Excel365Adapter` in your code. The
   included CLI does not yet read these fields automatically.

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
