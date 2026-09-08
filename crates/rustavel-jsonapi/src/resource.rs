//! `JsonApiResource` trait: resource types render into JSON:API documents.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::error::{JsonApiError, Result};
use crate::wire::{Document, Links, ResourceObject};

/// Sparse fieldset map: `fields[type]` → allowed attribute names.
///
/// Uses a `BTreeMap` so iteration order is deterministic for rendering.
#[derive(Debug, Clone, Default)]
pub struct SparseFields {
    /// Attribute allow-lists keyed by resource type.
    pub fields: BTreeMap<String, Vec<String>>,
}

impl SparseFields {
    /// Parse sparse fieldsets from query pairs (`fields[users]=name,email`).
    ///
    /// Accepts either the bracketed query key (`fields[users]`) or an
    /// already-unwrapped type key.
    pub fn parse(pairs: &[(&str, &str)]) -> Self {
        let mut fields = BTreeMap::new();
        for (key, value) in pairs {
            let type_name = key
                .strip_prefix("fields[")
                .and_then(|k| k.strip_suffix(']'))
                .unwrap_or(*key);
            let names: Vec<String> = value
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned)
                .collect();
            if !names.is_empty() {
                fields.insert(type_name.to_string(), names);
            }
        }
        SparseFields { fields }
    }
}

/// A relationship exposed on a resource.
#[derive(Debug, Clone)]
pub struct Relationship {
    /// Relationship name (`posts`).
    pub name: &'static str,
    /// Related resource type.
    pub r#type: &'static str,
    /// Related resource ids.
    pub ids: Vec<String>,
    /// Whether the related resources are loaded and renderable.
    pub loaded: bool,
}

impl Relationship {
    /// Create a relationship.
    pub fn new(name: &'static str, r#type: &'static str, ids: Vec<String>, loaded: bool) -> Self {
        Self {
            name,
            r#type,
            ids,
            loaded,
        }
    }
}

/// A loaded related resource, ready for inclusion.
#[derive(Debug, Clone)]
pub struct Inclusion {
    /// Relationship name this inclusion satisfies.
    pub relationship: &'static str,
    /// Rendered resource object.
    pub resource: ResourceObject,
}

/// JSON:API resource trait — implementors describe one resource type.
///
/// `attributes()` renders the full attribute map; sparse field filtering and
/// relationship validation happen centrally in [`JsonApiResource::render`].
pub trait JsonApiResource {
    /// Resource type name (`users`).
    fn r#type(&self) -> &'static str;

    /// Resource id.
    fn id(&self) -> String;

    /// Full attributes (pre-filtering).
    fn attributes(&self) -> Map<String, Value>;

    /// Top-level/resource links for this resource.
    fn links(&self) -> Option<Links> {
        None
    }

    /// Declared relationships (only loaded ones may be included).
    fn relationships(&self) -> Vec<Relationship> {
        Vec::new()
    }

    /// Render included resources for a loaded, requested relationship.
    fn include(&self, relationship: &str) -> Vec<Inclusion> {
        let _ = relationship;
        Vec::new()
    }

    /// Render this resource as a single-resource JSON:API document.
    ///
    /// Applies sparse field filtering when `fields` constrains this type and
    /// validates that every `include`-requested relationship is loaded —
    /// unloaded relations surface as [`JsonApiError::RelationNotLoaded`].
    fn render(&self, fields: &SparseFields, include: &[&str]) -> Result<Document> {
        let data = self.resource_object(fields)?;
        let mut included = Vec::new();
        for name in include {
            let rel = self.relationships().into_iter().find(|r| r.name == *name);
            match rel {
                Some(rel) if rel.loaded => {
                    included.extend(self.include(name).into_iter().map(|i| i.resource));
                }
                Some(_) => {
                    return Err(JsonApiError::RelationNotLoaded {
                        relationship: (*name).to_string(),
                        resource_type: self.r#type().to_string(),
                    });
                }
                None => {}
            }
        }
        Ok(Document::single(data, included, self.links()))
    }

