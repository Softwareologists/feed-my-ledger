# Public API Usage

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

When integrating with a cloud service, construct an adapter that implements
`CloudSpreadsheetService` and pass it to `SharedLedger` for multi-user access.
