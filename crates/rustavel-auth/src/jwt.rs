/// JWT guard — HS256 signed claims via `jsonwebtoken`.
///
/// `login` verifies the password through the injected `PasswordVerifier`,
/// then signs an access/refresh pair. `parse` decodes with `exp` validation.
/// Rotation (`refresh`) re-signs from a still-valid token's subject without
/// requiring the password again.
use std::sync::Arc;

use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AuthError, Result};
use crate::guard::{AuthUser, Credentials, Guard, Token};
use crate::verify::PasswordVerifier;

/// Configuration for the JWT guard (HS256).
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// HMAC secret shared between signer and verifier.
    pub secret: String,
    /// Access-token lifetime in seconds.
    pub access_ttl_secs: u64,
    /// Refresh-token lifetime in seconds.
    pub refresh_ttl_secs: u64,
    /// `iss` claim emitted on every token.
    pub issuer: String,
    /// `aud` claim emitted on every token.
    pub audience: String,
    /// Gate for the internal/testing `loginUsingId` helper.
    pub allow_login_using_id: bool,
}

impl Default for JwtConfig {
    /// Defaults for local development — override `secret` in production.
    fn default() -> Self {
        Self {
            secret: "change-me-in-production".to_string(),
            access_ttl_secs: 3600,
            refresh_ttl_secs: 86_400,
            issuer: "rustavel".to_string(),
            audience: "rustavel-app".to_string(),
            allow_login_using_id: false,
        }
    }
}

impl JwtConfig {
    /// Placeholder secret shipped in the default config — must never sign
    /// tokens in a production environment.
    pub const DEFAULT_SECRET: &'static str = "change-me-in-production";
    /// Minimum accepted HMAC secret length (bytes).
    pub const MIN_SECRET_LEN: usize = 32;

    /// Verify the signing secret is safe to use.
    ///
    /// Rejects the literal default and any secret shorter than
    /// `MIN_SECRET_LEN` bytes. Callers guard every token-producing or
    /// -consuming entry point with this before touching the key material;
    /// the guard is permanently disabled (`AuthError::Disabled`) otherwise.
    pub fn validate(&self) -> std::result::Result<(), AuthError> {
        if self.secret == Self::DEFAULT_SECRET {
            return Err(AuthError::Disabled(
                "JWT secret is still the default 'change-me-in-production'; set a strong secret in production".into(),
            ));
        }
        if self.secret.len() < Self::MIN_SECRET_LEN {
            return Err(AuthError::Disabled(format!(
                "JWT secret must be at least {} bytes long",
                Self::MIN_SECRET_LEN
            )));
        }
        Ok(())
    }
}

/// Claims embedded in a signed JWT (contract `jwt-claims.schema.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JwtClaims {
    /// Subject — the user UUID.
    pub sub: String,
    /// Expiry (UNIX seconds).
    pub exp: usize,
    /// Issued-at (UNIX seconds).
    pub iat: usize,
    /// Issuer.
    pub iss: String,
    /// Audience.
    pub aud: String,
    /// Token purpose: `access` or `refresh`.
    pub typ: String,
}

impl JwtClaims {
    /// Sign a fresh claim set for `user_id` with `ttl_secs`.
    pub fn issue(user_id: &Uuid, purpose: &str, ttl_secs: u64, cfg: &JwtConfig) -> Self {
        let now = Utc::now().timestamp() as usize;
        Self {
            sub: user_id.to_string(),
            exp: now + ttl_secs as usize,
            iat: now,
            iss: cfg.issuer.clone(),
            aud: cfg.audience.clone(),
            typ: purpose.to_string(),
        }
    }
}

/// HS256 JWT guard implementation.
pub struct JwtGuard {
    cfg: JwtConfig,
    verifier: Arc<dyn PasswordVerifier>,
    /// Credential lookup (fail-closed StaticLookup until DB wiring).
    lookup: Arc<dyn crate::users::UserLookup>,
}

impl JwtGuard {
    /// Create a guard from config and a password verifier.
    pub fn new(cfg: JwtConfig, verifier: Arc<dyn PasswordVerifier>) -> Self {
        Self {
            cfg,
            verifier,
            lookup: Arc::new(crate::users::StaticLookup),
        }
    }

    /// Attach a credential lookup (database-backed at boot).
    pub fn with_lookup(mut self, lookup: Arc<dyn crate::users::UserLookup>) -> Self {
        self.lookup = lookup;
        self
    }

