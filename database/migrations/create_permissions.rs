//! Creates the `permissions` table (RBAC).

use rustasea::orm::Migration;
use rustasea::OrmResult;

/// `create_permissions_table` migration.
pub struct CreatePermissions;

impl Migration for CreatePermissions {
    fn name(&self) -> &str {
        "2027_01_01_000005_create_permissions_table"
    }

    fn up(&self) -> OrmResult<String> {
        Ok(r#"CREATE TABLE permissions (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    guard VARCHAR(255) NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);"#
        .to_string())
    }

    fn down(&self) -> OrmResult<String> {
        Ok("DROP TABLE IF EXISTS permissions;".to_string())
    }
}
