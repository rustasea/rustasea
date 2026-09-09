//! Rustavel JSON:API — `JsonApiResource` with sparse fieldsets and inclusion.
//!
//! Sprint 07 (M6) scope: a `JsonApiResource` trait that renders the JSON:API
//! envelope (`data` + optional `included` + `links`), honors sparse fieldsets
//! (`fields[users]=name,email`), guards unloaded relations
//! (`RelationNotLoaded`), and serves `Content-Type: application/vnd.api+json`.

pub mod builder;
pub mod error;
pub mod resource;
pub mod wire;

pub use builder::ResourceBuilder;
pub use error::{JsonApiError, Result};
pub use resource::{Inclusion, JsonApiResource, Relationship, SparseFields};
pub use wire::{content_type, Document, Link, Links};
