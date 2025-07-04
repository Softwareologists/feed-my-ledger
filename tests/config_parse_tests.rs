use serde::Deserialize;

#[derive(Deserialize)]
struct GoogleSheetsConfig {
    credentials_path: String,
    spreadsheet_id: Option<String>,
    sheet_name: Option<String>,
}

#[derive(Deserialize)]
struct Config {
    name: String,
    password: Option<String>,
    google_sheets: GoogleSheetsConfig,
}

#[test]
fn parses_sheet_name() {
    let toml = r#"
name = "TestLedger"
[google_sheets]
credentials_path = "cred.json"
spreadsheet_id = "abc"
sheet_name = "Custom"
"#;
    let cfg: Config = toml::from_str(toml).unwrap();
    assert_eq!(cfg.google_sheets.sheet_name.as_deref(), Some("Custom"));
    assert_eq!(cfg.google_sheets.credentials_path, "cred.json");
    assert_eq!(cfg.google_sheets.spreadsheet_id.as_deref(), Some("abc"));
}

#[test]
fn parses_name_and_password() {
    let toml = r#"
name = "TestLedger"
password = "supersecret"
[google_sheets]
credentials_path = "cred.json"
spreadsheet_id = "abc"
sheet_name = "Custom"
"#;
    let cfg: Config = toml::from_str(toml).unwrap();
    assert_eq!(cfg.name, "TestLedger");
    assert_eq!(cfg.password.as_deref(), Some("supersecret"));
    assert_eq!(cfg.google_sheets.sheet_name.as_deref(), Some("Custom"));
    assert_eq!(cfg.google_sheets.credentials_path, "cred.json");
    assert_eq!(cfg.google_sheets.spreadsheet_id.as_deref(), Some("abc"));
}

#[test]
fn parses_name_without_password() {
    let toml = r#"
name = "TestLedger"
[google_sheets]
credentials_path = "cred.json"
spreadsheet_id = "abc"
sheet_name = "Custom"
"#;
    let cfg: Config = toml::from_str(toml).unwrap();
    assert_eq!(cfg.name, "TestLedger");
    assert_eq!(cfg.password, None);
}

#[test]
fn fails_without_name() {
    let toml = r#"
[google_sheets]
credentials_path = "cred.json"
spreadsheet_id = "abc"
sheet_name = "Custom"
"#;
    let result: Result<Config, _> = toml::from_str(toml);
    assert!(result.is_err(), "Config without 'name' should fail");
}
