# API: IntelligenceDelivery — AI SDK + Agents (12 providers + streaming + MCP + sub-agents)

> **Status:** P8 — 2026-09-07 | **Task:** TASK-013
> **Parents:** `design/api-contracts.md §5` · `requirements/prd FR-606..612` · `requirements/fsd FS-M6-05..07` · `requirements/tdd BC-6 AiProvider/Agent/Tool` · `design/domain BC-6` · BDD `@ai-sdk`, `@ai-agents`, `@vector-search`
> **Crates:** `rustasea-ai` (`optional`) · `rustasea-search` · `rustasea-storage` (deferred loaders) · `rustasea-broadcast` (WS streaming)

## 1. Standar Global

- **Provider trait:** `trait AiProvider { text/image/audio/embeddings/reranking/files/vector_stores -> AiResponse etc. }` — 12 adapters (`openai`/`anthropic`/`gemini`/`azure`/`bedrock`/`groq`/`xai`/`deepseek`/`mistral`/`ollama`/`openrouter`/`openai_compatible`) each `features=["openai"]`-flagged. Core without `ai` has no `async-openai` in `cargo tree --depth 1` (degraded-mode NFR-Sca-02).
- **Error:** `AiError::UnsupportedCapability{provider,capability}` typed; streaming without provider support → same code.
- **Agents:** `Agent::prompt(input)->Stream<Item=AiChunk>` `event: token` over WS; `Tool` `name`+`JsonSchema`/`call(args)`; anonymous `Ai::agent(|a| a.tool(MyTool))`.

## 2. Endpoints (AI producer surfaces as trait + example HTTP via app-level agent endpoints)

Example app-level endpoints are shown here — the true contract is the Rust trait; wire examples are how an app exposes `Agent` over HTTP/WS.

### 2.1 `Ai::provider(name).text/embeddings/rerank/files` — capability matrix

#### `text` (common via `Ai::provider("openai").text(prompt).send().await -> AiResponse{ text, usage, tool_calls }`)

```
POST /ai/text  { provider: "openai"|"anthropic"|..., prompt: "hello" } -> { text, usage }
```

**Params:**

| Name | Type | Req | Desc | Example |
|------|------|-----|------|---------|
| `provider` | enum 12 | yes | adapter name | `openai` |
| `prompt` | string 1..8192 | yes | text prompt | `hello` |

**Response (200):**

```json
{
  "data": { "type": "ai-response", "id": "resp-1", "attributes": { "text": "Hello! How can I help?", "usage": { "tokens": 42 } } },
  "meta": { "provider": "openai" },
  "tool_calls": []
}
```

**Switch preserves shape** (openai→anthropic same `AiResponse`).

**Error (501 UnsupportedCapability — e.g., `ollama` lacks `reranking`):**

```json
{
  "errors": [{
    "status": "501",
    "code": "AiError::UnsupportedCapability",
    "title": "Unsupported capability",
    "detail": "Provider ollama does not support capability reranking.",
    "meta": { "provider": "ollama", "capability": "reranking" }
  }]
}
```

**Usage (Rust):**

```rust
let r: AiResponse = Ai::provider("openai").text("hello").send().await?;
let r: AiResponse = Ai::provider("anthropic").text("hello").send().await?; // same shape, different adapter
```

#### `embeddings` / `toEmbeddings` (dim per provider — 1536 for `text-embedding-3-small`)

- **Descriptor:** `Str::to_embeddings("hello", provider:"openai")->Vec<f32>` via `AiProvider::embeddings` (M6 full; initial M2 via `data-orm/vector.md`).
- **URL example:** `POST /ai/embeddings  { provider:"openai", text:"hello" } -> { dim:1536, vector:[...] }`

**Response (200):**

```json
{ "data": { "type": "embedding", "attributes": { "dim": 1536, "vector": [0.012, -0.34] } } }
```

#### Other capabilities (not fully rendered wire; trait method names are the contract)

- `image(prompt)` / `audio(prompt)` / `rerank(query, docs)` / `files(op)` / `vector_stores(op)` — each may `UnsupportedCapability` per provider (fixture matrix `12×7`).

### 2.2 Agent streaming — `Agent::prompt(input)->Stream<Item=AiChunk>` over WS (+ HTTP SSE fallback)

