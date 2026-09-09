# Skenario Pengujian: IntelligenceDelivery (INTEL)

> Skenario pengujian untuk fitur IntelligenceDelivery — Broadcast/SSE, Storage, JSON:API, AI SDK, AI Agents, Vector full, Notifications.
> Per `qa-design §1–2`, `bdd-gherkin` + `api-contract-test` + `security-triage` + `chaos-engineering`.

## Header & Navigation

- [Module Overview](../../modules/intelligence-delivery/overview.md)
- API: [broadcast](../../api/intelligence-delivery/api-broadcast.md) · [storage](../../api/intelligence-delivery/api-storage.md) · [jsonapi](../../api/intelligence-delivery/api-jsonapi.md) · [ai](../../api/intelligence-delivery/api-ai.md)

## 1. Positive Cases (Happy Path)

| ID | User Story | Test Case | Pre-condition | Input Data | Expected Result | Priority |
|----|------------|-----------|---------------|------------|-----------------|----------|
| INTEL-POS-001 | US-M6-01 | Authorized WS receives broadcast | `ShouldBroadcast{ private-chat.1}` + user 1 authorized | user 1 subscribes + event broadcast | push `{event:UserCreated, data}` | High |
| INTEL-POS-002 | US-M6-01 | SSE streams 2 frames | `Response::eventStream(stream_of(["hello","world"]))` | client connects `/events` | `Content-Type: text/event-stream` + 2 `data:` frames | High |
| INTEL-POS-003 | US-M6-02 | Read falls through to fallback | `a/b.txt` only on `local` (read-through s3→local) | `Storage::get("a/b.txt")` | bytes from `local` | High |
| INTEL-POS-004 | US-M6-02 | copy_back writes to primary after fallback read | `copy_back:true` + only fallback | `Storage::get("a/b.txt")` | `s3` now also stores `a/b.txt` | High |
| INTEL-POS-005 | US-M6-03 | JSON:API fieldset+include | user Ada with posts loaded | `UserResource::new(user).include("posts").fields(["name"])` | `application/vnd.api+json` attributes only `name` + `included` posts | High |
| INTEL-POS-006 | US-M6-05 | Provider switch preserves shape | `openai` text success | `anthropic` same prompt | same `AiResponse{ text, usage}` shape | High |
| INTEL-POS-007 | US-M6-06 | Agent invokes tool + streams | `SupportAgent` with `SearchDocs` | `prompt "summarize ticket 42"` | `SearchDocs::call` invoked + chunks `event: token` streaming | High |
| INTEL-POS-008 | US-M6-06 | Generator scaffolds agent | `cargo rustasea make:agent SupportAgent` | generate | `app/ai/agents/support_agent.rs` with `Agent` + `rustfmt`-clean | High |
| INTEL-POS-009 | US-M6-06 | Sub-agent + middleware observed | `ParentAgent{sub_agents:[KnowledgeAgent], middleware:[Logging]}` requires sub-agent | `prompt` requiring sub-agent | `Logging` observes parent+sub-agent | High |
| INTEL-POS-010 | US-M6-06 | Broadcast streams agent output WS | agent streaming 1000 tokens + WS subscriber | stream | subscriber receives chunks in order `event: token` | High |
| INTEL-POS-011 | US-M6-06 | Deferred similarity loads before tool | `SimilaritySearch` deferred loader | `prompt` | `whereVectorSimilarTo` fetches docs before `Tool::call` | High |
| INTEL-POS-012 | US-M6-06 | Queued tool calls enqueued as jobs | agent tool configured `queue` | `Tool` invoke | job enqueued (JobId) | High |
| INTEL-POS-013 | US-M6-07 | Embedding returns expected dim | `openai` `text-embedding-3-small` | `Str::toEmbeddings("hello", provider:"openai")` | `Vec<f32>` dim 1536 | High |
| INTEL-POS-014 | US-M6-07 | Dropping vector index still returns search results | `products_embedding_index` exists | `dropVectorIndex("embedding")` | subsequent `whereVectorSimilarTo` still returns (seq scan) | High |
| INTEL-POS-015 | US-M6-02 | Missing files reported uniformly (outline over paths) | no store contains file | `Storage::get` | `NotFound` uniform | Medium |

## 2. Negative Cases (Validation & Errors)

