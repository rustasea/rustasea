//! M5 — DX, CLI & Testing Stubs
//! Traces to TC-M5-01..17, FR-500..509, FS-M5-01..04.

#[cfg(test)]
mod m5_cli_testing_stubs {
    #[test] #[ignore = "stub: cargo rustasea list --json not yet implemented (FR-500)"]
    fn tc_m5_01_list_json_has_migrate_and_controller() { assert!(true, "stub TC-M5-01: make:controller + migrate usage strings"); }
    #[test] #[ignore = "stub: #[usage] not yet implemented"]
    fn tc_m5_02_usage_in_help() { assert!(true, "stub TC-M5-02: app:send {user} in help"); }
    #[test] #[ignore = "stub: confirm prompt abort not yet implemented (FR-503)"]
    fn tc_m5_03_confirm_n_aborts_exit_1() { assert!(true, "stub TC-M5-03: confirm n → exit 1"); }
    #[test] #[ignore = "stub: Artisan::call not yet implemented (FR-505)"]
    fn tc_m5_04_artisan_call_in_process() { assert!(true, "stub TC-M5-04: in-process migrate without subprocess"); }
    #[test] #[ignore = "stub: #[hidden] not yet implemented"]
    fn tc_m5_05_hidden_absent_without_all() { assert!(true, "stub TC-M5-05: hidden absent from list"); }
    #[test] #[ignore = "stub: did-you-mean not yet implemented"]
    fn tc_m5_06_unknown_suggestion() { assert!(true, "stub TC-M5-06: make:controll→make:controller"); }
    #[test] #[ignore = "stub: make:controller not yet implemented (FR-501)"]
    fn tc_m5_07_make_controller_lint_clean() { assert!(true, "stub TC-M5-07: user_controller.rs formatted+clippy clean"); }
    #[test] #[ignore = "stub: make:model -m not yet implemented"]
    fn tc_m5_08_make_model_m_flag() { assert!(true, "stub TC-M5-08: Post + migration *_create_posts_table.rs"); }
    #[test] #[ignore = "stub: AlreadyExists without --force not yet implemented"]
    fn tc_m5_09_already_exists_without_force() { assert!(true, "stub TC-M5-09: GeneratorError::AlreadyExists"); }
    #[test] #[ignore = "stub: make:{job,event,listener} not yet implemented"]
    fn tc_m5_10_other_generators_lint_clean() { assert!(true, "stub TC-M5-10: job/event/listener lint clean per path matrix"); }
    #[tokio::test] #[ignore = "stub: #[tries] attr not yet implemented (FR-506)"]
    async fn tc_m5_11_tries_attr_drives_retry() { assert!(true, "stub TC-M5-11: 3 attempts then failed_jobs"); }
    #[test] #[ignore = "stub: attr shadows ShouldRetry warning not yet implemented"]
    fn tc_m5_12_attr_shadows_trait_warning() { assert!(true, "stub TC-M5-12: attribute wins + warning"); }
    #[tokio::test] #[ignore = "stub: TestCase isolated stores not yet implemented (FR-507)"]
    async fn tc_m5_13_isolated_stores_distinct_ports() { assert!(true, "stub TC-M5-13: random ports distinct per binary"); }
    #[tokio::test] #[ignore = "stub: Str reset not yet implemented (FR-508)"]
    async fn tc_m5_14_str_factory_resets_between_tests() { assert!(true, "stub TC-M5-14: user1@example.com after reset"); }
    #[test] #[ignore = "stub: paginator view not yet implemented (FR-509)"]
    fn tc_m5_15_paginator_bootstrap3() { assert!(true, "stub TC-M5-15: bootstrap-3 HTML"); }
    #[tokio::test] #[ignore = "stub: container startup timeout not yet implemented"]
    async fn tc_m5_16_container_timeout() { assert!(true, "stub TC-M5-16: TestError::ContainerTimeout"); }
    #[tokio::test] #[ignore = "stub: test teardown not yet implemented"]
    async fn tc_m5_17_teardown_no_lingering() { assert!(true, "stub TC-M5-17: no rustasea-test-* containers remain"); }
}
