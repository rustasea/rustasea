//! Integration tests for the starter-kit scaffolder.
//!
//! Positive coverage: every variant generates the shared core plus its own
//! `resources/` tree and `Cargo.toml` feature set. Negative coverage: unknown
//! variants and invalid names are rejected with typed errors.

use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};

use rustasea_scaffold::{Scaffold, ScaffoldError, StarterKitVariant};

/// Paths every variant must generate (ADR-0002 decision 9 + blueprint §5).
const REQUIRED_CORE: &[&str] = &[
    ".env.example",
    "Cargo.toml",
    "rustasea.toml",
    "lib.rs",
    "main.rs",
    "bootstrap/app.rs",
    "bootstrap/providers.rs",
    "bootstrap/commands.rs",
    "app/actions/auth/create_new_user.rs",
    "app/actions/auth/attempt_to_authenticate.rs",
    "app/actions/auth/ensure_login_is_not_throttled.rs",
    "app/actions/auth/redirect_if_authenticated.rs",
    "app/actions/auth/prepare_authenticated_session.rs",
    "app/concerns/password_validation_rules.rs",
    "app/concerns/profile_validation_rules.rs",
    "app/http/controllers/auth_controller.rs",
    "app/http/controllers/dashboard_controller.rs",
    "app/http/controllers/settings/profile_controller.rs",
    "app/http/controllers/settings/password_controller.rs",
    "app/http/middleware/ensure_email_is_verified.rs",
    "app/http/requests/settings/profile_update_request.rs",
    "app/http/requests/settings/password_update_request.rs",
    "app/models/user.rs",
    "app/providers/app_service_provider.rs",
    "app/providers/auth_service_provider.rs",
    "routes/mod.rs",
    "routes/web.rs",
    "routes/auth.rs",
    "routes/settings.rs",
    "routes/console.rs",
    "database/migrations/create_users.rs",
    "database/migrations/create_sessions.rs",
    "database/migrations/create_password_reset_tokens.rs",
    "database/factories/user_factory.rs",
    "database/seeders/database_seeder.rs",
    "tests/feature/auth_test.rs",
    "tests/unit/actions_test.rs",
    "config/app.toml",
    "config/auth.toml",
    "config/database.toml",
    "config/cache.toml",
    "config/queue.toml",
    "config/session.toml",
    "storage/app/.gitkeep",
    "storage/logs/.gitkeep",
];

/// Monotonic counter keeping parallel tests on distinct temp paths.
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Create a unique empty temporary directory for a test.
fn temp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_nanos();
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "rustasea-scaffold-{tag}-{}-{nanos}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&path).expect("create temp dir");
    path
}

