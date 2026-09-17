//! `UserFactory` — deterministic user fixtures.
//!
//! The default definition uses a monotonic sequence so fixtures are stable
//! across runs (`user1@example.test`, `user2@example.test`, …). For richer,
//! locale-aware data enable the umbrella crate's `faker` feature
//! (`rustasea = { version = "0.1", features = ["faker"] }`) and seed a
//! [`Faker`](rustasea::testing::faker::Faker) for reproducible randomness:
//!
//! ```ignore
//! use rustasea::testing::faker::Faker;
//!
//! // Deterministic: the same seed replays the same sequence every run.
//! let mut faker = Faker::from_config(42, "en_US");
//! let name = faker.name();
//! let email = faker.unique_email().expect("unique email");
//! ```

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
            two_factor_secret: None,
            two_factor_recovery_codes: None,
            two_factor_confirmed_at: None,
            remember_token: None,
            timezone: None,
            deleted_at: None,
            timestamps: Default::default(),
        }
    }

    fn count(&self) -> usize {
        self.count
    }
}
