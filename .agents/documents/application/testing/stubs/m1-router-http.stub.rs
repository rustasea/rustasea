//! M1 — Routing & HTTP Stubs
//!
//! Traces to `test-plan.md` §5 M1 row, `test-cases.md` TC-M1-01..16,
//! `qa-design.md` S-04, FR-100..109, FS-M1-01..06.

#[cfg(test)]
mod m1_router_http_stubs {
    #[test] #[ignore = "stub: rustasea-router not yet implemented"]
    fn tc_m1_01_resource_expands_seven_routes() { assert!(true, "stub TC-M1-01: resource 7 CRUD routes"); }
    #[test] #[ignore = "stub: rustasea-router prefix not yet implemented"]
    fn tc_m1_02_group_prefix_applied() { assert!(true, "stub TC-M1-02: /api/v1/users"); }
    #[test] #[ignore = "stub: rustasea-router duplicate name guard not yet implemented"]
    fn tc_m1_03_duplicate_named_route_conflict() { assert!(true, "stub TC-M1-03: RouteError::Conflict"); }
    #[tokio::test] #[ignore = "stub: domain routing not yet implemented (FR-102 #19)"]
    async fn tc_m1_04_domain_wins_over_non_domain() { assert!(true, "stub TC-M1-04: tenant catch-all wins docs.example.com/docs"); }
    #[tokio::test] #[ignore = "stub: domain routing not yet implemented"]
    async fn tc_m1_05_non_domain_fallback() { assert!(true, "stub TC-M1-05: non-domain handler when no subdomain"); }
    #[test] #[ignore = "stub: route:list binding_fields not yet implemented (FR-103 #20)"]
    fn tc_m1_06_route_list_binding_fields() { assert!(true, "stub TC-M1-06: binding_fields [slug]"); }
    #[test] #[ignore = "stub: route:list machine-output not yet implemented"]
    fn tc_m1_07_route_list_json_valid() { assert!(true, "stub TC-M1-07: valid JSON array with middleware per route"); }
    #[tokio::test] #[ignore = "stub: Throttle middleware not yet implemented"]
    async fn tc_m1_08_throttle_per_minute_61st_rejected() { assert!(true, "stub TC-M1-08: 60→61 429 Retry-After"); }
    #[tokio::test] #[ignore = "stub: CORS middleware not yet implemented"]
    async fn tc_m1_09_cors_allowlisted_origin() { assert!(true, "stub TC-M1-09: Access-Control-Allow-Origin for allowed origin"); }
    #[tokio::test] #[ignore = "stub: CORS not yet implemented"]
    async fn tc_m1_10_cors_denied_origin() { assert!(true, "stub TC-M1-10: no header for evil origin"); }
    #[tokio::test] #[ignore = "stub: Json<T> extractor not yet implemented (FR-105)"]
    async fn tc_m1_11_json_extractor_happy() { assert!(true, "stub TC-M1-11: 201 for valid Ada payload"); }
    #[tokio::test] #[ignore = "stub: ErrorBag 422 not yet implemented"]
    async fn tc_m1_12_error_bag_422() { assert!(true, "stub TC-M1-12: 422 ErrorBag email invalid"); }
    #[tokio::test] #[ignore = "stub: strict deserialize unknown field not yet implemented"]
    async fn tc_m1_13_strict_unknown_field_rejected() { assert!(true, "stub TC-M1-13: 422 unknown field age"); }
    #[tokio::test] #[ignore = "stub: Http client throw not yet implemented (wiremock fake)"]
    async fn tc_m1_14_http_throw_on_5xx() { assert!(true, "stub TC-M1-14: HttpError::Status 500 via throw(predicate)"); }
    #[tokio::test] #[ignore = "stub: idle timeout classification not yet implemented"]
    async fn tc_m1_15_idle_timeout_distinct() { assert!(true, "stub TC-M1-15: HttpError::Timeout Idle 6s vs 5s"); }
    #[test] #[ignore = "stub: throw policy predicate matrix not yet implemented"]
    fn tc_m1_16_throw_predicate_matrix() { assert!(true, "stub TC-M1-16: server_error+422→success, always+422→error"); }
}
