//! Table naming conventions.

/// Convert an English plural table name back to its singular form.
///
/// `categories` → `category`, `addresses` → `address`, `statuses` → `status`,
/// `posts` → `post`. Used for `belongs_to` FK derivation.
pub fn singular_from_plural(table_name: &str) -> String {
    if let Some(stem) = table_name.strip_suffix("ies") {
        return format!("{stem}y");
    }
    if let Some(stem) = table_name.strip_suffix("es") {
        return stem.to_string();
    }
    table_name
        .strip_suffix('s')
        .unwrap_or(table_name)
        .to_string()
}

/// Convert a type name to its `snake_plural` table name.
///
/// `User` → `users`, `FailedJob` → `failed_jobs`, `Address` → `addresses`,
/// `Category` → `categories`.
pub fn snake_plural(type_name: &str) -> String {
    let singular = to_snake_case(type_name);
    let suffix = plural_suffix(&singular);
    if suffix == "ies" {
        // Consonant + y: drop the trailing 'y' before appending "ies".
        let mut stem = singular;
        stem.pop();
        format!("{stem}ies")
    } else {
        format!("{singular}{suffix}")
    }
}

/// Convert `CamelCase` / `PascalCase` to `snake_case`.
pub fn to_snake_case(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 4);
    for (i, ch) in input.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
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
/// Handles the common Eloquent rules: `s`/`x`/`z`/`ch`/`sh` → `es`,
/// consonant+`y` → `ies`, otherwise `s`.
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
        let second_last = chars.next_back();
        if let Some(c) = second_last {
            if !"aeiou".contains(c) {
                return "ies";
            }
        }
    }
    "s"
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies snake_plural against the canonical Eloquent examples.
    #[test]
    fn converts_canonical_names() {
        assert_eq!(snake_plural("User"), "users");
        assert_eq!(snake_plural("Post"), "posts");
        assert_eq!(snake_plural("FailedJob"), "failed_jobs");
        assert_eq!(snake_plural("Address"), "addresses");
        assert_eq!(snake_plural("Category"), "categories");
        assert_eq!(snake_plural("Cache"), "caches");
    }

    /// Verifies singular_from_plural inverts the canonical plural rules.
    #[test]
    fn singularizes_plural_tables() {
        assert_eq!(singular_from_plural("categories"), "category");
        assert_eq!(singular_from_plural("addresses"), "address");
        assert_eq!(singular_from_plural("statuses"), "status");
        assert_eq!(singular_from_plural("posts"), "post");
        assert_eq!(singular_from_plural("users"), "user");
    }
}
