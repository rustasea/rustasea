//! Creates the `users` table.

use rustasea::orm::Migration;
use rustasea::OrmResult;

/// `create_users_table` migration.
pub struct CreateUsers;

impl Migration for CreateUsers {
    fn name(&self) -> &str {
        "2027_01_01_000001_create_users_table"
    }

    fn up(&self) -> OrmResult<String> {
        Ok(r#"CREATE TABLE users (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    password VARCHAR(255) NOT NULL,
    email_verified_at TIMESTAMPTZ NULL,
    -- Two-factor secret, stored encrypted at rest (ciphertext in this column).
    two_factor_secret TEXT NULL,
    -- JSON array of single-use 2FA recovery codes, stored as text.
    two_factor_recovery_codes TEXT NULL,
    two_factor_confirmed_at TIMESTAMPTZ NULL,
    -- Remember-me token for persistent logins.
    remember_token VARCHAR(100) NULL,
    -- IANA timezone name for the user's wall-clock preferences.
    timezone VARCHAR(64) NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    deleted_at TIMESTAMPTZ NULL
);"#
        .to_string())
    }

    fn down(&self) -> OrmResult<String> {
        Ok("DROP TABLE IF EXISTS users;".to_string())
    }
}
