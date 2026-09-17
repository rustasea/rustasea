//! Seeds the default application data.

use rustasea::orm::{Result as OrmResult, Seeder};

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
