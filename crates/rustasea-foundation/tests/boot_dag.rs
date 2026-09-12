//! Provider boot DAG tests — dependency ordering and typed cycle errors.

use std::sync::{Arc, Mutex};

use rustasea_foundation::{Application, BootError, ServiceProvider};

/// Test provider that records its boot into a shared log.
struct Recorder {
    name: &'static str,
    deps: &'static [&'static str],
    log: Arc<Mutex<Vec<&'static str>>>,
}

impl ServiceProvider for Recorder {
    /// Stable provider name used by dependency edges.
    fn name(&self) -> &'static str {
        self.name
    }

    /// Declared dependencies for this recorder.
    fn dependencies(&self) -> &'static [&'static str] {
        self.deps
    }

    /// Append this provider's name to the shared boot log.
    fn boot(&self, _app: &Application) {
        self.log.lock().expect("log lock poisoned").push(self.name);
    }
}

/// Build a recorder writing its boot order into `log`.
fn recorder(
    name: &'static str,
    deps: &'static [&'static str],
    log: &Arc<Mutex<Vec<&'static str>>>,
) -> Recorder {
    Recorder {
        name,
        deps,
        log: Arc::clone(log),
    }
}

/// Boots a declared dependency before its consumer, keeping registration order
/// as the tie-breaker for the remaining ready providers.
#[test]
fn boot_respects_dependencies_and_registration_order() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut app = Application::new();
    // The consumer registers first; the DAG must move its dependency ahead.
    app.provider(recorder("AppProvider", &["ConfigProvider"], &log));
    app.provider(recorder("ConfigProvider", &[], &log));
    app.provider(recorder("CacheProvider", &[], &log));

    app.boot().expect("acyclic provider graph should boot");

    let order = log.lock().expect("log lock poisoned").clone();
    assert_eq!(
        order,
        vec!["ConfigProvider", "AppProvider", "CacheProvider"]
    );
    assert!(app.is_booted());
}

/// Providers without dependencies keep their registration order.
#[test]
fn independent_providers_keep_registration_order() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut app = Application::new();
    app.provider(recorder("First", &[], &log));
    app.provider(recorder("Second", &[], &log));
    app.provider(recorder("Third", &[], &log));

    app.boot().expect("acyclic provider graph should boot");

    assert_eq!(
        *log.lock().expect("log lock poisoned"),
        vec!["First", "Second", "Third"]
    );
}

/// A provider cycle fails the boot with a typed error and boots nothing.
#[test]
fn provider_cycle_returns_typed_error() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut app = Application::new();
    app.provider(recorder("Alpha", &["Beta"], &log));
    app.provider(recorder("Beta", &["Alpha"], &log));

    let err = app.boot().expect_err("cycle must fail boot");
    match err {
        BootError::DependencyCycle { providers } => {
            assert!(providers.contains(&"Alpha".to_string()));
            assert!(providers.contains(&"Beta".to_string()));
        }
        other => panic!("expected DependencyCycle, got {other:?}"),
    }
    assert!(!app.is_booted());
    assert!(
        log.lock().expect("log lock poisoned").is_empty(),
        "no provider should boot on a cycle"
    );
}

/// A downstream consumer of a cycle must not be reported as a cycle member.
#[test]
fn cycle_excludes_downstream_consumers() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut app = Application::new();
    // The consumer registers first, so its index is the first residual node;
    // a DFS rooted there alone would miss the A <-> B cycle and fall back to
    // dumping every residual provider.
    app.provider(recorder("Consumer", &["A"], &log));
    app.provider(recorder("A", &["B"], &log));
    app.provider(recorder("B", &["A"], &log));

    let err = app.boot().expect_err("cycle must fail boot");
    match err {
        BootError::DependencyCycle { providers } => {
            assert!(providers.contains(&"A".to_string()));
            assert!(providers.contains(&"B".to_string()));
            assert!(
                !providers.contains(&"Consumer".to_string()),
                "downstream consumer must not be reported: {providers:?}"
            );
        }
        other => panic!("expected DependencyCycle, got {other:?}"),
    }
    assert!(!app.is_booted());
}

/// A self-dependency is reported as a one-node cycle.
#[test]
fn self_dependency_is_a_cycle() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut app = Application::new();
    app.provider(recorder("Selfish", &["Selfish"], &log));

    let err = app.boot().expect_err("self dependency must fail boot");
    assert_eq!(
        err,
        BootError::DependencyCycle {
            providers: vec!["Selfish".to_string()],
        }
    );
    assert!(!app.is_booted());
}

/// An unresolved dependency fails the boot with a typed error.
#[test]
fn unknown_dependency_returns_typed_error() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut app = Application::new();
    app.provider(recorder("AppProvider", &["MissingProvider"], &log));

    let err = app.boot().expect_err("unknown dependency must fail boot");
    assert_eq!(
        err,
        BootError::UnknownDependency {
            provider: "AppProvider".to_string(),
            dependency: "MissingProvider".to_string(),
        }
    );
    assert!(!app.is_booted());
}

/// Boxed providers participate in the DAG through the blanket impl.
#[test]
fn boxed_providers_participate_in_the_dag() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut app = Application::new();
    let boxed: Box<dyn ServiceProvider> = Box::new(recorder("Boxed", &[], &log));
    app.provider(boxed);

    app.boot().expect("boxed provider should boot");

    assert_eq!(*log.lock().expect("log lock poisoned"), vec!["Boxed"]);
}

/// A second boot is a no-op (idempotent).
#[test]
fn boot_is_idempotent() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut app = Application::new();
    app.provider(recorder("Once", &[], &log));

    app.boot().expect("first boot should succeed");
    app.boot().expect("second boot should succeed");

    assert_eq!(*log.lock().expect("log lock poisoned"), vec!["Once"]);
}
