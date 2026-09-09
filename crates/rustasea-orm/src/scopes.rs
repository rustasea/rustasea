//! Named, composable query scopes.
//!
//! A scope is a named closure that mutates a `QueryBuilder` — the Eloquent
//! pattern (`scopeActive`, `scopeVerified`) adapted to Rust. Registries are
//! keyed `table.name` so multiple models can declare identically-named scopes.

use crate::builder::QueryBuilder;
use std::collections::HashMap;

/// Named scope that can be applied to a builder.
pub type Scope = Box<dyn Fn(&mut QueryBuilder) + Send + Sync>;

/// Registry of named scopes per model table.
#[derive(Default)]
pub struct ScopeRegistry {
    scopes: HashMap<String, Vec<Scope>>,
}

impl ScopeRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a named scope for a table.
    pub fn register<F>(&mut self, table: &str, name: &str, scope: F)
    where
        F: Fn(&mut QueryBuilder) + Send + Sync + 'static,
    {
        self.scopes
            .entry(format!("{table}.{name}"))
            .or_default()
            .push(Box::new(scope));
    }

    /// Apply every scope registered under `table.name` to the builder.
    pub fn apply(&self, builder: &mut QueryBuilder, table: &str, name: &str) {
        if let Some(scopes) = self.scopes.get(&format!("{table}.{name}")) {
            for scope in scopes {
                scope(builder);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies scopes compose additively on a single builder.
    #[test]
    fn scopes_compose() {
        let mut registry = ScopeRegistry::new();
        registry.register("users", "active", |b| {
            *b = std::mem::take(b).where_null("deleted_at");
        });
        registry.register("users", "verified", |b| {
            *b = std::mem::take(b).where_null("email_verified_at");
        });
        let qb = QueryBuilder::table("users")
            .with_scope(&registry, "active")
            .with_scope(&registry, "verified");
        assert_eq!(
            qb.to_sql().unwrap(),
            "SELECT * FROM users WHERE deleted_at IS NULL AND email_verified_at IS NULL"
        );
    }
}