    /// Render a collection of same-type resources as a JSON:API document.
    ///
    /// Each resource renders with the same `fields`/`include`; `include`
    /// requests are honored per element only when every element marks the
    /// relationship loaded (first unloaded element errors).
    fn render_many(resources: &[Self], fields: &SparseFields, include: &[&str]) -> Result<Document>
    where
        Self: Sized,
    {
        let mut objects = Vec::with_capacity(resources.len());
        let mut included = Vec::new();
        for resource in resources {
            objects.push(resource.resource_object(fields)?);
            for name in include {
                let rel = resource
                    .relationships()
                    .into_iter()
                    .find(|r| r.name == *name);
                match rel {
                    Some(rel) if rel.loaded => {
                        included.extend(resource.include(name).into_iter().map(|i| i.resource));
                    }
                    Some(_) => {
                        return Err(JsonApiError::RelationNotLoaded {
                            relationship: (*name).to_string(),
                            resource_type: resource.r#type().to_string(),
                        });
                    }
                    None => {}
                }
            }
        }
        Ok(Document::collection(objects, included, None))
    }

    /// Build the filtered resource object for this type.
    fn resource_object(&self, fields: &SparseFields) -> Result<ResourceObject> {
        let attributes = self.attributes();
        let allow = fields.fields.get(self.r#type());
        let filtered: Map<String, Value> = match allow {
            None => attributes,
            Some(names) => attributes
                .into_iter()
                .filter(|(k, _)| names.contains(k))
                .collect(),
        };
        if let Some(names) = allow {
            if filtered.is_empty() && !names.is_empty() {
                return Err(JsonApiError::EmptySparseFieldset(self.r#type().to_string()));
            }
        }
        Ok(ResourceObject {
            r#type: self.r#type().to_string(),
            id: self.id(),
            attributes: filtered,
            relationships: self.relationship_links(),
            links: self.links(),
        })
    }

    /// Relationship links map (`name` → `{links: {related}}`).
    fn relationship_links(&self) -> Map<String, Value> {
        let mut map = Map::new();
        for rel in self.relationships() {
            let mut links = Map::new();
            links.insert(
                "related".to_string(),
                Value::String(format!("/{}/{}/{}", self.r#type(), self.id(), rel.name)),
            );
            let mut obj = Map::new();
            obj.insert("links".to_string(), Value::Object(links));
            map.insert(rel.name.to_string(), Value::Object(obj));
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct User {
        id: String,
        name: String,
        email: String,
        posts: Vec<String>,
    }

    impl JsonApiResource for User {
        fn r#type(&self) -> &'static str {
            "users"
        }

        fn id(&self) -> String {
            self.id.clone()
        }

        fn attributes(&self) -> Map<String, Value> {
            let mut m = Map::new();
            m.insert("name".to_string(), Value::String(self.name.clone()));
            m.insert("email".to_string(), Value::String(self.email.clone()));
            m
        }

        fn links(&self) -> Option<Links> {
            Some(
                Links::default()
                    .with_self_link(crate::wire::Link::new(format!("/users/{}", self.id))),
            )
        }

        fn relationships(&self) -> Vec<Relationship> {
            vec![Relationship::new(
                "posts",
                "posts",
                self.posts.clone(),
                true,
            )]
        }
    }

    fn user(posts: Vec<String>) -> User {
        User {
            id: "1".to_string(),
            name: "Ada".to_string(),
            email: "ada@example.test".to_string(),
            posts,
        }
    }

    #[test]
    fn sparse_fieldset_filters_attributes() {
        let fields = SparseFields::parse(&[("fields[users]", "name,email")]);
        let doc = user(vec!["10".to_string()]).render(&fields, &[]).unwrap();
        let attrs = doc.data.as_object().unwrap()["attributes"]
            .as_object()
            .unwrap()
            .clone();
        assert_eq!(attrs.len(), 2);
        assert!(attrs.contains_key("name"));
        assert!(attrs.contains_key("email"));
    }

    #[test]
    fn unknown_field_excluded_by_sparse_fieldset() {
        let fields = SparseFields::parse(&[("fields[users]", "name")]);
        let doc = user(Vec::new()).render(&fields, &[]).unwrap();
        let attrs = doc.data.as_object().unwrap()["attributes"]
            .as_object()
            .unwrap()
            .clone();
        assert!(attrs.contains_key("name"));
        assert!(!attrs.contains_key("email"));
    }

    #[test]
    fn collection_renders_array_data() {
        let fields = SparseFields::default();
        let doc = User::render_many(&[user(Vec::new())], &fields, &[]).unwrap();
        assert!(doc.data.is_array());
    }
}
