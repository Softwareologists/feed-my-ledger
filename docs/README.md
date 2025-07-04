# Documentation

This directory contains detailed documentation for the FeedMyLedger project. The following guides are available:

- [Module Architecture](architecture.md)
- [Data Model](data_model.md)
- [Public API Reference](api_usage.md)
- [Authentication Integration](authentication.md)
- [Extending Cloud Service Support](extending_cloud_support.md)
- [Release Process](release.md)
- [Scripting Examples](scripting.md)

# Configuration

FeedMyLedger uses a `config.toml` file for application settings. The following fields are supported:

- `name` (**required**): Unique, non-empty name for this ledger instance. Used for row signature generation and verification.
- `password` (optional): Secret used for row signature generation. If present, must be kept secure and never logged.
- `google_sheets`: Google Sheets configuration.
- `budgets`, `schedules`: Optional budget and schedule entries.

Example `config.toml`:

```toml
name = "MyLedger"
# password = "supersecret"  # Optional
[google_sheets]
credentials_path = "path_to_credentials.json"
spreadsheet_id = "<ID>"
sheet_name = "Custom" # optional

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
