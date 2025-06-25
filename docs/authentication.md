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
