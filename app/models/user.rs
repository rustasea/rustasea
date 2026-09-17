//! `users` model — the shared auth core's user entity.

use chrono::{DateTime, Utc};
use rustasea::orm::{Model, Timestamps};
use uuid::Uuid;

/// Application user account.
///
/// The same model is generated for every variant; only the presentation layer
/// differs (ADR-0002 decision 1).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct User {
    /// Primary key.
    pub id: Uuid,
    /// Display name.
    pub name: String,
    /// Unique, verified email address.
    pub email: String,
    /// Argon2id password hash — never the plaintext.
    ///
    /// Hidden from serialization, mirroring the kit's
    /// `#[Hidden(['password'])]`.
    #[serde(skip_serializing)]
    pub password: String,
    /// Set once the email address is verified.
    pub email_verified_at: Option<DateTime<Utc>>,
    /// Two-factor shared secret, stored encrypted at rest.
    ///
    /// The column holds the *ciphertext*; the encrypt/decrypt layer ships with
    /// the 2FA feature. Hidden from serialization, mirroring the kit's
    /// `#[Hidden(['two_factor_secret'])]`.
    #[serde(skip_serializing)]
    pub two_factor_secret: Option<String>,
    /// JSON array of single-use 2FA recovery codes, stored as text.
    ///
    /// Hidden from serialization, mirroring the kit's
    /// `#[Hidden(['two_factor_recovery_codes'])]`.
    #[serde(skip_serializing)]
    pub two_factor_recovery_codes: Option<String>,
    /// Set once two-factor authentication has been confirmed.
    pub two_factor_confirmed_at: Option<DateTime<Utc>>,
    /// Remember-me token backing persistent ("remember me") logins.
    ///
    /// Hidden from serialization, mirroring the kit's
    /// `#[Hidden(['remember_token'])]`.
    #[serde(skip_serializing)]
    pub remember_token: Option<String>,
    /// IANA timezone name for the user's wall-clock preferences.
    ///
    /// `None` means the account has no explicit preference and the resolver
    /// falls through to the application default.
    pub timezone: Option<String>,
    /// Soft-delete marker.
    ///
    /// The ORM soft-deletes by default (`uses_soft_deletes`); a hard delete is
    /// requested explicitly via the ORM's force-delete path when required.
    pub deleted_at: Option<DateTime<Utc>>,
    /// Created/updated timestamps.
    pub timestamps: Timestamps,
}

/// Hand-written ORM contract, mirroring the `make:model` generator.
///
/// The `#[derive(Model)]` macro expands to `rustasea_orm::…` paths, which the
/// generated app does not depend on directly; writing the impl against the
/// umbrella's `rustasea::orm::Model` keeps the app's dependency surface to the
/// single `rustasea` crate.
impl Model for User {
    /// Type name driving the default table derivation.
    fn type_name() -> &'static str {
        "User"
    }

    /// Explicit table name (`users`).
    fn table_name() -> String {
        "users".to_string()
    }

    /// Primary key value.
    fn primary_key(&self) -> Uuid {
        self.id
    }

    /// Assign a fresh client-generated UUID (v7) before persistence.
    fn assign_id(&mut self) -> Uuid {
        self.id = Uuid::now_v7();
        self.id
    }

    /// Soft deletes are enabled via `deleted_at`.
    fn uses_soft_deletes() -> bool {
        true
    }

    /// Timestamps are maintained via the `timestamps` column.
    fn uses_timestamps() -> bool {
        true
    }
}
