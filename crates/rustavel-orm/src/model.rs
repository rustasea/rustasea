//! Model trait, timestamps, soft deletes, and relations.

use crate::naming::snake_plural;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Timestamp columns managed by the ORM.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Timestamps {
    /// Row creation time (`created_at`).
    pub created_at: DateTime<Utc>,
    /// Last update time (`updated_at`), bumped via `set_updated_at()` trigger or ORM.
    pub updated_at: DateTime<Utc>,
}

impl Default for Timestamps {
    /// Default to "now" for both stamps.
    fn default() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            updated_at: now,
        }
    }
}

/// Soft delete contract — `deleted_at IS NULL` means active.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SoftDeletes {
    /// Soft-delete marker; `None` = active row.
    pub deleted_at: Option<DateTime<Utc>>,
}

impl SoftDeletes {
    /// Whether the row has been soft-deleted.
    pub fn trashed(&self) -> bool {
        self.deleted_at.is_some()
    }
}

/// Kind of relation between two models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    /// `User has_many posts` — FK `user_id` on the related table.
    HasMany,
    /// Inverse of HasMany — FK on this table.
    BelongsTo,
    /// Pivot-table relation (scaffolded; full impl M3+).
    ManyToMany,
}

/// A declared relation used by eager loading (`with`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relation {
    /// Relation name as used in `Model::with("posts")`.
    pub name: String,
    /// Related model's table name.
    pub related_table: String,
    /// FK column on the owning side (HasMany: related table; BelongsTo: this table).
    pub foreign_key: String,
    /// Local key on the parent side.
    pub local_key: String,
    /// Relation kind.
    pub kind: RelationKind,
}

impl Relation {
    /// Declare a HasMany relation using the `snake_singular` FK convention.
    pub fn has_many(name: &str, related_table: &str, local_model: &str) -> Self {
        let fk = format!("{}_id", crate::naming::to_snake_case(local_model));
        Self {
            name: name.to_string(),
            related_table: related_table.to_string(),
            foreign_key: fk,
            local_key: "id".to_string(),
            kind: RelationKind::HasMany,
        }
    }

    /// Declare a BelongsTo relation using the `snake_singular` FK convention.
    pub fn belongs_to(name: &str, parent_table: &str) -> Self {
        let fk = format!("{}_id", crate::naming::singular_from_plural(parent_table));
        Self {
            name: name.to_string(),
            related_table: parent_table.to_string(),
            foreign_key: fk,
            local_key: "id".to_string(),
            kind: RelationKind::BelongsTo,
        }
    }
}

/// Eager-loaded relation payloads attached to a fetched model.
///
/// Survives `serde` round-trips so `relations` are preserved (FS-M2-02, #13).
pub type Relations = HashMap<String, serde_json::Value>;

/// Base model contract implemented by `#[derive(Model)]` (macro in a later sprint).
pub trait Model: Send + Sync {
    /// Table name, derived from `snake_plural(type_name)`.
    fn table_name() -> String
    where
        Self: Sized,
    {
        snake_plural(Self::type_name())
    }

    /// Type name used to derive the table name.
    fn type_name() -> &'static str
    where
        Self: Sized;

    /// Primary key value (`id UUID`).
    fn primary_key(&self) -> Uuid;

    /// Create a new UUID before first persistence.
    fn assign_id(&mut self) -> Uuid;

    /// Declared relations for eager loading.
    fn relations() -> Vec<Relation>
    where
        Self: Sized,
    {
        Vec::new()
    }

    /// Whether this model uses `deleted_at` soft deletes.
    fn uses_soft_deletes() -> bool
    where
        Self: Sized,
    {
        true
    }

    /// Whether this model maintains `created_at`/`updated_at`.
    fn uses_timestamps() -> bool
    where
        Self: Sized,
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies belongs_to derives a `snake_singular` FK across plural rules.
    #[test]
    fn belongs_to_singularizes_parent_table() {
        let r = Relation::belongs_to("category", "categories");
        assert_eq!(r.foreign_key, "category_id");
        let r = Relation::belongs_to("post", "posts");
        assert_eq!(r.foreign_key, "post_id");
    }
}
