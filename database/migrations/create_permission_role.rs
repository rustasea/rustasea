//! Creates the `permission_role` pivot table (RBAC permission ↔ role grants).

use rustasea::orm::Migration;
use rustasea::OrmResult;

/// `create_permission_role_table` migration.
pub struct CreatePermissionRole;

impl Migration for CreatePermissionRole {
    fn name(&self) -> &str {
        "2027_01_01_000007_create_permission_role_table"
    }

    fn up(&self) -> OrmResult<String> {
        Ok(r#"CREATE TABLE permission_role (
    permission_id UUID NOT NULL REFERENCES permissions (id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES roles (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (permission_id, role_id)
);
CREATE INDEX permission_role_role_id_index ON permission_role (role_id);"#
        .to_string())
    }

    fn down(&self) -> OrmResult<String> {
        Ok("DROP TABLE IF EXISTS permission_role;".to_string())
    }
}
