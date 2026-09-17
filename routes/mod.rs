//! Route tables — `web`, `auth`, `settings`, and `console`.
//!
//! Each table registers into one shared [`Router`] via its `register` function;
//! [`router`] then compiles the DSL table to an axum router and threads the
//! shared [`AppState`] through it.

pub mod auth;
// `console` exports CLI command metadata (the `cargo artisan` registry), not
// HTTP routes — it has no `register(&mut Router)` and is deliberately never
// called from [`router`]. It lives here so the module tree mirrors the kit's
// `routes/` directory.
pub mod console;
pub mod settings;
pub mod web;

use std::sync::Arc;

use rustasea::http::AppState;
use rustasea::router::Router;

/// Build the application router from every generated route table.
///
/// The shared [`AppState`] is created once during boot in `main` and threaded
/// in here, so every route table serves the same state instead of each
/// constructing a disconnected one.
pub fn router(state: Arc<AppState>) -> axum::Router {
    let mut table = Router::new();
    register_placeholder_middleware(&mut table);
    web::register(&mut table);
    auth::register(&mut table);
    settings::register(&mut table);

    // Compile the DSL table to axum. Every middleware id the tables declare is
    // registered above, so this cannot fail with `RouteError::UnknownMiddleware`.
    let router = table
        .try_into_axum_router()
        .expect("every referenced middleware id is registered");

    // Handlers take no extractors yet, so the shared state is attached as a
    // request extension rather than through the `State` extractor. Swap this
    // for `.with_state(state)` once handlers consume `State<Arc<AppState>>`.
    router.layer(axum::Extension(state))
}

/// Register the middleware ids referenced by the generated route tables.
///
/// Each id is registered as a **pass-through placeholder**: it satisfies the
/// router's build-time middleware resolution — an id that was never registered
/// is a typed `RouteError::UnknownMiddleware` — but it does **not** enforce
/// anything yet. The route metadata records the intended guard (`auth`,
/// `verified`, `password.confirm`); replace each closure with the real layer
/// (the session guard, `EnsureEmailIsVerified`, and the confirm-password gate)
/// when that enforcement lands.
fn register_placeholder_middleware(table: &mut Router) {
    table.register_middleware("auth", |method_router| method_router);
    table.register_middleware("verified", |method_router| method_router);
    table.register_middleware("password.confirm", |method_router| method_router);
}
