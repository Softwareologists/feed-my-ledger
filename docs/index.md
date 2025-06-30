---
title: Overview
---
# Rusty Ledger

Rust-fast, cloud-backed ledger.

## Features

- Immutable data entries.
- Append-only adjustments.
- Cloud service integration.
- User authentication via OAuth2.
- Data sharing with granular permissions.
- Resilient API calls with retries.

## Usage

```rust
use rusty_ledger::core::{Ledger, Record};

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
).unwrap();
ledger.append(record);
```
