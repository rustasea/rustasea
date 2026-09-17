//! Creates the `sessions` table used by the browser session guard.

use rustasea::orm::Migration;
use rustasea::OrmResult;

/// `create_sessions_table` migration.
pub struct CreateSessions;

impl Migration for CreateSessions {
    fn name(&self) -> &str {
        "2027_01_01_000002_create_sessions_table"
    }

    fn up(&self) -> OrmResult<String> {
        Ok(r#"CREATE TABLE sessions (
    id VARCHAR(255) PRIMARY KEY,
    user_id UUID NULL,
    payload TEXT NOT NULL,
    last_activity BIGINT NOT NULL
);"#
        .to_string())
    }

    fn down(&self) -> OrmResult<String> {
        Ok("DROP TABLE IF EXISTS sessions;".to_string())
    }
}