- **URL:** WS `ws://localhost:3000/ai/stream` (or app-level `POST /ai/agent/prompt` returning WS URL) + SSE `GET /ai/stream` with `text/event-stream` frames `event: token` in order. Tool queue `WS chunk in order with token events`; broadcasts via `rustasea-broadcast` WS `event: token` (see `api-broadcast` adjacency).
- **Kontrol Akses:** `Bearer JWT` over WS handshake.

#### Sub-agents + middleware + deferred loaders

- **Sub-agents:** `ParentAgent{ sub_agents:[KnowledgeAgent] }` invoked as Tools (see `ai-agents.md`).
- **Middleware:** `Logging` observes `prompt→next→output` for both parent+sub-agent (log spy in tests).
- **Deferred loaders:** `SimilaritySearch`/`FileStorage`/`ToolSearch` inject before `Tool::call` (`whereVectorSimilarTo` via `rustasea-search`).
- **Anonymous:** `Ai::agent(|a| a.tool(MyTool)).prompt("hi").stream().await?`.
- **MCP:** `feature="mcp"` else `AgentError::McpUnavailable` with hint (wire `501`).
- **Generators:** `make:agent`/`make:tool` (see `developer-platform/generators.md` — M6 adjacency).

#### Frames

```
event: token
data: Hello

event: token
data:  world

```

Up to 1000 tokens ordered; truncation mid-`event: token` → WS close frame not silent success.

**HTTP SSE fallback example:**

```bash
curl -N -X POST http://localhost:3000/ai/agent/prompt \
  -H 'Content-Type: application/json' -H 'Authorization: Bearer '"$JWT" \
  -d '{"agent":"SupportAgent","prompt":"summarize ticket 42","stream":true}' \
  | sed -n 's/^data: //p'
```

**WS example:**

```bash
npx wscat -c ws://localhost:3000/ai/stream -H "Authorization: Bearer $JWT"
# send {"event":"prompt","agent":"SupportAgent","data":"summarize ticket 42"}
# expect ordered data: frames {event:"token", data:"Hello"} {event:"token", data:" world"}
```

**Error (McpUnavailable when flag off — §7 in `ai-agents.md`):**

```json
{
  "errors": [{
    "status": "501",
    "code": "AgentError::McpUnavailable",
    "title": "MCP unavailable",
    "detail": "Enable feature flag mcp to discover MCP tools. Hint: add features=[\"mcp\"] to rustasea-ai.",
    "meta": { "hint": "feature: mcp" }
  }]
}
```

### 2.3 `Str::toEmbeddings` + `dropVectorIndex` (vector M6 full adjacency)

- `Str::toEmbeddings("hello", provider:"openai")->Vec<f32>` dim matches column `vector(1536)` (see `data-orm/vector.md`).
- `Schema::table("products",|t| t.dropVectorIndex("embedding")) -> DROP INDEX` + seq scan fallback until reindexed (TC-M6-25).

### 2.4 Queued notifications `#[deleteWhenMissingModels]` (mail/notifications adjacency #17)

- `#[deleteWhenMissingModels] struct WelcomeNotification{ user: User }` where `User 9` deleted before queue processes → job skipped, not retried, `NotificationSkipped{ reason: MissingModel }` log. App-level endpoint is queue worker behavior not wire.

## 3. OpenAPI 3.0 Snippet