| ID | User Story | Test Case | Pre-condition | Input Data | Expected Result | Priority |
|----|------------|-----------|---------------|------------|-----------------|----------|
| INTEL-NEG-001 | US-M6-01 | Unauthorized WS rejected | user 2 not authorized `private-chat.1` | subscribe over WS | `Unauthorized private-chat.1` / close `4403` | High |
| INTEL-NEG-002 | US-M6-02 | Path traversal rejected | none (validation) | `path "../../etc/passwd"` via `Storage::path` | `PathTraversal` without fs access (corpus row) | High |
| INTEL-NEG-003 | US-M6-03 | Include not-loaded → RelationNotLoaded | posts not eager-loaded | `include("posts")` then `to_response()` | `RelationNotLoaded{ relation:"posts"}` | High |
| INTEL-NEG-004 | US-M6-05 | Unsupported capability per provider (table) | `ollama` lacks `reranking`, `groq` lacks `files` | request that capability | `UnsupportedCapability{provider,capability}` typed | High |
| INTEL-NEG-005 | US-M6-06 | MCP unavailable when flag off | `mcp` feature disabled | `Agent` MCP tool discovery | `McpUnavailable` with feature hint | High |
| INTEL-NEG-006 | US-M6-04 | Notification skipped MissingModel no retry | queued `WelcomeNotification{user:9}` deleted before process + `#[deleteWhenMissingModels]` | queue processes notification | skipped `MissingModel`, no retry | High |
| INTEL-NEG-007 | US-M6-02 | Missing store also NotFound (update before) | `Storage::get` fallback missing | both missing | `NotFound` (same as fallback case) | Medium |
| INTEL-NEG-008 | US-M6-06 | Embedding dim mismatch | `vector(1536)` vs query 768 | `whereVectorSimilarTo` | `VectorDimensionMismatch` | Medium |

## 3. Monkey Testing (Chaos & Stability)

| ID | Focus | Test Case | Pre-condition | Expected Result |
|----|-------|-----------|---------------|-----------------|
| INTEL-MNK-001 | Backpressure | slow consumer bounded `mpsc(64)` | WS/SSE lag | `Lagged` signal, no OOM/stall |
| INTEL-MNK-002 | Truncated stream | drop WS mid-`event: token` stream | AI streaming 1k tokens | client sees close frame, not partial success `AiResponse` |
| INTEL-MNK-003 | Trail traversal fuzz | `%2e%2e`, `..\\`, long chains, symlink corpus | `testing/fixtures/path-traversal.corpus.json` | all variants → `PathTraversal` |
| INTEL-MNK-004 | Vector index dropped mid-search | `dropVectorIndex` while `whereVectorSimilarTo` active | active query | fallback seq scan returns results (TC-M6-25) |
| INTEL-MNK-005 | Provider 500 injection | inject 500 per provider | `Ai::provider(name).text(...)` | `AiError::Provider` typed not panic; capability fallback not masked |
| INTEL-MNK-006 | Opt-in flag gate | no `ai` feature | `cargo check -p rustasea-router` | no `async-openai` in `cargo tree` |
| INTEL-MNK-007 | Sparse fieldset controls exact visible | `fields name` → `name`, `fields name,email` → `name,email` (outline) | `UserResource` sparse matrix | attributes contain exactly visible list |

## 4. Security Testing

| ID | Role | Test Case | Action | Expected Result |
|----|------|-----------|--------|-----------------|
| INTEL-SEC-001 | Attacker | Private-channel without auth | subscribe `private-chat.1` unauthenticated | `403`/`4403` (RegisterSec03 related) |
| INTEL-SEC-002 | Attacker | Storage traversal bypass | `%2e%2e%2f` double-encode `..` | decoded then canonical `starts_with(disk_root)` still `PathTraversal` |
| INTEL-SEC-003 | Attacker | JSON:API includes leak | `include=posts` without eager scope but cache poisoned | still `RelationNotLoaded` (not data leak) |
| INTEL-SEC-004 | Attacker | AI provider prompt injection via files/vector_stores | evil `FileOp` | provider boundary sanitized, not hostPath leak |
| INTEL-SEC-005 | Auditor | Notification queue payload allow-list | `Notification{user}` serialized `User` model | queued payload `serializable_classes` gating adjacency (RegisterSec02) |
| INTEL-SEC-006 | Auditor | WS `Lagged` does not disclose other-channel data | mpsc overflow on `private-chat.1` | dropped chunk not from `private-chat.2` (isolation) |


---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
