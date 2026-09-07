# API: IntelligenceDelivery — Broadcast (WebSocket + SSE)

> **Status:** P8 — 2026-09-07 | **Task:** TASK-013
> **Parents:** `design/api-contracts.md §5` · `requirements/prd FR-600..601, FR-610` · `requirements/fsd FS-M6-01` · `requirements/tdd BC-6 broadcast` · `design/domain BC-6`
> **Crates:** `rustavel-broadcast` · `rustavel-ai` streaming adjacency · **BDD:** `@broadcast`, `@contracts-expansion`

## 1. Standar Global

- **Base URL:** `http://localhost:3000` (WS: `ws://localhost:3000/broadcasting/auth`)
- **Content-Type SSE:** `text/event-stream` via `Response::eventStream(stream)`.
- **Channel auth:** `WS /broadcasting/auth` checks `Authorize` gate on `Channel::Private`; `403`/`4403` when unauthorized.
- **Format Tanggal:** RFC3339.
- **Backpressure:** Bounded `mpsc(64)` → `Lagged` (not OOM) — see chaos note.

## 2. Endpoints

### 2.1 `WebSocket /broadcasting/auth` — Subscribe + push

- **URL:** `ws://localhost:3000/broadcasting/auth` (or `POST /broadcasting/auth` auth probe returning 200/403; this spec documents the WS).
- **Deskripsi:** `ShouldBroadcast { fn broadcastOn(&self)->Channel::Private("chat.1") }` emits `{ event:"UserCreated", channel:"private-chat.1", data:{...}}` over `axum::extract::ws` + `tokio-tungstenite`; authorization via `Authorize` gate.
- **Kontrol Akses:** Bearer token in `Authorization` header during WS handshake (private channels); public channels no auth; `4403` close on failure.

#### Headers (WS handshake)

| Name | Type | Req | Desc | Example |
|------|------|-----|------|---------|
| `Authorization` | `Bearer <JWT>` | yes* | *required for private/presence channels | `Bearer eyJ...` |
| `Sec-WebSocket-Protocol` | string | no | subprotocol | `broadcast` |

#### Messages

**Client subscribe:**

```json
{ "event": "subscribe", "channel": "private-chat.1" }
```

**Server push (authorized):**

```json
{ "event": "UserCreated", "channel": "private-chat.1", "data": { "id": "b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1", "name": "Ada" } }
```

**Server error (unauthorized):**

- HTTP `403 Forbidden`:

```json
{ "errors": [{ "status": "403", "code": "BroadcastError::Unauthorized", "title": "Unauthorized", "detail": "User 2 is not authorized for private-chat.1.", "meta": { "channel": "private-chat.1" } }] }
```

- WS close `4403`:

```
CloseCode 4403 (Forbidden) "BroadcastError::Unauthorized"
```

#### Usage

```bash
# probe auth (HTTP)
curl -s -X POST http://localhost:3000/broadcasting/auth \
  -H 'Authorization: Bearer '"$JWT" \
  -H 'Content-Type: application/json' \
  -d '{"channel_name":"private-chat.1"}' | jq .

# observe pushes with wscat
npx wscat -c ws://localhost:3000/broadcasting/auth -H "Authorization: Bearer $JWT"
# send {"event":"subscribe","channel":"private-chat.1"}
# expect {"event":"UserCreated","channel":"private-chat.1","data":{...}}
```

### 2.2 `GET /events` — Server-Sent Events (`eventStream`)

- **URL:** `GET /events`
- **Deskripsi:** `Response::eventStream(stream_of(["hello","world"]))` → clients stream `text/event-stream` frames; 2 frames in the test probe.
- **Kontrol Akses:** `Bearer` when configured; SSE inherits same `ShouldBroadcast` channel guard when tied to channels.

#### Response

**Headers:**

```
Content-Type: text/event-stream
Cache-Control: no-cache
Connection: keep-alive
```

**Body frames:**

```
data: hello

data: world

```

Up to 1000-token AI streaming scenario pushes `event: token` frames in order (see `api-ai`).

#### Usage

```bash
curl -N http://localhost:3000/events -H 'Accept: text/event-stream' | sed -n 's/^data: //p'
# hello
# world
```

## 3. OpenAPI 3.0 Snippet

```yaml
openapi: 3.0.3
info:
  title: Rustavel Broadcast — WebSocket + SSE
  version: 0.1.0
  description: ShouldBroadcast channels + WS auth + SSE eventStream
servers:
  - url: http://localhost:3000
paths:
  /broadcasting/auth:
    get:
      summary: WebSocket subscribe (and HTTP auth probe)
      description: WS handshake at ws://localhost:3000/broadcasting/auth; HTTP POST probe returns 200/403
      security:
        - bearerAuth: []
      responses:
        '101':
          description: Switching Protocols (WebSocket)
        '200':
          description: Auth probe OK (HTTP POST variant)
          content:
            application/json:
              example: { channel: private-chat.1, authorized: true }
        '403':
          description: Unauthorized channel
          content:
            application/json:
              example:
                errors:
                  - status: '403'
                    code: BroadcastError::Unauthorized
                    title: Unauthorized
                    meta: { channel: private-chat.1 }
  /events:
    get:
      summary: Server-Sent Events stream
      responses:
        '200':
          description: text/event-stream with data: frames
          content:
            text/event-stream:
              example: |
                data: hello

                data: world
        '401':
          description: Unauthorized
          content:
            application/json:
              example:
                errors:
                  - status: '401'
                    code: AuthError::InvalidToken
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
  schemas:
    PushEvent:
      type: object
      properties:
        event: { type: string, example: UserCreated }
        channel: { type: string, example: private-chat.1 }
        data: { type: object, example: { id: b2e8f3c0-9a1d-4f6b-8c2e-1d3fa9b7e5c1, name: Ada } }
    AiTokenFrame:
      type: object
      properties:
        event: { type: string, example: token }
        data: { type: string, example: hello }
```

## 4. Error Catalogue

| HTTP / WS | Typed error | When |
|-----------|-------------|------|
| 403 / 4403 | `BroadcastError::Unauthorized` | subscribe to `private-*` without auth `Authorize` |
| 200 | — | push `{event,channel,data}` |
| text/event-stream | `Lagged` | bounded `mpsc(64)` overflow (backpressure) |

## 5. Cross-References

- Module: [broadcast.md](../../modules/intelligence-delivery/broadcast.md)
- Design: `api-contracts.md §5 §7.1` · `tdd.md BC-6 ShouldBroadcast` · `architecture.md BC-6`
- Testing: [testing/intelligence-delivery/test-advanced.md](../../testing/intelligence-delivery/test-advanced.md) · BDD `@broadcast` · `testing/stubs/m6-advanced.stub.rs`
- AI streaming: [api-ai](./api-ai.md) §2 (streaming over WS)

## 6. A-Gate

- [x] 403/4403 + `PushEvent` examples with realistic `data.id`.
- [x] YAML valid, `security`, constraints, `text/event-stream` response.
- [x] curl/wscat valid examples.

## 7. Chaos & Resilience Note

Bounded `mpsc(64)` slow-consumer → `Lagged` (no OOM) is chaos-guarded nightly; truncated mid-`event: token` AI stream → WS close frame is success criteria (see `ai-agents.md` chain).

