# ADR-0001 — jsonwebtoken 10 and workspace MSRV 1.88

> **Status:** Accepted
> **Date:** 2026-09-11
> **Deciders:** Tech Lead, Platform
> **Milestone:** M3 (Auth; spans workspace toolchain)
> **Related:** PR #1 (`dependabot/cargo/cargo-faef625f8c`) · `prd.md` C-01 / NFR-Com-01 · ADR-003

## Context

Dependabot opened PR #1 to bump `jsonwebtoken` from 9.x to 10.x. `jsonwebtoken`
10.x is published with `edition = "2024"` and declares `rust-version = "1.88"`,
so the dependency graph can no longer be built by the workspace's previous MSRV
of Rust 1.80.

`jsonwebtoken` 10 also made its cryptographic backend an explicit, mandatory
feature choice: with `default-features = false` and no backend selected, the
`encode`/`decode` entry points have no implementation and panic at runtime.
The bump therefore requires both a toolchain decision and a feature decision —
a manifest-only version change is not sufficient.

The rest of the workspace remains on `edition = "2021"`; only the dependency
requires the newer compiler. The CI toolchain is `stable` (currently 1.98.1),
so no CI runner change is needed.

## Decision

1. **Adopt `jsonwebtoken` 10.x** with the pure-Rust RustCrypto backend:

   ```toml
   jsonwebtoken = { version = "10", default-features = false, features = ["rust_crypto"] }
   ```

   `rust_crypto` avoids a C toolchain / system `libcrypto` dependency and keeps
   cross-compilation and `cargo check` hermetic on CI.

2. **Raise the workspace MSRV from 1.80 to 1.88** via
   `[workspace.package] rust-version = "1.88"` and update the MSRV references
   in `prd.md` (C-01, NFR-Com-01), `brd.md`, `architecture.md`,
   `design-system.md`, `test-plan.md`, and `manifest.md`.

## Consequences

- The minimum supported toolchain is now Rust **1.88**; contributors and CI must
  use 1.88+ (`stable` is already far ahead).
- `jsonwebtoken` 10.4.0 resolves with RustCrypto crates (`hmac`, `sha2`,
  `ed25519-dalek`, `p256`, `p384`, `rsa`) in `Cargo.lock`.
- The workspace itself stays on `edition = "2021"`; no source migration to
  edition 2024 is implied.
- ADR-003's "MSRV 1.80+" statement is superseded by this ADR; its single-`tokio`
  runtime decision is unaffected.
- The declarative `rust-version` key lives in `[workspace.package]`; member
  crates do not currently inherit it, so the value is documentation/policy
  rather than a per-crate `cargo` enforcement (a future change can add
  `rust-version.workspace = true` to each member).

## Alternatives

| Option | Pros | Cons | Verdict |
|---|---|---|---|
| `jsonwebtoken` 10 + `rust_crypto` (chosen) | Pure Rust; no system TLS/crypto deps; hermetic CI | MSRV 1.88; larger RustCrypto dependency subtree | **Chosen** |
| `jsonwebtoken` 10 + `aws_lc_rs` | AWS-maintained backend; FIPS-oriented | Pulls `aws-lc-rs` (C/asm build, `cmake`/`nasm`); heavier CI | Rejected |
| Pin `jsonwebtoken` 10.3.0 to keep MSRV 1.85 | Smaller toolchain jump | Still raises MSRV above 1.80; adds a version-pin to revisit; does not avoid the backend feature requirement | Rejected |
| Stay on `jsonwebtoken` 9.x | No MSRV change | Leaves the security/Dependabot update unmerged; diverges from upstream | Rejected |

## References

- PR #1 — `dependabot/cargo/cargo-faef625f8c`.
- `Cargo.toml` — `[workspace.package] rust-version`, `jsonwebtoken` feature.
- `Cargo.lock` — `jsonwebtoken` 10.4.0 dependency list.
- ADR-003 — async stack (MSRV clause superseded here).
