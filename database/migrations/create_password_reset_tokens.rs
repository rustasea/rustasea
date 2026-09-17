//! Creates the `password_reset_tokens` table.

use rustasea::orm::Migration;
use rustasea::OrmResult;

/// `create_password_reset_tokens_table` migration.
pub struct CreatePasswordResetTokens;

impl Migration for CreatePasswordResetTokens {
    fn name(&self) -> &str {
        "2027_01_01_000003_create_password_reset_tokens_table"
    }

    fn up(&self) -> OrmResult<String> {
        Ok(r#"CREATE TABLE password_reset_tokens (
    email VARCHAR(255) PRIMARY KEY,
    token VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);"#
        .to_string())
    }

    fn down(&self) -> OrmResult<String> {
        Ok("DROP TABLE IF EXISTS password_reset_tokens;".to_string())
    }
}
