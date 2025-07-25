# Module Architecture

FeedMyLedger is organized into two main modules:

- **core** – Provides the immutable ledger logic and record structures. It defines the `Record`, `Ledger`, and sharing primitives that control access and apply adjustments.
- **cloud_adapters** – Contains implementations for interacting with remote spreadsheet services. Adapters implement the `CloudSpreadsheetService` trait and can be wrapped with utilities like batching and retry logic.

Each module exposes a minimal surface area so that applications can choose the pieces they need. The `lib.rs` file simply re-exports these modules.

The core module defines an `Account` type that stores hierarchical names like `Assets:Bank:Checking`.
Ledger helper methods can aggregate balances across subaccounts using this structure.