```yaml
openapi: 3.0.3
info:
  title: RustaSea AI — SDK + Agents + Streaming + MCP
  version: 0.1.0
  description: Provider-agnostic AiProvider (12) + Agent/Tool streaming/broadcast/queue/sub-agents + MCP + deferred loaders
servers:
  - url: http://localhost:3000
paths:
  /ai/text:
    post:
      summary: AiProvider text (provider-agnostic)
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [provider, prompt]
              properties:
                provider: { type: string, enum: [openai, anthropic, gemini, azure, bedrock, groq, xai, deepseek, mistral, ollama, openrouter, openai_compatible], example: openai }
                prompt: { type: string, minLength: 1, maxLength: 8192, example: hello }
      responses:
        '200':
          description: AiResponse
          content:
            application/vnd.api+json:
              example:
                data: { type: ai-response, id: resp-1, attributes: { text: Hello, usage: { tokens: 42 } } }
        '501':
          description: Unsupported capability (e.g., reranking on ollama)
          content:
            application/json:
              example:
                errors:
                  - status: '501'
                    code: AiError::UnsupportedCapability
                    meta: { provider: ollama, capability: reranking }
  /ai/embeddings:
    post:
      summary: AiProvider embeddings / Str::toEmbeddings
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [provider, text]
              properties:
                provider: { type: string, example: openai }
                text: { type: string, minLength: 1, maxLength: 8192, example: hello }
      responses:
        '200':
          description: Embedding vector
          content:
            application/json:
              example: { data: { type: embedding, attributes: { dim: 1536, vector: [0.012] } } }
  /ai/agent/prompt:
    post:
      summary: Agent prompt streaming (WS + SSE)
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [agent, prompt]
              properties:
                agent: { type: string, example: SupportAgent }
                prompt: { type: string, example: summarize ticket 42 }
                stream: { type: boolean, example: true }
                tools: { type: array, items: { type: string }, example: [SearchDocs] }
      responses:
        '200':
          description: Stream via WS/SSE (stub — wire is WS upgrade or text/event-stream)
          content:
            text/event-stream:
              example: |
                event: token
                data: Hello
                event: token
                data:  world
        '501':
          description: MCP unavailable when flag off
          content:
            application/json:
              example:
                errors:
                  - status: '501'
                    code: AgentError::McpUnavailable
                    title: MCP unavailable
  /ai/tools/{name}:
    post:
      summary: Tool call (direct)
      parameters:
        - name: name
          in: path
          required: true
          schema: { type: string, example: SearchDocs }
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              example: { query: summarize ticket 42 }
      responses:
        '200':
          description: Tool result
          content:
            application/json:
              example: { result: { docs: [{ id: 1, score: 0.9 }] } }
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
  schemas:
    AiResponse:
      type: object
      properties:
        text: { type: string }
        usage: { type: object }
        tool_calls: { type: array, items: { type: object } }
    AiChunk:
      type: object
      properties:
        event: { type: string, example: token }
        data: { type: string, example: Hello }
security:
  - bearerAuth: []
```

## 4. Error Catalogue

| HTTP | Typed error | When |
|------|-------------|------|
| 200 | `AiResponse` / `AiChunk` `event: token` | provider success |
| 501 | `UnsupportedCapability{provider,capability}` | `ollama` reranking, `groq` files etc. |
| 501 | `McpUnavailable` + hint | MCP discovery without `feature="mcp"` |
| 404 | `ToolNotFound` | agent tool not found |
| 500 | `StoreUnavailable` (queue fallback adjacency) | streamed tool queued but queue down |

## 5. Cross-References

- Module: [ai-sdk.md](../../modules/intelligence-delivery/ai-sdk.md) · [ai-agents.md](../../modules/intelligence-delivery/ai-agents.md) · `data-orm/vector.md` (M2 initial) + `developer-platform/generators.md` `make:agent/tool`
- Testing: [testing/intelligence-delivery/test-advanced.md](../../testing/intelligence-delivery/test-advanced.md) · BDD `@ai-sdk`, `@ai-agents`, `@vector-search` · `testing/stubs/m6-advanced.stub.rs`

## 6. A-Gate

- [x] 200/501 examples (reranking + McpUnavailable with hint). Fallback example in storage via corpus.
- [x] YAML valid OpenAPI 3.0 with `security`, enum on provider, constraints (minLength/maxLength, dim).
- [x] curl/wscat valid examples (Rust provider switch snipped in doc).

## 7. Chaos & Resilience Note

AI streaming 1000 tokens → WS chunk order is a bounded `mpsc` success gate (broadcast adjacency). Truncated mid-`event: token` → close frame not silent `AiResponse` is the `ai-agents.md` chaos trace; `dropVectorIndex` mid-search seq-scan fallback (TC-M6-25) is nightly chaos.


---

> **Archive note (rebrand 2026-09-09):** project renamed from Rustavel to **RustaSea**.
> This document is archived as-is under the historical `Rustavel` name for traceability;
> current branding is RustaSea (`rustasea` crates, `RustaSea` prose).
