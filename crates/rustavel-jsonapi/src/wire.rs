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
        }
    }
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
