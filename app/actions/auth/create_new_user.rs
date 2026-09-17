//! Creates and persists a user during registration.
//!
//! Registration is an [`Action`](rustasea::action::Action): the same unit of
//! work is reachable from the HTTP registration controller, a queued job, the
//! CLI, or an event without duplicating the persistence logic.

use crate::app::models::User;

/// Validated registration input.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NewUser {
    /// Display name.
    pub name: String,
    /// Email address.
    pub email: String,
    /// Plaintext password — hashed with argon2id before persistence.
    pub password: String,
}

/// The registration action.
pub struct CreateNewUser;

#[rustasea::action::async_trait]
impl rustasea::action::Action for CreateNewUser {
    type Input = NewUser;
    type Output = User;
    type Error = rustasea::auth::AuthError;

    /// Create a user, hashing the password and applying registration defaults.
    ///
    /// The session guard hashes with argon2id and rotates the session id on
    /// login; this action owns persistence only.
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
        let _ = input;
        todo!("persist the user via the ORM writer")
    }
}
