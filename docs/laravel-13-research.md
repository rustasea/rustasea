# Laravel 13 Feature Research — RustaSea Discovery

**Date:** 2026-09-07 | **Target:** Laravel 13.0.0 (2026-03-17) + patch 13.x to v13.30.1 | **Baseline:** Laravel 12

## Sources

| # | URL | Used For |
|---|-----|----------|
| 1 | https://laravel.com/docs/releases | Official release notes (Laravel 13 headline features) |
| 2 | https://laravel.com/docs/13.x/upgrade | Upgrade guide — breaking changes, deprecations, config deltas |
| 3 | https://laravel.com/docs/13.x/ai-sdk | AI SDK docs — provider matrix, agents, tools |
| 4 | https://github.com/laravel/framework/releases/tag/v13.0.0 | Framework changelog (1183+ commits since) |
| 5 | https://github.com/laravel/framework/releases (v13.26.0–v13.30.1) | Post-GA patch notes |
| 6 | https://laravel.com/docs/13.x | Installation / PHP 8.3 requirement |

Cross-checked: (1) vs (2) vs (4) consistent on every headline feature.

## Support Policy & Requirements

| Version | PHP | Release | Bug Fixes | Security |
|---------|-----|---------|-----------|----------|
| 12 | 8.2–8.5 | 2025-02-24 | 2026-08-13 | 2027-02-24 |
| **13** | **8.3–8.5** | **2026-03-17** | Q3 2027 | 2028-03-17 |

**Minimum PHP bumped to 8.3** (from 8.2 in Laravel 12) — source (1) §PHP 8.3 + (4) PR #54763 + (2) §Updating Dependencies.

## Laravel 13 vs Laravel 12 — Positioning

Release notes (1) explicitly state: *"relatively minor upgrade in terms of effort, while still delivering substantial new capabilities — most applications may upgrade without changing much code."* Focus areas: AI-native workflows, stronger security defaults, more expressive declarative APIs. Incremental QOL shipped all year via minors rather than one breaking drop.

## Feature Inventory — Categorized (New vs Improved)

> Type: **NEW** = no equivalent in Laravel 12 · **IMPROVED** = strengthens/replaces existing · **CHANGED** = behavior/deprecation.

| # | Feature | Category | Type | vs Laravel 12 | One-Line Description |
|---|---------|----------|------|---------------|----------------------|
| 1 | Laravel AI SDK (`laravel/ai`) | AI / Core | NEW | Absent | First-party unified API for agents, text, images, audio, embeddings, reranking, files, vector stores across 12 providers (OpenAI, Anthropic, Gemini, Azure, Bedrock, Groq, xAI, DeepSeek, Mistral, Ollama, OpenRouter, OpenAI-Compatible) |
| 2 | AI Agents (Promptable, tools, structured output, streaming, broadcasting, queueing, MCP) | AI | NEW | Absent | `php artisan make:agent`, `make:tool`, `Agent` contracts, `SimilaritySearch`, `FileStorage`, `ToolSearch` deferred loading, sub-agents, middleware, anonymous agents |
| 3 | JSON:API Resources | Database / HTTP | NEW | Absent | First-party `JsonApiResource` — sparse fieldsets, relationship inclusion, links, JSON:API headers |
| 4 | Queue Routing by Class (`Queue::route()`) | Queue/Jobs | NEW | Absent — only per-dispatch `onQueue`/`onConnection` | Central routing rules: `Queue::route(ProcessPodcast::class, connection:'redis', queue:'podcasts')` |
| 5 | Cache TTL Extension (`Cache::touch()`) | Cache | NEW | Absent | Extend TTL without get/set; added to `Store` + `Repository` contracts (breaking for custom stores) |
| 6 | Semantic / Vector Search | Database / AI | NEW | Absent | `whereVectorSimilarTo()`, `Str::toEmbeddings()`, `dropVectorIndex()`/`vector` Blueprint methods, pgvector + MariaDB vector distance |
| 7 | Expanded PHP Attributes (declarative) | DX / Core | NEW | Limited attributes | `#[Middleware]`, `#[Authorize]`, `#[Tries]`, `#[Backoff]`, `#[Timeout]`, `#[FailOnTimeout]`, plus Eloquent/events/notifications/validation/testing/resource attributes; `#[RepairToolCalls]`, `#[WithoutBroadcasting]`, Artisan `#[Usage]`/`#[Help]`/`#[Hidden]` etc. |
| 8 | Laravel Cloud Facade & Cloud Queue | Queue / DevOps | NEW | Absent | `Cloud` facade, `managedQueues()`, `totalXSize`, `pendingSize`/`delayedSize`/`reservedSize`/`creationTimeOfOldestPendingJob` added to Queue contract |
| 9 | Read-through Filesystem | Filesystem | NEW | Absent | Primary + fallback disk with optional copy; `Storage::path()` confined to disk root |
| 10 | Schedule Pause/Resume | Scheduling | NEW | Absent | `php artisan schedule:pause` / `schedule:resume` + `SchedulePaused`/`ScheduleResumed` events, `withScheduling` now deferred |
| 11 | Request Forgery Protection (origin-aware) | Auth/Security | IMPROVED | `VerifyCsrfToken` → `PreventRequestForgery` | Adds `Sec-Fetch-Site` origin verification; old classes remain as deprecated aliases; new `preventRequestForgery()` middleware API |
| 12 | Cache & Session Hardening | Cache / Security | IMPROVED | Open unserialize defaults | `serializable_classes: false` by default, `session.serialization: json` (was `php`), cache/Redis prefixes now hyphenated (`-cache-` vs `_cache_`) |
| 13 | Eloquent Collection Serialization | Database/Eloquent | IMPROVED | Relations lost on serialize | `Collection` deserialization now restores eager-loaded relations |
| 14 | Database Upsert & Delete Improvements | Database | IMPROVED | Silent failure | `upsert` now throws on empty `uniqueBy`; MySQL `DELETE … JOIN … ORDER BY/LIMIT` now compiled (may throw where previously silently ignored) |
| 15 | Eloquent / Query Builder Additions | Database/Eloquent | IMPROVED | Missing methods | `insertOrIgnoreReturning()`, `saveOrIgnore()`, `refreshForUpdate()`, `whereBinary()`, `chunkBy()`, `orWhereKey()`/`orWhereKeyNot()`, `StraightJoin`, `PDO FETCH` modes, `Arr::dot($depth)` |
| 16 | Event/Queue Contracts Expansion | Queue / Events | IMPROVED | Incomplete contracts | `JobAttempted::$exception` (was `bool $exceptionOccurred`), `QueueBusy::$connectionName` (was `$connection`), `Dispatcher::dispatchAfterResponse`, `ResponseFactory::eventStream`, `MustVerifyEmail::markEmailAsUnverified`, `Monitor::starting` |
| 17 | Mail/Notification Defaults | Mail/Notification | IMPROVED | Old subjects | Password-reset subject `"Reset your password"` (was `"Reset Password Notification"`); queued notifications now respect `#[DeleteWhenMissingModels]` |
| 18 | HTTP Client & Process | HTTP / Tooling | IMPROVED | Ad-hoc | `Response::throw($callback)` signatures formalized, `CarbonInterval` for process timeouts, `FakeInvokedProcess::stop()`/`ensureNotTimedOut()`, dedicated idle-timeout exception |
| 19 | Routing & Validation | Routing / Validation | IMPROVED | Registration-order dependent | Domain routes prioritized before non-domain (catch-all subdomains now stable); `in_array`/`contains`/`doesnt_contain` now strict comparison; `ErrorBag` attribute for FormRequest |
| 20 | Observability & Tooling | DX / Observability | IMPROVED | Missing | `ModelInspector` returns data object for `show:model`, `route:list` shows binding fields, paginator views renamed `bootstrap-3`, `Str` factories reset between tests, `Manager::extend` closures now bound to manager |

