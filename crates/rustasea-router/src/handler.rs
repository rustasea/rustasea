//! Controller-action dispatch primitives — the [`Handler`] adapter.
//!
//! A [`Handler`] erases the extractor tuple `T` behind a trait object while
//! delegating actual extraction to axum, so controller methods, free async
//! fns, and closures all bind through one typed path. [`BoundAction`] pairs a
//! pre-built axum method router with its introspectable label.

use axum::handler::Handler as AxumHandler;
use axum::routing::{any, delete, get, options, patch, post, put, MethodRouter};

/// Adapter binding a controller action to an HTTP method router.
///
/// Implemented for every function, `async fn`, or closure axum accepts as a
/// handler for the unit state, so `UserController::index` and free functions
/// bind directly. The extractor tuple `T` (path/query/body/state) is erased at
/// this boundary; axum still performs typed extraction when dispatching.
pub trait Handler<T = (), S = ()>: Clone + Send + Sized + 'static {
    /// Build the method router that dispatches this action for `method`.
    ///
    /// `method` is matched case-insensitively after ASCII uppercasing, so a
    /// lowercase verb from `Router::action` or `#[route]` still maps to its
    /// concrete method router; an unknown verb falls back to
    /// [`axum::routing::any`].
    fn into_method_router(self, method: &str) -> MethodRouter<S>;
}

impl<H, T, S> Handler<T, S> for H
where
    H: AxumHandler<T, S>,
    T: 'static,
    S: Clone + Send + Sync + 'static,
{
    /// See [`Handler::into_method_router`].
    fn into_method_router(self, method: &str) -> MethodRouter<S> {
        match method.to_ascii_uppercase().as_str() {
            "GET" => get(self),
            "POST" => post(self),
            "PUT" => put(self),
            "DELETE" => delete(self),
            "PATCH" => patch(self),
            "OPTIONS" => options(self),
            _ => any(self),
        }
    }
}

/// Replayable factory that builds a method router for a controller action.
///
/// A registered action is resolved for every HTTP verb its resource route
/// declares (e.g. `update` for both `PUT` and `PATCH`); the handler is cloned
/// per resolution. Only [`Send`] is required — axum handlers are `Send` but not
/// necessarily `Sync`, and the router resolves factories sequentially.
pub(crate) type ActionFactory = Box<dyn Fn(&str) -> MethodRouter<()> + Send>;

/// Build an [`ActionFactory`] from a concrete controller action.
pub(crate) fn action_factory<H, T>(handler: H) -> ActionFactory
where
    H: Handler<T, ()>,
    T: 'static,
{
    Box::new(move |method: &str| handler.clone().into_method_router(method))
}

/// A route already bound to a concrete controller action.
pub(crate) struct BoundAction {
    /// HTTP verb the action was registered for.
    pub(crate) method: String,
    /// Normalized route path the action is bound to.
    pub(crate) path: String,
    /// Domain/host constraint captured at registration; `None` is a catch-all.
    ///
    /// Kept alongside `method`/`path` so resolution matches the action to the
    /// exact route entry it was registered for, not merely the first route that
    /// happens to share a method+path after domain-first sorting.
    pub(crate) domain: Option<String>,
    /// Pre-built axum method router performing the dispatch.
    pub(crate) router: MethodRouter<()>,
}

/// Fallback handler used when a route has no bound controller action.
pub(crate) async fn stub_handler() -> &'static str {
    "ok"
}
