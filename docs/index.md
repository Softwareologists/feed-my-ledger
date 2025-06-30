---
title: Overview
---
## 📦 Features
- Immutable Data Entries: Once data is committed, it becomes read-only.
- Append-Only Adjustments: Modifications are handled by appending new records that reference the original entries.
- Cloud Service Integration: Supports integration with services like Google Sheets and Microsoft Excel 365.
- Local File Storage: Save ledger data to CSV files using the `FileAdapter`.
- User Authentication: Users authenticate via OAuth2 to link their cloud accounts.
- Data Sharing: Users can share their data with others, controlling access permissions.
- Resilient API Calls: Automatically retries transient errors with exponential backoff.

## 🚀 Getting Started
### Prerequisites
- Rust (version 1.74 or higher)
- Google Cloud account with Sheets API enabled
- OAuth2 credentials for Google Sheets API
- Microsoft account with Excel 365 access

### Installation
Add the following to your Cargo.toml:
```toml
[dependencies]
rusty-ledger = "2.0.0"
```

### Usage
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

To work with a live Google Sheet, construct a `GoogleSheets4Adapter` that
communicates with the official Google Sheets REST API. This approach avoids
extra third‑party wrappers and keeps the dependency surface minimal. You may
optionally specify the worksheet name when creating the adapter; otherwise, it
defaults to `Ledger`:

```rust,no_run
use rusty_ledger::cloud_adapters::GoogleSheets4Adapter;
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

To integrate with Microsoft Excel 365 instead, use the `Excel365Adapter` which
talks to the Microsoft Graph API:

```rust,no_run
use rusty_ledger::cloud_adapters::Excel365Adapter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // `auth` must provide OAuth tokens scoped for Microsoft Graph
    let mut service = Excel365Adapter::new(auth);
    let sheet_id = service.create_sheet("ledger")?;
    service.append_row(&sheet_id, vec!["hello".into()])?;
    Ok(())
}
```

If you prefer to avoid cloud services entirely, `FileAdapter` stores rows in local CSV files:

```rust,no_run
use rusty_ledger::cloud_adapters::FileAdapter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut service = FileAdapter::new("./ledger_data");
    let sheet_id = service.create_sheet("ledger")?;
    service.append_row(&sheet_id, vec!["hello".into()])?;
    Ok(())
}
```

#### Command Line Interface

The crate ships with a small CLI for local experimentation. To add a record and
view the stored data:

```bash
$ cargo run --bin ledger -- add \
    --description "Coffee" \
    --debit cash --credit expenses \
    --amount 3.5 --currency USD
$ cargo run --bin ledger -- list
```

Add `--local-dir <DIR>` to store data in local CSV files:

```bash
$ cargo run --bin ledger -- --local-dir ledger_data add \
    --description "Coffee" \
    --debit cash --credit expenses \
    --amount 3.5 --currency USD
$ cargo run --bin ledger -- --local-dir ledger_data list
```

Split transactions use the same command with an additional `--splits` argument
containing a JSON array of extra postings:

```bash
$ cargo run --bin ledger -- add \
    --description "Shopping" \
    --debit expenses:grocery --credit cash \
    --amount 30 --currency USD \
    --splits '[{"debit":"expenses:supplies","credit":"cash","amount":20}]'
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

Import statements from existing files. Supported formats are **csv**, **qif**, **ofx**, **ledger**, and **json**:

```bash
$ cargo run --bin ledger -- import --format csv --file transactions.csv \
    --map-description desc --map-debit debit --map-credit credit \
    --map-amount value --map-currency curr
```
Mapping flags override the default column names when importing CSV files.

Ledger text and JSON formats can also be imported:

```bash
$ cargo run --bin ledger -- import --format ledger --file statement.ledger
$ cargo run --bin ledger -- import --format json --file data.json
```

When compiled with the `bank-api` feature, you can download statements directly:

```bash
$ cargo run --bin ledger -- download --url "https://bank.example.com/statement.ofx"
```

## 🛠️ Configuration
Rusty Ledger looks for a `config.toml` file in the same directory as the
binary. This file stores your OAuth credentials and the spreadsheet ID used by
the CLI. When running with `--local-dir`, only the sheet ID is saved and no
OAuth configuration is needed.

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

To work with Microsoft Excel 365 you will need an application registered in
Azure and a workbook stored in OneDrive or SharePoint.

1. Sign in to the [Azure Portal](https://portal.azure.com/) and navigate to
   **Azure Active Directory > App registrations**. Create a new registration.
2. Under **API permissions** add the **Files.ReadWrite** delegated permission
   for Microsoft Graph and grant admin consent.
3. In **Certificates & secrets** create a client secret and note its value. From
   the **Overview** page also record the **Application (client) ID** and
   **Directory (tenant) ID**.
4. Create or open the workbook you want Rusty Ledger to use and copy its ID from
   the share link (or obtain it via the Graph Explorer).
5. Add the following section to `config.toml`:
   ```toml
   [excel_365]
   tenant_id = "<TENANT_ID>"
   client_id = "<CLIENT_ID>"
   client_secret = "<CLIENT_SECRET>"
   workbook_id = "<WORKBOOK_ID>"
   # optional: defaults to "Ledger"
   sheet_name = "Ledger"
   ```
6. Use these values when constructing an `Excel365Adapter` in your own
   application. The CLI does not yet load this configuration automatically.

## 🧪 Running Tests
```bash
cargo test
```

## 📄 Documentation
Comprehensive documentation is available in the docs directory, covering:
- [Public API usage](./api_usage)
- [Authentication integration](./authentication)
- [Instructions for extending cloud service support](./extending_cloud_support)

# 🤝 Contributing
Contributions are welcome! Please read the [CONTRIBUTING](CONTRIBUTING.md) for guidelines on how to contribute to this project

# 📄 License
This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
