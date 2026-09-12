//! The Inertia page envelope shared by the server and the WASM client.

use serde::{Deserialize, Serialize};

/// Inertia page envelope sent to the client.
///
/// `T` is the typed props value at construction. At the wire boundary the
/// server serializes props to a JSON object (`T = serde_json::Value`) so that
/// partial reloads can filter by prop name — typed structs are used for
/// construction, not for partial filtering (ADR-0002 §7).
///
/// ```rust
/// use rustasea_inertia::Page;
/// use serde_json::json;
///
/// let page = Page::new(
///     "auth/login",
///     json!({ "email": "a@b.c" }),
///     "/login",
///     "v1",
/// );
/// assert_eq!(page.component, "auth/login");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page<T> {
    /// Client component key, e.g. `"auth/login"`.
    pub component: String,
    /// Page props (typed at construction, a JSON object on the wire).
    pub props: T,
    /// Current request URL (path + query).
    pub url: String,
    /// Asset version for cache-busting and the `409` redirect.
    pub version: String,
}

impl<T> Page<T> {
    /// Create a page envelope from its four parts.
    pub fn new(
        component: impl Into<String>,
        props: T,
        url: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            component: component.into(),
            props,
            url: url.into(),
            version: version.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The page serializes to the four-key Inertia wire shape.
    #[test]
    fn serializes_to_the_wire_shape() {
        let page = Page::new("dashboard", json!({ "user": "Ada" }), "/dashboard", "v1");
        let value = serde_json::to_value(&page).expect("serialize page");

        assert_eq!(value["component"], "dashboard");
        assert_eq!(value["props"]["user"], "Ada");
        assert_eq!(value["url"], "/dashboard");
        assert_eq!(value["version"], "v1");
    }

    /// The page round-trips through JSON without loss.
    #[test]
    fn round_trips_through_json() {
        let page = Page::new("dashboard", json!({ "user": "Ada" }), "/dashboard", "v1");
        let encoded = serde_json::to_string(&page).expect("encode");
        let decoded: Page<serde_json::Value> = serde_json::from_str(&encoded).expect("decode");

        assert_eq!(decoded, page);
    }
}
