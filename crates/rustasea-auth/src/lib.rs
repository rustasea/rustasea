//! RustaSea auth and security crate — guards, CSRF, throttling.
//!
//! Security boundary for M3 (Laravel-13 hardened defaults):
//! - JWT guard (HS256 via `jsonwebtoken`) + real `tower-sessions` session guard
//!   (store-backed login/parse/refresh/logout with id rotation)
//! - `Auth` manager: named guard registration, `Auth::extend`, typed `GuardMismatch`
//! - Origin-aware `PreventRequestForgery` (`Sec-Fetch-Site` + allow-list)
//! - `RateLimiter` trait + in-memory `Throttle` (`429` + `Retry-After`)
//! - Session/cache hardening: JSON serialization, hyphenated prefixes,
//!   `serializable_classes` allow-list
//!
//! Per ADR-0007 there are no global facades: an `AuthManager`/`SessionPolicy`
//! lives inside `AppState` and flows through `axum::extract::State`.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod csrf;
pub mod error;
pub mod guard;
pub mod jwt;
pub mod session;
pub mod session_cookie;
pub mod throttle;
pub mod users;
pub mod verification;
pub mod verify;

pub use csrf::{CsrfLayer, PreventRequestForgery, SecFetchSite};
pub use error::{AuthError, CsrfError, SerializationError, ThrottleError};
pub use guard::{Auth, AuthManager, AuthUser, Credentials, Guard, GuardRegistrar, Token};
pub use jwt::{JwtClaims, JwtConfig, JwtGuard};
pub use session::{DeserializationAllowList, SessionGuard, SessionPolicy, SessionUser};
pub use session_cookie::SessionCookieConfig;
pub use throttle::layer::{ThrottleLayer, ThrottleService};
pub use throttle::limiter::MemoryRateLimiter;
pub use throttle::{BucketState, KeyBy, Limit, RateLimiter, ThrottleConfig, ThrottleDecision};
pub use verification::{EmailVerification, MemoryEmailVerification};

/// Alias kept for documentation parity (`Auth::guard(...)`).
pub type Result<T> = error::Result<T>;

/// Reserved for tests that exercise the deny-list on `logout` (S05).
pub const _RESERVED: u8 = 0;
