//! `make:migration` template — database/migrations/<timestamp>_<name>.rs.
//!
//! Migration names are snake_case (`create_users_table`); the struct class is
//! the PascalCase form and the filename carries the `YYYY_MM_DD_HHMMSS_`
//! prefix so the file system orders migrations chronologically (design
//! `flows.md` migration row).

use std::path::Path;

use crate::error::CliResult;
use crate::generator::{Generated, Generator};
use crate::generators::kinds::{migration_prefix, write_scaffold};
use crate::generators::MakeOptions;

/// Normalize any accepted spelling (`CreateUsersTable` or `create_users_table`)
/// to snake_case for the file name.
fn slug(name: &str) -> String {
    Generator::snake(name)
}

/// Convert a snake_case migration name to its PascalCase struct class
/// (`create_users_table` → `CreateUsersTable`).
fn pascal(slug: &str) -> String {
    slug.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut word = first.to_ascii_uppercase().to_string();
                    word.push_str(&part[first.len_utf8()..].to_ascii_lowercase());
                    word
                }
                None => String::new(),
            }
        })
        .collect()
}

/// Guess the affected table from a create-style migration name
/// (`create_users_table` → `users`); other names fall back to the full slug
/// as an editable placeholder.
fn table_hint(slug: &str) -> String {
    let rest = slug.strip_prefix("create_").unwrap_or(slug);
    let rest = rest.strip_suffix("_table").unwrap_or(rest);
    if rest.is_empty() {
        slug.to_string()
    } else {
        rest.to_string()
    }
}

/// Validate a migration name: a non-empty snake_case identifier.
pub fn validate_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('_')
        && !name.ends_with('_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Render and write the migration file.
pub fn scaffold(root: &Path, opts: &MakeOptions) -> CliResult<Generated> {
    let slug = slug(&opts.name);
    let class = pascal(&slug);
    let table = table_hint(&slug);
    let migration = format!("{}_{}", migration_prefix(), slug);
    let rel = format!("database/migrations/{migration}.rs");
    let source = format!(
        r#"//! Migration scaffold — {migration}.
//!
//! Edit `up`/`down` to describe the schema change; `up` must be
//! re-runnable-safe only where noted. Model changes with `make:model -m`
//! pre-fill the create-table shape.

use rustasea::orm::migration::Migration;
use rustasea::orm::Result;

/// Applies and reverts the `{migration}` change.
pub struct {class};

impl Migration for {class} {{
    /// Unique migration name (timestamp-prefixed).
    fn name(&self) -> &str {{
        "{migration}"
    }}

    /// Apply the migration forward.
    fn up(&self) -> Result<String> {{
        Ok(format!(
            "CREATE TABLE IF NOT EXISTS {{}} (
                id UUID PRIMARY KEY,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                deleted_at TIMESTAMPTZ
            )",
            "{table}"
        ))
    }}

    /// Revert the migration.
    fn down(&self) -> Result<String> {{
        Ok(format!("DROP TABLE IF EXISTS {{}};", "{table}"))
    }}
}}
"#,
        class = class,
        migration = migration,
        table = table,
    );
    write_scaffold(root, rel, source, opts.force)
}

#[cfg(test)]
mod tests {
    use super::{pascal, table_hint, validate_name};

    #[test]
    fn pascal_capitalizes_snake_parts() {
        assert_eq!(pascal("create_users_table"), "CreateUsersTable");
    }

    #[test]
    fn pascal_tolerates_pascal_input() {
        let slug = crate::generator::Generator::snake("CreateUsersTable");
        assert_eq!(pascal(&slug), "CreateUsersTable");
    }

    #[test]
    fn table_hint_extracts_create_target() {
        assert_eq!(table_hint("create_users_table"), "users");
        assert_eq!(table_hint("add_avatar_to_users"), "add_avatar_to_users");
    }

    #[test]
    fn validation_accepts_snake_only() {
        assert!(validate_name("create_users_table"));
        assert!(validate_name("User"));
        assert!(!validate_name(""));
        assert!(!validate_name("create users"));
        assert!(!validate_name("_leading"));
    }
}
