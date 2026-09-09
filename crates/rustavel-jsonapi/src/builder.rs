//! Builder over a [`JsonApiResource`] for the fluent M6 surface:
//!
//! `UserResource::new(user).include("posts").fields(["name"]).to_response()`
//! mirrors the api-jsonapi contract — sparse fieldsets, eager-only
//! inclusion, and a `Content-Type: application/vnd.api+json` HTTP response.

use std::marker::PhantomData;

use serde_json::Value;

use crate::error::Result;
use crate::resource::{JsonApiResource, SparseFields};
use crate::wire::Document;

/// Fluent resource renderer.
///
/// Collects sparse field requests and `include` names, then renders the
/// resource on demand. `include` on a relation that was never eager-loaded
/// surfaces [`crate::JsonApiError::RelationNotLoaded`].
#[derive(Debug, Clone)]
pub struct ResourceBuilder<R> {
    /// Sparse fieldsets gathered so far.
    fields: SparseFields,
    /// Relationship names requested for inclusion.
    include: Vec<String>,
    /// Resource being rendered.
    marker: PhantomData<R>,
}

impl<R> Default for ResourceBuilder<R> {
    /// Create an empty builder (renders full attributes, no inclusions).
    fn default() -> Self {
        Self {
            fields: SparseFields::default(),
            include: Vec::new(),
            marker: PhantomData,
        }
    }
}

impl<R> ResourceBuilder<R> {
    /// Create an empty builder for resource type `R`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Restrict `type_name`'s attributes to `names` (sparse fieldset).
    pub fn fields(mut self, type_name: impl Into<String>, names: &[&str]) -> Self {
        self.fields.fields.insert(
            type_name.into(),
            names.iter().map(|s| s.to_string()).collect(),
        );
        self
    }

    /// Request inclusion of `relation` (must be eager-loaded).
    pub fn include(mut self, relation: impl Into<String>) -> Self {
        self.include.push(relation.into());
        self
    }

    /// Consume the builder and render `resource`.
    pub fn render(self, resource: &R) -> Result<Document>
    where
        R: JsonApiResource,
    {
        let include: Vec<&str> = self.include.iter().map(String::as_str).collect();
        resource.render(&self.fields, &include)
    }

    /// Render and serialize to an HTTP response (`application/vnd.api+json`).
    pub fn to_response(self, resource: &R) -> http::Response<Vec<u8>>
    where
        R: JsonApiResource,
    {
        match self.render(resource) {
            Ok(document) => document.to_response(),
            Err(e) => error_response(&e),
        }
    }
}

/// Serialize a JSON:API error into a 500 `application/vnd.api+json` response.
fn error_response(e: &crate::JsonApiError) -> http::Response<Vec<u8>> {
    let body = serde_json::json!({
        "errors": [{
            "status": "500",
            "code": format!("JsonApiError::{e:?}"),
            "title": "Relation not loaded",
            "detail": e.to_string(),
        }]
    });
    http::Response::builder()
        .status(http::StatusCode::INTERNAL_SERVER_ERROR)
        .header(
            http::header::CONTENT_TYPE,
            crate::wire::JSON_API_CONTENT_TYPE,
        )
        .body(serde_json::to_vec(&body).unwrap_or_default())
        .unwrap_or_else(|_| http::Response::new(Vec::new()))
}

/// Convenience alias for a unit struct that carries no state beyond `Value`
/// — retained for documentation symmetry with the trait-based builder.
pub type JsonApiValue = Value;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::tests::sample_user;
    use crate::wire::JSON_API_CONTENT_TYPE;

    #[test]
    fn builder_renders_document_with_correct_content_type() {
        let user = sample_user(vec!["p1".to_string()]);
        let response = ResourceBuilder::<crate::resource::tests::User>::new()
            .fields("users", &["name"])
            .include("posts")
            .to_response(&user);

        assert_eq!(
            response.headers().get(http::header::CONTENT_TYPE).unwrap(),
            JSON_API_CONTENT_TYPE
        );
        assert_eq!(response.status(), http::StatusCode::OK);
        let doc: Value = serde_json::from_slice(response.body()).unwrap();
        assert_eq!(doc["data"]["attributes"]["name"], "Ada");
        assert!(doc["data"]["attributes"].get("email").is_none());
        assert!(doc["included"].is_array());
    }
}
