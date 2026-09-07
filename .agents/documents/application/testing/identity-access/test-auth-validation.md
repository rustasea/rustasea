# Skenario Pengujian: IdentityAccess (IDENT)

> Skenario pengujian untuk fitur IdentityAccess — Auth, CSRF, Session hardening, Validation + Throttle.
> Per `qa-design §1–2`, `bdd-gherkin` + `api-contract-test` + `security-triage` + `chaos`.

## Header & Navigation

- [Module Overview](../../modules/identity-access/overview.md)
- [API Specification](../../api/identity-access/api-auth.md) · [Validation](../../api/identity-access/api-validation.md)

## 1. Positive Cases (Happy Path)

| ID | User Story | Test Case | Pre-condition | Input Data | Expected Result | Priority |
|----|------------|-----------|---------------|------------|-----------------|----------|
| IDENT-POS-001 | US-M3-01 | JWT login+parse | user Ada `argon2` hash | `Auth::guard("jwt").login({email,password})` | token issued + `parse(token)==Ada` | High |
| IDENT-POS-002 | US-M3-02 | Same-origin valid token accepted | `csrf_origins=["https://app.example.com"]` | `POST /form` `Sec-Fetch-Site: same-origin` + valid token | `200` | High |
| IDENT-POS-003 | US-M3-02 | Older client fallback token-only accepted | older browser no `Sec-Fetch-Site` | `POST /form` valid token | `200` | High |
| IDENT-POS-004 | US-M3-03 | Default session JSON + -session- prefix | default `session.serialization="json"` | store session entry | key contains `-session-` + payload JSON | High |
| IDENT-POS-005 | US-M3-03 | Cache prefix -cache- hyphen | default `CACHE_PREFIX` | inspect `Cache::store("redis").prefix()` | contains `-cache-` not `_cache_` | High |
| IDENT-POS-006 | US-M3-04 | Middleware declaration protects handler | handler `#[middleware("auth:jwt")]` | guest GET | `401 Unauthorized` | High |
| IDENT-POS-007 | US-M3-05 | Throttle first window passes | `limit.per_minute(3).by_ip()` | 3 requests from `1.2.3.4` in 60s | all `200` | High |
| IDENT-POS-008 | US-M3-04 | Strict containment valid path | `contains_strict="admin"` | `role="admin"` | valid | Medium |

## 2. Negative Cases (Validation & Errors)

| ID | User Story | Test Case | Pre-condition | Input Data | Expected Result | Priority |
|----|------------|-----------|---------------|------------|-----------------|----------|
| IDENT-NEG-001 | US-M3-01 | GuardMismatch typed | only `jwt` registered | `Auth::guard("api").user()` | `GuardMismatch{expected:"jwt",actual:"api"}` → `401` | High |
| IDENT-NEG-002 | US-M3-01 | Bad/expired/malformed credential classified (table) | valid user | `wrong_password` / `expired_token` / `malformed_token` | `BadCredentials` / `ExpiredToken` / `InvalidToken` row-wise (BDD §2.4 outline) | High |
| IDENT-NEG-003 | US-M3-02 | Cross-site untrusted origin rejected | valid token + `Sec-Fetch-Site: cross-site` + `Origin:https://evil.com` | `POST /form` | `403 CsrfError::UntrustedOrigin` | High |
| IDENT-NEG-004 | US-M3-03 | Deser allow-list rejects outside type | `serializable_classes=["App::UserDto"]` | cached `AdminDto` `get` | `NotAllowed{type_name:"AdminDto"}` | High |
| IDENT-NEG-005 | US-M3-04 | Strict contains rejects type-mismatched ("1" vs 1) | handler `in_array:[1,2,3]` | `identifier="1"` (string) | invalid (`422`) — `1` (int) valid | High |
| IDENT-NEG-006 | US-M3-04 | Multiple field errors grouped per field | `email` invalid + `password` too short | validated | `ErrorBag` diagnostics for both `email` and `password` | High |
| IDENT-NEG-007 | US-M3-05 | Fourth request throttled | 3 successes within 60s | 4th `1.2.3.4` in same window | `429` + `Retry-After` | High |
| IDENT-NEG-008 | US-M3-04 | Authorization before body | handler `#[authorize("update", User)]` | non-owner request | `403 Forbidden` typed `AuthorizationError` | High |
| IDENT-NEG-009 | US-M3-04 | Validation runs before handler body | `#[validate] length(min=3)` name | `name="ab"` | `422` before handler body executes | High |

## 3. Monkey Testing (Chaos & Stability)

| ID | Focus | Test Case | Pre-condition | Expected Result |
|----|-------|-----------|---------------|-----------------|
| IDENT-MNK-001 | Concurrency | 10 parallel `login` same user | `argon2` + constant-time verify | all tokens valid, no race on guard registry |
| IDENT-MNK-002 | Spec drift | `Sec-Fetch-Site` absent (older browsers) burst | older clients omit header | token-only fallback passes deterministically under `oha` |
| IDENT-MNK-003 | Throttle window edge | `per_minute 60→61` (boundary N) | throttle 60 inclusive + window reset | 61st 429 then next window allowed |
| IDENT-MNK-004 | `None` as same-origin | `Sec-Fetch-Site: none` (direct nav) treated as same-origin | direct nav `POST` | accepted with valid token |
| IDENT-MNK-005 | Cache touch missing key | `touch` on missing `k` | `touch("k",60s)` | `false` not error (isolation via `async-workloads` cache trait default) |
| IDENT-MNK-006 | CORS suffix trick burst | `evil-app.example.com` vs `app.example.com` | `Origin: https://evil-app.example.com` | not allowed |

## 4. Security Testing

| ID | Role | Test Case | Action | Expected Result |
|----|------|-----------|--------|-----------------|
| IDENT-SEC-001 | Attacker | CSRF cross-site even with stolen token | `Sec-Fetch-Site: cross-site` + `Origin:https://evil.com` + valid token | `403 UntrustedOrigin` even with valid token (NFR-Sec-01) |
| IDENT-SEC-002 | Attacker | Cache deserialization allow-list bypass | craft payload for `AdminDto` not in `App::UserDto` list | `NotAllowed` before `serde` instantiation |
| IDENT-SEC-003 | Attacker | `X-Forwarded-For` spoof unless `trusted_proxies` | proxies not trusted + spoofed `9.9.9.9` but peer `1.2.3.4` | checked identity `1.2.3.4` |
| IDENT-SEC-004 | Attacker | `argon2` timing channel | meas. `verify` with wrong vs right password | constant-time (no timing oracle) |
| IDENT-SEC-005 | Auditor | Hyphen prefix collision | attempt `_cache_` prefix spoof | default `-cache-` invariant holds; `CACHE_PREFIX` override documented |
| IDENT-SEC-006 | Auditor | Strict `in_array` type confusion | `contains_strict` `"admin"` vs `"Admin"` vs `1` | value+type failure, not loose |

