//! M0 — Bootstrap & Core Stubs
//!
//! Traces to `test-plan.md` §5 M0 row, `test-cases.md` TC-M0-01..12,
//! `qa-design.md` S-03 + FS-01, FR-000..008, FS-M0-01..04.
//!
//! Each test is `#[ignore = "stub"]` until `rustavel-foundation` + `rustavel-config` land.
//! The assertions verify the stub declares its own trace so `cargo test -- --list` inventories coverage.

#[cfg(test)]
mod m0_foundation_stubs {
    /// TC-M0-01 — DAG-ordered providers before AppState.
    #[test]
    #[ignore = "stub: crate `rustavel-foundation` not yet implemented (TASK-010 harness)"]
    fn tc_m0_01_provider_dag_before_boot() {
        assert!(true, "stub TC-M0-01: traces FR-000/FS-M0-01, concern=State, @milestone-m0");
    }

    /// TC-M0-02 — Layered config env > file.
    #[test]
    #[ignore = "stub: crate `rustavel-config` not yet implemented"]
    fn tc_m0_02_layered_config_env_wins() {
        let file_port = 3000u16;
        let env_port = 4000u16;
        assert!(env_port != file_port, "stub TC-M0-02: FR-001/FS-M0-03 concern=Service");
        let _ = (file_port, env_port);
    }

    /// TC-M0-03 — Invalid TOML diagnostic with file+line.
    #[test]
    #[ignore = "stub: crate `rustavel-config` not yet implemented"]
    fn tc_m0_03_invalid_toml_diagnostic() {
        assert!(true, "stub TC-M0-03: file=database.toml line=7, P0, must not panic");
    }

    /// TC-M0-04 — Circular dependency Cycle error.
    #[test]
    #[ignore = "stub: crate `rustavel-foundation` not yet implemented"]
    fn tc_m0_04_cycle_detection() {
        assert!(true, "stub TC-M0-04: BootError::Cycle {A,B} must be returned, not Running");
    }

    /// TC-M0-05 — Graceful drain: SIGTERM while GET /slow in-flight → 200 + exit 0.
    #[tokio::test]
    #[ignore = "stub: Application + tokio::signal not yet implemented"]
    async fn tc_m0_05_graceful_drain() {
        assert!(true, "stub TC-M0-05: FR-004/FS-M0-04 State, 3s vs 10s timeout");
    }

    /// TC-M0-06 — Singleton Arc::ptr_eq.
    #[test]
    #[ignore = "stub: Container Singleton not yet implemented"]
    fn tc_m0_06_singleton_ptr_eq() {
        assert!(true, "stub TC-M0-06: FR-002 Singleton ptr_eq, Bind !ptr_eq");
    }

    /// TC-M0-07 — Make<Option<T>> == None when unbound.
    #[test]
    #[ignore = "stub: Container nullable-class semantics not yet implemented"]
    fn tc_m0_07_make_optional_none() {
        assert!(true, "stub TC-M0-07: FR-002 Make::<Option<Mailer>> must be None");
    }

    /// TC-M0-08 — Make<T> when unbound → NotFound.
    #[test]
    #[ignore = "stub: Container NotFound not yet implemented"]
    fn tc_m0_08_make_not_found() {
        assert!(true, "stub TC-M0-08: ContainerError::NotFound { PaymentGateway }");
    }

    /// TC-M0-09 — Manager::extend bound closure sees correct prefix.
    #[test]
    #[ignore = "stub: Manager::extend bound closure not yet implemented (FR-006)"]
    fn tc_m0_09_manager_extend_bound_closure() {
        assert!(true, "stub TC-M0-09: FR-006 #20 fix, closure self == manager");
    }

    /// TC-M0-10 — Scaffold `cargo rustavel new demo` produces workspace.
    #[test]
    #[ignore = "stub: cargo-rustavel xtask not yet implemented (FR-005)"]
    fn tc_m0_10_scaffold_new() {
        assert!(true, "stub TC-M0-10: files bootstrap/app.rs, config/, routes/web.rs, .env.example must exist; cargo check passes");
    }

    /// TC-M0-11 — Missing config fallback + App singleton via AppState::app().
    #[test]
    #[ignore = "stub: App singleton + fallback not yet implemented"]
    fn tc_m0_11_missing_config_fallback_app_singleton() {
        assert!(true, "stub TC-M0-11: missing config/app.toml → defaults, same Application");
    }

    /// TC-M0-12 — Graceful drain timeout forces exit 1.
    #[tokio::test]
    #[ignore = "stub: shutdown timeout expiry path"]
    async fn tc_m0_12_drain_timeout_expiry() {
        assert!(true, "stub TC-M0-12: 1s timeout vs 5s request => exit 1 + ShutdownTimeout log");
    }
}
