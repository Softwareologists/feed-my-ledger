# Authentication Integration

Authentication is provided through the `AuthManager` type in
`cloud_adapters::auth`. You supply an `AuthProvider` implementation to perform
OAuth2 flows and a `TokenStore` for persisting tokens.

```rust,no_run
use rusty_ledger::cloud_adapters::auth::{AuthManager, MemoryTokenStore, OAuth2Token, AuthProvider};

struct MyProvider;
impl AuthProvider for MyProvider {
    fn authorize(&mut self) -> Result<OAuth2Token, AuthError> {
        unimplemented!()
    }
    fn refresh(&mut self, _refresh: &str) -> Result<OAuth2Token, AuthError> {
        unimplemented!()
    }
}

let mut manager = AuthManager::new(MyProvider, MemoryTokenStore::new());
let token = manager.authenticate("user1")?;
```

For Google Sheets, you can perform the initial OAuth login programmatically or
via the CLI. The helper `initial_oauth_login` persists the obtained tokens so
future requests are authenticated automatically.

```rust,no_run
use rusty_ledger::cloud_adapters::auth::initial_oauth_login;

// Runs the interactive OAuth flow and saves tokens to `tokens.json`.
tokio::runtime::Runtime::new().unwrap().block_on(async {
    initial_oauth_login("client_secret.json", "tokens.json").await.unwrap();
});
```

From the command line you can run:

```bash
$ cargo run --bin ledger -- login
```
