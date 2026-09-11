# Sprint 07 — M6 Advanced (Broadcast, Search, Filesystem, AI SDK, Real-time)

> **Milestone:** M6 · **Window:** 2027-07-01 → 2027-12-31 · **Status:** In Progress — core surfaces landed (M5 generators, M6 broadcast/search/storage/ai), drivers/SDKs pending · Delivered ahead of 2027 window per 2026-09 automation (core landed 2026-09-09)
> **Parents:** `../roadmap.md` · `prd.md` FR-600–FR-612 · `fsd.md` FS-M6-01–FS-M6-07 · `design/architecture.md`
> **Depends On:** M1–M5 (Sprints 01–06)
> **Crates:** `rustasea-broadcast`, `rustasea-storage`, `rustasea-search`, `rustasea-ai` (`optional` feature, provider adapters feature-flagged)
> **Planning vs as-built:** Sprint scope/window is the plan. Live status: [`docs/milestones.md`](../../../../docs/milestones.md) — the authoritative as-built status source (TASK-003).

---

## 1. Goal

Differentiate RustaSea with AI-native capabilities and complete Laravel 13 parity on advanced surface — so the framework ships as a shippable `Should` that proves "Laravel ergonomics × Rust safety" differentiates.

## 2. Scope (In / Out)

**In:**
- Broadcasting & real-time: WebSocket via `axum::extract::ws` + `tokio-tungstenite`, channel auth (`private`/`presence`), `ShouldBroadcast` trait, SSE via `Response::eventStream`
- Search: `whereVectorSimilarTo` integration (full, extending M2 initial), `Str::toEmbeddings`, `dropVectorIndex`, embedding provider trait (`async-openai` + per-provider adapters)
- Filesystem: read-through `Storage` (primary + fallback disk, optional copy-back) via `object_store` + `tokio::fs`; `Storage::path()` confinement (`PathTraversal` on escape, fuzzed)
- JSON:API resources: `JsonApiResource` with sparse fieldsets (`fields[users]=name,email`), relationship inclusion (`include=posts`), links, headers, `Content-Type: application/vnd.api+json`
- Notifications & mail queued with `#[deleteWhenMissingModels]`
- AI SDK (`rustasea-ai`): provider-agnostic `AiProvider` trait over **12 providers** (OpenAI, Anthropic, Gemini, Azure, Bedrock, Groq, xAI, DeepSeek, Mistral, Ollama, OpenRouter, OpenAI-Compatible) — capabilities: text/image/audio/embeddings/reranking/files/vector-stores
- AI Agents: tools, structured output, streaming, broadcasting, queueing, sub-agents, middleware, anonymous agents, deferred loaders `SimilaritySearch`/`FileStorage`/`ToolSearch`, MCP (Model Context Protocol) tool discovery, `make:agent`/`make:tool` (`#[derive(Tool)]`)
- Feature-flag contract: `rustasea-ai` is `optional`; core `cargo check` does not pull `async-openai` (NFR-Sca-02); degraded-mode AI so `ai` feature is additive
- `app/ai/agents/` + `app/ai/tools/`, `resources/views/` (askama/minijinja)

