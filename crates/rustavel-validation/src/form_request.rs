/// Form request extraction — `FormRequest<T>`.
///
/// Mirrors Laravel's `FormRequest`: a handler declares `FormRequest<T>`; the
/// request payload is deserialized into `T`, validated against declared
/// rules, and only then handed to the handler. Failures become the standard
/// `422` `ErrorBag` before the handler body runs (FR-308/FR-309).
///
/// This is a pure wrapper layer — the payload types it wraps implement the
/// `Validatable` trait (see `rules.rs`), and the axum extractor glue that
/// parses the wire body and runs `Validatable::validate` lives behind the
/// `#[validate]` proc-macro in `rustavel-macros`.
use std::marker::PhantomData;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, serde::Deserialize)]
    struct CreateUser {
        name: String,
    }

    /// FormRequest derefs to the validated inner payload.
    #[test]
    fn form_request_derefs_to_inner() {
        let fr: FormRequest<CreateUser> =
            FormRequest::new(CreateUser { name: "Ada".into() }, vec!["required".into()]);
        assert_eq!(fr.name, "Ada");
        assert_eq!(fr.validated().name, "Ada");
        assert_eq!(fr.into_inner().name, "Ada");
    }
}
