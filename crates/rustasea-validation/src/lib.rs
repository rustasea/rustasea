/// Validation crate for rustasea-validation.
///
/// RustaSea validation layers strict, type+value rules (`in_array`,
/// `contains`, `doesnt_contain`) over the `validator` crate and groups
/// failures into an `ErrorBag` keyed by field. The `FormRequest` trait
/// gives handlers a Laravel-style `validated()` payload.
pub mod error_bag;
pub mod form_request;
pub mod rules;
pub mod strict;

pub use error_bag::{ErrorBag, ValidationError};
pub use form_request::FormRequest;
pub use rules::{to_value, Rules, Validatable};
pub use strict::{contains_strict, doesnt_contain, in_array_strict, matches_strict, StrictValue};

/// Re-exported JSON crate so downstream apps can build payload values without
/// declaring `serde_json` themselves.
pub use serde_json;
