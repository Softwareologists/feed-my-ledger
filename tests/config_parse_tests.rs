use serde::Deserialize;

#[derive(Deserialize)]
struct GoogleSheetsConfig {
    credentials_path: String,
    spreadsheet_id: Option<String>,
    sheet_name: Option<String>,
}

#[derive(Deserialize)]
struct Config {
    google_sheets: GoogleSheetsConfig,
}

#[test]
fn parses_sheet_name() {
    let toml = r#"
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