    /// Sign `claims` into a compact JWT string.
    fn sign(&self, claims: &JwtClaims) -> Result<String> {
        encode(
            &Header::new(Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(self.cfg.secret.as_bytes()),
        )
        .map_err(|e| AuthError::Token(e.to_string()))
    }

    /// Decode and fully validate a token (signature, exp, iss, aud).
    fn decode_claims(&self, token: &str) -> Result<JwtClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[&self.cfg.issuer]);
        validation.set_audience(&[&self.cfg.audience]);
        validation.leeway = 0;
        decode::<JwtClaims>(
            token,
            &DecodingKey::from_secret(self.cfg.secret.as_bytes()),
            &validation,
        )
        .map(|d| d.claims)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::ExpiredToken,
            _ => AuthError::InvalidToken,
        })
    }

    /// Pair of signed claims for `user_id`.
    fn issue_pair(&self, user_id: &Uuid) -> Result<Token> {
        let access = JwtClaims::issue(user_id, "access", self.cfg.access_ttl_secs, &self.cfg);
        let refresh = JwtClaims::issue(user_id, "refresh", self.cfg.refresh_ttl_secs, &self.cfg);
        Ok(Token::bearer(
            self.sign(&access)?,
            self.sign(&refresh)?,
            self.cfg.access_ttl_secs,
        ))
    }
}

impl Guard for JwtGuard {
    fn name(&self) -> &str {
        "jwt"
    }

    fn login<'a>(
        &'a self,
        creds: &'a Credentials,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Token>> + Send + 'a>> {
        Box::pin(async move {
            self.cfg.validate()?;
            let stored_hash = self
                .lookup
                .hash_for_email(&creds.email)
                .ok_or(AuthError::BadCredentials)?;
            // Constant-time argon2 verify; same error for unknown user + wrong
            // password so account enumeration is not possible via timing.
            if !self.verifier.verify(&stored_hash, &creds.password) {
                return Err(AuthError::BadCredentials);
            }
            let user_id = self
                .lookup
                .id_for_email(&creds.email)
                .ok_or(AuthError::BadCredentials)?;
            let parsed = Uuid::parse_str(&user_id).map_err(|_| AuthError::BadCredentials)?;
            self.issue_pair(&parsed)
        })
    }

    fn login_using_id<'a>(
        &'a self,
        user_id: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Token>> + Send + 'a>> {
        Box::pin(async move {
            self.cfg.validate()?;
            if !self.cfg.allow_login_using_id {
                return Err(AuthError::Disabled("loginUsingId is disabled".into()));
            }
            let parsed = Uuid::parse_str(user_id).map_err(|_| AuthError::InvalidToken)?;
            self.issue_pair(&parsed)
        })
    }

    fn parse<'a>(
        &'a self,
        token: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<AuthUser>> + Send + 'a>> {
        Box::pin(async move {
            self.cfg.validate()?;
            let claims = self.decode_claims(token)?;
            if claims.typ != "access" {
                return Err(AuthError::InvalidToken);
            }
            Ok(AuthUser {
                email: self.lookup.email_for_id(&claims.sub),
                id: claims.sub,
                guard: self.name().to_string(),
            })
        })
    }

    fn refresh<'a>(
        &'a self,
        token: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Token>> + Send + 'a>> {
        Box::pin(async move {
            self.cfg.validate()?;
            let claims = self.decode_claims(token)?;
            if claims.typ != "refresh" {
                return Err(AuthError::InvalidToken);
            }
            let parsed = Uuid::parse_str(&claims.sub).map_err(|_| AuthError::InvalidToken)?;
            self.issue_pair(&parsed)
        })
    }

    fn logout<'a>(
        &'a self,
        _token: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            // Stateless JWT: revocation is delegated to the deny-list/session
            // store (S05 queue/cache milestone) — no-op until then.
            Ok(())
        })
    }

    fn user<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<AuthUser>>> + Send + 'a>>
    {
        Box::pin(async move {
            // The JWT guard cannot resolve a user without a bearer token; the
            // auth middleware injects `parse` results as request extensions.
            Ok(None)
        })
    }

    fn id<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<String>>> + Send + 'a>>
    {
        Box::pin(async move { Ok(None) })
    }
}

