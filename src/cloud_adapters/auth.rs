use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::path::PathBuf;

/// OAuth2 token representation containing expiry information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OAuth2Token {
    /// Bearer token used for authenticated requests.
    pub access_token: String,
    /// Refresh token used to obtain a new access token when expired.
    pub refresh_token: String,
    /// Time at which the access token expires.
    pub expires_at: DateTime<Utc>,
}

/// Errors that can occur when authenticating with a remote service.
#[derive(Debug, Clone, PartialEq)]
pub enum AuthError {
    /// Credentials were rejected by the service.
    InvalidCredentials,
    /// Refreshing the token failed.
    RefreshFailed,
    /// A generic error occurred.
    Other(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::InvalidCredentials => {
                write!(
                    f,
                    "credentials were rejected by the authentication provider"
                )
            }
            AuthError::RefreshFailed => write!(f, "unable to refresh OAuth token"),
            AuthError::Other(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for AuthError {}

/// Pluggable interface for OAuth2 providers.
pub trait AuthProvider {
    /// Perform the full authorization flow and return the acquired token.
    fn authorize(&mut self) -> Result<OAuth2Token, AuthError>;
    /// Refresh an expired token.
    fn refresh(&mut self, refresh_token: &str) -> Result<OAuth2Token, AuthError>;
}

/// Storage backend for persisting tokens.
pub trait TokenStore {
    /// Save a token for the given user.
    fn save_token(&mut self, user_id: &str, token: OAuth2Token);
    /// Retrieve a previously stored token.
    fn get_token(&self, user_id: &str) -> Option<OAuth2Token>;
}

/// In-memory token storage used primarily for tests.
#[derive(Default)]
pub struct MemoryTokenStore {
    tokens: HashMap<String, OAuth2Token>,
}

impl MemoryTokenStore {
    /// Create a new empty token store.
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
        }
    }
}

impl TokenStore for MemoryTokenStore {
    fn save_token(&mut self, user_id: &str, token: OAuth2Token) {
        self.tokens.insert(user_id.to_string(), token);
    }

    fn get_token(&self, user_id: &str) -> Option<OAuth2Token> {
        self.tokens.get(user_id).cloned()
    }
}

/// File-based token storage using JSON serialization.
pub struct FileTokenStore {
    path: PathBuf,
    tokens: HashMap<String, OAuth2Token>,
}

impl FileTokenStore {
    /// Create a store backed by the given file path. Existing data is loaded if available.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let tokens = std::fs::read_to_string(&path)
            .ok()
            .and_then(|data| serde_json::from_str(&data).ok())
            .unwrap_or_default();
        Self { path, tokens }
    }

    fn persist(&self) {
        if let Ok(data) = serde_json::to_string(&self.tokens) {
            let _ = std::fs::write(&self.path, data);
        }
    }
}

impl TokenStore for FileTokenStore {
    fn save_token(&mut self, user_id: &str, token: OAuth2Token) {
        self.tokens.insert(user_id.to_string(), token);
        self.persist();
    }

    fn get_token(&self, user_id: &str) -> Option<OAuth2Token> {
        self.tokens.get(user_id).cloned()
    }
}

/// Manages acquiring and refreshing tokens using a provider and store.
pub struct AuthManager<P: AuthProvider, S: TokenStore> {
    pub provider: P,
    store: S,
}

impl<P: AuthProvider, S: TokenStore> AuthManager<P, S> {
    /// Create a new manager with the given provider and storage backend.
    pub fn new(provider: P, store: S) -> Self {
        Self { provider, store }
    }

    /// Ensure a valid token exists for the given user.
    pub fn authenticate(&mut self, user_id: &str) -> Result<OAuth2Token, AuthError> {
        if let Some(token) = self.store.get_token(user_id) {
            if token.expires_at > Utc::now() {
                return Ok(token);
            }
            // token expired - try refresh
            let refreshed = self.provider.refresh(&token.refresh_token)?;
            self.store.save_token(user_id, refreshed.clone());
            return Ok(refreshed);
        }

        let token = self.provider.authorize()?;
        self.store.save_token(user_id, token.clone());
        Ok(token)
    }
}

/// Perform the OAuth installed flow and persist tokens to disk.
pub async fn initial_oauth_login(
    credentials_path: &str,
    token_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use google_sheets4::api::Scope;
    use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};

    if !std::path::Path::new(credentials_path).exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "credentials json file was not found",
        )
        .into());
    }
    let secret = yup_oauth2::read_application_secret(credentials_path)
        .await
        .map_err(|e| {
            Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error>
        })?;
    let auth = InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::Interactive)
        .persist_tokens_to_disk(token_path)
        .build()
        .await?;
    let _ = auth
        .token(&[Scope::DriveFile.as_ref(), Scope::Spreadsheet.as_ref()])
        .await?;
    Ok(())
}
