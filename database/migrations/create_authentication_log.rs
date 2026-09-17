//! Creates the `authentication_log` table (sign-in history).
//!
//! `user_id` is a `VARCHAR(255)`, not a UUID: the auth guards carry an opaque
//! string identity, so the log accepts the same value the guards use.

use rustasea::orm::Migration;
use rustasea::OrmResult;

/// `create_authentication_log_table` migration.
pub struct CreateAuthenticationLog;

impl Migration for CreateAuthenticationLog {
    fn name(&self) -> &str {
        "2027_01_01_000009_create_authentication_log_table"
    }

    fn up(&self) -> OrmResult<String> {
        Ok(r#"CREATE TABLE authentication_log (
    id UUID PRIMARY KEY,
    user_id VARCHAR(255) NULL,
    email VARCHAR(255) NULL,
    guard_name VARCHAR(255) NULL,
    event VARCHAR(64) NOT NULL,
    ip_address VARCHAR(45) NULL,
    user_agent TEXT NULL,
    successful BOOLEAN NOT NULL,
    login_at TIMESTAMPTZ NULL,
    logout_at TIMESTAMPTZ NULL,
    cleared_by_user_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX authentication_log_user_id_index ON authentication_log (user_id);
CREATE INDEX authentication_log_event_index ON authentication_log (event);
CREATE INDEX authentication_log_ip_address_index ON authentication_log (ip_address);"#
        .to_string())
    }

    fn down(&self) -> OrmResult<String> {
        Ok("DROP TABLE IF EXISTS authentication_log;".to_string())
    }
}
