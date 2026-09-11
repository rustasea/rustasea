//! Expressive router builder — Laravel-inspired registration DSL.
//!
//! The builder accumulates introspectable [`RouteEntry`] metadata and, for
//! routes registered with a concrete action, the executable handler factory.
//! Compilation into an axum router lives in [`crate::dispatch`].

use axum::Router as AxumRouter;

use crate::handler::{action_factory, ActionFactory, BoundAction, Handler};
use crate::route::{
    join_prefix, normalize_prefix, parse_binding_fields, prefixed_name, ControllerRef, RouteEntry,
};

/// Expressive router builder with Laravel-inspired API.
pub struct Router {
    routes: Vec<RouteEntry>,
    prefix: String,
    name_prefix: String,
    pending_middleware: Vec<String>,
    pending_domain: Option<String>,
    pending_controller: Option<String>,
    pub(crate) layers: Vec<Box<dyn FnOnce(AxumRouter) -> AxumRouter + Send>>,
    /// Routes explicitly bound to a concrete controller action.
    pub(crate) actions: Vec<BoundAction>,
    /// Controller actions keyed by `(controller, action)` for resource dispatch.
    pub(crate) controller_actions: Vec<(String, String, ActionFactory)>,
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
            actions: Vec::new(),
            controller_actions: Vec::new(),
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

    /// Register a GET route bound to a real controller action.
    ///
    /// The equivalent of Laravel's `Route::get("/users", [UserController,
    /// "index"])`: `UserController::index` is dispatched for `GET /users`
    /// with axum performing path/query/body extraction.
    pub fn get_action<H, T>(&mut self, path: &str, handler: H) -> &mut Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.push_action("GET", path, handler)
    }

    /// Register a POST route bound to a real controller action.
    pub fn post_action<H, T>(&mut self, path: &str, handler: H) -> &mut Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.push_action("POST", path, handler)
    }

    /// Register a PUT route bound to a real controller action.
    pub fn put_action<H, T>(&mut self, path: &str, handler: H) -> &mut Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.push_action("PUT", path, handler)
    }

    /// Register a DELETE route bound to a real controller action.
    pub fn delete_action<H, T>(&mut self, path: &str, handler: H) -> &mut Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.push_action("DELETE", path, handler)
    }

    /// Register a PATCH route bound to a real controller action.
    pub fn patch_action<H, T>(&mut self, path: &str, handler: H) -> &mut Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.push_action("PATCH", path, handler)
    }

    /// Register an OPTIONS route bound to a real controller action.
    pub fn options_action<H, T>(&mut self, path: &str, handler: H) -> &mut Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.push_action("OPTIONS", path, handler)
    }

    /// Register a route bound to a real controller action for any method.
    ///
    /// Mirrors [`Router::any`] — the action is cloned across the six concrete
    /// methods so every entry stays introspectable.
    pub fn any_action<H, T>(&mut self, path: &str, handler: H) -> &mut Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        for method in ["GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS"] {
            self.push_action(method, path, handler.clone());
        }
        self
    }

    /// Register a route bound to a real action for an explicit HTTP method.
    pub fn action<H, T>(&mut self, method: &str, path: &str, handler: H) -> &mut Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.push_action(method, path, handler)
    }

    /// Register a route from `#[route]` metadata plus a real controller action.
    ///
    /// `meta` is the doc-hidden `__RUSTASEA_ROUTE_<Fn>` tuple emitted by the
    /// `#[route]` attribute, so the macro's method + path are consumed at
    /// registration instead of being read only by tests.
    pub fn route_meta<H, T>(&mut self, meta: (&str, &str), handler: H) -> &mut Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.push_action(meta.0, meta.1, handler)
    }

    /// Register a controller action resolvable by `{controller}@{action}`.
    ///
    /// The controller name comes from the preceding [`Router::controller`]
    /// call, so `resource("users", "UserController")` dispatches to the action
    /// registered here for `(UserController, index)`, `(UserController, show)`,
    /// etc.
    pub fn controller_action<H, T>(&mut self, action: &str, handler: H) -> &mut Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        let controller = self.pending_controller.clone().unwrap_or_default();
        self.controller_action_for(&controller, action, handler)
    }

    /// Register a controller action under an explicit controller name.
    pub fn controller_action_for<H, T>(
        &mut self,
        controller: &str,
        action: &str,
        handler: H,
    ) -> &mut Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.controller_actions.push((
            controller.to_string(),
            action.to_string(),
            action_factory(handler),
        ));
        self
    }

    /// Apply a tower layer to the compiled router.
    ///
    /// Bounds mirror `axum::Router::layer` — any tower layer whose service
    /// wraps axum's `Route` — so `CorsConfig::default().layer()` and
    /// `ThrottleLayer` compose directly:
    ///
    /// ```rust,ignore
    /// use rustasea_http::CorsConfig;
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
            actions: Vec::new(),
            controller_actions: Vec::new(),
        };
        f(&mut sub);
        self.routes.extend(sub.routes);
        self.actions.extend(sub.actions);
        self.controller_actions.extend(sub.controller_actions);
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

    /// Convert into Axum router (snake_case alias).
    pub fn into_axum_router_owned(self) -> AxumRouter {
        self.into_axum_router()
    }

    /// Push an action-bound route and record its executable method router.
    fn push_action<H, T>(&mut self, method: &str, path: &str, handler: H) -> &mut Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.push(method, path);
        let label = std::any::type_name::<H>().to_string();
        if let Some(entry) = self.routes.last_mut() {
            // An explicit action is authoritative — drop the pending controller
            // ref so resolution does not fall back to `{controller}@handle`.
            entry.controller = None;
            entry.handler = Some(label);
        }
        let full = self
            .routes
            .last()
            .expect("push always appends a route entry")
            .path
            .clone();
        let router = handler.into_method_router(method);
        self.actions.push(BoundAction {
            method: method.to_ascii_uppercase(),
            path: full,
            domain: self.pending_domain.clone(),
            router,
        });
        self
    }

    fn push(&mut self, method: &str, path: &str) {
        let full = join_prefix(&self.prefix, path);
        self.routes.push(RouteEntry {
            method: method.to_ascii_uppercase(),
            path: full.clone(),
            name: prefixed_name(&self.name_prefix, None),
            middleware: self.pending_middleware.clone(),
            domain: self.pending_domain.clone(),
            binding_fields: parse_binding_fields(&full),
            controller: self.pending_controller.as_ref().map(|c| ControllerRef {
                name: c.clone(),
                action: "handle".to_string(),
            }),
            handler: None,
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
            handler: None,
        });
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}
