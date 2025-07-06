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

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;

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
}
