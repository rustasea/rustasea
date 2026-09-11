//! Unit tests for router registration metadata.
//!
//! Kept out of `router.rs` so the builder stays within the 500-line file
//! budget; every case exercises only the public registration surface.

use crate::Router;

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

/// Action-bound routes expose the resolved handler label for introspection.
#[test]
fn action_route_records_handler_label() {
    async fn index() -> &'static str {
        "index"
    }
    let mut router = Router::new();
    router.get_action("/users", index);
    let route = &router.get_routes()[0];
    assert!(route
        .handler
        .as_deref()
        .unwrap_or_default()
        .contains("index"));
    assert!(route.controller.is_none());
}
