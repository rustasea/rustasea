//! `make:model` template — app/models/<snake>.rs + optional migration.
//!
//! The model derives the ORM `Model` contract and ships a `SequenceFactory`
//! for seed/test data; `-m` additionally scaffolds the paired
//! `database/migrations/*_create_<table>_table.rs`.

use std::path::Path;

use crate::error::CliResult;
use crate::generator::{Generated, Generator};
use crate::generators::kinds::{migration_prefix, slug, write_scaffold};
use crate::generators::MakeOptions;

/// Render and write the model file.
pub fn scaffold(root: &Path, opts: &MakeOptions) -> CliResult<Generated> {
    let rel = format!("app/models/{}.rs", slug(&opts.name));
    let table = Generator::table(&opts.name);
    let source = format!(
        r#"//! ORM model scaffold — {name}.
//!
//! The model maps to the `{table}` table. Customize fields, casts and
//! relations; `Model` supplies the ORM contract.

use rustavel::orm::factory::SequenceFactory;
use rustavel::orm::model::{{SoftDeletes, Timestamps}};
use rustavel::orm::Model;

/// Row type for the `{table}` table.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct {name} {{
    /// UUID primary key.
    pub id: uuid::Uuid,
    /// Timestamps (`created_at`/`updated_at`).
    pub timestamps: Timestamps,
    /// Soft delete marker.
    pub soft_deletes: SoftDeletes,
}}

impl Model for {name} {{
    /// Type name driving table derivation.
    fn type_name() -> &'static str {{
        "{name}"
    }}

    /// Primary key value.
    fn primary_key(&self) -> uuid::Uuid {{
        self.id
    }}

    /// Assign a fresh client-generated UUID (v7).
    fn assign_id(&mut self) -> uuid::Uuid {{
        let id = uuid::Uuid::now_v7();
        self.id = id;
        id
    }}
}}

/// Factory producing test/seed {name} rows with a reset-able sequence.
pub type {name}Factory = SequenceFactory<{name}, Box<dyn Fn(usize) -> {name} + Send + Sync>>;

/// Build a fresh {name} factory.
pub fn factory() -> {name}Factory {{
    SequenceFactory::new(Box::new(|_seq: usize| {name} {{
        id: uuid::Uuid::now_v7(),
        timestamps: Timestamps::default(),
        soft_deletes: SoftDeletes::default(),
    }}))
}}
"#,
        table = table,
        name = opts.name,
    );
    write_scaffold(root, rel, source, opts.force)
}

/// Render and write the paired create-table migration.
pub fn scaffold_migration(root: &Path, opts: &MakeOptions) -> CliResult<Generated> {
    let table = Generator::table(&opts.name);
    let class = format!("Create{}Table", opts.name);
    let migration = format!("{}_create_{}_table", migration_prefix(), table);
    let rel = format!("database/migrations/{migration}.rs");
    let source = format!(
        r#"//! Migration scaffold — create the `{table}` table.

use rustavel::orm::migration::Migration;
use rustavel::orm::Result;

/// Creates and drops the `{table}` table.
pub struct {class};

impl Migration for {class} {{
    /// Unique migration name (timestamp-prefixed).
    fn name(&self) -> &str {{
        "{migration}"
    }}

    /// Apply the migration forward.
    fn up(&self) -> Result<String> {{
        Ok(format!(
            "CREATE TABLE {{}} (
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
