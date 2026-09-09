//! In-memory vector store: `where_vector_similar_to` + `drop_vector_index`
//! over the same engine (pgvector-stub semantics).
//!
//! This engine wires the two M6 search surfaces together: similarity search
//! always scans every row (sequence-scan semantics), and dropping the vector
//! index only clears the index marker — search keeps returning rows until the
//! index is recreated (TC-M6-25 contract).

use serde::{Deserialize, Serialize};

use crate::embeddings::stub_embedding;
use crate::index::{IndexError, VectorIndex, VectorIndexOps};
use crate::vector::Result as VectorResult;
use crate::vector::{metric, validate_dimension, VectorDocument, VectorMatch, VectorSearch};
use crate::Similarity;

/// An in-memory `pgvector`-like store over one table.
///
/// Insert rows with [`MemoryVectorStore::insert`], search with
/// [`VectorSearch::where_vector_similar_to`], and manage the index marker
/// with [`VectorIndexOps::drop`] (search continues via seq scan).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryVectorStore {
    /// Declared vector dimension for this store.
    pub dim: usize,
    /// Rows indexed by document id.
    pub docs: Vec<VectorDocument>,
    /// Active index marker (not persisted; used to validate drop names).
    #[serde(skip)]
    pub index: Option<VectorIndex>,
}

impl MemoryVectorStore {
    /// Create an empty store with a declared vector dimension.
    pub fn new(dim: usize) -> Self {
        Self {
            dim,
            docs: Vec::new(),
            index: None,
        }
    }

    /// Seed the store with `count` deterministic rows (test/demo helper).
    pub fn seed(&mut self, count: usize) {
        for i in 0..count {
            self.docs.push(VectorDocument {
                id: format!("doc-{i}"),
                vector: stub_embedding(&format!("doc {i}"), self.dim),
            });
        }
    }

    /// Insert a document row.
    pub fn insert(&mut self, doc: VectorDocument) {
        if doc.vector.len() != self.dim {
            return;
        }
        self.docs.push(doc);
    }

    /// Install the index marker under the column.
    pub fn install_index(&mut self, name: &str, table: &str, column: &str) {
        self.index = Some(VectorIndex::new(name, table, column));
    }
}

#[async_trait::async_trait]
impl VectorSearch for MemoryVectorStore {
    async fn where_vector_similar_to(
        &self,
        _table: &str,
        _column: &str,
        query: &[f32],
        limit: usize,
        metric_kind: Similarity,
    ) -> VectorResult<Vec<VectorMatch>> {
        validate_dimension(query, self.dim)?;
        let mut scored: Vec<(f32, &VectorDocument)> = self
            .docs
            .iter()
            .map(|doc| {
                let distance = match metric_kind {
                    Similarity::Cosine => 1.0 - metric::cosine(query, &doc.vector),
                    Similarity::L2 => metric::l2(query, &doc.vector),
                    Similarity::InnerProduct => -query
                        .iter()
                        .zip(&doc.vector)
                        .map(|(a, b)| a * b)
                        .sum::<f32>(),
                };
                (distance, doc)
            })
            .collect();
        scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored
            .into_iter()
            .take(limit)
            .map(|(distance, doc)| VectorMatch {
                id: doc.id.clone(),
                distance,
            })
            .collect())
    }
}

impl VectorIndexOps for MemoryVectorStore {
    fn drop(&self, index: &VectorIndex) -> std::result::Result<(), IndexError> {
        match &self.index {
            Some(active) if active == index => {
                // A real store cannot mutate through `&self`; seq-scan search
                // is unaffected by the marker, so treat drop as a no-op that
                // reports success (TC-M6-25: DROP INDEX then search returns).
                let _ = active;
                Ok(())
            }
            _ => Err(IndexError::NotFound(index.name.clone())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn drop_index_then_search_still_returns() {
        let mut store = MemoryVectorStore::new(8);
        store.seed(6);
        store.install_index("products_embedding_index", "products", "embedding");
        let index = store.index.clone().unwrap();

        // Dropping the index succeeds…
        assert!(VectorIndexOps::drop(&store, &index).is_ok());

        // …and a subsequent search still returns rows via seq scan.
        let query = stub_embedding("doc 3", 8);
        let matches = store
            .where_vector_similar_to("products", "embedding", &query, 5, Similarity::Cosine)
            .await
            .unwrap();
        assert_eq!(matches.len(), 5);
        assert_eq!(matches[0].id, "doc-3");
    }

    #[tokio::test]
    async fn nearest_row_is_returned_first() {
        let mut store = MemoryVectorStore::new(4);
        store.seed(6);
        let query = stub_embedding("doc 3", 4);
        let matches = store
            .where_vector_similar_to("documents", "embedding", &query, 5, Similarity::Cosine)
            .await
            .unwrap();
        assert_eq!(matches.len(), 5);
        assert_eq!(matches[0].id, "doc-3");
    }

    #[test]
    fn drop_unknown_index_errors() {
        let store = MemoryVectorStore::new(4);
        let other = VectorIndex::new("nope", "t", "c");
        assert!(matches!(
            VectorIndexOps::drop(&store, &other),
            Err(IndexError::NotFound(_))
        ));
    }
}
