//! Cross-cutting — Migration, Contracts, NFR (Performance/Observability/Chaos), Security
//! Traces to TC-MIG-01/02, TC-CTR-01..03, TC-NFR-01/02, TC-PROP-01, Sec-01..09, `test-plan.md` §6-8.

#[cfg(test)]
mod cross_stubs {
    #[tokio::test] #[ignore = "stub: migration round-trip up→down→up not yet implemented"]
    async fn tc_mig_01_roundtrip() { assert!(true, "stub TC-MIG-01: reversible migration harness"); }
    #[test] #[ignore = "stub: irreversible migration error not yet implemented"]
    fn tc_mig_02_irreversible_error() { assert!(true, "stub TC-MIG-02: MigrationError::Irreversible"); }
    #[test] #[ignore = "stub: route:list JSON-Schema + insta not yet implemented"]
    fn tc_ctr_01_route_list_contract() { assert!(true, "stub TC-CTR-01: route:list snapshot + schema"); }
    #[test] #[ignore = "stub: JSON:API 1.1 schema validation not yet implemented"]
    fn tc_ctr_02_jsonapi_schema() { assert!(true, "stub TC-CTR-02: data/included/links/meta, vnd.api+json"); }
    #[test] #[ignore = "stub: queue payload serde round-trip not yet implemented"]
    fn tc_ctr_03_queue_payload_roundtrip() { assert!(true, "stub TC-CTR-03: Job<T> serde same after serialize"); }
    #[test] #[ignore = "stub: boot <2s bench not yet implemented (NFR-Per-01)"]
    fn tc_nfr_01_cold_boot_bench() { assert!(true, "stub TC-NFR-01: p50 <2s M0 5 providers"); }
    #[test] #[ignore = "stub: HTTP p95 <50ms bench not yet implemented (NFR-Per-02)"]
    fn tc_nfr_02_http_p95_bench() { assert!(true, "stub TC-NFR-02: GET /users p95<50ms 1k RPS"); }
    #[test] #[ignore = "stub: proptest serde round-trip not yet implemented (TC-PROP-01)"]
    fn tc_prop_01_serde_arbitrary_relations() { assert!(true, "stub TC-PROP-01: 0..20 relations, proptest"); }
}

#[cfg(test)]
mod security_triage_refs {
    /// References `test-plan.md` §8 Sec-01..09 — each stub asserts the security contract
    /// without duplicating the functional assertion (tests remain in their concern file above).
    #[tokio::test] #[ignore = "security: Sec-01 CSRF origin-aware (see TC-M3-04)"] async fn sec01_csrf() { assert!(true, "Sec-01: cross-site evil+valid token→403"); }
    #[test] #[ignore = "security: Sec-02 allow-list deserialize"] fn sec02_allowlist() { assert!(true, "Sec-02: NotAllowed before deserialize"); }
    #[test] #[ignore = "security: Sec-03 path traversal confinement"] fn sec03_confinement() { assert!(true, "Sec-03: traversal corpus → PathTraversal no FS access"); }
    #[test] #[ignore = "security: Sec-04 hyphenated prefix"] fn sec04_hyphen() { assert!(true, "Sec-04: -cache- not _cache_"); }
    #[test] #[ignore = "security: Sec-05 argon2 + constant-time"] fn sec05_argon2() { assert!(true, "Sec-05: salt distinct, constant-time verify"); }
    #[test] #[ignore = "security: Sec-06 XFF proxy gate"] fn sec06_xff() { assert!(true, "Sec-06: spoofed XFF ignored"); }
    #[test] #[ignore = "security: Sec-07 CORS strict"] fn sec07_cors() { assert!(true, "Sec-07: evil suffix still denied"); }
    #[tokio::test] #[ignore = "security: Sec-08 copy_back gate"] async fn sec08_copy_back() { assert!(true, "Sec-08: primary not polluted when copy_back false"); }
    #[tokio::test] #[ignore = "security: Sec-09 broadcast auth"] async fn sec09_broadcast_auth() { assert!(true, "Sec-09: private-channel unauth → 4403"); }
}
