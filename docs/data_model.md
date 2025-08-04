# Data Model

A ledger entry is represented by the `Record` struct. Important fields include:

- `id` – Unique identifier generated for each record.
- `timestamp` – Time of creation in UTC.
- `description` – Human readable explanation of the transaction.
- `debit_account` and `credit_account` – The primary accounts affected by the entry.
- `amount` and `currency` – Monetary value stored as a positive number.
- `splits` – Optional additional postings for split transactions.
- `reference_id` – Optional link to another record when posting an adjustment.
- `external_reference` – Optional external identifier such as an invoice number.
- `tags` – Free form strings used for categorisation.
- `transaction_description` – Original description from an imported statement line.

Records are immutable after being committed to the ledger. Adjustments are stored as new records referencing the original entry.

Currency conversion is handled by a separate `PriceDatabase`. Rates are keyed by date and currency pair. When requesting a balance in a target currency the ledger converts each matching record using the latest rate available on or before the record date.
