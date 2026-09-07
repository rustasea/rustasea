//! M6 — Advanced Stubs (Broadcast, Storage, Search, AI SDK) — feature-gated
//! Traces to TC-M6-01..26, FR-600..612, FS-M6-01..07. Feature flags `ai`, `broadcast`, `storage` gate the live suites.

#[cfg(test)]
mod m6_advanced_stubs {
    #[tokio::test] #[ignore = "stub: WebSocket broadcast (ask) not yet implemented (FR-600)"]
    async fn tc_m6_01_ws_authorized_receives_broadcast() { assert!(true, "stub TC-M6-01: private-chat.1 authorized receives"); }
    #[tokio::test] #[ignore = "stub: channel auth gate not yet implemented (Sec-03)"]
    async fn tc_m6_02_ws_unauthorized_4403() { assert!(true, "stub TC-M6-02: 4403 Unauthorized private-chat.1"); }
    #[tokio::test] #[ignore = "stub: SSE eventStream not yet implemented (FR-601 #16)"]
    async fn tc_m6_03_sse_event_stream_two_frames() { assert!(true, "stub TC-M6-03: text/event-stream 2 data frames"); }
    #[tokio::test] #[ignore = "stub: read-through storage not yet implemented (FR-603 #9)"]
    async fn tc_m6_04_readthrough_fallback() { assert!(true, "stub TC-M6-04: local fallback bytes"); }
    #[test] #[ignore = "stub: Storage::path confinement not yet implemented (Sec-03, FR-611)"]
    fn tc_m6_05_path_traversal_rejected() { assert!(true, "stub TC-M6-05: traversal corpus → PathTraversal"); }
    #[tokio::test] #[ignore = "stub: storage copy_back not yet implemented (FR-603)"]
    async fn tc_m6_06_copy_back_promotes_to_primary() { assert!(true, "stub TC-M6-06: fallback read promotes to primary s3"); }
    #[test] #[ignore = "stub: storage NotFound not yet implemented"]
    fn tc_m6_07_storage_missing_not_found() { assert!(true, "stub TC-M6-07: NotFound for missing.bin"); }
    #[tokio::test] #[ignore = "stub: JsonApiResource not yet implemented (FR-604 #3)"]
    async fn tc_m6_08_jsonapi_fieldset_include() { assert!(true, "stub TC-M6-08: vnd.api+json attrs=name only, included posts"); }
    #[test] #[ignore = "stub: RelationNotLoaded not yet implemented"]
    fn tc_m6_09_jsonapi_not_loaded_relation() { assert!(true, "stub TC-M6-09: RelationNotLoaded posts"); }
    #[test] #[ignore = "stub: sparse fieldset matrix not yet implemented"]
    fn tc_m6_10_sparse_fieldset_matrix() { assert!(true, "stub TC-M6-10: fields name vs name,email → exact visible"); }
    #[tokio::test] #[ignore = "stub: #[deleteWhenMissingModels] not yet implemented (FR-605 #17)"]
    async fn tc_m6_11_skipped_when_missing() { assert!(true, "stub TC-M6-11: MissingModel skipped no retry"); }
    #[tokio::test] #[ignore = "stub: notification delivered not yet implemented"]
    async fn tc_m6_12_notification_delivered() { assert!(true, "stub TC-M6-12: delivered when user exists"); }
    #[tokio::test] #[ignore = "stub: Ai provider-agnostic trait not yet implemented (FR-606 #1)"]
    async fn tc_m6_13_provider_switch_same_shape() { assert!(true, "stub TC-M6-13: openai vs anthropic same AiResponse shape"); }
    #[test] #[ignore = "stub: UnsupportedCapability not yet implemented"]
    fn tc_m6_14_unsupported_capability_matrix() { assert!(true, "stub TC-M6-14: ollama×reranking, groq×files"); }
    #[test] #[ignore = "stub: ai feature-gate dep graph not yet implemented (FR-612)"]
    fn tc_m6_15_feature_gated_no_dep_without_flag() { assert!(true, "stub TC-M6-15: async-openai absent from dep graph without ai flag"); }
    #[test] #[ignore = "stub: 12-provider adapter availability not yet implemented"]
    fn tc_m6_16_twelve_providers_available() { assert!(true, "stub TC-M6-16: 12 adapters openai..openai_compatible"); }
    #[tokio::test] #[ignore = "stub: Agent streaming not yet implemented (FR-607/610 #2)"]
    async fn tc_m6_17_agent_streams_tool_then_tokens() { assert!(true, "stub TC-M6-17: SearchDocs invoked, event:token streamed WS ordered"); }
    #[test] #[ignore = "stub: make:agent not yet implemented (FR-608)"]
    fn tc_m6_18_make_agent_lint_clean() { assert!(true, "stub TC-M6-18: support_agent.rs with Agent marker"); }
    #[tokio::test] #[ignore = "stub: sub-agent + middleware not yet implemented"]
    async fn tc_m6_19_sub_agent_middleware_observed() { assert!(true, "stub TC-M6-19: ParentAgent→KnowledgeAgent logging observed"); }
    #[tokio::test] #[ignore = "stub: deferred SimilaritySearch not yet implemented"]
    async fn tc_m6_20_deferred_loader_fetch_before_tool() { assert!(true, "stub TC-M6-20: whereVectorSimilarTo before tool call"); }
    #[test] #[ignore = "stub: MCP gate not yet implemented (FR-609)"]
    fn tc_m6_21_mcp_unavailable_without_flag() { assert!(true, "stub TC-M6-21: McpUnavailable with hint"); }
    #[tokio::test] #[ignore = "stub: broadcast streaming not yet implemented (FR-610)"]
    async fn tc_m6_22_broadcast_streams_1000_tokens_ws() { assert!(true, "stub TC-M6-22: 1000 tokens event:token ordered"); }
    #[tokio::test] #[ignore = "stub: queued tool calls not yet implemented"]
    async fn tc_m6_23_tool_call_enqueued_as_job() { assert!(true, "stub TC-M6-23: tool call → job"); }
    #[tokio::test] #[ignore = "stub: Str::toEmbeddings not yet implemented (FR-602)"]
    async fn tc_m6_24_to_embeddings_dim_1536() { assert!(true, "stub TC-M6-24: Vec<f32> dim 1536"); }
    #[tokio::test] #[ignore = "stub: dropVectorIndex not yet implemented (FR-602)"]
    async fn tc_m6_25_drop_index_still_returns_seq_scan() { assert!(true, "stub TC-M6-25: DROP INDEX then whereVectorSimilarTo still returns"); }
    #[test] #[ignore = "stub: embedding provider availability not yet implemented"]
    fn tc_m6_26_embedding_providers_available() { assert!(true, "stub TC-M6-26: openai, gemini adapters"); }
}
