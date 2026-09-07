# Testing: IntelligenceDelivery (M6 — Broadcast, Storage, JSON:API, AI)

> **Status:** P8 — 2026-09-07 | **Task:** TASK-013
> **Module:** [modules/intelligence-delivery/overview.md](../../modules/intelligence-delivery/overview.md)
> **BDD:** `@broadcast`, `@storage-readthrough`, `@jsonapi`, `@ai-sdk`, `@ai-agents`, `@vector-search` · **FSD:** FS-M6-01..07

## 1. Scope

Covers WebSocket `ShouldBroadcast` channels (`Private` `403`/`4403`, SSE `text/event-stream`), read-through `Storage` (`primary+fallback+copy_back+PathTraversal`, corpus), `JsonApiResource` sparse+include+`RelationNotLoaded`, AI SDK provider-agnostic across 12 (switch shape, `UnsupportedCapability`, opt-in `ai` flag), and AI Agents (tool+streaming+broadcast+queue+MCP flag+sub-agents+middleware+deferred loaders+`make:agent/tool`+`Str::toEmbeddings`+`dropVectorIndex` seq-scan fallback + `NotificationSkipped`).

## 2. Trace

- Stubs: `testing/stubs/m6-advanced.stub.rs` (+ `cross-nfr-contracts.stub.rs` vector/search).
- Contracts: `testing/contracts/jsonapi.schema.json` + `__snapshots__/jsonapi-user.json.snap`, `model-inspector.schema.json` + `__snapshots__/model-inspector.json.snap`, `testing/fixtures/path-traversal.corpus.json`, `testing/fixtures/vector-dim.json` adjacency (`data-orm/vector.md`).
- BDD: `bdd-scenarios.md §2.7` (7 Features: broadcast/storage/jsonapi/notifications/ai-sdk/agents/vector-ext).

## 3. Links

- Specs: [test-advanced.md](test-advanced.md)
- API: [api-broadcast](../../api/intelligence-delivery/api-broadcast.md) · [api-storage](../../api/intelligence-delivery/api-storage.md) · [api-jsonapi](../../api/intelligence-delivery/api-jsonapi.md) · [api-ai](../../api/intelligence-delivery/api-ai.md)
- Module: [broadcast](../../modules/intelligence-delivery/broadcast.md) · [storage](../../modules/intelligence-delivery/storage.md) · [jsonapi](../../modules/intelligence-delivery/jsonapi.md) · [ai-sdk](../../modules/intelligence-delivery/ai-sdk.md) · [ai-agents](../../modules/intelligence-delivery/ai-agents.md)
