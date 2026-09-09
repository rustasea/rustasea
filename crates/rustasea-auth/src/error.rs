/// Typed errors for the rustasea-auth crate.
///
/// Every variant carries a stable `code` string, a user-facing `hint`, and
/// preserves the underlying cause so callers can map to JSON error envelopes.
use thiserror::Error;

/// Alias for results produced by auth operations.
pub type Result<T> = std::result::Result<T, AuthError>;

/// Top-level authentication error type.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AuthError {
    /// No guard is registered under the requested name.
    ///
    /// `expected` names the guard the manager knows (typically the default),
    /// `actual` the name the caller asked for.
    #[error("guard mismatch: expected {expected:?}, actual {actual:?}")]
    GuardMismatch {
        /// Guard name that is registered (the manager default).
        expected: String,
        /// Guard name that was requested.
        actual: String,
    },

    /// Login credentials do not match any known user / password.
    #[error("bad credentials")]
    BadCredentials,

    /// Supplied bearer token could not be decoded or verified.
    #[error("invalid token")]
    InvalidToken,

    /// Supplied bearer token is well-formed but its `exp` claim is in the past.
    #[error("token expired")]
    ExpiredToken,

    /// Guard is disabled by configuration (e.g. `loginUsingId` in production).
    #[error("guard operation disabled: {0}")]
    Disabled(String),

    /// Underlying JWT / crypto failure with a stable code.
    #[error("token error: {0}")]
    Token(String),

    /// Password hash could not be verified (malformed stored hash).
    #[error("hash error: {0}")]
    Hash(String),

    /// Session store read/write failure (Redis/DB down).
    #[error("session store unavailable")]
    StoreUnavailable,
}

impl AuthError {
    /// Stable machine-readable code, e.g. `AuthError::GuardMismatch`.
    pub fn code(&self) -> String {
        let variant = match self {
            AuthError::GuardMismatch { .. } => "GuardMismatch",
            AuthError::BadCredentials => "BadCredentials",
            AuthError::InvalidToken => "InvalidToken",
            AuthError::ExpiredToken => "ExpiredToken",
            AuthError::Disabled(_) => "Disabled",
            AuthError::Token(_) => "Token",
            AuthError::Hash(_) => "Hash",
            AuthError::StoreUnavailable => "StoreUnavailable",
        };
        format!("AuthError::{variant}")
    }

    /// Short user-facing remediation hint.
    pub fn hint(&self) -> &'static str {
        match self {
            AuthError::GuardMismatch { .. } => {
                "Request a guard name that is registered (e.g. jwt, session)."
            }
            AuthError::BadCredentials => "Verify the email/password combination.",
            AuthError::InvalidToken => "Re-authenticate to obtain a fresh token.",
            AuthError::ExpiredToken => "Refresh the token before it expires.",
            AuthError::Disabled(_) => "This guard operation is disabled by configuration.",
            AuthError::Token(_) => "The token could not be processed; obtain a new one.",
            AuthError::Hash(_) => "Re-hash the stored password.",
            AuthError::StoreUnavailable => "The session store is unreachable; retry later.",
        }
    }
}

/// Typed CSRF (forgery-protection) errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CsrfError {
    /// Request is missing or carries an invalid `X-CSRF-TOKEN`/`_token`.
    #[error("csrf token mismatch")]
    TokenMismatch,

    /// `Sec-Fetch-Site: cross-site` with an origin outside the allow-list.
    #[error("origin {origin} is not in the csrf allow-list")]
    UntrustedOrigin {
        /// The offending Origin header value.
        origin: String,
    },
}

impl CsrfError {
    /// Stable machine-readable code, e.g. `CsrfError::UntrustedOrigin`.
    pub fn code(&self) -> String {
        let variant = match self {
            CsrfError::TokenMismatch => "TokenMismatch",
            CsrfError::UntrustedOrigin { .. } => "UntrustedOrigin",
        };
        format!("CsrfError::{variant}")
    }
}

/// Typed session/cache serialization-policy errors (hardened deserialization).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SerializationError {
    /// Attempted to deserialize a type outside `serializable_classes`.
    #[error("type {type_name} is not in the serializable_classes allow-list")]
    NotAllowed {
        /// Type name that was rejected.
        type_name: String,
    },
}

/// Typed throttle errors surfaced by the rate limiter.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ThrottleError {
    /// A custom key function returned no key (request identity unknown).
    #[error("throttle key unavailable: {0}")]
    KeyUnavailable(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Codes follow the `AuthError::Variant` wire contract (api-auth.md §4).
    #[test]
    fn error_codes_match_wire_contract() {
        assert_eq!(
            AuthError::BadCredentials.code(),
            "AuthError::BadCredentials"
        );
        assert_eq!(
            AuthError::GuardMismatch {
                expected: "jwt".into(),
                actual: "api".into()
            }
            .code(),
            "AuthError::GuardMismatch"
        );
        assert_eq!(
            CsrfError::UntrustedOrigin {
                origin: "https://evil.com".into()
            }
            .code(),
            "CsrfError::UntrustedOrigin"
        );
    }
}
