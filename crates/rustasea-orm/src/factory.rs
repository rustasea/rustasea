//! Concrete seeder/factory helpers built on the migration traits.
//!
//! These give `database/seeders/` and test code a usable baseline; sequence
//! reset per `TestCase` hooks in when the test harness lands (S06).

pub use crate::migration::{Factory, Seeder};
use crate::{Model, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Generic seeder that runs a fixed SQL statement set.
#[derive(Debug, Clone)]
pub struct SqlSeeder {
    /// Seeder name.
    pub name: String,
    /// Idempotent SQL statements (expected `INSERT … ON CONFLICT DO NOTHING`).
    pub statements: Vec<String>,
}

#[async_trait]
impl Seeder for SqlSeeder {
    /// Seeder name reported by [`crate::Migrator::seed`].
    fn name(&self) -> &str {
        &self.name
    }

    /// Join the idempotent statements into one executable script.
    fn sql(&self) -> Result<String> {
        Ok(self.statements.join(";\n"))
    }
}

/// A named attribute variation applied on top of a factory's base definition.
pub struct FactoryState<T> {
    /// State name (`admin`, `guest`, …).
    name: String,
    /// Attribute transform applied to each produced instance.
    overrides: Arc<dyn Fn(T) -> T + Send + Sync>,
}

impl<T> FactoryState<T> {
    /// Create a state from its name and attribute transform.
    pub fn new<G>(name: impl Into<String>, overrides: G) -> Self
    where
        G: Fn(T) -> T + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            overrides: Arc::new(overrides),
        }
    }

    /// The state's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Apply the state transform to one instance.
    pub fn apply(&self, value: T) -> T {
        (self.overrides)(value)
    }
}

/// Concrete factory producing model instances with an auto-incrementing sequence.
pub struct SequenceFactory<T, F>
where
    F: Fn(usize) -> T + Send + Sync,
{
    generator: F,
    counter: Arc<AtomicUsize>,
    state: Option<FactoryState<T>>,
}

impl<T, F> SequenceFactory<T, F>
where
    F: Fn(usize) -> T + Send + Sync,
{
    /// Create a factory from a per-sequence generator closure.
    pub fn new(generator: F) -> Self {
        Self {
            generator,
            counter: Arc::new(AtomicUsize::new(0)),
            state: None,
        }
    }

    /// Activate a named attribute state (`Factory::state("admin")`).
    ///
    /// The transform runs after the base definition, so a state can override
    /// any field while the sequence keeps advancing as usual.
    pub fn state<G>(&mut self, name: impl Into<String>, overrides: G) -> &mut Self
    where
        G: Fn(T) -> T + Send + Sync + 'static,
    {
        self.state = Some(FactoryState::new(name, overrides));
        self
    }

    /// Remove the active state, returning to the base definition.
    pub fn reset_state(&mut self) {
        self.state = None;
    }

    /// The active state's name, when one is set.
    pub fn state_name(&self) -> Option<&str> {
        self.state.as_ref().map(FactoryState::name)
    }

    /// Reset the sequence counter (per-TestCase hook).
    pub fn reset(&self) {
        self.counter.store(0, Ordering::SeqCst);
    }

    /// Produce `n` instances (`UserFactory::create(5)`).
    pub fn create_many(&mut self, n: usize) -> Vec<T> {
        (0..n).map(|_| self.definition()).collect()
    }
}

impl<T, F> Factory<T> for SequenceFactory<T, F>
where
    F: Fn(usize) -> T + Send + Sync,
{
    /// Produce the next instance, bumping the sequence and applying any state.
    fn definition(&mut self) -> T {
        let seq = self.counter.fetch_add(1, Ordering::SeqCst);
        let value = (self.generator)(seq);
        match &self.state {
            Some(state) => state.apply(value),
            None => value,
        }
    }

    /// Instances produced so far.
    fn count(&self) -> usize {
        self.counter.load(Ordering::SeqCst)
    }
}

/// Example domain model exercising the Model contract end-to-end.
///
/// Mirrors the `users` table from `design/database.md` §2 (BC-2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// UUID primary key.
    pub id: uuid::Uuid,
    /// Display name.
    pub name: String,
    /// Unique email.
    pub email: String,
    /// Argon2 password hash.
    pub password: String,
    /// Timestamps (`created_at`/`updated_at`).
    pub timestamps: crate::Timestamps,
    /// Soft delete marker.
    pub soft_deletes: crate::SoftDeletes,
}

impl Model for User {
    /// Type name for table derivation.
    fn type_name() -> &'static str {
        "User"
    }

    /// Primary key.
    fn primary_key(&self) -> uuid::Uuid {
        self.id
    }

    /// Assign a fresh UUID (v7, client-generated).
    fn assign_id(&mut self) -> uuid::Uuid {
        self.id = uuid::Uuid::now_v7();
        self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies User derives the `users` table name via snake_plural.
    #[test]
    fn user_maps_to_users_table() {
        assert_eq!(<User as Model>::table_name(), "users");
    }

    /// Verifies factory sequence increments and resets.
    #[test]
    fn factory_sequence_increments() {
        let mut f = SequenceFactory::new(|seq: usize| format!("user-{seq}@example.test"));
        assert_eq!(f.definition(), "user-0@example.test");
        assert_eq!(f.definition(), "user-1@example.test");
        assert_eq!(f.count(), 2);
        f.reset();
        assert_eq!(f.count(), 0);
    }

    /// Verifies a named state overrides attributes while the sequence advances.
    #[test]
    fn factory_state_varies_attributes() {
        let mut f = SequenceFactory::new(|seq: usize| format!("user-{seq}@example.test"));
        f.state("admin", |mut email: String| {
            email = email.replace("@example.test", "@admin.test");
            email
        });
        assert_eq!(f.state_name(), Some("admin"));
        assert_eq!(f.definition(), "user-0@admin.test");
        assert_eq!(f.definition(), "user-1@admin.test");

        f.reset_state();
        assert_eq!(f.state_name(), None);
        assert_eq!(f.definition(), "user-2@example.test");
    }
}
