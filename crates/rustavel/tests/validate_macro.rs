//! Integration tests for the `#[validate]` attribute macro (FS-M3-05).
//!
//! The macro re-emits a struct with a `Validatable` impl that runs the
//! per-field rules declared via `#[validate("rule|rule")]` and returns an
//! `ErrorBag` aggregating every field failure (FR-308).

use rustavel::macros::validate;
use rustavel::validation::{ErrorBag, Rules, Validatable};
use serde::Deserialize;

/// Payload mirroring the api-validation.md `POST /users` contract.
#[validate]
#[derive(Debug, Deserialize, serde::Serialize)]
struct CreateUser {
    #[validate("required|min:3")]
    name: String,
    #[validate("required|email")]
    email: String,
    #[validate("contains_strict:admin")]
    role: String,
}

/// A valid payload passes validation.
#[test]
fn valid_payload_passes() {
    let payload = CreateUser {
        name: "Ada".to_string(),
        email: "ada@example.com".to_string(),
        role: "admin".to_string(),
    };
    assert!(payload.validate().is_ok());
}

/// Multiple field failures aggregate into one ErrorBag (dual-field 422).
#[test]
fn invalid_payload_aggregates_errors_per_field() {
    let payload = CreateUser {
        name: "ab".to_string(),
        email: "not-an-email".to_string(),
        role: "Admin".to_string(),
    };
    let bag = payload.validate().expect_err("invalid payload errors");
    assert!(bag.get("name").iter().any(|e| e.code == "min"));
    assert!(bag.get("email").iter().any(|e| e.code == "email"));
    assert!(bag.get("role").iter().any(|e| e.code == "contains_strict"));
    // Field-level rules carry human messages that serialize into the 422 body.
    let body = bag.into_json_body();
    assert_eq!(body["message"], "The given data was invalid.");
    assert!(body["errors"]["name"][0]
        .as_str()
        .expect("message is a string")
        .contains("at least 3"));
    assert!(body["errors"]["email"][0]
        .as_str()
        .expect("message is a string")
        .contains("valid email"));
}

/// The macro supports single name-value rules (`contains_strict="admin"`).
#[test]
fn name_value_rule_form_supported() {
    #[validate]
    #[derive(Debug, Deserialize, serde::Serialize)]
    struct RolePayload {
        #[validate(contains_strict = "admin")]
        role: String,
    }
    let ok = RolePayload {
        role: "admin".to_string(),
    };
    assert!(ok.validate().is_ok());
    let bad = RolePayload {
        role: "Admin".to_string(),
    };
    let bag = bad.validate().expect_err("strict mismatch fails");
    assert!(bag.get("role").iter().any(|e| e.code == "contains_strict"));
}

/// Strict `in_array` via Rules distinguishes int from string ("1" != 1).
#[test]
fn strict_in_array_semantics_hold() {
    let rules = Rules::new().field("identifier", "in_array:1,2,3");
    assert!(rules
        .validate(&serde_json::json!({ "identifier": 1 }))
        .is_ok());
    let bag = rules
        .validate(&serde_json::json!({ "identifier": "1" }))
        .expect_err("string 1 rejected");
    assert!(bag.get("identifier").iter().any(|e| e.code == "in_array"));
}

/// The macro-generated impl honors the Validatable value path (FR-309).
#[test]
fn validate_value_parses_then_validates() {
    let value = serde_json::json!({
        "name": "Ada",
        "email": "ada@example.com",
        "role": "admin"
    });
    let parsed = CreateUser::validate_value(&value).expect("valid JSON parses");
    assert_eq!(parsed.name, "Ada");

    let bad = serde_json::json!({
        "name": "ab",
        "email": "nope",
        "role": "editor"
    });
    let bag: ErrorBag = CreateUser::validate_value(&bad).expect_err("invalid payload");
    assert_eq!(bag.field_count(), 3);
}

/// An optional field is skipped when absent; `required` still fires.
#[test]
fn optional_absent_fields_pass_via_macro() {
    #[validate]
    #[derive(Debug, Deserialize, serde::Serialize)]
    struct WithOption {
        #[validate("required")]
        name: String,
        #[validate("contains_strict:admin")]
        role: Option<String>,
    }
    let present = WithOption {
        name: "Ada".to_string(),
        role: Some("admin".to_string()),
    };
    assert!(present.validate().is_ok());
    // Deserializing a missing optional yields None -> rule skipped.
    let payload = WithOption {
        name: "Ada".to_string(),
        role: None,
    };
    assert!(payload.validate().is_ok());
}
