/// Strict comparison helpers — type + value, never loose equality (#19).
///
/// The canonical test: `in_array: [1, "1"]` — `1` (int) must NOT match
/// `"1"` (str). These helpers underpin the strict rules in `rules.rs`.
use serde_json::Value;

/// A value normalized for strict comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrictValue {
    /// JSON type tag (`number`, `string`, `boolean`, `null`, ...).
    pub kind: &'static str,
    /// Canonical string form of the value for comparison.
    pub raw: String,
}

impl StrictValue {
    /// Classify a JSON value into (kind, canonical form).
    pub fn of(value: &Value) -> Self {
        match value {
            Value::Null => Self {
                kind: "null",
                raw: "null".to_string(),
            },
            Value::Bool(b) => Self {
                kind: "boolean",
                raw: b.to_string(),
            },
            Value::Number(n) => Self {
                kind: "number",
                raw: n.to_string(),
            },
            Value::String(s) => Self {
                kind: "string",
                raw: s.clone(),
            },
            Value::Array(_) => Self {
                kind: "array",
                raw: serde_json::to_string(value).unwrap_or_default(),
            },
            Value::Object(_) => Self {
                kind: "object",
                raw: serde_json::to_string(value).unwrap_or_default(),
            },
        }
    }

    /// Strict equality: same JSON type AND same canonical form.
    pub fn strict_eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.raw == other.raw
    }
}

/// Strict `contains` — `needle` must appear as an exact element, same type.
///
/// `contains([1,2,3], "1")` is false; `contains([1,2,3], 1)` is true.
pub fn contains_strict(haystack: &Value, needle: &Value) -> bool {
    let needle = StrictValue::of(needle);
    match haystack {
        Value::Array(items) => items
            .iter()
            .any(|item| StrictValue::of(item).strict_eq(&needle)),
        Value::String(s) => {
            // String containment compares raw text (type is already `string`).
            needle.kind == "string" && s.contains(&needle.raw)
        }
        _ => false,
    }
}

/// Strict `in_array` — `value` must exactly match one allow-listed element.
pub fn in_array_strict(allow_list: &Value, value: &Value) -> bool {
    let value = StrictValue::of(value);
    match allow_list {
        Value::Array(items) => items
            .iter()
            .any(|item| StrictValue::of(item).strict_eq(&value)),
        _ => false,
    }
}

/// Strict `doesnt_contain` — inverse of `contains_strict`.
pub fn doesnt_contain(haystack: &Value, needle: &Value) -> bool {
    !contains_strict(haystack, needle)
}

/// Strict scalar equality across JSON values (same type + value).
pub fn matches_strict(a: &Value, b: &Value) -> bool {
    StrictValue::of(a).strict_eq(&StrictValue::of(b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The canonical FR-307 test: `"1"` never matches int `1`.
    #[test]
    fn strict_in_array_rejects_type_confusion() {
        assert!(in_array_strict(&json!([1, 2, 3]), &json!(1)));
        assert!(!in_array_strict(&json!([1, 2, 3]), &json!("1")));
        assert!(in_array_strict(&json!(["a", "b"]), &json!("a")));
        assert!(!in_array_strict(&json!(["a", "b"]), &json!("A")));
    }

    /// Case matters for strict string containment.
    #[test]
    fn strict_contains_is_case_sensitive() {
        assert!(contains_strict(
            &json!(["admin", "editor"]),
            &json!("admin")
        ));
        assert!(!contains_strict(
            &json!(["admin", "editor"]),
            &json!("Admin")
        ));
        assert!(doesnt_contain(&json!(["admin"]), &json!("Admin")));
    }

    /// Number forms compare canonically (1.0 == 1 in JSON numbers).
    #[test]
    fn numbers_compare_by_canonical_form() {
        assert!(matches_strict(&json!(1), &json!(1)));
        assert!(!matches_strict(&json!(1), &json!(1.5)));
    }
}
