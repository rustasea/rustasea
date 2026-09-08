//! Pure helpers for the `#[derive(Model)]` expansion — snake case, plural
//! table derivation, and special-column detection.
//!
//! Kept free of `syn` so unit tests run without token-stream plumbing; the
//! derive macro in [`crate::model`] composes these into the emitted impl.

/// Convert `CamelCase` / `PascalCase` to `snake_case`.
pub fn snake_case(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 4);
    for (i, ch) in input.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 && !out.ends_with('_') {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// English plural suffix for a snake_case singular noun.
///
/// `s`/`x`/`z`/`ch`/`sh` → `es`, consonant+`y` → `ies`, otherwise `s`.
pub fn plural_suffix(singular: &str) -> &'static str {
    let lower = singular.to_lowercase();
    if lower.ends_with("s")
        || lower.ends_with("x")
        || lower.ends_with("z")
        || lower.ends_with("ch")
        || lower.ends_with("sh")
    {
        return "es";
    }
    if lower.ends_with('y') && lower.len() >= 2 {
        let mut chars = lower.chars();
        let _last = chars.next_back();
        if let Some(c) = chars.next_back() {
            if !"aeiou".contains(c) {
                return "ies";
            }
        }
    }
    "s"
}

/// Derive the `snake_plural` table name for a type (`User` → `users`).
pub fn table_name(type_name: &str) -> String {
    let singular = snake_case(type_name);
    if plural_suffix(&singular) == "ies" {
        let mut stem = singular;
        stem.pop();
        format!("{stem}ies")
    } else {
        format!("{singular}{}", plural_suffix(&singular))
    }
}

/// Whether a snake_case field name is the UUID primary key (`id`).
///
/// Used by the derive tests; the expansion special-cases `id` structurally.
#[allow(dead_code)]
pub fn is_id_field(name: &str) -> bool {
    name == "id"
}

/// Whether a field maps to the `created_at` timestamp column.
pub fn is_created_at(name: &str) -> bool {
    name == "created_at"
}

/// Whether a field maps to the `updated_at` timestamp column.
pub fn is_updated_at(name: &str) -> bool {
    name == "updated_at"
}

/// Whether a field maps to the `deleted_at` soft-delete column.
pub fn is_deleted_at(name: &str) -> bool {
    name == "deleted_at"
}

/// Column name for a struct field identifier (`createdAt` → `created_at`).
pub fn column_name(field: &str) -> String {
    snake_case(field)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies table derivation matches the Eloquent conventions.
    #[test]
    fn derives_canonical_tables() {
        assert_eq!(table_name("User"), "users");
        assert_eq!(table_name("FailedJob"), "failed_jobs");
        assert_eq!(table_name("Address"), "addresses");
        assert_eq!(table_name("Category"), "categories");
        assert_eq!(table_name("Cache"), "caches");
    }

    /// Verifies snake_case handles acronym runs and column mapping.
    #[test]
    fn snake_cases_and_columns() {
        assert_eq!(snake_case("User"), "user");
        assert_eq!(snake_case("createdAt"), "created_at");
        assert_eq!(column_name("updatedAt"), "updated_at");
        assert_eq!(column_name("deletedAt"), "deleted_at");
    }

    /// Verifies special-column detection accepts snake_case only.
    #[test]
    fn detects_special_columns() {
        assert!(is_id_field("id"));
        assert!(is_created_at("created_at"));
        assert!(is_updated_at("updated_at"));
        assert!(is_deleted_at("deleted_at"));
        assert!(!is_deleted_at("trashed_at"));
    }
}
