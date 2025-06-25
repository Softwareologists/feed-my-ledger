use chrono::{Duration, Utc};
use rusty_ledger::cloud_adapters::auth::{FileTokenStore, OAuth2Token, TokenStore};
use uuid::Uuid;

#[test]
fn saves_and_loads_tokens() {
    let path = std::env::temp_dir().join(format!("tokens_{}.json", Uuid::new_v4()));
    {
        let mut store = FileTokenStore::new(&path);
        store.save_token(
            "user",
            OAuth2Token {
                access_token: "t1".into(),
                refresh_token: "r1".into(),
                expires_at: Utc::now() + Duration::hours(1),
            },
        );
    }
    let store = FileTokenStore::new(&path);
    let token = store.get_token("user").unwrap();
    assert_eq!(token.access_token, "t1");
    let _ = std::fs::remove_file(path);
}

#[test]
fn loading_missing_file_is_empty() {
    let path = std::env::temp_dir().join(format!("missing_{}.json", Uuid::new_v4()));
    let store = FileTokenStore::new(&path);
    assert!(store.get_token("user").is_none());
}
