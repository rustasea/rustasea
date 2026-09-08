/// Validation rule declarations and the `Validatable` trait.
///
/// Rule strings mirror Laravel's validation grammar (`required`, `email`,
/// `min:3`, `max:255`, `in:a,b,c`, `contains:admin`, `doesnt_contain:x`).
/// Strict rules (`in_array`, `contains`, `doesnt_contain`) compare type AND
/// value — never loose equality (#19).
use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error_bag::{ErrorBag, ValidationError};
use crate::strict::{contains_strict, doesnt_contain, in_array_strict};

/// Declarative validation rule set for one payload.
#[derive(Debug, Clone, Default)]
pub struct Rules {
    /// field -> ordered rule strings.
    pub fields: HashMap<String, Vec<String>>,
}

impl Rules {
    /// Create an empty rule set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add rules for a field (comma/pipe separated, Laravel grammar).
    pub fn add(&mut self, field: impl Into<String>, rules: &str) -> &mut Self {
        let parsed = rules.split('|').map(|r| r.trim()).filter(|r| !r.is_empty());
        self.fields
            .entry(field.into())
            .or_default()
            .extend(parsed.map(String::from));
        self
    }

    /// Fluent builder alias (`Rules::new().field("name", "required|min:3")`).
    pub fn field(mut self, field: impl Into<String>, rules: &str) -> Self {
        self.add(field, rules);
        self
    }

    /// Validate one JSON payload against the declared rules.
    pub fn validate(&self, data: &Value) -> Result<(), ErrorBag> {
        let mut bag = ErrorBag::new();
        for (field, rules) in &self.fields {
            let value = data.get(field);
            for rule in rules {
                if let Err(err) = apply_rule(field, value, rule, data) {
                    bag.add(field.clone(), err);
                }
            }
        }
        if bag.is_empty() {
            Ok(())
        } else {
            Err(bag)
        }
    }
}

/// Apply a single rule string to a field value.
fn apply_rule(
    field: &str,
    value: Option<&Value>,
    rule: &str,
    _whole: &Value,
) -> std::result::Result<(), ValidationError> {
    let (name, arg) = match rule.split_once(':') {
        Some((n, a)) => (n, Some(a)),
        None => (rule, None),
    };

    // `required` needs the raw presence, not the dereferenced value.
    if name == "required" {
        return if value.is_none() || value == Some(&Value::Null) {
            Err(ValidationError::new(
                "required",
                format!("The {field} field is required."),
            ))
        } else {
            Ok(())
        };
    }

    let present = match value {
        Some(Value::Null) | None => {
            // Optional fields: skip non-required rules when absent/null.
            return Ok(());
        }
        Some(v) => v,
    };

    let label = humanize(field);
    match name {
        "email" => {
            use validator::ValidateEmail;
            let valid = present
                .as_str()
                .map(|s| s.validate_email())
                .unwrap_or(false);
            if valid {
                Ok(())
            } else {
                Err(ValidationError::new(
                    "email",
                    format!("The {label} must be a valid email address."),
                ))
            }
        }
        "min" => {
            let n = parse_arg::<u64>(arg).unwrap_or(0);
            let len = value_len(present);
            if len >= n {
                Ok(())
            } else {
                Err(ValidationError::new(
                    "min",
                    format!("The {label} must be at least {n} characters."),
                ))
            }
        }
        "max" => {
            let n = parse_arg::<u64>(arg).unwrap_or(u64::MAX);
            let len = value_len(present);
            if len <= n {
                Ok(())
            } else {
                Err(ValidationError::new(
                    "max",
                    format!("The {label} must not exceed {n} characters."),
                ))
            }
        }
        "in_array" | "in" => {
            let list = parse_list(arg);
            let allow = Value::Array(
                list.iter()
                    .filter_map(|s| serde_json::from_str(s).ok())
                    .collect(),
            );
            if in_array_strict(&allow, present) {
                Ok(())
            } else {
                Err(ValidationError::new(
                    "in_array",
                    format!("The selected {label} is invalid."),
                ))
            }
        }
        "contains" | "contains_strict" => {
            let needle = parse_needle(arg);
            if contains_strict(present, &needle) {
                Ok(())
            } else {
                Err(ValidationError::new(
                    "contains_strict",
                    format!("The {label} must contain {arg:?} strictly."),
                ))
            }
        }
        "doesnt_contain" => {
            let needle = parse_needle(arg);
            if doesnt_contain(present, &needle) {
                Ok(())
            } else {
                Err(ValidationError::new(
                    "doesnt_contain",
                    format!("The {label} must not contain {arg:?}."),
                ))
            }
        }
        _ => Err(ValidationError::new(
            "unknown_rule",
            format!("Unknown validation rule {rule:?}."),
        )),
    }
}