**Out (Won't now):** Nova/Filament admin, Blade runtime full parity, hosting PaaS, PHP bridge — per `prd.md` §6.

## 3. Tasks

| # | Task | FR | FSD | Deliverable | Est. | Acceptance |
|---|------|----|-----|-------------|------|------------|
| S07-T01 | Broadcasting & real-time (WebSocket channel auth + `ShouldBroadcast` + SSE `eventStream`) | FR-600, FR-601 | FS-M6-01 | `crates/rustasea-broadcast/src/{channel,ws,sse}.rs` | M | `private-chat.1` without auth → `403`; `Response::eventStream(stream)` → `Content-Type: text/event-stream` chunks streamed; broadcast over WebSocket verified in `cargo test` ws client |
| S07-T02 | Search full (`whereVectorSimilarTo` + `toEmbeddings` + `dropVectorIndex` + embedding trait) | FR-602, FR-207 (extends) | FS-M6-02 | `crates/rustasea-search/src/{vector,embeddings,index}.rs` + `rustasea-ai` embedding capability | M | `Document` with `vector` → `whereVectorSimilarTo("vector",&emb,limit:5)` 5 nearest; `Str::toEmbeddings` trait per provider; `dropVectorIndex` removes index; `vector-dim.json` fixture covers dim mismatch |
| S07-T03 | Storage read-through + path confinement | FR-603, FR-611 | FS-M6-03 | `crates/rustasea-storage/src/{storage,path,disks}.rs` (`object_store` + `tokio::fs`) | M | `readThrough { primary:"s3", fallback:"local" }` + file only on `local` → `Storage::get("a/b.txt")` from fallback (optional copy-back); `Storage::path("../../etc/passwd")` → `PathTraversal`; `path-traversal.corpus.json` fuzz green |
| S07-T04 | JSON:API resources (`JsonApiResource` + sparse fieldsets + inclusion + links/headers) | FR-604 | FS-M6-04 | `crates/rustasea-search` or dedicated `rustasea-http` extension `jsonapi.rs` (decide by crate DAG) | M | `UserResource::new(user).include("posts")` → `{data,included,links}` with `Content-Type: application/vnd.api+json`; `fields[users]=name,email` filters attributes; `jsonapi.schema.json` snapshot green |
| S07-T05 | Queued notifications `#[deleteWhenMissingModels]` + mail defaults | FR-605 | FS-M6-05 | `crates/rustasea-queue` hook + `crates/rustasea-ai` notification path | S | Queued notification for deleted `User` → job skipped not retried; mail password-reset subject default asserted |
| S07-T06 | AI SDK provider trait over 12 providers (text/image/audio/embeddings/reranking/files/vector-stores) | FR-606, FR-612 | FS-M6-06 | `crates/rustasea-ai/src/{provider,capabilities,adapters/*}.rs` (per-provider adapter) | L | `Ai::provider("anthropic").text(prompt)` same trait per provider; 12 adapters behind per-provider features; `cargo check -p rustasea-router` does not pull `async-openai`; versioned trait + semver per adapter |
| S07-T07 | AI Agents (tools, structured output, streaming, broadcasting, queueing, sub-agents, middleware, anonymous agents, deferred loaders, MCP) | FR-607, FR-609, FR-610, FR-608 | FS-M6-07 | `crates/rustasea-ai/src/{agent,tool,middleware,streaming,mcp,loaders}.rs` + `make:agent`/`make:tool` runtime | L | `Agent` with `Tool SearchDocs` + `middleware(Logging)` → tools invoked, output streamed, middleware observed; `SimilaritySearch`/`FileStorage`/`ToolSearch` deferred loaders lazy; MCP server tools included; streaming 1k tokens → ordered `event: token` chunks over WebSocket; queued tool calls via `deadpool-redis`; `m6-advanced.stub.rs` contract passes |

## 4. Dependencies

- **Upstream:** S01–S06 — S07 is the capstone (depends on M1–M5 per README). In particular: S02 WebSocket/SSE plumbing, S03 vector initial, S05 queue/cache for AI tool queueing, S06 `make:agent`/`make:tool` scaffolds.
- **Downstream:** None — terminal milestone; feeds `1.0.0` candidate after dog-food (NFR-Usa-01 ≥4/5 on "feels like Laravel").

## 5. Deliverables

- Crates `rustasea-broadcast`, `rustasea-storage`, `rustasea-search`, `rustasea-ai` (feature-flagged).
- `app/ai/agents/` + `app/ai/tools/`, `resources/views/` (askama/minijinja), `make:agent`/`make:tool`.
- Tag `v0.7.0`; `1.0.0` candidate checklist started; docs for AI provider binding + Storage confinement + JSON:API usage.

## 6. Acceptance (Sprint Done)

- [ ] `Agent` with `Tool` streams response over WebSocket with ordered `event: token` chunks + structured output.
- [ ] `Storage::get` falls through to fallback disk; `Storage::path()` never escapes root (`PathTraversal` on traversal, fuzz corpus green).
- [ ] `JsonApiResource` renders `Content-Type: application/vnd.api+json` with sparse fieldsets + `include=posts` + links/headers.
- [ ] 12-provider AI trait `cargo check` clean; `rustasea-ai` opt-in (`cargo tree` audit proves core does not pull AI deps).
- [ ] MCP tool discovery includes MCP-provided tools; sub-agents + middleware + broadcast + queueing green.
- [ ] CI + `harness.stub.rs` + `m6-advanced.stub.rs` + snapshot tests (`jsonapi`, `route-list`, `model-inspector`) green.

## 7. Risks

- R-02 AI provider drift — per-provider adapter crate + per-provider feature-flag + trait semver.
- R-03/R-07 scope creep — M6 is `Should` (prd §6); RFC required to promote any M6 FR to `Must`; do not gate M0–M5 on AI churn.
- R-06 `pgvector` managed-DB gap — `has_extension("vector")` guard + doc workaround (already in S03; S07 extends).

---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
