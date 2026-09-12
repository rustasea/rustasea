//! Deferred loaders: `SimilaritySearch`, `FileStorage`, `ToolSearch`.
//!
//! These loaders lazily resolve heavyweight M6 integrations. Constructing one
//! never performs I/O; the first use binds or degrades. With the `search`
//! feature [`SimilaritySearch`] wraps a `VectorSearch` engine and with the
//! `storage` feature [`FileStorage`] wraps a `Storage` engine — otherwise each
//! degrades to a typed capability denial (NFR-Sca-02). [`ToolSearch`] ranks
//! candidate tools with a dependency-free BM25-lite scorer over names and
//! descriptions. Each loader implements [`crate::streaming::DeferredLoader`] so
//! agents can inject context on demand before a `Tool::call`.

use std::sync::Arc;

use crate::error::{AiError, Result};

/// Map a backend failure into the AI error space.
#[cfg(any(feature = "search", feature = "storage"))]
fn backend_error(backend: &str, message: impl std::fmt::Display) -> AiError {
    AiError::Backend {
        backend: backend.to_string(),
        message: message.to_string(),
    }
}

/// Backing engine type for [`SimilaritySearch`].
#[cfg(feature = "search")]
type Engine = Arc<dyn rustasea_search::VectorSearch>;

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

    /// Create a loader backed by an in-memory vector store of `dim` dimensions.
    #[cfg(feature = "search")]
    pub fn in_memory(name: impl Into<String>, dim: usize) -> Self {
        Self {
            name: name.into(),
            inner: Some(Arc::new(rustasea_search::MemoryVectorStore::new(dim))),
        }
    }

    /// Bind a vector-search engine for later use.
    #[cfg(feature = "search")]
    pub fn bind(&mut self, engine: Arc<dyn rustasea_search::VectorSearch>) -> &mut Self {
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
    pub fn handle(&self) -> Result<Arc<dyn rustasea_search::VectorSearch>> {
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

    /// Run a nearest-neighbour query against the bound engine.
    ///
    /// Delegates to [`rustasea_search::VectorSearch::where_vector_similar_to`]
    /// and maps a backend failure into [`AiError::Backend`]. Errors when no
    /// engine is bound or the `search` feature is disabled. Available only with
    /// the `search` feature (degraded callers use [`SimilaritySearch::handle`]).
    #[cfg(feature = "search")]
    pub async fn search(
        &self,
        table: &str,
        column: &str,
        query: &[f32],
        limit: usize,
        metric: rustasea_search::Similarity,
    ) -> Result<Vec<rustasea_search::VectorMatch>> {
        self.handle()?
            .where_vector_similar_to(table, column, query, limit, metric)
            .await
            .map_err(|error| backend_error(&self.name, error))
    }
}

/// Deferred-loader contract for the bound similarity engine.
#[cfg(feature = "search")]
impl crate::streaming::DeferredLoader for SimilaritySearch {
    /// Engine type resolved by the loader.
    type Target = dyn rustasea_search::VectorSearch;

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
    inner: Option<Arc<dyn rustasea_storage::Storage>>,
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

    /// Create a loader backed by a local filesystem disk rooted at `dir`.
    #[cfg(feature = "storage")]
    pub fn local(name: impl Into<String>, dir: impl Into<std::path::PathBuf>) -> Self {
        Self {
            name: name.into(),
            inner: Some(Arc::new(rustasea_storage::LocalDisk::new(dir))),
        }
    }

    /// Bind a storage engine for later use.
    #[cfg(feature = "storage")]
    pub fn bind(&mut self, engine: Arc<dyn rustasea_storage::Storage>) -> &mut Self {
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
    pub fn handle(&self) -> Result<Arc<dyn rustasea_storage::Storage>> {
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

    /// Read an object from the bound engine, mapping backend failures.
    #[cfg(feature = "storage")]
    pub async fn get(&self, key: &str) -> Result<Vec<u8>> {
        self.handle()?
            .get(key)
            .await
            .map_err(|error| backend_error(&self.name, error))
    }

    /// Write an object through the bound engine, mapping backend failures.
    #[cfg(feature = "storage")]
    pub async fn put(&self, key: &str, bytes: &[u8]) -> Result<()> {
        self.handle()?
            .put(key, bytes)
            .await
            .map_err(|error| backend_error(&self.name, error))
    }

    /// Probe object existence through the bound engine.
    #[cfg(feature = "storage")]
    pub async fn exists(&self, key: &str) -> Result<bool> {
        self.handle()?
            .exists(key)
            .await
            .map_err(|error| backend_error(&self.name, error))
    }

    /// Delete an object through the bound engine.
    #[cfg(feature = "storage")]
    pub async fn delete(&self, key: &str) -> Result<()> {
        self.handle()?
            .delete(key)
            .await
            .map_err(|error| backend_error(&self.name, error))
    }
}

/// Deferred-loader contract for the bound storage engine.
#[cfg(feature = "storage")]
impl crate::streaming::DeferredLoader for FileStorage {
    /// Engine type resolved by the loader.
    type Target = dyn rustasea_storage::Storage;

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
#[derive(Debug)]
pub struct UnavailableEngine {
    _private: (),
}

/// One ranked tool match produced by [`ToolSearch::rank`].
#[derive(Debug, Clone, PartialEq)]
pub struct ToolMatch {
    /// Tool name.
    pub name: String,
    /// BM25-lite relevance score (higher is more relevant; `> 0`).
    pub score: f32,
}

/// Deferred tool-search loader: ranks registered agent tools by relevance.
///
/// Ranking is a dependency-free BM25-lite scorer over each tool's name and
/// description (see [`crate::tool_rank`]); no search engine is pulled.
pub struct ToolSearch {
    name: String,
    query: Option<String>,
    candidates: Vec<(String, String)>,
}

impl ToolSearch {
    /// Create a deferred loader (never performs I/O).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            query: None,
            candidates: Vec::new(),
        }
    }

    /// Record the search query for later execution.
    pub fn with_query(&mut self, query: impl Into<String>) -> &mut Self {
        self.query = Some(query.into());
        self
    }

    /// Register the candidate tools `(name, description)` to rank against.
    pub fn with_candidates<I, N, D>(&mut self, candidates: I) -> &mut Self
    where
        I: IntoIterator<Item = (N, D)>,
        N: Into<String>,
        D: Into<String>,
    {
        self.candidates = candidates
            .into_iter()
            .map(|(name, description)| (name.into(), description.into()))
            .collect();
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

    /// Rank `tools` against the query, returning positive-score matches in
    /// descending relevance order (stable for ties).
    pub fn rank(&self, tools: &[(String, String)]) -> Vec<ToolMatch> {
        let Some(query) = self.query.as_deref() else {
            return Vec::new();
        };
        let borrowed: Vec<(&str, &str)> = tools
            .iter()
            .map(|(name, description)| (name.as_str(), description.as_str()))
            .collect();
        let mut scored: Vec<ToolMatch> = tools
            .iter()
            .zip(crate::tool_rank::score(query, &borrowed))
            .filter(|(_, score)| *score > 0.0)
            .map(|((name, _), score)| ToolMatch {
                name: name.clone(),
                score,
            })
            .collect();
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored
    }

    /// Rank the candidates registered with [`ToolSearch::with_candidates`].
    pub fn search_candidates(&self) -> Vec<ToolMatch> {
        self.rank(&self.candidates)
    }

    /// Rank a list of tool names (no descriptions) against the query.
    ///
    /// Convenience wrapper preserving the historical name-based API; returns
    /// matching names in descending relevance order.
    pub fn search(&self, tools: &[&'static str]) -> Vec<&'static str> {
        let candidates: Vec<(String, String)> = tools
            .iter()
            .map(|name| ((*name).to_string(), String::new()))
            .collect();
        let ranked = self.rank(&candidates);
        tools
            .iter()
            .copied()
            .filter(|name| ranked.iter().any(|hit| hit.name == *name))
            .collect()
    }
}

/// Deferred-loader contract for tool search: the loaded target is the ranked
/// list of registered tools the agent can invoke.
impl crate::streaming::DeferredLoader for ToolSearch {
    /// Ranked tool-name snapshot produced by the loader.
    type Target = Vec<ToolMatch>;

    /// Load (rank) the registered candidates; an empty query yields an empty set.
    fn load(&self) -> Result<Arc<Self::Target>> {
        Ok(Arc::new(self.search_candidates()))
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
    fn tool_search_ranks_relevant_tools_first() {
        let mut loader = ToolSearch::new("find-tools");
        loader.with_query("search documentation").with_candidates([
            ("send_email", "Send an email to a recipient"),
            ("search_docs", "Search the documentation for a phrase"),
            ("web_search", "Search the public web"),
        ]);
        let matches = loader.search_candidates();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].name, "search_docs");
        assert!(matches.iter().all(|hit| hit.score > 0.0));
    }

    #[test]
    fn tool_search_name_only_convenience_still_matches() {
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
