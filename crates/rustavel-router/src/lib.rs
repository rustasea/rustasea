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
    /// Path parameters parsed from `{var}` / `{var:field}` segments.
    pub binding_fields: Vec<String>,
    /// Controller binding for routes registered via [`Router::resource`].
    pub controller: Option<ControllerRef>,
}

/// Controller reference carried by resource routes.
///
/// Routes produced by [`Router::resource`] remember the target controller so
/// a later controller-binding pass (FS-M1-01) can attach real handlers;
/// until then the router compiles stub handlers only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerRef {
    /// Registered controller name, e.g. `UserController`.
    pub name: String,
    /// Controller action this route dispatches to.
    pub action: String,
}

/// Expressive router builder with Laravel-inspired API.
pub struct Router {
    routes: Vec<RouteEntry>,
    prefix: String,
    name_prefix: String,
    pending_middleware: Vec<String>,
    pending_domain: Option<String>,
    pending_controller: Option<String>,
    layers: Vec<Box<dyn FnOnce(AxumRouter) -> AxumRouter + Send>>,
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
            pending_controller: None,
            layers: Vec::new(),
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
        // Expand to the six explicit methods — each entry stays introspectable
        // and binding-aware; a router that is never compiled to Axum still sees
        // concrete methods instead of a synthetic ANY catch-all.
        self.get(path)
            .post(path)
            .put(path)
            .delete(path)
            .patch(path)
            .options(path);
        self
    }

    /// Set the default controller for subsequent routes.
    ///
    /// Routes registered without an explicit action (plain method helpers)
    /// bind to `{controller}@handle`; resource routes bind per-action.
    pub fn controller(&mut self, controller: &str) -> &mut Self {
        self.pending_controller = Some(controller.to_string());
        self
    }

    /// Apply a tower layer to the compiled router.
    ///
    /// Bounds mirror `axum::Router::layer` — any tower layer whose service
    /// wraps axum's `Route` — so `CorsConfig::default().layer()` and
    /// `ThrottleLayer` compose directly:
    ///
    /// ```rust,ignore
    /// use rustavel_http::CorsConfig;
    /// route_table.layer(CorsConfig::default().layer());
    /// ```
    ///
    /// Layers apply in registration order after all routes are merged.
    pub fn layer<L>(&mut self, layer: L)
    where
        L: tower::Layer<axum::routing::Route> + Clone + Send + 'static,
        L::Service: tower::Service<axum::http::Request<axum::body::Body>> + Clone + Send + 'static,
        <L::Service as tower::Service<axum::http::Request<axum::body::Body>>>::Response:
            axum::response::IntoResponse + 'static,
        <L::Service as tower::Service<axum::http::Request<axum::body::Body>>>::Error:
            Into<std::convert::Infallible> + 'static,
        <L::Service as tower::Service<axum::http::Request<axum::body::Body>>>::Future:
            Send + 'static,
    {
        self.layers
            .push(Box::new(move |router: AxumRouter| router.layer(layer)));
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
            pending_controller: self.pending_controller.clone(),
            layers: Vec::new(),
        };
        f(&mut sub);
        self.routes.extend(sub.routes);
        self
    }

    /// Register a RESTful resource (7 routes) for a given name.
    ///
    /// `controller` is the class name resolved at bind time — plain
    /// `resource("photos")` defers to the route-group controller (see
    /// [`Router::controller`]). The `update` and `destroy` actions accept
    /// both `PUT` and `PATCH` (Laravel-style), so introspection yields the
    /// full request-method surface.
    pub fn resource(&mut self, name: &str, controller: &str) -> &mut Self {
        let resolved = if controller.is_empty() {
            self.pending_controller.clone().unwrap_or_default()
        } else {
            controller.to_string()
        };
        let base = format!("/{name}");
        let item = format!("/{name}/{{id}}");
        let create = format!("/{name}/create");
        let edit = format!("/{name}/{{id}}/edit");

        self.resource_route("GET", &base, name, "index", &resolved);
        self.resource_route("GET", &create, name, "create", &resolved);
        self.resource_route("POST", &base, name, "store", &resolved);
        self.resource_route("GET", &item, name, "show", &resolved);
        self.resource_route("GET", &edit, name, "edit", &resolved);
        self.resource_route("PUT", &item, name, "update", &resolved);
        self.resource_route("PATCH", &item, name, "update", &resolved);
        self.resource_route("DELETE", &item, name, "destroy", &resolved);
        self
    }

    /// Register a single named resource route with an optional controller.
    fn resource_route(
        &mut self,
        method: &str,
        path: &str,
        resource: &str,
        action: &str,
        controller: &str,
    ) {
        self.push_with_name(method, path, &format!("{resource}.{action}"));
        if let Some(entry) = self.routes.last_mut() {
            entry.controller = (!controller.is_empty()).then(|| ControllerRef {
                name: controller.to_string(),
                action: action.to_string(),
            });
        }
    }

    /// Return all registered routes, domain-constrained first.
    pub fn get_routes(&self) -> Vec<RouteEntry> {
        let mut routes = self.routes.clone();
        routes.sort_by_key(|r| !r.domain.is_some());
        routes
    }

    /// Alias for get_routes for Laravel naming parity.
    ///
    /// Non-snake-case by design: mirrors the Laravel `getRoutes` collector name
    /// so framework docs map 1:1 onto the Rust surface.
    #[allow(non_snake_case)]
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
            let overlaps = registered
                .iter()
                .any(|(m, p)| p == &entry.path && m == &entry.method);
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
                // Defensive: any() expands per-method, but keep a catch-all arm
                // so a legacy ANY entry still compiles into a router.
                _ => AxumRouter::new().route(&path, axum::routing::any(stub_handler)),
            };
            router = router.merge(r);
        }
        for apply in self.layers {
            router = apply(router);
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
            path: full.clone(),
            name: prefixed_name(&self.name_prefix, None),
            middleware: self.pending_middleware.clone(),
            domain: self.pending_domain.clone(),
            binding_fields: parse_binding_fields(&full),
            controller: self.pending_controller.as_ref().map(|c| ControllerRef {
                name: c.clone(),
                action: "handle".to_string(),
            }),
        });
    }

    fn push_with_name(&mut self, method: &str, path: &str, name: &str) {
        let full = join_prefix(&self.prefix, path);
        self.routes.push(RouteEntry {
            method: method.to_string(),
            path: full.clone(),
            name: prefixed_name(&self.name_prefix, Some(name)),
            middleware: self.pending_middleware.clone(),
            domain: self.pending_domain.clone(),
            binding_fields: parse_binding_fields(&full),
            controller: None,
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

/// Extract binding fields from an axum-style path.
///
/// Segments may be `{id}` or `{user:slug}`; the field after the colon wins
/// when present (Laravel-style route model binding). The scan is a simple
/// brace walk — deliberately regex-free — that never mis-parses an
/// already-normalized path.
fn parse_binding_fields(path: &str) -> Vec<String> {
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

    /// Resource expansion registers all seven actions across 8 method rows.
    #[test]
    fn resource_registers_seven_rest_routes() {
        let mut router = Router::new();
        router.resource("photos", "PhotoController");
        let routes = router.get_routes();
        let names: Vec<&str> = routes.iter().filter_map(|r| r.name.as_deref()).collect();
        assert_eq!(
            names,
            vec![
                "photos.index",
                "photos.create",
                "photos.store",
                "photos.show",
                "photos.edit",
                "photos.update",
                "photos.update",
                "photos.destroy",
            ]
        );
        let methods: Vec<&str> = routes.iter().map(|r| r.method.as_str()).collect();
        assert_eq!(
            methods,
            vec!["GET", "GET", "POST", "GET", "GET", "PUT", "PATCH", "DELETE"]
        );
        assert!(routes.iter().all(|r| r.controller.is_some()));
        let update = routes
            .iter()
            .find(|r| r.method == "PUT" && r.name.as_deref() == Some("photos.update"))
            .unwrap();
        assert_eq!(update.binding_fields, vec!["id".to_string()]);
        let destroy = routes.iter().find(|r| r.method == "DELETE").unwrap();
        assert_eq!(
            destroy.controller.as_ref().map(|c| c.action.as_str()),
            Some("destroy")
        );
    }

    /// any() expands to the six concrete HTTP methods.
    #[test]
    fn any_expands_to_all_methods() {
        let mut router = Router::new();
        router.any("/hook");
        let routes = router.get_routes();
        let methods: Vec<&str> = routes.iter().map(|r| r.method.as_str()).collect();
        assert_eq!(
            methods,
            vec!["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"]
        );
    }

    /// Plain method routes expose binding fields and controller default.
    #[test]
    fn plain_route_carries_bindings_and_controller() {
        let mut router = Router::new();
        router.controller("UserController").get("/users/{id}");
        let route = &router.get_routes()[0];
        assert_eq!(route.binding_fields, vec!["id".to_string()]);
        assert_eq!(
            route.controller.as_ref().map(|c| c.name.as_str()),
            Some("UserController")
        );
        assert_eq!(
            route.controller.as_ref().map(|c| c.action.as_str()),
            Some("handle")
        );
    }

    /// Domain-constrained routes sort ahead of catch-all duplicates.
    #[test]
    fn domain_routes_are_prioritized() {
        let mut router = Router::new();
        router.get("/home");
        router.domain("api.example.com").get("/home");
        let routes = router.get_routes();
        assert_eq!(routes[0].domain.as_deref(), Some("api.example.com"));
        assert_eq!(routes[1].domain, None);
    }
}
