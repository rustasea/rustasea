/// Error bag — groups validation failures by field.
///
/// Laravel's `ErrorBag` (#19): multiple fields coexist, and a single field
/// can hold multiple errors. Serializes to the documented 422 shape:
///
/// ```json
/// { "message": "The given data was invalid.", "errors": { "email": ["..."] } }
/// ```
use std::collections::BTreeMap;

use serde::Serialize;

/// Single validation failure for a field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ValidationError {
    /// Stable rule code (e.g. `email`, `length`, `in_array`).
    pub code: String,
    /// Human message shown to the client.
    pub message: String,
    /// Optional field path for nested errors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl ValidationError {
    /// Create an error with a code and message.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            path: None,
        }
    }

    /// Attach a nested path.
    pub fn at(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }
}

impl From<validator::ValidationError> for ValidationError {
    /// Map a `validator` error onto a typed validation error.
    fn from(err: validator::ValidationError) -> Self {
        let message = err
            .message
            .clone()
            .map(|m| m.to_string())
            .unwrap_or_else(|| format!("validation failed for rule {:?}", err.code));
        Self::new(err.code.to_string(), message)
    }
}

/// Field-keyed collection of validation errors.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ErrorBag {
    errors: BTreeMap<String, Vec<ValidationError>>,
}

impl ErrorBag {
    /// Create an empty bag.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an error under a field, preserving multi-error coexistence.
    pub fn add(&mut self, field: impl Into<String>, err: ValidationError) {
        self.errors.entry(field.into()).or_default().push(err);
    }

    /// Add a plain message under a field (auto `code = "validation"`).
    pub fn add_message(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.add(field, ValidationError::new("validation", message));
    }

    /// Merge another bag's errors into this one.
    pub fn merge(&mut self, other: ErrorBag) {
        for (field, mut errs) in other.errors {
            self.errors.entry(field).or_default().append(&mut errs);
        }
    }

    /// Whether the bag holds no errors.
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Whether the bag holds any errors.
    pub fn has_errors(&self) -> bool {
        !self.is_empty()
    }

    /// Errors for a single field.
    pub fn get(&self, field: &str) -> &[ValidationError] {
        self.errors.get(field).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// All (field, errors) pairs in stable (sorted-field) order.
    pub fn fields(&self) -> impl Iterator<Item = (&String, &Vec<ValidationError>)> {
        self.errors.iter()
    }

    /// Field names that hold at least one error.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.errors.keys()
    }

    /// Number of fields with errors.
    pub fn field_count(&self) -> usize {
        self.errors.len()
    }

    /// Total number of individual errors across all fields.
    pub fn total(&self) -> usize {
        self.errors.values().map(|v| v.len()).sum()
    }

    /// Convert into the documented 422 JSON body.
    pub fn into_json_body(self) -> serde_json::Value {
        let errors = self
            .errors
            .into_iter()
            .map(|(field, errs)| {
                let messages: Vec<String> = errs.into_iter().map(|e| e.message).collect();
                (
                    field,
                    serde_json::Value::Array(
                        messages
                            .into_iter()
                            .map(serde_json::Value::String)
                            .collect(),
                    ),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        serde_json::json!({
            "message": "The given data was invalid.",
            "errors": serde_json::Value::Object(errors),
        })
    }
}

/// Build a 422 JSON response from an `ErrorBag` (axum integration).
impl From<ErrorBag> for axum::response::Response {
    fn from(bag: ErrorBag) -> Self {
        use axum::http::StatusCode;
        use axum::response::IntoResponse;
        use axum::Json;
        (StatusCode::UNPROCESSABLE_ENTITY, Json(bag.into_json_body())).into_response()
    }
}

/// Collect errors from the `validator` crate into an `ErrorBag`.
pub fn from_validator(errors: validator::ValidationErrors) -> ErrorBag {
    let mut bag = ErrorBag::new();
    for (field, field_errors) in errors.field_errors() {
        for err in field_errors {
            bag.add(field.to_string(), ValidationError::from(err.clone()));
        }
    }
    bag
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Multiple fields and multiple errors per field coexist.
    #[test]
    fn multi_field_multi_error_coexist() {
        let mut bag = ErrorBag::new();
        bag.add(
            "email",
            ValidationError::new("email", "The email must be a valid email address."),
        );
        bag.add(
            "password",
            ValidationError::new("length", "The password must be at least 8 characters."),
        );
        bag.add(
            "password",
            ValidationError::new("confirmed", "The password confirmation does not match."),
        );
        assert_eq!(bag.field_count(), 2);
        assert_eq!(bag.total(), 3);
        assert_eq!(bag.get("email").len(), 1);
        assert_eq!(bag.get("password").len(), 2);
    }

    /// The JSON body matches the api-validation.md 422 contract.
    #[test]
    fn json_body_matches_422_contract() {
        let mut bag = ErrorBag::new();
        bag.add(
            "name",
            ValidationError::new("length", "The name must be at least 3 characters."),
        );
        bag.add(
            "email",
            ValidationError::new("email", "The email must be a valid email address."),
        );
        let body = bag.into_json_body();
        assert_eq!(body["message"], "The given data was invalid.");
        assert_eq!(
            body["errors"]["name"][0],
            "The name must be at least 3 characters."
        );
        assert_eq!(
            body["errors"]["email"][0],
            "The email must be a valid email address."
        );
    }

    /// An empty bag serializes to no `errors` keys.
    #[test]
    fn empty_bag_is_empty() {
        let bag = ErrorBag::new();
        assert!(bag.is_empty());
        assert!(!bag.has_errors());
    }
}
