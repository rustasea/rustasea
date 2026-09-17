//! Validated profile-update input.
//!
//! Implemented against the real validation contract: the payload derives
//! `serde::Deserialize` (required by `Validatable: DeserializeOwned`) and
//! `serde::Serialize` (used to build the JSON value the rules run against),
//! and `Validatable::validate` builds a `Rules` set with the fluent
//! `Rules::field(field, "rule|rule")` grammar. Handlers consume it through the
//! `rustasea::validation::FormRequest<ProfileUpdateRequest>` extractor.

use rustasea::validation::{ErrorBag, Rules, Validatable};

/// Profile update form request.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ProfileUpdateRequest {
    /// New display name.
    pub name: String,
    /// New email address.
    pub email: String,
}

impl Validatable for ProfileUpdateRequest {
    /// Validate `name` and `email` with the implemented rule grammar.
    fn validate(&self) -> Result<(), ErrorBag> {
        let rules = Rules::new()
            .field("name", "required|max:255")
            .field("email", "required|email");
        let data = rustasea::validation::serde_json::to_value(self)
            .map_err(|error| ErrorBag::from_message(error.to_string()))?;
        rules.validate(&data)
    }
}
