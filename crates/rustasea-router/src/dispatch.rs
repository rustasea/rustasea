//! Axum compilation — resolve bound actions and emit a dispatchable router.

use axum::routing::MethodRouter;
use axum::Router as AxumRouter;

use crate::handler::{stub_handler, ActionFactory, BoundAction, Handler};
use crate::route::RouteEntry;
use crate::router::Router;

impl Router {
    /// Convert into an Axum router that dispatches to real controller actions.
    ///
    /// Iterates domain-first ordering via [`Router::get_routes`]; the first
    /// route to claim a method+path slot wins, so a domain-constrained route
    /// shadows an overlapping catch-all instead of triggering an Axum
    /// "Overlapping method route" panic. Each slot resolves to an explicitly
    /// bound action, a controller-registry action, or the stub handler.
    ///
    /// Slot bookkeeping stays on the Laravel-style [`RouteEntry::path`], while
    /// the string handed to `axum::Router::route` is translated by
    /// [`to_axum_path`] — Axum 0.7 (matchit 0.7) understands `:param`, not
    /// `{param}`.
    pub fn into_axum_router(self) -> AxumRouter {
        let entries = self.get_routes();
        let mut actions = self.actions;
        let controller_actions = self.controller_actions;
        let layers = self.layers;

        let mut router = AxumRouter::new();
        let mut registered: Vec<(String, String)> = Vec::new();
        for entry in entries {
            let overlaps = registered
                .iter()
                .any(|(m, p)| p == &entry.path && m == &entry.method);
            if overlaps {
                continue;
            }
            registered.push((entry.method.clone(), entry.path.clone()));
            let method_router = resolve(&entry, &mut actions, &controller_actions);
            let axum_path = to_axum_path(&entry.path);
            router = router.merge(AxumRouter::new().route(&axum_path, method_router));
        }
        for apply in layers {
            router = apply(router);
        }
        router
    }
}

/// Resolve the executable method router for a route entry.
///
/// Precedence: an explicit action bound to the same method+path+domain, then a
/// controller action registered for the entry's `{controller}@{action}` ref,
/// then the stub handler. Resolution never fails — an unbound route still
/// compiles so partial route tables remain servable.
///
/// A route only consumes a bound action when it is itself bound
/// (`entry.handler.is_some()`) and the action's domain matches the entry's
/// domain exactly. This prevents a domain-constrained entry, sorted ahead of
/// the catch-all it overlaps, from stealing the catch-all's action, and stops
/// an unbound route from consuming an action bound to another route.
fn resolve(
    entry: &RouteEntry,
    actions: &mut Vec<BoundAction>,
    controller_actions: &[(String, String, ActionFactory)],
) -> MethodRouter<()> {
    if entry.handler.is_some() {
        let bound = actions.iter().position(|a| {
            a.method == entry.method && a.path == entry.path && a.domain == entry.domain
        });
        if let Some(index) = bound {
            return actions.swap_remove(index).router;
        }
    }
    if let Some(controller) = &entry.controller {
        let found = controller_actions
            .iter()
            .find(|(name, action, _)| name == &controller.name && action == &controller.action);
        if let Some((_, _, factory)) = found {
            return factory(&entry.method);
        }
    }
    stub_handler.into_method_router(&entry.method)
}

/// Translate a Laravel-style route path into Axum (matchit) syntax.
///
/// Laravel placeholders become Axum parameters: `{id}` → `:id` and the
/// route-model-bound `{user:slug}` → `:user` (the `:slug` selector is a
/// Laravel-only model-binding hint and is dropped from the match pattern).
/// Text outside placeholders — including paths already written in `:param`
/// form — passes through verbatim, so the translation is idempotent and the
/// introspectable [`RouteEntry::path`] stays the single source of truth. An
/// unterminated `{` is emitted unchanged rather than silently swallowed.
fn to_axum_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut rest = path;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let tail = &rest[open + 1..];
        let Some(close) = tail.find('}') else {
            // Unterminated placeholder — emit the remainder verbatim.
            out.push_str(&rest[open..]);
            return out;
        };
        let inner = &tail[..close];
        let param = inner.split(':').next().unwrap_or(inner);
        if !param.is_empty() {
            out.push(':');
            out.push_str(param);
        }
        rest = &tail[close + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::to_axum_path;

    /// Simple, typed, and multi-segment placeholders translate to `:param`.
    #[test]
    fn to_axum_path_translates_laravel_placeholders() {
        assert_eq!(to_axum_path("/users/{id}"), "/users/:id");
        assert_eq!(to_axum_path("/users/{user:slug}"), "/users/:user");
        assert_eq!(
            to_axum_path("/teams/{team}/members/{member:slug}"),
            "/teams/:team/members/:member"
        );
    }

    /// Paths without placeholders — and already-normalized `:param` paths —
    /// pass through unchanged; an unterminated brace is not swallowed.
    #[test]
    fn to_axum_path_leaves_plain_and_normalized_paths_untouched() {
        assert_eq!(to_axum_path("/"), "/");
        assert_eq!(to_axum_path("/users/create"), "/users/create");
        assert_eq!(to_axum_path("/users/:id/edit"), "/users/:id/edit");
        assert_eq!(to_axum_path("/users/{id"), "/users/{id");
    }
}
