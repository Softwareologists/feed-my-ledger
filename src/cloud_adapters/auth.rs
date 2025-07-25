use base64::Engine;
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
    key: [u8; 32],
    tokens: HashMap<String, OAuth2Token>,
}

impl FileTokenStore {
    /// Create a store backed by the given file path. Existing data is loaded if available.
    pub fn new(path: impl Into<PathBuf>, key: [u8; 32]) -> Self {
        use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
        let path = path.into();
        let tokens = std::fs::read_to_string(&path)
            .ok()
            .and_then(|data| {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(data)
                    .ok()?;
                if bytes.len() < 12 {
                    return None;
                }
                let (nonce_bytes, cipher_text) = bytes.split_at(12);
                let cipher = Aes256Gcm::new_from_slice(&key).ok()?;
                cipher
                    .decrypt(Nonce::from_slice(nonce_bytes), cipher_text)
                    .ok()
            })
            .and_then(|plain| serde_json::from_slice(&plain).ok())
            .unwrap_or_default();
        Self { path, key, tokens }
    }

    fn persist(&self) {
        use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
        use rand::RngCore;
        if let Ok(data) = serde_json::to_vec(&self.tokens) {
            let cipher = Aes256Gcm::new_from_slice(&self.key).expect("key");
            let mut nonce = [0u8; 12];
            rand::rng().fill_bytes(&mut nonce);
            if let Ok(mut encrypted) = cipher.encrypt(Nonce::from_slice(&nonce), data.as_ref()) {
                let mut out = nonce.to_vec();
                out.append(&mut encrypted);
                let encoded = base64::engine::general_purpose::STANDARD.encode(out);
                let _ = std::fs::write(&self.path, encoded);
            }
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
        .token(&[
            "https://www.googleapis.com/auth/drive.file",
            "https://www.googleapis.com/auth/spreadsheets",
        ])
        .await?;
    Ok(())
}