/// Length of a JSON value treated as text or collection.
fn value_len(v: &Value) -> u64 {
    match v {
        Value::String(s) => s.chars().count() as u64,
        Value::Array(a) => a.len() as u64,
        Value::Object(o) => o.len() as u64,
        Value::Number(n) => n.to_string().len() as u64,
        Value::Bool(_) | Value::Null => 0,
    }
}

/// Parse `min:3` / `max:255` arguments.
fn parse_arg<T: std::str::FromStr>(arg: Option<&str>) -> Option<T> {
    arg.and_then(|a| a.trim().parse().ok())
}

/// Parse `in:a,b,c` into individual string items.
fn parse_list(arg: Option<&str>) -> Vec<String> {
    arg.map(|a| {
        a.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    })
    .unwrap_or_default()
}

/// Parse a scalar needle from a rule argument (JSON if possible, else raw).
fn parse_needle(arg: Option<&str>) -> Value {
    match arg {
        Some(a) => serde_json::from_str(a).unwrap_or_else(|_| Value::String(a.to_string())),
        None => Value::Null,
    }
}

/// Field label for messages (snake_case -> words).
fn humanize(field: &str) -> String {
    field.replace(['_', '.'], " ").to_string()
}

/// Payload contract: types that can be validated.
///
/// Mirrors Laravel's `FormRequest` + `validator` derive: parse JSON into `T`
/// (via `DeserializeOwned`), then validate the resulting structure.
pub trait Validatable: DeserializeOwned {
    /// Validate `&self` and return `ErrorBag` on failure.
    fn validate(&self) -> std::result::Result<(), ErrorBag>;

    /// Validate a JSON value by deserializing then validating.
    fn validate_value(value: &Value) -> std::result::Result<Self, ErrorBag> {
        let parsed: Self = serde_json::from_value(value.clone())
            .map_err(|e| ErrorBag::from_message(e.to_string()))?;
        parsed.validate()?;
        Ok(parsed)
    }
}

impl ErrorBag {
    /// Build a bag from a single top-level message.
    pub fn from_message(message: impl Into<String>) -> Self {
        let mut bag = ErrorBag::new();
        bag.add("payload", ValidationError::new("parse", message));
        bag
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Required + min length enforce presence and size.
    #[test]
    fn required_and_min_length_rules() {
        let rules = Rules::new().field("name", "required|min:3").clone();
        assert!(rules.validate(&json!({ "name": "Ada" })).is_ok());
        assert!(rules.validate(&json!({ "name": "ab" })).is_err());
        assert!(rules.validate(&json!({})).is_err());
    }

    /// Strict `in` distinguishes int from string.
    #[test]
    fn strict_in_rule_rejects_string_int() {
        let rules = Rules::new().field("identifier", "in_array:1,2,3").clone();
        assert!(rules.validate(&json!({ "identifier": 1 })).is_ok());
        let bag = rules.validate(&json!({ "identifier": "1" })).unwrap_err();
        assert!(bag.get("identifier").iter().any(|e| e.code == "in_array"));
    }

    /// Optional fields skip rules when absent; required still fires.
    #[test]
    fn optional_absent_fields_pass() {
        let rules = Rules::new().field("role", "contains_strict:admin").clone();
        assert!(rules.validate(&json!({})).is_ok());
        assert!(rules.validate(&json!({ "role": null })).is_ok());
    }
}