/// Read a generated file as a string.
fn read(root: &Path, relative: &str) -> String {
    std::fs::read_to_string(root.join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

/// Assert every shared-core path exists.
fn assert_core_layout(root: &Path) {
    for relative in REQUIRED_CORE {
        assert!(
            root.join(relative).exists(),
            "missing core file: {relative}"
        );
    }
}

/// Assert the generated `Cargo.toml` carries `needle`.
fn assert_manifest_contains(root: &Path, needle: &str) {
    let manifest = read(root, "Cargo.toml");
    assert!(
        manifest.contains(needle),
        "Cargo.toml missing `{needle}`:\n{manifest}"
    );
}

#[test]
fn blade_variant_generates_askama_layout() {
    let root = temp_dir("blade");
    Scaffold::new("demo-app", StarterKitVariant::Blade)
        .generate(&root)
        .expect("generate blade");

    assert_core_layout(&root);
    assert!(root.join("resources/views/dashboard.html").exists());
    assert!(root.join("resources/views/auth/login.html").exists());
    assert!(root.join("askama.toml").exists());
    assert!(!root.join("resources/js").exists());
    assert!(!root.join("config/inertia.toml").exists());
    assert_manifest_contains(&root, "features = [\"view\"]");
    assert!(read(&root, "rustasea.toml").contains("variant = \"blade\""));

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn react_variant_generates_dioxus_inertia_layout() {
    let root = temp_dir("react");
    Scaffold::new("demo-app", StarterKitVariant::React)
        .generate(&root)
        .expect("generate react");

    assert_core_layout(&root);
    assert!(root.join("resources/js/main.rs").exists());
    assert!(root.join("resources/js/pages/mod.rs").exists());
    assert!(root.join("resources/js/pages/dashboard.rs").exists());
    assert!(root
        .join("app/http/middleware/handle_inertia_requests.rs")
        .exists());
    assert!(root.join("config/inertia.toml").exists());
    assert!(!root.join("askama.toml").exists());
    assert_manifest_contains(&root, "wasm-dioxus");
    assert_manifest_contains(&root, "features = [\"react\"]");

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn vue_variant_generates_leptos_inertia_layout() {
    let root = temp_dir("vue");
    Scaffold::new("demo-app", StarterKitVariant::Vue)
        .generate(&root)
        .expect("generate vue");

    assert_core_layout(&root);
    assert!(root.join("resources/js/main.rs").exists());
    assert!(root.join("resources/js/pages/settings_profile.rs").exists());
    assert!(root
        .join("app/http/middleware/handle_inertia_requests.rs")
        .exists());
    assert!(root.join("config/inertia.toml").exists());
    assert_manifest_contains(&root, "wasm-leptos");
    assert_manifest_contains(&root, "features = [\"vue\"]");

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn livewire_variant_generates_htmx_layout() {
    let root = temp_dir("livewire");
    Scaffold::new("demo-app", StarterKitVariant::Livewire)
        .generate(&root)
        .expect("generate livewire");

    assert_core_layout(&root);
    assert!(root.join("resources/views/dashboard.html").exists());
    assert!(root.join("resources/views/partials/counter.html").exists());
    assert!(root
        .join("resources/views/partials/login-form.html")
        .exists());
    assert!(root.join("askama.toml").exists());
    assert!(!root.join("resources/js").exists());
    assert_manifest_contains(&root, "features = [\"view\", \"broadcast\"]");

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn all_variants_render_without_leftover_placeholders() {
    for variant in StarterKitVariant::ALL {
        let files = Scaffold::new("my-app", variant).render().expect("render");
        assert!(!files.is_empty());
        for file in &files {
            assert!(
                !file.contents.contains("@@"),
                "unsubstituted placeholder in {} ({variant})",
                file.path
            );
        }
    }
}

#[test]
fn generated_router_receives_shared_state() {
    for variant in StarterKitVariant::ALL {
        let files = Scaffold::new("my-app", variant).render().expect("render");

        let routes_mod = files
            .iter()
            .find(|file| file.path == "routes/mod.rs")
            .expect("routes/mod.rs present");
        assert!(
            routes_mod
                .contents
                .contains("pub fn router(state: Arc<AppState>) -> axum::Router"),
            "router must accept the shared state ({variant})"
        );
        assert!(
            !routes_mod.contents.contains("AppState::new("),
            "router must not construct its own AppState ({variant})"
        );

        let main = files
            .iter()
            .find(|file| file.path == "main.rs")
            .expect("main.rs present");
        assert!(
            main.contents.contains("bootstrap::app::configure()"),
            "main must boot the application ({variant})"
        );
        assert!(
            main.contents.contains("routes::router(state)"),
            "main must pass the shared state into the router ({variant})"
        );
    }
}

#[test]
fn placeholder_substitution_uses_all_three_name_forms() {
    let files = Scaffold::new("MyCool-App", StarterKitVariant::Blade)
        .render()
        .expect("render");
    let manifest = files
        .iter()
        .find(|file| file.path == "Cargo.toml")
        .expect("Cargo.toml present");
    assert!(manifest.contents.contains("name = \"my-cool-app\""));
    assert!(manifest.contents.contains("name = \"my_cool_app\""));
    assert!(manifest.contents.contains("my-cool-app"));
}

#[test]
fn unknown_variant_is_rejected() {
    let error = StarterKitVariant::from_str("svelte").expect_err("must reject");
    assert!(matches!(error, ScaffoldError::UnknownVariant { .. }));
    let message = error.to_string();
    assert!(message.contains("svelte"));
    assert!(message.contains("blade"));
    assert!(message.contains("livewire"));
}

#[test]
fn invalid_app_name_is_rejected() {
    let root = temp_dir("invalid-name");
    let error = Scaffold::new("2fast", StarterKitVariant::Blade)
        .generate(&root)
        .expect_err("must reject leading digit");
    assert!(matches!(error, ScaffoldError::InvalidAppName { .. }));
    assert_eq!(std::fs::read_dir(&root).expect("read dir").count(), 0);

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn existing_files_conflict_unless_forced() {
    let root = temp_dir("conflict");
    Scaffold::new("demo-app", StarterKitVariant::Blade)
        .generate(&root)
        .expect("first generate");

    let error = Scaffold::new("demo-app", StarterKitVariant::Blade)
        .generate(&root)
        .expect_err("second generate must conflict");
    assert!(matches!(error, ScaffoldError::AlreadyExists { .. }));

    let regenerated = Scaffold::new("demo-app", StarterKitVariant::Blade)
        .with_force(true)
        .generate(&root)
        .expect("forced generate");
    assert!(!regenerated.is_empty());

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn target_that_is_a_file_is_rejected() {
    let root = temp_dir("file-target");
    let file = root.join("not-a-dir");
    std::fs::write(&file, b"x").expect("write file");
    let error = Scaffold::new("demo-app", StarterKitVariant::Blade)
        .generate(&file)
        .expect_err("file target must be rejected");
    assert!(matches!(error, ScaffoldError::InvalidTarget { .. }));

    std::fs::remove_dir_all(&root).ok();
}
