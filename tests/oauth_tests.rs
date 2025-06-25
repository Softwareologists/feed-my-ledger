use chrono::{Duration, Utc};
use rusty_ledger::cloud_adapters::auth::{
    AuthError, AuthManager, AuthProvider, MemoryTokenStore, OAuth2Token, TokenStore,
};

#[derive(Default)]
struct MockProvider {
    authorize_called: bool,
    refresh_called: bool,
}

impl AuthProvider for MockProvider {
    fn authorize(&mut self) -> Result<OAuth2Token, AuthError> {
        self.authorize_called = true;
        Ok(OAuth2Token {
            access_token: "token1".into(),
            refresh_token: "refresh1".into(),
            expires_at: Utc::now() + Duration::hours(1),
        })
    }

    fn refresh(&mut self, _refresh_token: &str) -> Result<OAuth2Token, AuthError> {
        self.refresh_called = true;
        Ok(OAuth2Token {
            access_token: "token2".into(),
            refresh_token: "refresh2".into(),
            expires_at: Utc::now() + Duration::hours(1),
        })
    }
}

#[test]
fn acquire_token_when_missing() {
    let provider = MockProvider::default();
    let store = MemoryTokenStore::new();
    let mut manager = AuthManager::new(provider, store);

    let token = manager.authenticate("user").unwrap();
    assert_eq!(token.access_token, "token1");
    assert!(manager.provider.authorize_called);
}

#[test]
fn refresh_expired_token() {
    let provider = MockProvider::default();
    let mut store = MemoryTokenStore::new();
    store.save_token(
        "user",
        OAuth2Token {
            access_token: "old".into(),
            refresh_token: "oldRefresh".into(),
            expires_at: Utc::now() - Duration::hours(1),
        },
    );
    let mut manager = AuthManager::new(provider, store);
    let token = manager.authenticate("user").unwrap();
    assert_eq!(token.access_token, "token2");
    assert!(manager.provider.refresh_called);
}

#[derive(Default)]
struct FailingRefresh;

impl AuthProvider for FailingRefresh {
    fn authorize(&mut self) -> Result<OAuth2Token, AuthError> {
        Err(AuthError::InvalidCredentials)
    }

    fn refresh(&mut self, _refresh_token: &str) -> Result<OAuth2Token, AuthError> {
        Err(AuthError::RefreshFailed)
    }
}

#[test]
fn propagate_refresh_error() {
    let mut store = MemoryTokenStore::new();
    store.save_token(
        "user",
        OAuth2Token {
            access_token: "old".into(),
            refresh_token: "bad".into(),
            expires_at: Utc::now() - Duration::hours(1),
        },
    );
    let provider = FailingRefresh;
    let mut manager = AuthManager::new(provider, store);
    let err = manager.authenticate("user").unwrap_err();
    assert_eq!(err, AuthError::RefreshFailed);
}
