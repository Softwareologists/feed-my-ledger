# rusty-ledger
Rust-based library that enables applications to interact with cloud-based spreadsheet services (e.g., Google Sheets) as immutable, append-only databases. It ensures that once data is committed, it cannot be edited or deleted. Adjustments are made by appending new records, akin to double-entry bookkeeping.

# 📦 Features
- Immutable Data Entries: Once data is committed, it becomes read-only.
- Append-Only Adjustments: Modifications are handled by appending new records that reference the original entries.
- Cloud Service Integration: Supports integration with services like Google Sheets.
- User Authentication: Users authenticate via OAuth2 to link their cloud accounts.
- Data Sharing: Users can share their data with others, controlling access permissions.

# 🚀 Getting Started
## Prerequisites
- Rust (version 1.60 or higher)
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
use rusty_ledger::{Client, Record};

fn main() {
    let client = Client::new("path_to_credentials.json");
    let record = Record::new("Sample data");
    client.commit(record);
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
This project is licensed under the MIT License - see the [LICENSE](LICENSE.md) file for details.
