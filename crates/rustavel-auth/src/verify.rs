/// Argon2 password hashing and constant-time verification.
///
/// Delegates to the RustCrypto `argon2` crate with a per-password random salt
/// (PHC string format) and constant-time comparison on verify.
use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, SaltString};
use argon2::{Argon2, PasswordVerifier as Argon2PasswordVerifier};

use crate::error::{AuthError, Result};

/// Password hashing/verification contract.
///
/// Abstracted so JWT/session guards can be tested without real KDF cost and
/// so a future upgrade path (argon2 params via config) stays internal.
pub trait PasswordVerifier: Send + Sync {
    /// Verify `password` against a stored PHC `hash` in constant time.
    fn verify(&self, hash: &str, password: &str) -> bool;

    /// Hash `password` with a fresh random salt (PHC string).
    fn hash(&self, password: &str) -> Result<String>;
}

/// Default verifier backed by `argon2` (Argon2id, default params).
#[derive(Debug, Default)]
pub struct Argon2Verifier;

impl Argon2Verifier {
    /// Create a verifier with Argon2id defaults.
    pub fn new() -> Self {
        Self
    }
}

impl PasswordVerifier for Argon2Verifier {
    fn verify(&self, hash: &str, password: &str) -> bool {
        let parsed = match PasswordHash::new(hash) {
            Ok(p) => p,
            Err(_) => return false,
        };
        // Trait method; imported under alias to avoid a name clash with our
        // own `PasswordVerifier` contract above.
        Argon2PasswordVerifier::verify_password(&Argon2::default(), password.as_bytes(), &parsed)
            .is_ok()
    }

    fn hash(&self, password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| AuthError::Hash(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A hash verifies against its own plaintext and rejects others.
    #[test]
    fn hash_then_verify_round_trip() {
        let verifier = Argon2Verifier::new();
        let hash = verifier
            .hash("s3cr3tPass")
            .expect("argon2 hashing succeeds");
        assert!(verifier.verify(&hash, "s3cr3tPass"));
        assert!(!verifier.verify(&hash, "wrong-password"));
        assert!(!verifier.verify(&hash, ""));
    }

    /// Malformed stored hashes fail closed (never panic, never pass).
    #[test]
    fn malformed_hash_fails_closed() {
        let verifier = Argon2Verifier::new();
        assert!(!verifier.verify("not-a-phc-hash", "s3cr3tPass"));
        assert!(!verifier.verify("", "s3cr3tPass"));
    }
}
