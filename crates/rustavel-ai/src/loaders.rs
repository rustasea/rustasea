//! Deferred loaders: `SimilaritySearch`, `FileStorage`, `ToolSearch`.
//!
//! These loaders lazily resolve heavyweight M6 integrations. Constructing one
//! never performs I/O; the first use binds or degrades. With the `search`
//! feature [`SimilaritySearch`] wraps a `VectorSearch` engine and with the
//! `storage` feature [`FileStorage`] wraps a `Storage` engine — otherwise
//! each degrades to a typed capability denial (NFR-Sca-02).

#[cfg(any(feature = "search", feature = "storage"))]
use std::sync::Arc;

use crate::error::{AiError, Result};

/// Backing engine type for [`SimilaritySearch`].
#[cfg(feature = "search")]
type Engine = Arc<dyn rustavel_search::VectorSearch>;

/// Cached handle to a lazily-loaded similarity search engine.
#[cfg(feature = "search")]
pub struct SimilaritySearch {
    name: String,
    inner: Option<Engine>,
}

/// Degraded similarity-search handle (no `search` feature).
#[cfg(not(feature = "search"))]
pub struct SimilaritySearch {
    name: String,
}

impl SimilaritySearch {
    /// Create a deferred loader (never performs I/O).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            #[cfg(feature = "search")]
            inner: None,
        }
    }

    /// Bind a vector-search engine for later use.
    #[cfg(feature = "search")]
    pub fn bind(&mut self, engine: Arc<dyn rustavel_search::VectorSearch>) -> &mut Self {
        self.inner = Some(engine);
        self
    }

    /// Loader name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether a backing engine is bound.
    pub fn ready(&self) -> bool {
        #[cfg(feature = "search")]
        {
            self.inner.is_some()
        }
        #[cfg(not(feature = "search"))]
        {
            false
        }
    }

    /// Access the bound engine; errors when unbound or feature-disabled.
    #[cfg(feature = "search")]
    pub fn handle(&self) -> Result<Arc<dyn rustavel_search::VectorSearch>> {
        self.inner.clone().ok_or_else(|| {
            AiError::NotConfigured(format!(
                "similarity search '{}' has no engine bound",
                self.name
            ))
        })
    }

    /// Access the engine when the `search` feature is off (degraded mode).
    #[cfg(not(feature = "search"))]
    pub fn handle(&self) -> Result<crate::loaders::UnavailableEngine> {
        let _ = &self.name;
        Err(AiError::unsupported("similarity-search", "vector store"))
    }
}

/// Cached handle to a lazily-loaded file-storage engine.
#[cfg(feature = "storage")]
pub struct FileStorage {
    name: String,
    inner: Option<Arc<dyn rustavel_storage::Storage>>,
}

/// Degraded file-storage handle (no `storage` feature).
#[cfg(not(feature = "storage"))]
pub struct FileStorage {
    name: String,
}

impl FileStorage {
    /// Create a deferred loader (never performs I/O).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            #[cfg(feature = "storage")]
            inner: None,
        }
    }

    /// Bind a storage engine for later use.
    #[cfg(feature = "storage")]
    pub fn bind(&mut self, engine: Arc<dyn rustavel_storage::Storage>) -> &mut Self {
        self.inner = Some(engine);
        self
    }

    /// Loader name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether a backing engine is bound.
    pub fn ready(&self) -> bool {
        #[cfg(feature = "storage")]
        {
            self.inner.is_some()
        }
        #[cfg(not(feature = "storage"))]
        {
            false
        }
    }

    /// Access the bound engine; errors when unbound or feature-disabled.
    #[cfg(feature = "storage")]
    pub fn handle(&self) -> Result<Arc<dyn rustavel_storage::Storage>> {
        self.inner.clone().ok_or_else(|| {
            AiError::NotConfigured(format!("file storage '{}' has no engine bound", self.name))
        })
    }

    /// Access the engine when the `storage` feature is off (degraded mode).
    #[cfg(not(feature = "storage"))]
    pub fn handle(&self) -> Result<crate::loaders::UnavailableEngine> {
        let _ = &self.name;
        Err(AiError::unsupported("file-storage", "storage"))
    }
}

/// Degraded-mode engine type returned by feature-disabled loaders.
///
/// Never constructed — only used as the error-side type so loader signatures
/// stay stable across feature combinations (NFR-Sca-02).
pub struct UnavailableEngine {
    _private: (),
}

/// Deferred tool-search loader: finds registered agent tools by name.
///
/// Tool search over the agent tool registry is delegated lazily; this loader
/// holds the query contract without pulling a search engine.
pub struct ToolSearch {
    name: String,
    query: Option<String>,
}

impl ToolSearch {
    /// Create a deferred loader (never performs I/O).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            query: None,
        }
    }

    /// Record the search query for later execution.
    pub fn with_query(&mut self, query: impl Into<String>) -> &mut Self {
        self.query = Some(query.into());
        self
    }

    /// Loader name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Pending query, if any.
    pub fn query(&self) -> Option<&str> {
        self.query.as_deref()
    }

    /// Resolve matching tool names from a candidate list (stub scoring).
    ///
    /// Matches tools whose name contains any query term (case-insensitive).
    pub fn search(&self, tools: &[&'static str]) -> Vec<&'static str> {
        let Some(query) = &self.query else {
            return Vec::new();
        };
        let terms: Vec<String> = query.split_whitespace().map(|t| t.to_lowercase()).collect();
        tools
            .iter()
            .copied()
            .filter(|tool| {
                let lower = tool.to_lowercase();
                terms.iter().any(|t| lower.contains(t.as_str()))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn similarity_search_degrades_without_feature() {
        let loader = SimilaritySearch::new("docs");
        assert!(!loader.ready());
        assert!(loader.handle().is_err());
    }

    #[test]
    fn tool_search_scores_by_substring() {
        let mut loader = ToolSearch::new("find-tools");
        loader.with_query("search");
        let matches = loader.search(&["search_docs", "send_email", "web_search"]);
        assert_eq!(matches, vec!["search_docs", "web_search"]);
    }
}
