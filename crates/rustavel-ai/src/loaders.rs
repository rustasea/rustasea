//! Deferred loaders: `SimilaritySearch`, `FileStorage`, `ToolSearch`.
//!
//! These loaders lazily resolve heavyweight M6 integrations. Constructing one
//! never performs I/O; the first use binds or degrades. With the `search`
//! feature [`SimilaritySearch`] wraps a `VectorSearch` engine and with the
//! `storage` feature [`FileStorage`] wraps a `Storage` engine — otherwise
//! each degrades to a typed capability denial (NFR-Sca-02). Each loader also
//! implements [`crate::streaming::DeferredLoader`] so agents can inject
//! context on demand before a `Tool::call`.

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

/// Deferred-loader contract for the bound similarity engine.
#[cfg(feature = "search")]
impl crate::streaming::DeferredLoader for SimilaritySearch {
    /// Engine type resolved by the loader.
    type Target = dyn rustavel_search::VectorSearch;

    /// Resolve the bound vector-search engine.
    fn load(&self) -> Result<Arc<Self::Target>> {
        self.handle()
    }

    /// Whether the engine is bound.
    fn loaded(&self) -> bool {
        self.ready()
    }
}

/// Deferred-loader contract in degraded mode (no `search` feature).
#[cfg(not(feature = "search"))]
impl crate::streaming::DeferredLoader for SimilaritySearch {
    /// Never-constructed degraded engine type.
    type Target = UnavailableEngine;

    /// Always errors with a capability denial.
    fn load(&self) -> Result<Arc<Self::Target>> {
        let _engine = self.handle()?;
        unreachable!("no-feature similarity search handle always errors")
    }

    /// Never loaded in degraded mode.
    fn loaded(&self) -> bool {
        false
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

/// Deferred-loader contract for the bound storage engine.
#[cfg(feature = "storage")]
impl crate::streaming::DeferredLoader for FileStorage {
    /// Engine type resolved by the loader.
    type Target = dyn rustavel_storage::Storage;

    /// Resolve the bound storage engine.
    fn load(&self) -> Result<Arc<Self::Target>> {
        self.handle()
    }

    /// Whether the engine is bound.
    fn loaded(&self) -> bool {
        self.ready()
    }
}

/// Deferred-loader contract in degraded mode (no `storage` feature).
#[cfg(not(feature = "storage"))]
impl crate::streaming::DeferredLoader for FileStorage {
    /// Never-constructed degraded engine type.
    type Target = UnavailableEngine;

    /// Always errors with a capability denial.
    fn load(&self) -> Result<Arc<Self::Target>> {
        let _engine = self.handle()?;
        unreachable!("no-feature file-storage handle always errors")
    }

    /// Never loaded in degraded mode.
    fn loaded(&self) -> bool {
        false
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

/// Deferred-loader contract for tool search: the loaded target is the list of
/// registered tools the agent can invoke.
impl crate::streaming::DeferredLoader for ToolSearch {
    /// Tool-name snapshot produced by the loader.
    type Target = Vec<&'static str>;

    /// Load (search) matching tool names; empty query yields an empty set.
    fn load(&self) -> Result<Arc<Self::Target>> {
        Ok(Arc::new(self.search(&[])))
    }

    /// Tool search is available as soon as the query is recorded.
    fn loaded(&self) -> bool {
        self.query.is_some()
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

    #[test]
    fn deferred_loaders_report_readiness() {
        use crate::streaming::DeferredLoader;

        // SimilaritySearch without the search feature is never loaded.
        let similarity = SimilaritySearch::new("docs");
        assert!(!similarity.loaded());
        assert!(similarity.load().is_err());

        // ToolSearch loads once a query is recorded.
        let mut tool_search = ToolSearch::new("find-tools");
        assert!(!tool_search.loaded());
        tool_search.with_query("search");
        assert!(tool_search.loaded());
        assert!(tool_search.load().is_ok());

        // FileStorage without the storage feature is never loaded.
        let storage = FileStorage::new("assets");
        assert!(!storage.loaded());
        assert!(storage.load().is_err());
    }
}