> Count: 20 distinct — exceeds ≥15 requirement. Types: 10 NEW, 10 IMPROVED/CHANGED.

## Breaking & Deprecation Highlights (High → Very Low Impact)

| Impact | Change | Replacement |
|--------|--------|-------------|
| High | PHP ≥8.3 required; `laravel/framework ^13.0`, `phpunit ^12`, `pest ^4`, `laravel/tinker ^3`, `laravel/boost ^2` | Update `composer.json` |
| High | CSRF middleware `VerifyCsrfToken` renamed | `PreventRequestForgery` (alias remains, deprecated) |
| Medium | `cache.serializable_classes` must allow-list objects | Explicit class list or migrate to arrays |
| Medium | `upsert` with empty `uniqueBy` now throws | Pass valid `uniqueBy` or rely on PK/unique indexes |
| Low | Cache prefixes hyphenated; session cookie names change | Set `CACHE_PREFIX`/`REDIS_PREFIX`/`SESSION_COOKIE` to preserve |
| Low | `Container::call` nullable class defaults now return `null` (was auto-resolved instance) | Update call-site logic |
| Low | `JobAttempted::$exceptionOccurred` → `$exception` | Update listeners |
| Low | `QueueBusy::$connection` → `$connectionName` | Rename property access |
| Low | Polymorphic pivot table name now plural | Set explicit `$table` on custom pivots |
| Very Low | `Cache\Store::touch`, `Queue::pendingSize` etc., `Dispatcher::dispatchAfterResponse`, `MustVerifyEmail::markEmailAsUnverified` added to contracts | Implement on custom drivers |

## RustaSea Implications (Opportunities to Borrow)

1. **AI-native is the headline** — parity via a `rustasea-ai` crate with provider-agnostic trait + agentic workflow (tools, streams, queues) would differentiate from Goravel.
2. **Declarative attributes** — Rust proc-macros (`#[middleware]`, `#[tries(3)]`, `#[authorize]`) are idiomatic; mirrors Laravel 13's expanded attribute surface.
3. **Queue routing + Cloud** — central `Queue::route::<Job>(queue:)` maps cleanly to a Rust builder/registry.
4. **Vector search first-class** — `whereVectorSimilarTo` + `toEmbeddings` suggests `rustasea` should ship a `vector` query extension + embedding trait from day one (pgvector).
5. **Security defaults hardening** — JSON session serialization + `serializable_classes` allow-list is a pattern to adopt in Rust (serde allow-listing) even more naturally.

## Verification

- 2-source cross-check: headline features (AI SDK, JSON:API, Queue routing, Cache touch, Vector search) appear in both (1) and (4); breaking changes in (2) match commits in (4).
- Patch releases v13.26–v13.30 confirm vector, queue, redis-cluster, and read-through features continued post-GA.
