//! M2 — ORM & Database Stubs
//!
//! Traces to `test-plan.md` §5 M2 row, `test-cases.md` TC-M2-01..21,
//! `qa-design.md`, FR-200..210, FS-M2-01..06. pgvector feature flagged.

#[cfg(test)]
mod m2_orm_stubs {
    #[tokio::test] #[ignore = "stub: rustasea-orm drivers not yet implemented"]
    async fn tc_m2_01_driver_abstraction_postgres_vs_sqlite() { assert!(true, "stub TC-M2-01: driver-specific SQL"); }
    #[tokio::test] #[ignore = "stub: #[derive(Model)] not yet implemented"]
    async fn tc_m2_02_model_derive_timestamps_soft_delete() { assert!(true, "stub TC-M2-02: id,created_at,updated_at,deleted_at + snake_plural"); }
    #[tokio::test] #[ignore = "stub: query builder not yet implemented"]
    async fn tc_m2_03_fluent_where_first() { assert!(true, "stub TC-M2-03: where status active → Some(User 1)"); }
    #[tokio::test] #[ignore = "stub: pessimistic lock + transaction not yet implemented"]
    async fn tc_m2_04_tx_lock_serializes_concurrent_updates() { assert!(true, "stub TC-M2-04: second blocks until first commit"); }
    #[tokio::test] #[ignore = "stub: firstOrFail not yet implemented"]
    async fn tc_m2_05_first_or_fail_not_found() { assert!(true, "stub TC-M2-05: QueryError::NotFound"); }
    #[tokio::test] #[ignore = "stub: chunkBy not yet implemented"]
    async fn tc_m2_06_chunkby_10k_into_20x500() { assert!(true, "stub TC-M2-06: 10k→20 batches ×500 no OOM"); }
    #[tokio::test] #[ignore = "stub: insertOrIgnoreReturning not yet implemented"]
    async fn tc_m2_07_insert_or_ignore_returning() { assert!(true, "stub TC-M2-07: only non-conflicting inserted, IDs returned"); }
    #[tokio::test] #[ignore = "stub: whereBinary not yet implemented"]
    async fn tc_m2_08_where_binary_incompatible() { assert!(true, "stub TC-M2-08: QueryError::IncompatibleColumn"); }
    #[test] #[ignore = "stub: upsert strict uniqueBy not yet implemented (FR-204 #14)"]
    fn tc_m2_09_upsert_empty_unique_by() { assert!(true, "stub TC-M2-09: UpsertError::EmptyUniqueBy before DB trip"); }
    #[test] #[ignore = "stub: MySQL DELETE JOIN compilation not yet implemented"]
    fn tc_m2_10_mysql_delete_join() { assert!(true, "stub TC-M2-10: DELETE users FROM users JOIN orders"); }
    #[test] #[ignore = "stub: toSql/toRawSql snapshot not yet implemented"]
    fn tc_m2_11_to_sql_snapshot() { assert!(true, "stub TC-M2-11: insta snapshot of SQL"); }
    #[tokio::test] #[ignore = "stub: collection serde round-trip not yet implemented (#13)"]
    async fn tc_m2_12_collection_roundtrip_preserves_relations() { assert!(true, "stub TC-M2-12: relation_loaded(posts) with 2 posts"); }
    #[tokio::test] #[ignore = "stub: collection serde empty relation not yet implemented"]
    async fn tc_m2_13_collection_empty_relation() { assert!(true, "stub TC-M2-13: [] not None"); }
    #[tokio::test] #[ignore = "stub: migrations not yet implemented"]
    async fn tc_m2_14_migration_create_users() { assert!(true, "stub TC-M2-14: posts collection exists after migrate"); }
    #[tokio::test] #[ignore = "stub: migrate idempotence not yet implemented"]
    async fn tc_m2_15_migrate_idempotence() { assert!(true, "stub TC-M2-15: migrate→fresh--seed→migrate same version"); }
    #[test] #[ignore = "stub: fetch_mode not yet implemented (Could)"]
    fn tc_m2_16_fetch_mode_assoc() { assert!(true, "stub TC-M2-16: rows as assoc maps"); }
    #[tokio::test] #[ignore = "stub: Factory Str reset not yet implemented (FR-209)"]
    async fn tc_m2_17_factory_sequence_reset() { assert!(true, "stub TC-M2-17: sequence reset to user1 between tests"); }
    #[tokio::test] #[ignore = "stub: MissingTable not yet implemented"]
    async fn tc_m2_18_missing_table_without_migration() { assert!(true, "stub TC-M2-18: QueryError::MissingTable posts"); }
    #[tokio::test] #[ignore = "stub: pgvector whereVectorSimilarTo not yet implemented (#6)"]
    async fn tc_m2_19_pgvector_nearest_neighbor() { assert!(true, "stub TC-M2-19: 10 nearest by <=> cosine"); }
    #[tokio::test] #[ignore = "stub: PgVector extension check not yet implemented"]
    async fn tc_m2_20_pgvector_extension_missing() { assert!(true, "stub TC-M2-20: PgVectorError::ExtensionMissing hint"); }
    #[test] #[ignore = "stub: VectorDimensionMismatch not yet implemented"]
    fn tc_m2_21_vector_dimension_mismatch() { assert!(true, "stub TC-M2-21: 1536 vs 768 mismatch"); }
}
