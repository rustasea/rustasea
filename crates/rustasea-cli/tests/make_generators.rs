//! Generator e2e smoke tests — every `make:*` kind writes a real, non-empty
//! `.rs` file whose source names the requested scaffold (FS-M5-02, FR-501).
//!
//! Scaffolds land in a unique tempdir per case, so the repository tree is
//! never touched. Compile/rustfmt cleanliness of the emitted sources is
//! exercised separately against an app fixture (see the M5 pipeline) — these
//! tests lock the generator *wiring*: correct output path, non-trivial source,
//! `AlreadyExists` guard and `--force` overwrite.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use rustasea_cli::generators::{generate, Kind, MakeOptions};

/// Unique temp root per invocation, removed on success.
fn temp_root() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "rustasea-cli-make-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    root
}

/// One scaffold expectation: kind, PascalCase name, output rel path, and a
/// marker the source must contain.
struct Case {
    kind: Kind,
    name: &'static str,
    rel: &'static str,
    marker: &'static str,
    resource: bool,
}

fn class_cases() -> Vec<Case> {
    vec![
        Case {
            kind: Kind::Controller,
            name: "PostController",
            rel: "app/http/controllers/post_controller.rs",
            marker: "PostController",
            resource: false,
        },
        Case {
            kind: Kind::Controller,
            name: "PostController",
            rel: "app/http/controllers/post_controller.rs",
            marker: "destroy",
            resource: true,
        },
        Case {
            kind: Kind::Middleware,
            name: "EnsureTokenIsValid",
            rel: "app/http/middleware/ensure_token_is_valid.rs",
            marker: "EnsureTokenIsValid",
            resource: false,
        },
        Case {
            kind: Kind::Request,
            name: "StorePostRequest",
            rel: "app/http/requests/store_post_request.rs",
            marker: "StorePostRequest",
            resource: false,
        },
        Case {
            kind: Kind::Model,
            name: "Post",
            rel: "app/models/post.rs",
            marker: "struct Post",
            resource: false,
        },
        Case {
            kind: Kind::Provider,
            name: "AnalyticsProvider",
            rel: "app/providers/analytics_provider.rs",
            marker: "AnalyticsProvider",
            resource: false,
        },
        Case {
            kind: Kind::Command,
            name: "GenerateReport",
            rel: "app/console/commands/generate_report.rs",
            marker: "GenerateReport",
            resource: false,
        },
        Case {
            kind: Kind::Job,
            name: "SendEmail",
            rel: "app/jobs/send_email.rs",
            marker: "SendEmail",
            resource: false,
        },
        Case {
            kind: Kind::Event,
            name: "OrderPlaced",
            rel: "app/events/order_placed.rs",
            marker: "OrderPlaced",
            resource: false,
        },
        Case {
            kind: Kind::Listener,
            name: "SendOrderConfirmation",
            rel: "app/listeners/send_order_confirmation.rs",
            marker: "SendOrderConfirmation",
            resource: false,
        },
        Case {
            kind: Kind::Observer,
            name: "PostObserver",
            rel: "app/observers/post_observer.rs",
            marker: "PostObserver",
            resource: false,
        },
        Case {
            kind: Kind::Test,
            name: "UserTest",
            rel: "tests/feature/user_test.rs",
            marker: "TestCase",
            resource: false,
        },
        Case {
            kind: Kind::Seeder,
            name: "DatabaseSeeder",
            rel: "database/seeders/database_seeder.rs",
            marker: "DatabaseSeeder",
            resource: false,
        },
        Case {
            kind: Kind::Agent,
            name: "SupportAgent",
            rel: "app/ai/agents/support_agent.rs",
            marker: "SupportAgent",
            resource: false,
        },
        Case {
            kind: Kind::Tool,
            name: "SearchDocs",
            rel: "app/ai/tools/search_docs.rs",
            marker: "SearchDocs",
            resource: false,
        },
    ]
}

/// Every class-based `make:*` generator emits a real scaffold file.
#[test]
fn class_generators_write_real_sources() {
    for case in class_cases() {
        let root = temp_root();
        let opts = MakeOptions {
            name: case.name.to_string(),
            force: false,
            resource: case.resource,
            with_migration: false,
        };
        let files = generate(case.kind, &root, &opts).expect("scaffold succeeds");
        let target = root.join(case.rel);
        assert!(
            target.is_file(),
            "{:?} was not written at {}",
            case.kind.command(),
            case.rel
        );

        let source = std::fs::read_to_string(&target).expect("read scaffold");
        assert!(
            source.contains(case.marker),
            "{:?} source lacks `{}`: {}",
            case.kind.command(),
            case.marker,
            source
        );
        assert!(
            source.len() > 120,
            "{:?} source looks like a stub ({} bytes)",
            case.kind.command(),
            source.len()
        );
        assert_eq!(
            files.len(),
            1,
            "{:?} should write exactly one file",
            case.kind.command()
        );

        let _ = std::fs::remove_dir_all(&root);
    }
}

