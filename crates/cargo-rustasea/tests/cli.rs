//! End-to-end tests for the `cargo-rustasea` binary.
//!
//! These exercise the real process: argument normalization (`cargo rustasea` /
//! `cargo artisan` tokens), variant validation, exit codes, and on-disk output.

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

/// Path to the compiled binary under test (set by Cargo for integration tests).
const BIN: &str = env!("CARGO_BIN_EXE_cargo-rustasea");

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
        "cargo-rustasea-{tag}-{}-{nanos}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&path).expect("create temp dir");
    path
}

/// Run the binary with `args`, capturing stdout/stderr.
fn run(dir: &PathBuf, args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run cargo-rustasea")
}

/// Assert success and render stderr on failure.
fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn unknown_variant_exits_non_zero_with_clear_error() {
    let dir = temp_dir("unknown-variant");
    let output = run(&dir, &["new", "demo", "--variant", "svelte"]);

    assert!(!output.status.success(), "must exit non-zero");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("svelte"),
        "error must name the bad variant:\n{stderr}"
    );
    assert!(
        stderr.contains("blade") && stderr.contains("livewire"),
        "error must list supported variants:\n{stderr}"
    );
    assert!(!dir.join("demo").exists(), "no tree on failure");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn generates_blade_app_without_git() {
    let dir = temp_dir("blade");
    let output = run(&dir, &["new", "demo-app", "--variant", "blade", "--no-git"]);
    assert_success(&output);

    let app = dir.join("demo-app");
    assert!(app.join("Cargo.toml").exists());
    assert!(app.join("routes/web.rs").exists());
    assert!(app.join("resources/views/dashboard.html").exists());
    assert!(!app.join(".git").exists(), "--no-git must skip git init");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn rustasea_token_is_stripped() {
    let dir = temp_dir("rustasea-token");
    // Cargo invokes `cargo-rustasea rustasea new ...`; the token must be dropped.
    let output = run(
        &dir,
        &[
            "rustasea",
            "new",
            "demo-app",
            "--variant",
            "vue",
            "--no-git",
        ],
    );
    assert_success(&output);
    assert!(dir.join("demo-app/resources/js/main.rs").exists());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn artisan_alias_generates_livewire_app() {
    let dir = temp_dir("artisan");
    let output = run(
        &dir,
        &[
            "artisan",
            "new",
            "demo-app",
            "--variant",
            "livewire",
            "--no-git",
        ],
    );
    assert_success(&output);

    let app = dir.join("demo-app");
    assert!(app.join("resources/views/partials/counter.html").exists());
    let manifest = std::fs::read_to_string(app.join("Cargo.toml")).expect("read manifest");
    assert!(manifest.contains("features = [\"view\", \"broadcast\"]"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn rerun_conflicts_without_force_and_succeeds_with_force() {
    let dir = temp_dir("force");
    let first = run(&dir, &["new", "demo-app", "--variant", "react", "--no-git"]);
    assert_success(&first);

    let conflict = run(&dir, &["new", "demo-app", "--variant", "react", "--no-git"]);
    assert!(!conflict.status.success(), "rerun must conflict");

    let forced = run(
        &dir,
        &[
            "new",
            "demo-app",
            "--variant",
            "react",
            "--no-git",
            "--force",
        ],
    );
    assert_success(&forced);

    std::fs::remove_dir_all(&dir).ok();
}
