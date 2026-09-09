//! Wire model: JSON:API envelope, links, and content type constant.

use serde::Serialize;
use serde_json::{Map, Value};

/// Media type mandated by the JSON:API spec.
pub const JSON_API_CONTENT_TYPE: &str = "application/vnd.api+json";

/// Return the JSON:API media type for header use.
pub fn content_type() -> &'static str {
    JSON_API_CONTENT_TYPE
}

/// A named link (self/related/first/last/next).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Link {
    /// Link href.
    pub href: String,
    /// Optional link meta.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
}

impl Link {
    /// Create a plain href link.
    pub fn new(href: impl Into<String>) -> Self {
        Self {
            href: href.into(),
            meta: None,
        }
    }
}

/// Link collection (`self`, `related`, paging links).
#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
pub struct Links {
    /// Self link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_link: Option<Link>,
    /// Related link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related: Option<Link>,
    /// First page link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first: Option<Link>,
    /// Last page link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<Link>,
    /// Next page link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<Link>,
}

impl Links {
    /// Set the self link.
    pub fn with_self_link(mut self, link: Link) -> Self {
        self.self_link = Some(link);
        self
    }
}

/// One resource object (`type` + `id` + `attributes` + optional links).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ResourceObject {
    /// Resource type (`users`, `posts`).
    pub r#type: String,
    /// Resource id.
    pub id: String,
    /// Attributes after sparse-field filtering.
    pub attributes: Map<String, Value>,
    /// Relationship links (`self`/`related` per spec).
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub relationships: Map<String, Value>,
    /// Resource links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Links>,
}

/// Top-level JSON:API document.
///
/// `data` is the JSON:API primary data: a resource object for single
/// resources, an array for collections, or `Value::Null` when empty.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Document {
    /// Primary data (`null` | object | array).
    pub data: Value,
    /// Included related resources.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub included: Vec<ResourceObject>,
    /// Top-level links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Links>,
    /// Top-level meta (e.g. provider, pagination metadata).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
}

impl Document {
    /// Build a single-resource document.
    pub fn single(
        resource: ResourceObject,
        included: Vec<ResourceObject>,
        links: Option<Links>,
    ) -> Self {
        Self {
            data: serde_json::to_value(resource).unwrap_or(Value::Null),
            included,
            links,
            meta: None,
        }
    }

    /// Build a collection document (`data` is an array).
    pub fn collection(
        resources: Vec<ResourceObject>,
        included: Vec<ResourceObject>,
        links: Option<Links>,
    ) -> Self {
        let values: Vec<Value> = resources
            .into_iter()
            .filter_map(|r| serde_json::to_value(r).ok())
            .collect();
        Self {
            data: Value::Array(values),
            included,
            links,
            meta: None,
        }
    }

    /// Attach top-level `meta`.
    pub fn with_meta(mut self, meta: Value) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Serialize this document and wrap it in an HTTP response.
    ///
    /// The response carries `Content-Type: application/vnd.api+json` with a
    /// 200 status (FS-M6-03 contract); serialization failures surface as a
    /// 500 response with a JSON error body.
    pub fn to_response(&self) -> http::Response<Vec<u8>> {
        match serde_json::to_vec(self) {
            Ok(bytes) => http::Response::builder()
                .status(http::StatusCode::OK)
                .header(http::header::CONTENT_TYPE, JSON_API_CONTENT_TYPE)
                .body(bytes)
                .unwrap_or_else(|_| internal_error()),
            Err(_) => internal_error(),
        }
    }
}

/// Build a 500 JSON error response (programmer-error path).
fn internal_error() -> http::Response<Vec<u8>> {
    http::Response::builder()
        .status(http::StatusCode::INTERNAL_SERVER_ERROR)
        .header(http::header::CONTENT_TYPE, JSON_API_CONTENT_TYPE)
        .body(br#"{"errors":[{"status":"500","code":"JsonApiError::Serialization","title":"Failed to serialize document"}]}"#.to_vec())
        .unwrap_or_else(|_| http::Response::new(Vec::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_type_is_vnd_api() {
        assert_eq!(JSON_API_CONTENT_TYPE, "application/vnd.api+json");
    }

    fn sample() -> ResourceObject {
        ResourceObject {
            r#type: "users".to_string(),
            id: "1".to_string(),
            attributes: Map::new(),
            relationships: Map::new(),
            links: None,
        }
    }

    #[test]
    fn document_omits_empty_included() {
        let doc = Document::single(sample(), Vec::new(), None);
        let json = serde_json::to_value(&doc).unwrap();
        assert!(json.get("included").is_none());
    }

    #[test]
    fn collection_document_is_array() {
        let doc = Document::collection(vec![sample()], Vec::new(), None);
        let json = serde_json::to_value(&doc).unwrap();
        assert!(json["data"].is_array());
    }
}
