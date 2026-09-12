//! Database layer: migrations, factories, and seeders.
//!
//! Mirrors Laravel's `database/{migrations,factories,seeders}`. Migrations are
//! reversible (`up` + `down`) per the project database standards.

use super::TemplateFile;

/// Database templates (shared by every variant).
pub fn entries() -> Vec<TemplateFile> {
    vec![
        ("database/mod.rs", DATABASE_MOD),
        ("database/migrations/mod.rs", MIGRATIONS_MOD),
        ("database/migrations/create_users.rs", CREATE_USERS),
        ("database/migrations/create_sessions.rs", CREATE_SESSIONS),
        (
            "database/migrations/create_password_reset_tokens.rs",
            CREATE_PASSWORD_RESET_TOKENS,
        ),
        ("database/factories/mod.rs", FACTORIES_MOD),
        ("database/factories/user_factory.rs", USER_FACTORY),
        ("database/seeders/mod.rs", SEEDERS_MOD),
        ("database/seeders/database_seeder.rs", DATABASE_SEEDER),
    ]
}

const DATABASE_MOD: &str = r##"//! Database layer — migrations, factories, and seeders.

pub mod factories;
pub mod migrations;
pub mod seeders;
"##;

const MIGRATIONS_MOD: &str = r##"//! Versioned, reversible schema migrations.

pub mod create_password_reset_tokens;
pub mod create_sessions;
pub mod create_users;
"##;

const CREATE_USERS: &str = r##"//! Creates the `users` table.

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
"##;

const CREATE_SESSIONS: &str = r##"//! Creates the `sessions` table used by the browser session guard.

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
"##;

const CREATE_PASSWORD_RESET_TOKENS: &str = r##"//! Creates the `password_reset_tokens` table.

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
"##;

const FACTORIES_MOD: &str = r##"//! Model factories for tests and seeders.

pub mod user_factory;

pub use user_factory::UserFactory;
"##;

const USER_FACTORY: &str = r##"//! `UserFactory` — deterministic user fixtures.

use rustasea::orm::Factory;
use uuid::Uuid;

use crate::app::models::User;

/// Produces `User` instances with sequence-unique emails.
#[derive(Default)]
pub struct UserFactory {
    count: usize,
}

impl Factory<User> for UserFactory {
    fn definition(&mut self) -> User {
        self.count += 1;
        User {
            id: Uuid::new_v4(),
            name: format!("User {}", self.count),
            email: format!("user{}@example.test", self.count),
            password: "hashed-placeholder".to_string(),
            email_verified_at: None,
            deleted_at: None,
            timestamps: Default::default(),
        }
    }

    fn count(&self) -> usize {
        self.count
    }
}
"##;

const SEEDERS_MOD: &str = r##"//! Database seeders.

pub mod database_seeder;

pub use database_seeder::DatabaseSeeder;
"##;

const DATABASE_SEEDER: &str = r##"//! Seeds the default application data.

use rustasea::orm::{Seeder, OrmResult};

/// `DatabaseSeeder` — inserts the default application records.
pub struct DatabaseSeeder;

impl Seeder for DatabaseSeeder {
    fn name(&self) -> &str {
        "DatabaseSeeder"
    }

    fn sql(&self) -> OrmResult<String> {
        // Seed statements must be idempotent (`ON CONFLICT DO NOTHING`).
        Ok(String::new())
    }
}
"##;
