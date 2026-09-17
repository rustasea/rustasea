//! Creates the `role_user` pivot table (RBAC user ↔ role assignments).

use rustasea::orm::Migration;
use rustasea::OrmResult;

/// `create_role_user_table` migration.
pub struct CreateRoleUser;

impl Migration for CreateRoleUser {
    fn name(&self) -> &str {
        "2027_01_01_000006_create_role_user_table"
    }

    fn up(&self) -> OrmResult<String> {
        Ok(r#"CREATE TABLE role_user (
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES roles (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (user_id, role_id)
);
CREATE INDEX role_user_role_id_index ON role_user (role_id);"#
        .to_string())
    }

    fn down(&self) -> OrmResult<String> {
        Ok("DROP TABLE IF EXISTS role_user;".to_string())
    }
}
