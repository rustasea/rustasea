//! Mounting and router-state tests for the Inertia WASM adapters.

use http::{HeaderMap, HeaderValue};
use rustasea_inertia_adapters::{
    inertia_registry, install_registry, mount_installed, mount_page, ClientError,
    NavigationOutcome, RouterState, Value,
};

/// Mount function that accepts only the expected `email` prop.
fn mount_login(props: &Value) -> Result<(), ClientError> {
    match props.get("email").and_then(Value::as_str) {
        Some("a@b.c") => Ok(()),
        _ => Err(ClientError::Mount {
            component: "auth/login".to_string(),
            message: "missing email".to_string(),
        }),
    }
}

inertia_registry!(TestRegistry {
    "auth/login" => mount_login,
});

/// A well-formed login page envelope.
const LOGIN_PAGE: &str =
    r#"{"component":"auth/login","props":{"email":"a@b.c"},"url":"/login","version":"v1"}"#;

/// Build the headers of an Inertia JSON response.
fn inertia_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("X-Inertia", HeaderValue::from_static("true"));
    headers
}

/// Positive: a registered component is mounted and its props are forwarded.
#[test]
fn mount_page_mounts_component_with_props() {
    let registry = TestRegistry::new();
    let page = mount_page(&registry, LOGIN_PAGE).expect("mount known component");

    assert_eq!(page.component, "auth/login");
    assert_eq!(page.props["email"], "a@b.c");
    assert_eq!(page.url, "/login");
}

/// Negative: an unregistered component key is an `UnknownComponent` error.
#[test]
fn mount_page_rejects_unregistered_component() {
    let registry = TestRegistry::new();
    let error = mount_page(
        &registry,
        r#"{"component":"missing","props":{},"url":"/","version":"v1"}"#,
    )
    .expect_err("unknown component must fail");

    assert!(matches!(
        error,
        ClientError::UnknownComponent { component } if component == "missing"
    ));
}

/// Negative: a registered component whose props fail is a `Mount` error.
#[test]
fn mount_page_rejects_invalid_props() {
    let registry = TestRegistry::new();
    let error = mount_page(
        &registry,
        r#"{"component":"auth/login","props":{},"url":"/login","version":"v1"}"#,
    )
    .expect_err("missing email must fail");

    assert!(matches!(error, ClientError::Mount { .. }));
}

/// `hydrate_with` mounts through the supplied registry and stores the page.
#[test]
fn router_hydrate_with_stores_page() {
    let registry = TestRegistry::new();
    let mut state = RouterState::new();

    let page = state.hydrate_with(&registry, LOGIN_PAGE).expect("hydrate");

    assert_eq!(page.component, "auth/login");
    assert_eq!(state.component(), Some("auth/login"));
    assert_eq!(state.url(), Some("/login"));
    assert_eq!(state.version(), Some("v1"));
    assert_eq!(
        state.props().map(|props| &props["email"]),
        Some(&Value::from("a@b.c"))
    );
}

/// `hydrate` mounts through the registry installed at boot.
#[test]
fn router_hydrate_uses_installed_registry() {
    install_registry(Box::new(TestRegistry::new()));
    let mut state = RouterState::new();

    state
        .hydrate(LOGIN_PAGE)
        .expect("hydrate via installed registry");

    assert_eq!(state.component(), Some("auth/login"));
}

/// An Inertia `200` response is mounted and stored.
#[test]
fn router_handle_response_mounts_inertia_page() {
    install_registry(Box::new(TestRegistry::new()));
    let mut state = RouterState::new().with_version("v1");

    let outcome = state
        .handle_response(200, &inertia_headers(), LOGIN_PAGE)
        .expect("handle response");

    assert!(matches!(outcome, NavigationOutcome::Mount(_)));
    assert_eq!(state.component(), Some("auth/login"));
}

/// A `409` response yields a hard navigation and leaves the page untouched.
#[test]
fn router_handle_response_hard_navigates_on_conflict() {
    install_registry(Box::new(TestRegistry::new()));
    let mut state = RouterState::new();
    let mut headers = HeaderMap::new();
    headers.insert("X-Inertia-Location", HeaderValue::from_static("/dashboard"));

    let outcome = state
        .handle_response(409, &headers, "")
        .expect("handle conflict");

    assert_eq!(
        outcome,
        NavigationOutcome::HardNavigate("/dashboard".to_string())
    );
    assert!(state.component().is_none());
}

/// Mounting without an installed registry is a `Mount` error, not a panic.
#[test]
fn mount_installed_without_registry_is_error() {
    let error = mount_installed("auth/login", &Value::Null).expect_err("no registry installed");

    assert!(matches!(error, ClientError::Mount { .. }));
}