/// `make:model -m` appends a timestamped migration alongside the model.
#[test]
fn model_with_migration_writes_two_files() {
    let root = temp_root();
    let opts = MakeOptions {
        name: "Post".to_string(),
        force: false,
        resource: false,
        with_migration: true,
    };
    let files = generate(Kind::Model, &root, &opts).expect("model + migration scaffold");
    assert_eq!(files.len(), 2);

    let model = root.join("app/models/post.rs");
    let migration_rel = files[1].path.clone();
    let migration = root.join(&migration_rel);
    assert!(
        migration.is_file(),
        "migration not written at {migration_rel}"
    );
    assert!(
        migration_rel.contains("_create_posts_table.rs"),
        "unexpected migration path: {migration_rel}"
    );
    let source = std::fs::read_to_string(&migration).expect("read migration");
    assert!(source.contains("CreatePostTable"));

    assert!(model.is_file());
    let _ = std::fs::remove_dir_all(&root);
}

/// `make:migration` accepts a snake_case name and prefixes a timestamp.
#[test]
fn migration_generator_writes_timestamped_file() {
    let root = temp_root();
    let opts = MakeOptions {
        name: "create_users_table".to_string(),
        force: false,
        resource: false,
        with_migration: false,
    };
    let files = generate(Kind::Migration, &root, &opts).expect("migration scaffold");
    assert_eq!(files.len(), 1);

    let rel = files[0].path.clone();
    let target = root.join(&rel);
    assert!(target.is_file(), "migration not written at {rel}");
    let file_name = target.file_name().expect("file name").to_string_lossy();
    assert!(
        file_name.starts_with("20"),
        "expected timestamp prefix, got {file_name}"
    );

    let source = std::fs::read_to_string(&target).expect("read migration");
    assert!(source.contains("CreateUsersTable"));
    let _ = std::fs::remove_dir_all(&root);
}

/// A second run without `--force` fails; with `--force` it overwrites.
#[test]
fn existing_scaffold_respects_force_flag() {
    let root = temp_root();
    let opts = MakeOptions {
        name: "Post".to_string(),
        force: false,
        resource: false,
        with_migration: false,
    };
    generate(Kind::Model, &root, &opts).expect("first scaffold");

    let again = generate(Kind::Model, &root, &opts);
    assert!(
        again.is_err(),
        "second scaffold without --force must report AlreadyExists"
    );

    let forced = generate(
        Kind::Model,
        &root,
        &MakeOptions {
            name: "Post".to_string(),
            force: true,
            resource: false,
            with_migration: false,
        },
    );
    assert!(forced.is_ok(), "scaffold with --force must overwrite");
    let _ = std::fs::remove_dir_all(&root);
}

/// Invalid class names are rejected before any file is written.
#[test]
fn invalid_class_name_is_rejected() {
    let root = temp_root();
    let opts = MakeOptions {
        name: "post_controller".to_string(), // snake_case where PascalCase required
        force: false,
        resource: false,
        with_migration: false,
    };
    assert!(generate(Kind::Controller, &root, &opts).is_err());
    assert!(!root
        .join("app/http/controllers/post_controller.rs")
        .exists());
    let _ = std::fs::remove_dir_all(&root);
}

/// `make:middleware` / `make:request` reject a non-PascalCase name with the
/// typed `GenerationFailed` error before touching the filesystem.
#[test]
fn http_generators_reject_invalid_names() {
    for (kind, rel) in [
        (Kind::Middleware, "app/http/middleware/ensure_token.rs"),
        (Kind::Request, "app/http/requests/store_post.rs"),
    ] {
        let root = temp_root();
        let opts = MakeOptions {
            name: "ensure_token".to_string(),
            force: false,
            resource: false,
            with_migration: false,
        };
        let err = generate(kind, &root, &opts).expect_err("invalid name must fail");
        assert!(
            matches!(err, rustasea_cli::CliError::GenerationFailed { .. }),
            "{:?} must surface a typed GenerationFailed, got {err:?}",
            kind.command()
        );
        assert!(
            !root.join(rel).exists(),
            "{:?} must not write on an invalid name",
            kind.command()
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// Generated HTTP skeletons reference the framework types they compile against:
/// axum's function-middleware surface and the validation `Validatable` trait.
#[test]
fn http_generators_emit_framework_wired_sources() {
    let root = temp_root();

    let middleware = generate(
        Kind::Middleware,
        &root,
        &MakeOptions {
            name: "EnsureTokenIsValid".to_string(),
            force: false,
            resource: false,
            with_migration: false,
        },
    )
    .expect("middleware scaffold");
    let source =
        std::fs::read_to_string(root.join(&middleware[0].path)).expect("read middleware source");
    assert!(
        source.contains("use axum::extract::Request;"),
        "middleware must use axum's request extractor: {source}"
    );
    assert!(
        source.contains("use axum::middleware::Next;"),
        "middleware must use axum's Next: {source}"
    );
    assert!(
        source.contains("next.run(request).await"),
        "middleware must delegate to the wrapped handler: {source}"
    );

    let request = generate(
        Kind::Request,
        &root,
        &MakeOptions {
            name: "StorePostRequest".to_string(),
            force: false,
            resource: false,
            with_migration: false,
        },
    )
    .expect("request scaffold");
    let source = std::fs::read_to_string(root.join(&request[0].path)).expect("read request source");
    assert!(
        source.contains("use rustasea::validation::{ErrorBag, FormRequest, Rules, Validatable};"),
        "request must import the validation surface: {source}"
    );
    assert!(
        source.contains("impl Validatable for StorePostRequest"),
        "request must implement Validatable: {source}"
    );
    assert!(
        source.contains("fn validate(&self) -> Result<(), ErrorBag>"),
        "request must declare the validation hook: {source}"
    );

    let _ = std::fs::remove_dir_all(&root);
}
