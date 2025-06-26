# Public API Reference

The easiest way to use the library is to work with the `Ledger` type directly:

```rust
use rusty_ledger::core::{Ledger, Record};

let mut ledger = Ledger::default();
let record = Record::new(
    "example".into(),
    "cash".into(),
    "revenue".into(),
    10.0,
    "USD".into(),
    None,
    None,
    vec!["sample".into()],
).unwrap();
ledger.commit(record);
```

## Ledger Operation Examples

### Reading a record

```rust
let retrieved = ledger.get_record(record.id).unwrap();
println!("{} -> {}", retrieved.debit_account, retrieved.credit_account);
```

### Listing records

```rust
for entry in ledger.records() {
    println!("{}: {}", entry.id, entry.description);
}
```

### Applying an adjustment

```rust
let adj = Record::new(
    "refund".into(),
    "revenue".into(),
    "cash".into(),
    10.0,
    "USD".into(),
    None,
    None,
    vec![],
).unwrap();
ledger.apply_adjustment(record.id, adj).unwrap();

for item in ledger.adjustment_history(record.id) {
    println!("adjustment {}", item.id);
}
```

When integrating with a cloud service, construct an adapter that implements
`CloudSpreadsheetService` and pass it to `SharedLedger` for multi-user access.

```rust
use rusty_ledger::cloud_adapters::GoogleSheetsAdapter;
use rusty_ledger::core::{Permission, SharedLedger};

let adapter = GoogleSheetsAdapter::new();
let ledger = SharedLedger::new(adapter, "owner@example.com").unwrap();
ledger.commit("owner@example.com", record.clone()).unwrap();
let all = ledger.records("owner@example.com").unwrap();
ledger.share_with("reader@example.com", Permission::Read).unwrap();
```

### Importing statements

Use the parsers in the `import` module to convert existing statements into `Record`s.
Each parser returns a vector of records ready for insertion:

```rust
use rusty_ledger::import::{csv, ofx, qif};
use std::path::Path;

let records = csv::parse(Path::new("transactions.csv"))?;
```

## API Overview

### Core

- `Record` – immutable ledger entry structure.
- `RecordError` – validation errors returned by `Record::new`.
- `Ledger` – in-memory append-only store for `Record`s.
- `LedgerError` – failures that can occur when using `Ledger`.
- `SharedLedger` – multi-user wrapper around a `Ledger` backed by a spreadsheet service.
- `Permission` – access levels for `SharedLedger` operations.
- `AccessError` – errors produced by `SharedLedger` methods.

### Cloud Adapters

- `CloudSpreadsheetService` – trait abstracting spreadsheet backends.
- `SpreadsheetError` – common error type returned by services.
- `GoogleSheetsAdapter` – in-memory adapter useful for tests.
- `GoogleSheets4Adapter` – adapter using the real Google Sheets API.
- `BatchingCacheService` – wrapper that batches writes and caches reads.
- `EvictionPolicy` – strategy used by `BatchingCacheService` when caching.
- `RetryingService` – wrapper adding retry logic with exponential backoff.
- `AuthManager` – manages OAuth tokens using an `AuthProvider` and `TokenStore`.
- `AuthProvider` and `TokenStore` – traits for pluggable authentication.
- `MemoryTokenStore` and `FileTokenStore` – built-in `TokenStore` implementations.
- `OAuth2Token` and `AuthError` – types describing authentication tokens and failures.
- `initial_oauth_login` – helper function to perform the installed OAuth flow.
- `HyperClient` and `HyperConnector` – client types re-exported for Google Sheets integrations.
