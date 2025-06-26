use rusty_ledger::cloud_adapters::auth::initial_oauth_login;

#[tokio::test]
async fn initial_login_fails_with_missing_credentials() {
    let result = initial_oauth_login("missing.json", "tokens.json").await;
    assert!(result.is_err());
}
