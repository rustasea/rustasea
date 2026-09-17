//! Creates the `roles` table (RBAC).

use rustasea::orm::Migration;
use rustasea::OrmResult;

/// `create_roles_table` migration.
pub struct CreateRoles;

impl Migration for CreateRoles {
    fn name(&self) -> &str {
        "2027_01_01_000004_create_roles_table"
    }

    fn up(&self) -> OrmResult<String> {
        Ok(r#"CREATE TABLE roles (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    guard VARCHAR(255) NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);"#
        .to_string())
    }

    fn down(&self) -> OrmResult<String> {
        Ok("DROP TABLE IF EXISTS roles;".to_string())
    }
}
