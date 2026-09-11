//! Route metadata model — introspectable route entries and controller refs.
//!
//! Entries are pure data: they carry method, path, binding fields, middleware
//! and the optional controller/handler labels that `route:list` renders. The
//! executable handler lives separately in the router's bound-action registry.

use serde::{Deserialize, Serialize};

/// Single route definition with method, path, and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteEntry {
    /// HTTP method (GET, POST, etc.) or ANY.
    pub method: String,
    /// Normalized URI path.
    pub path: String,
    /// Optional route name.
    pub name: Option<String>,
    /// Middleware identifiers applied to this route.
    pub middleware: Vec<String>,
    /// Optional domain/host constraint.
    pub domain: Option<String>,
    /// Path parameters parsed from `{var}` / `{var:field}` segments.
    pub binding_fields: Vec<String>,
    /// Controller binding for routes registered via [`crate::Router::resource`].
    pub controller: Option<ControllerRef>,
    /// Resolved handler label when the route is bound to a real action.
    ///
    /// Populated from the handler's type path (e.g. `UserController::index`)
    /// so `route:list` can show the concrete dispatch target; `None` means the
    /// route still compiles to the stub handler.
    #[serde(default)]
    pub handler: Option<String>,
}

/// Controller reference carried by resource routes.
///
/// Routes produced by [`crate::Router::resource`] remember the target
/// controller so a later controller-binding pass can attach real handlers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerRef {
    /// Registered controller name, e.g. `UserController`.
    pub name: String,
    /// Controller action this route dispatches to.
    pub action: String,
}

/// Normalize a prefix to start with / and not end with /.
pub(crate) fn normalize_prefix(prefix: &str) -> String {
    if prefix.is_empty() || prefix == "/" {
        return String::new();
    }
    let mut p = prefix.to_string();
    if !p.starts_with('/') {
        p = format!("/{p}");
    }
    p.trim_end_matches('/').to_string()
}

/// Join prefix and path with slash normalization.
pub(crate) fn join_prefix(prefix: &str, path: &str) -> String {
    let p = if path == "/" {
        "/".to_string()
    } else {
        let mut s = path.to_string();
        if !s.starts_with('/') {
            s = format!("/{s}");
        }
        s
    };
    if prefix.is_empty() {
        return p;
    }
    if p == "/" {
        return prefix.to_string();
    }
    format!("{prefix}{p}")
}

/// Build prefixed route name.
pub(crate) fn prefixed_name(prefix: &str, name: Option<&str>) -> Option<String> {
    match (prefix.is_empty(), name) {
        (true, None) => None,
        (true, Some(n)) => Some(n.to_string()),
        (false, None) => None,
        (false, Some(n)) => Some(format!("{prefix}{n}")),
    }
}

/// Extract binding fields from an axum-style path.
///
/// Segments may be `{id}` or `{user:slug}`; the field after the colon wins
/// when present (Laravel-style route model binding). The scan is a simple
/// brace walk — deliberately regex-free — that never mis-parses an
/// already-normalized path.
pub(crate) fn parse_binding_fields(path: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut rest = path;
    while let Some(open) = rest.find('{') {
        let tail = &rest[open + 1..];
        let Some(close) = tail.find('}') else {
            break;
        };
        let inner = &tail[..close];
        if !inner.is_empty() {
            let binding = inner.split(':').next_back().unwrap_or(inner);
            fields.push(binding.to_string());
        }
        rest = &tail[close + 1..];
    }
    fields
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Binding fields parse from {var} and {var:field} segments.
    #[test]
    fn binding_fields_parse_simple_and_prefixed() {
        let fields = parse_binding_fields("/users/{user:slug}/posts/{post}");
        assert_eq!(fields, vec!["slug".to_string(), "post".to_string()]);
        assert!(parse_binding_fields("/users/create").is_empty());
        assert!(parse_binding_fields("/").is_empty());
    }
}
