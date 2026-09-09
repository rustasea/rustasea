/// Form request extraction — `FormRequest<T>`.
///
/// Mirrors Laravel's `FormRequest`: a handler declares `FormRequest<T>`; the
/// request payload is deserialized into `T`, validated against declared
/// rules, and only then handed to the handler. Failures become the standard
/// `422` `ErrorBag` before the handler body runs (FR-308/FR-309).
///
/// The payload types it wraps implement the [`Validatable`] trait
/// (see `rules.rs`); the axum extractor glue in this file parses the wire
/// body and runs `Validatable::validate` before the handler runs.
use std::marker::PhantomData;

use axum::extract::FromRequest;
use axum::response::{IntoResponse, Response};

use crate::error_bag::ErrorBag;
use crate::rules::Validatable;

/// Maximum accepted JSON body size for a form request (256 KiB).
const MAX_BODY_BYTES: usize = 256 * 1024;

/// A validated request payload plus the rules that ran.
#[derive(Debug)]
pub struct FormRequest<T> {
    /// The parsed and validated inner payload.
    pub inner: T,
    /// Declared rules that ran against the payload.
    pub rules: Vec<String>,
    /// Payload marker for type-level rule lookup.
    marker: PhantomData<fn() -> T>,
}

impl<T> FormRequest<T> {
    /// Wrap a validated payload.
    pub fn new(inner: T, rules: Vec<String>) -> Self {
        Self {
            inner,
            rules,
            marker: PhantomData,
        }
    }

    /// Access the validated inner value.
    pub fn validated(&self) -> &T {
        &self.inner
    }

    /// Consume and return the validated inner value.
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> std::ops::Deref for FormRequest<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Validate a deserialized payload before the handler body runs.
///
/// Reads the JSON body (bounded to [`MAX_BODY_BYTES`]), deserializes it into
/// `T`, then runs `T::validate`. Failures are turned into the documented
/// `422` `ErrorBag` response; success yields a `FormRequest<T>` whose `rules`
/// are populated from the payload type's `Validatable::validate` invocation
/// (the exact rule strings are not introspectable from the trait, so the
/// vector records the validation pass marker).
#[axum::async_trait]
impl<S, T> FromRequest<S> for FormRequest<T>
where
    S: Send + Sync,
    T: Validatable + Send + 'static,
{
    type Rejection = Response;

    async fn from_request(
        req: axum::http::Request<axum::body::Body>,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let bytes = axum::body::to_bytes(req.into_body(), MAX_BODY_BYTES)
            .await
            .map_err(|_| {
                (
                    axum::http::StatusCode::PAYLOAD_TOO_LARGE,
                    axum::Json(serde_json::json!({
                        "message": "The request body was too large.",
                        "errors": {}
                    })),
                )
                    .into_response()
            })?;
        let payload: T = serde_json::from_slice(&bytes).map_err(|err| {
            let bag = ErrorBag::from_message(format!("The payload is not valid JSON: {err}"));
            Response::from(bag)
        })?;
        payload.validate().map_err(|bag| Response::from(bag))?;
        Ok(FormRequest::new(payload, vec!["validated".into()]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::body::Body;
    use axum::http::Request;
    use serde::Deserialize;
    use serde_json::json;

    #[derive(Debug, PartialEq, Deserialize, serde::Serialize)]
    struct CreateUser {
        name: String,
        email: String,
    }

    impl Validatable for CreateUser {
        fn validate(&self) -> std::result::Result<(), ErrorBag> {
            let rules = crate::rules::Rules::new()
                .field("name", "required|min:3")
                .field("email", "required|email");
            rules.validate(&serde_json::to_value(self).expect("payload serializes"))
        }
    }

    /// FormRequest derefs to the validated inner payload.
    #[test]
    fn form_request_derefs_to_inner() {
        let fr: FormRequest<CreateUser> = FormRequest::new(
            CreateUser {
                name: "Ada".into(),
                email: "ada@example.com".into(),
            },
            vec!["required".into()],
        );
        assert_eq!(fr.name, "Ada");
        assert_eq!(fr.validated().name, "Ada");
        assert_eq!(fr.into_inner().name, "Ada");
    }

    /// A valid JSON body extracts into a validated `FormRequest<T>`.
    #[tokio::test]
    async fn from_request_extracts_valid_payload() {
        let req = Request::builder()
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({ "name": "Ada", "email": "ada@example.com" }).to_string(),
            ))
            .expect("request builds");
        let extracted: Result<FormRequest<CreateUser>, Response> =
            FormRequest::from_request(req, &()).await;
        let fr = extracted.expect("valid payload extracts");
        assert_eq!(fr.inner.name, "Ada");
    }

    /// An invalid payload is rejected with a 422 `ErrorBag` response before
    /// the handler runs.
    #[tokio::test]
    async fn from_request_rejects_invalid_payload_with_422() {
        let req = Request::builder()
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({ "name": "ab", "email": "not-an-email" }).to_string(),
            ))
            .expect("request builds");
        let extracted: Result<FormRequest<CreateUser>, Response> =
            FormRequest::from_request(req, &()).await;
        let resp = extracted.expect_err("invalid payload is rejected");
        assert_eq!(resp.status(), axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    }

    /// A malformed JSON body is rejected with a 422 response.
    #[tokio::test]
    async fn from_request_rejects_malformed_json() {
        let req = Request::builder()
            .method("POST")
            .body(Body::from("{not json"))
            .expect("request builds");
        let extracted: Result<FormRequest<CreateUser>, Response> =
            FormRequest::from_request(req, &()).await;
        let resp = extracted.expect_err("malformed JSON is rejected");
        assert_eq!(resp.status(), axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    }
}
