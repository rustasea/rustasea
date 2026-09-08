//! Integration checks — router composes HTTP middleware layers (task C).
//!
//! The router crate must stay free of an http dependency (the HTTP layer
//! depends on the router), so the composition proof lives here, in the
//! crate that owns both ends of the DAG.

use rustavel_http::CorsConfig;
use rustavel_router::Router;

/// A CORS layer built from config composes onto the router's axum output.
#[test]
fn cors_layer_composes_onto_router() {
    let mut route_table = Router::new();
    route_table.get("/").get("/health");
    route_table.layer(CorsConfig::default().layer());

    let router = route_table.into_axum_router();
    // Type-level proof only: layers were applied without an axum
    // "overlapping method route" panic or a bound failure.
    let _ = router;
}
