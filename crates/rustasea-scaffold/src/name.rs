//! Application-name normalization.
//!
//! A single user-supplied token (`my-app`, `my_app`, or `MyApp`) drives three
//! derived forms used by the templates: a kebab-case directory/crate name, a
//! snake_case Rust module name, and a PascalCase Rust type prefix.

use crate::error::{ScaffoldError, ScaffoldResult};

/// The three normalized forms of a requested application name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppName {
    /// Kebab-case form (`my-app`) — directory and crate name.
    pub kebab: String,
    /// Snake-case form (`my_app`) — Rust module/package identifiers.
    pub snake: String,
    /// PascalCase form (`MyApp`) — generated Rust type names.
    pub pascal: String,
}

impl AppName {
    /// Parse and normalize a user-supplied application name.
    ///
    /// Rejects empty input, input that does not start with an ASCII letter, and
    /// input containing characters outside `[A-Za-z0-9_-]`.
    pub fn parse(input: &str) -> ScaffoldResult<Self> {
        let trimmed = input.trim();
        if !Self::is_valid(trimmed) {
            return Err(ScaffoldError::InvalidAppName {
                name: input.to_string(),
            });
        }
        let words = Self::words(trimmed);
        // `is_valid` guarantees at least one word and an alphabetic first char.
        let kebab = words.join("-");
        let snake = words.join("_");
        let pascal = words
            .iter()
            .map(|w| {
                let mut chars = w.chars();
                match chars.next() {
                    Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect::<String>();
        Ok(Self {
            kebab,
            snake,
            pascal,
        })
    }

    /// Whether `name` is a non-empty identifier starting with an ASCII letter.
    fn is_valid(name: &str) -> bool {
        let mut chars = name.chars();
        let Some(first) = chars.next() else {
            return false;
        };
        if !first.is_ascii_alphabetic() {
            return false;
        }
        chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    }

    /// Split a name into lowercase words on separators and camelCase boundaries.
    fn words(name: &str) -> Vec<String> {
        let mut words: Vec<String> = Vec::new();
        let mut current = String::new();
        let mut prev_lower_or_digit = false;
        for ch in name.chars() {
            if ch == '-' || ch == '_' {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
                prev_lower_or_digit = false;
                continue;
            }
            if ch.is_ascii_uppercase() && prev_lower_or_digit && !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            current.push(ch.to_ascii_lowercase());
            prev_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        }
        if !current.is_empty() {
            words.push(current);
        }
        words
    }
}

#[cfg(test)]
mod tests {
    use super::AppName;

    #[test]
    fn normalizes_kebab_snake_and_pascal() {
        let name = AppName::parse("my-cool_app").expect("valid name");
        assert_eq!(name.kebab, "my-cool-app");
        assert_eq!(name.snake, "my_cool_app");
        assert_eq!(name.pascal, "MyCoolApp");
    }

    #[test]
    fn splits_camel_case() {
        let name = AppName::parse("MyApp").expect("valid name");
        assert_eq!(name.kebab, "my-app");
        assert_eq!(name.snake, "my_app");
        assert_eq!(name.pascal, "MyApp");
    }

    #[test]
    fn rejects_leading_digit_and_symbols() {
        assert!(AppName::parse("2fast").is_err());
        assert!(AppName::parse("bad name").is_err());
        assert!(AppName::parse("").is_err());
    }
}
