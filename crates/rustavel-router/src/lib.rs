//! Rustavel router — expressive Axum-backed routing with groups and domains.

use axum::Router as AxumRouter;
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
}

/// Expressive router builder with Laravel-inspired API.
pub struct Router {
    routes: Vec<RouteEntry>,
    prefix: String,
    name_prefix: String,
    pending_middleware: Vec<String>,
    pending_domain: Option<String>,
}

impl Router {
    /// Create a new empty router.
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            prefix: String::new(),
            name_prefix: String::new(),
            pending_middleware: Vec::new(),
            pending_domain: None,
        }
    }

    /// Append a URI prefix for subsequent routes and groups (concatenates with parent).
    pub fn prefix(&mut self, prefix: &str) -> &mut Self {
        let child = normalize_prefix(prefix);
        if !child.is_empty() {
            self.prefix = join_prefix(&self.prefix, &child);
        }
        self
    }

    /// Append a name prefix for subsequent routes (concatenates with parent).
    pub fn name(&mut self, name: &str) -> &mut Self {
        self.name_prefix = format!("{}{}", self.name_prefix, name);
        self
    }

    /// Set domain constraint for subsequent routes.
    pub fn domain(&mut self, domain: &str) -> &mut Self {
        self.pending_domain = Some(domain.to_string());
        self
    }

    /// Add middleware identifier for subsequent routes.
    pub fn middleware(&mut self, middleware: &str) -> &mut Self {
        self.pending_middleware.push(middleware.to_string());
        self
    }

    /// Register a GET route.
    pub fn get(&mut self, path: &str) -> &mut Self {
        self.push("GET", path);
        self
    }

    /// Register a POST route.
    pub fn post(&mut self, path: &str) -> &mut Self {
        self.push("POST", path);
        self
    }

    /// Register a PUT route.
    pub fn put(&mut self, path: &str) -> &mut Self {
        self.push("PUT", path);
        self
    }

    /// Register a DELETE route.
    pub fn delete(&mut self, path: &str) -> &mut Self {
        self.push("DELETE", path);
        self
    }

    /// Register a PATCH route.
    pub fn patch(&mut self, path: &str) -> &mut Self {
        self.push("PATCH", path);
        self
    }

    /// Register an OPTIONS route.
    pub fn options(&mut self, path: &str) -> &mut Self {
        self.push("OPTIONS", path);
        self
    }

    /// Register a route matching any HTTP method.
    pub fn any(&mut self, path: &str) -> &mut Self {
        self.push("ANY", path);
        self
    }

    /// Create a grouped sub-router sharing prefix, name, middleware, and domain.
    pub fn group<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(&mut Router),
    {
        let mut sub = Router {
            routes: Vec::new(),
            prefix: self.prefix.clone(),
            name_prefix: self.name_prefix.clone(),
            pending_middleware: self.pending_middleware.clone(),
            pending_domain: self.pending_domain.clone(),
        };
        f(&mut sub);
        self.routes.extend(sub.routes);
        self
    }

    /// Register a RESTful resource (7 routes) for a given name.
    pub fn resource(&mut self, name: &str, _controller: &str) -> &mut Self {
        let base = format!("/{name}");
        let item = format!("/{name}/{{id}}");
        let create = format!("/{name}/create");
        let edit = format!("/{name}/{{id}}/edit");
        self.push_with_name("GET", &base, &format!("{name}.index"));
        self.push_with_name("GET", &create, &format!("{name}.create"));
        self.push_with_name("POST", &base, &format!("{name}.store"));
        self.push_with_name("GET", &item, &format!("{name}.show"));
        self.push_with_name("GET", &edit, &format!("{name}.edit"));
        self.push_with_name("PUT", &item, &format!("{name}.update"));
        self.push_with_name("DELETE", &item, &format!("{name}.destroy"));
        self
    }

    /// Return all registered routes, domain-constrained first.
    pub fn get_routes(&self) -> Vec<RouteEntry> {
        let mut routes = self.routes.clone();
        routes.sort_by(|a, b| b.domain.is_some().cmp(&a.domain.is_some()));
        routes
    }

    /// Alias for get_routes for Laravel naming parity.
    pub fn getRoutes(&self) -> Vec<RouteEntry> {
        self.get_routes()
    }

    /// Convert into an Axum router with stub handlers.
    ///
    /// Iterates domain-first ordering via get_routes(); the first route to claim a
    /// method+path slot wins, so a domain-constrained route shadows an overlapping
    /// catch-all instead of triggering an Axum "Overlapping method route" panic.
    pub fn into_axum_router(self) -> AxumRouter {
        let mut router = AxumRouter::new();
        let mut registered: Vec<(String, String)> = Vec::new();
        for entry in self.get_routes() {
            let overlaps = registered.iter().any(|(m, p)| {
                p == &entry.path && (m == &entry.method || m == "ANY" || entry.method == "ANY")
            });
            if overlaps {
                continue;
            }
            registered.push((entry.method.clone(), entry.path.clone()));
            let path = entry.path.clone();
            let method = entry.method.clone();
            let r: AxumRouter = match method.as_str() {
                "GET" => AxumRouter::new().route(&path, axum::routing::get(stub_handler)),
                "POST" => AxumRouter::new().route(&path, axum::routing::post(stub_handler)),
                "PUT" => AxumRouter::new().route(&path, axum::routing::put(stub_handler)),
                "DELETE" => AxumRouter::new().route(&path, axum::routing::delete(stub_handler)),
                "PATCH" => AxumRouter::new().route(&path, axum::routing::patch(stub_handler)),
                "OPTIONS" => AxumRouter::new().route(&path, axum::routing::options(stub_handler)),
                _ => AxumRouter::new().route(&path, axum::routing::any(stub_handler)),
            };
            router = router.merge(r);
        }
        router
    }

    /// Convert into Axum router (snake_case alias).
    pub fn into_axum_router_owned(self) -> AxumRouter {
        self.into_axum_router()
    }

    fn push(&mut self, method: &str, path: &str) {
        let full = join_prefix(&self.prefix, path);
        self.routes.push(RouteEntry {
            method: method.to_string(),
            path: full,
            name: prefixed_name(&self.name_prefix, None),
            middleware: self.pending_middleware.clone(),
            domain: self.pending_domain.clone(),
        });
    }

    fn push_with_name(&mut self, method: &str, path: &str, name: &str) {
        let full = join_prefix(&self.prefix, path);
        self.routes.push(RouteEntry {
            method: method.to_string(),
            path: full,
            name: prefixed_name(&self.name_prefix, Some(name)),
            middleware: self.pending_middleware.clone(),
            domain: self.pending_domain.clone(),
        });
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

/// Stub handler for route registration.
async fn stub_handler() -> &'static str {
    "ok"
}

/// Normalize a prefix to start with / and not end with /.
fn normalize_prefix(prefix: &str) -> String {
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
fn join_prefix(prefix: &str, path: &str) -> String {
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
fn prefixed_name(prefix: &str, name: Option<&str>) -> Option<String> {
    match (prefix.is_empty(), name) {
        (true, None) => None,
        (true, Some(n)) => Some(n.to_string()),
        (false, None) => None,
        (false, Some(n)) => Some(format!("{prefix}{n}")),
    }
}
