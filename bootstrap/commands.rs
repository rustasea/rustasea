//! CLI command registry - `cargo artisan` console commands.
//!
//! This registry gives the previously-empty command site a real home; commands
//! generated into `app/console/commands/*` are appended here. [`register_default`]
//! is the boot-time entry point: it installs the framework's built-in command
//! surface and registers the application migrations into the process-wide ORM
//! migrator so `cargo artisan migrate` never reports an empty registry.

use std::sync::OnceLock;

use rustasea::orm::register_migration;

use crate::database::migrations::{
    create_audit_log::CreateAuditLog,
    create_authentication_log::CreateAuthenticationLog,
    create_password_reset_tokens::CreatePasswordResetTokens,
    create_permission_role::CreatePermissionRole,
    create_permissions::CreatePermissions,
    create_role_user::CreateRoleUser,
    create_roles::CreateRoles,
    create_sessions::CreateSessions,
    create_users::CreateUsers,
};

/// Names of the console commands registered for the application.
pub fn commands() -> Vec<&'static str> {
    vec![]
}

/// Guards the application-migration registration so a second `configure()`
/// call (tests, embedded boots) never registers a migration twice.
static MIGRATIONS_REGISTERED: OnceLock<()> = OnceLock::new();

/// Register the framework command surface and the application migrations.
///
/// `rustasea::cli::load_default_commands()` installs the built-in commands and
/// the framework queue migrations (`jobs`, `failed_jobs`, `job_batches`); it is
/// idempotent. The application's own nine migrations are then registered into
/// the process-wide migrator in execution order (foreign keys depend on the
/// `users`/`roles`/`permissions` tables existing first), guarded so repeated
/// boots register them exactly once.
pub fn register_default() {
    rustasea::cli::load_default_commands();
    MIGRATIONS_REGISTERED.get_or_init(|| {
        register_migration(CreateUsers);
        register_migration(CreateSessions);
        register_migration(CreatePasswordResetTokens);
        register_migration(CreateRoles);
        register_migration(CreatePermissions);
        register_migration(CreateRoleUser);
        register_migration(CreatePermissionRole);
        register_migration(CreateAuditLog);
        register_migration(CreateAuthenticationLog);
    });
}
