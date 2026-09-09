//! Harness — TestCase + Factory + Testcontainers + Fakes
//! Documents the shared harness that `rustasea-testing` MUST provide per FS-M5-04 + NFR-*.
//! Traces to `test-plan.md` §3 harness conventions, `qa-design.md` §7 fixtures.

/// Contract for every harness test: `TestCase` MUST expose these invariants.
/// This module is documentation + a smoke that the constants match `qa-design.md`.
#[cfg(test)]
mod harness_contract {
    const CONTAINER_TIMEOUT_SECS: u64 = 30;
    const ISOLATED_PORT_RANGE_NOTE: &str = "random per test binary via testcontainers";
    const STR_RESET_HOOK: &str = "Factory::sequence reset between tests";
    const ENV_TESTING: &str = ".env.testing overlay: process env > file";
    const SEEDER_IDEMPOTENCE: &str = "Seeder::run is idempotent; migrate:fresh --seed re-up same version";
    const NO_UNWRAP_IN_FRAMEWORK: &str = "#[deny(clippy::unwrap_used)] in framework crates";

    #[test]
    #[ignore = "harness contract: not runtime — verifies doc constants exist"]
    fn harness_constants_present() {
        let _ = (
            CONTAINER_TIMEOUT_SECS,
            ISOLATED_PORT_RANGE_NOTE,
            STR_RESET_HOOK,
            ENV_TESTING,
            SEEDER_IDEMPOTENCE,
            NO_UNWRAP_IN_FRAMEWORK,
        );
        assert_eq!(CONTAINER_TIMEOUT_SECS, 30);
    }

    /// Smoke shape — replace body when `rustasea-testing` lands.
    #[tokio::test]
    #[ignore = "stub: TestCase harness not yet implemented — shape only"]
    async fn test_case_shape_boot_with_testcontainers() {
        // Expected implementation:
        // let state = MyTest {}.setup().await;
        // assert!(state.config::<AppConfig>().port != 0);
        // state.teardown().await;
        assert!(true, "shape: setup → isolated PG+Redis via testcontainers → migrate once → Str reset → teardown kills containers");
    }
}
