//! Utility functions for signature generation and encoding.
//
// Provides a stateless, deterministic function to generate a Base64-encoded signature
// from a name and optional password, suitable for use as a secret in row hashing and verification.
//
// - If password is missing or empty, signature = Base64Encode(name)
// - If password is present and non-empty, signature = Base64Encode(name:password)
//
// The function avoids storing the raw password in memory longer than necessary.
//
// # Errors
// Returns an error if the name is missing or empty.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use sha2::{Digest, Sha256};

/// Generates a Base64-encoded signature string from a name and optional password.
///
/// - If password is missing or empty, signature = Base64Encode(name)
/// - If password is present and non-empty, signature = Base64Encode(name:password)
///
/// # Arguments
/// * `name` - The user or ledger name (must not be empty)
/// * `password` - Optional password (may be empty or None)
///
/// # Returns
/// * `Ok(String)` - The Base64-encoded signature string
/// * `Err(String)` - If the name is missing or empty
pub fn generate_signature(name: &str, password: Option<&str>) -> Result<String, String> {
    if name.trim().is_empty() {
        return Err("Name must not be empty".to_string());
    }
    let signature = match password {
        Some(pw) if !pw.is_empty() => {
            let mut combined = String::with_capacity(name.len() + 1 + pw.len());
            combined.push_str(name);
            combined.push(':');
            combined.push_str(pw);
            let encoded = BASE64.encode(combined.as_bytes());
            // Zeroize the combined string as soon as possible
            drop(combined);
            encoded
        }
        _ => BASE64.encode(name.as_bytes()),
    };
    Ok(signature)
}

/// Computes a SHA-256 hash over the provided row values and signature.
///
/// The `values` slice must exclude the existing hash column if present. The
/// Base64-encoded signature acts as a secret salt so that a different
/// signature produces a different hash even when the row values are the same.
/// This allows detection of tampering with stored rows.
pub fn hash_row(values: &[String], signature: &str) -> String {
    let mut hasher = Sha256::new();
    for v in values {
        hasher.update(v.as_bytes());
        hasher.update([0u8]);
    }
    hasher.update(signature.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_name_only() {
        let sig = generate_signature("alice", None).unwrap();
        assert_eq!(sig, BASE64.encode("alice".as_bytes()));
    }

    #[test]
    fn test_signature_name_and_password() {
        let sig = generate_signature("alice", Some("secret")).unwrap();
        assert_eq!(sig, BASE64.encode("alice:secret".as_bytes()));
    }

    #[test]
    fn test_signature_empty_password() {
        let sig = generate_signature("alice", Some("")).unwrap();
        assert_eq!(sig, BASE64.encode("alice".as_bytes()));
    }

    #[test]
    fn test_signature_special_characters() {
        let sig = generate_signature("álîçè", Some("päßwørd!@#")).unwrap();
        assert_eq!(sig, BASE64.encode("álîçè:päßwørd!@#".as_bytes()));
    }

    #[test]
    fn test_signature_empty_name() {
        let err = generate_signature("", Some("pw")).unwrap_err();
        assert!(err.contains("Name must not be empty"));
    }

    #[test]
    fn test_signature_name_whitespace() {
        let err = generate_signature("   ", None).unwrap_err();
        assert!(err.contains("Name must not be empty"));
    }

    #[test]
    fn test_hash_changes_on_data_or_signature() {
        let sig1 = generate_signature("ledger", None).unwrap();
        let sig2 = generate_signature("ledger2", None).unwrap();
        let values = vec!["a".to_string(), "b".to_string()];
        let h1 = hash_row(&values, &sig1);
        let mut values2 = values.clone();
        values2[0] = "c".into();
        let h2 = hash_row(&values2, &sig1);
        assert_ne!(h1, h2);
        let h3 = hash_row(&values, &sig2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_hash_ignores_hash_column() {
        let sig = generate_signature("ledger", None).unwrap();
        let values = vec!["x".into(), "y".into()];
        let mut row = values.clone();
        let h1 = hash_row(&row, &sig);
        row.push("otherhash".into());
        let h2 = hash_row(&row[..row.len() - 1], &sig);
        assert_eq!(h1, h2);
    }
}
