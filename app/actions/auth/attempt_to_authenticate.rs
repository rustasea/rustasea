//! Attempts to authenticate a login request against the session guard.
//!
//! Authentication is an [`Action`](rustasea::action::Action): the free
//! `attempt` helper holds the guard-generic logic, and `AttemptToAuthenticate`
//! exposes it through the action trait so it can run from any adapter.

use std::sync::Arc;

use rustasea::auth::{AuthError, SessionGuard};

/// Login credentials accepted by the authentication action.
#[derive(Debug, Clone)]
pub struct AttemptInput {
    /// Email address supplied by the login form.
    pub email: String,
    /// Plaintext password supplied by the login form.
    pub password: String,
}

/// Authenticate `email` + `password`, rotating the session id on success.
pub fn attempt<S>(guard: &SessionGuard<S>, email: &str, password: &str) -> Result<(), AuthError>
where
    S: tower_sessions::SessionStore + Send + Sync + 'static,
{
    let _ = (guard, email, password);
    todo!("verify credentials and rotate the session id")
}

/// The authentication action, holding the session guard to authenticate against.
pub struct AttemptToAuthenticate<S>
where
    S: tower_sessions::SessionStore + Send + Sync + 'static,
{
    /// Session guard the credentials are verified against.
    pub guard: Arc<SessionGuard<S>>,
}

#[rustasea::action::async_trait]
impl<S> rustasea::action::Action for AttemptToAuthenticate<S>
where
    S: tower_sessions::SessionStore + Send + Sync + 'static,
{
    type Input = AttemptInput;
    type Output = ();
    type Error = AuthError;

    /// Authenticate the submitted credentials via [`attempt`].
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
        attempt(&self.guard, &input.email, &input.password).map(|_| ())
    }
}
