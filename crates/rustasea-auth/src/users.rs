/// User lookup seam for the JWT guard.
///
/// The M2 ORM sprint ships the `users` table; until a typed `User` model +
/// repository is wired through `AppState`, guards resolve credentials through
/// this object-safe trait. `StaticLookup` is the fail-closed default; tests
/// seed `MemoryUserRegistry`.
use std::collections::HashMap;
use std::sync::RwLock;

/// Minimal user record the auth layer needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthUserRecord {
    /// User UUID.
    pub id: String,
    /// Login email.
    pub email: String,
    /// Argon2 PHC hash of the password.
    pub password_hash: String,
    /// `email_verified_at` cleared by `markEmailAsUnverified`.
    pub email_verified_at: Option<String>,
}

/// Credential lookup contract injected into guards.
pub trait UserLookup: Send + Sync {
    /// Resolve the stored PHC hash for an email.
    fn hash_for_email(&self, email: &str) -> Option<String>;

    /// Resolve the user UUID for an email.
    fn id_for_email(&self, email: &str) -> Option<String>;

    /// Resolve the email for a user UUID.
    fn email_for_id(&self, id: &str) -> Option<String>;
}

/// Fail-closed lookup: never yields credentials.
///
/// The real database-backed lookup replaces this via `JwtGuard::with_lookup`
/// at boot (ADR-0007 explicit AppState wiring).
#[derive(Debug, Default)]
pub struct StaticLookup;

impl UserLookup for StaticLookup {
    fn hash_for_email(&self, _email: &str) -> Option<String> {
        None
    }

    fn id_for_email(&self, _email: &str) -> Option<String> {
        None
    }

    fn email_for_id(&self, _id: &str) -> Option<String> {
        None
    }
}

/// In-memory lookup for tests and local development.
#[derive(Debug, Default)]
pub struct MemoryUserRegistry {
    by_email: RwLock<HashMap<String, AuthUserRecord>>,
}

impl MemoryUserRegistry {
    /// Seed a user record for guard tests.
    pub fn seed(&self, record: AuthUserRecord) {
        // Poisoned lock => registry is unusable; fail closed (no-op) rather
        // than panic in a framework crate.
        if let Ok(mut by_email) = self.by_email.write() {
            by_email.insert(record.email.clone(), record);
        }
    }

    /// Look up a record by email.
    pub fn by_email(&self, email: &str) -> Option<AuthUserRecord> {
        self.by_email.read().ok()?.get(email).cloned()
    }
}

impl UserLookup for MemoryUserRegistry {
    fn hash_for_email(&self, email: &str) -> Option<String> {
        self.by_email(email).map(|r| r.password_hash)
    }

    fn id_for_email(&self, email: &str) -> Option<String> {
        self.by_email(email).map(|r| r.id)
    }

    fn email_for_id(&self, id: &str) -> Option<String> {
        self.by_email
            .read()
            .ok()?
            .values()
            .find(|r| r.id == id)
            .map(|r| r.email.clone())
    }
}
