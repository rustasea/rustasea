//! Validated password-update input.
//!
//! Implemented against the real validation contract. The confirmation is
//! modelled as an explicit second field with a plain `required|min:12` rule
//! because the `confirmed` rule does not exist yet (AUTH-005 adds it); the
//! explicit field is the same shape the rule will compare against.

use rustasea::validation::{ErrorBag, Rules, Validatable};

/// Password update form request.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PasswordUpdateRequest {
    /// Current password, re-checked before rotation.
    pub current_password: String,
    /// New password.
    pub password: String,
    /// Confirmation of the new password.
    pub password_confirmation: String,
}

impl Validatable for PasswordUpdateRequest {
    /// Validate the current/new/confirmation fields.
    fn validate(&self) -> Result<(), ErrorBag> {
        let rules = Rules::new()
            .field("current_password", "required")
            .field("password", "required|min:12")
            // TODO(AUTH-005): replace this explicit field with `confirmed` on
            // `password`, which compares it against `password_confirmation`.
            .field("password_confirmation", "required|min:12");
        let data = rustasea::validation::serde_json::to_value(self)
            .map_err(|error| ErrorBag::from_message(error.to_string()))?;
        rules.validate(&data)
    }
}