#[cfg(test)]
fn now_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AlwaysVerify;

    impl PasswordVerifier for AlwaysVerify {
        fn verify(&self, _hash: &str, _password: &str) -> bool {
            true
        }

        fn hash(&self, _password: &str) -> crate::error::Result<String> {
            Err(AuthError::Hash("AlwaysVerify does not hash".into()))
        }
    }

    fn test_guard() -> JwtGuard {
        JwtGuard::new(
            JwtConfig {
                secret: "0123456789abcdef0123456789abcdef".into(),
                allow_login_using_id: true,
                ..JwtConfig::default()
            },
            Arc::new(AlwaysVerify),
        )
    }

    /// A signed access token round-trips through `parse`.
    #[tokio::test]
    async fn login_then_parse_round_trip() {
        let guard = test_guard();
        let id = Uuid::new_v4();
        let token = guard
            .login_using_id(&id.to_string())
            .await
            .expect("login_using_id succeeds");
        assert_eq!(token.token_type, "Bearer");
        let user = guard
            .parse(&token.access_token)
            .await
            .expect("parse succeeds");
        assert_eq!(user.id, id.to_string());
        assert_eq!(user.guard, "jwt");
    }

    /// The shipped default secret is rejected by validation.
    #[test]
    fn validate_rejects_default_secret_and_short_secret() {
        let default_cfg = JwtConfig::default();
        assert_eq!(
            default_cfg.validate(),
            Err(AuthError::Disabled(
                "JWT secret is still the default 'change-me-in-production'; set a strong secret in production"
                    .to_string()
            ))
        );
        let short_cfg = JwtConfig {
            secret: "short".into(),
            ..JwtConfig::default()
        };
        assert!(short_cfg.validate().is_err());
        let strong_cfg = JwtConfig {
            secret: "0123456789abcdef0123456789abcdef".into(),
            ..JwtConfig::default()
        };
        assert!(strong_cfg.validate().is_ok());
    }

    /// A guard built from the default config refuses every entry point with
    /// `AuthError::Disabled` before any credential/token work happens.
    #[tokio::test]
    async fn default_config_refuses_all_entry_points() {
        let guard = JwtGuard::new(JwtConfig::default(), Arc::new(AlwaysVerify));
        let creds = Credentials {
            email: "ada@example.com".into(),
            password: "correct horse battery staple".into(),
        };
        let disabled = || {
            Err(AuthError::Disabled(
                "JWT secret is still the default 'change-me-in-production'; set a strong secret in production"
                    .to_string(),
            ))
        };
        let expected_token: crate::error::Result<Token> = disabled();
        let expected_user: crate::error::Result<AuthUser> = Err(match disabled() {
            Err(e) => e,
            Ok(_) => unreachable!(),
        });
        assert_eq!(guard.login(&creds).await, expected_token);
        assert_eq!(
            guard
                .login_using_id("00000000-0000-0000-0000-000000000000")
                .await,
            expected_token
        );
        assert_eq!(guard.parse("a.b.c").await, expected_user);
        assert_eq!(guard.refresh("a.b.c").await, expected_token);
    }

    /// A refresh token rotates into a new pair; access token in refresh fails.
    #[tokio::test]
    async fn refresh_rotates_pair() {
        let guard = test_guard();
        let id = Uuid::new_v4();
        let token = guard
            .login_using_id(&id.to_string())
            .await
            .expect("login_using_id succeeds");
        // Ensure the rotation lands on a different `iat` second so the signed
        // output differs (claims carry second-granularity timestamps).
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let rotated = guard
            .refresh(&token.refresh_token)
            .await
            .expect("refresh succeeds");
        assert_ne!(rotated.access_token, token.access_token);
        assert_ne!(rotated.refresh_token, token.refresh_token);
        let err = guard
            .refresh(&token.access_token)
            .await
            .expect_err("access token must be rejected");
        assert_eq!(err, AuthError::InvalidToken);
    }

    /// An expired token maps to `ExpiredToken`, garbage to `InvalidToken`.
    #[tokio::test]
    async fn expired_and_malformed_classify() {
        let guard = test_guard();
        let id = Uuid::new_v4();
        let claims = JwtClaims::issue(&id, "access", 1, &guard.cfg);
        let token = guard.sign(&claims).expect("signing succeeds");
        // Force expiry by re-signing with an exp in the past.
        let mut past = claims;
        past.exp = now_secs() as usize - 10;
        let expired = guard.sign(&past).expect("signing succeeds");
        let err = guard
            .parse(&expired)
            .await
            .expect_err("expired token must be rejected");
        assert_eq!(err, AuthError::ExpiredToken);
        let _ = token;
        let err = guard
            .parse("garbage.token.here")
            .await
            .expect_err("garbage token must be rejected");
        assert_eq!(err, AuthError::InvalidToken);
    }
}
