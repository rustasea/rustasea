//! Vector search trait, shared document contract, and errors.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::Similarity;

/// Alias for results produced by vector search operations.
pub type Result<T> = std::result::Result<T, VectorSearchError>;

/// Top-level vector search error type.
#[derive(Debug, Error)]
pub enum VectorSearchError {
    /// Query/column dimension mismatch with the stored vectors.
    #[error("vector dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch {
        /// Expected embedding dimension.
        expected: usize,
        /// Supplied embedding dimension.
        actual: usize,
    },

    /// The requested vector index does not exist.
    #[error("vector index not found: {0}")]
    IndexNotFound(String),

    /// The vector store engine is unavailable (extension/driver missing).
    #[error("vector store unavailable: {0}")]
    StoreUnavailable(String),
}

/// A searchable row carrying a vector plus a stable id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorDocument {
    /// Row primary key.
    pub id: String,
    /// Raw vector (must equal the store's declared dimension).
    pub vector: Vec<f32>,
}

/// Result of a similarity search: nearest rows in distance order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorMatch {
    /// Matched row primary key.
    pub id: String,
    /// Distance to the query vector (lower is closer).
    pub distance: f32,
}

/// Free similarity helpers shared by engine adapters.
///
/// Kept outside the [`VectorSearch`] trait so the trait stays `dyn`
/// compatible (no static methods on an object-safe trait).
pub mod metric {
    /// Cosine similarity between two vectors.
    pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        let mut dot = 0.0_f32;
        let mut na = 0.0_f32;
        let mut nb = 0.0_f32;
        for (x, y) in a.iter().zip(b.iter()) {
            dot += x * y;
            na += x * x;
            nb += y * y;
        }
        let denom = (na.sqrt() * nb.sqrt()).max(f32::EPSILON);
        dot / denom
    }

    /// L2 distance between two vectors.
    pub fn l2(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return f32::MAX;
        }
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y) * (x - y))
            .sum::<f32>()
            .sqrt()
    }
}

/// Nearest-neighbour search over a vector column (pgvector parity).
///
/// `where_vector_similar_to` returns the `limit` closest rows to `query`
/// under the chosen [`Similarity`]. Engine adapters implement this trait;
/// adapters enforce dimension equality (FS-M6-02 contract: 5 nearest for a
/// 5-limit query) via [`validate_dimension`].
#[async_trait::async_trait]
pub trait VectorSearch: Send + Sync + 'static {
    /// Search `table.column` for the `limit` rows nearest `query`.
    async fn where_vector_similar_to(
        &self,
        table: &str,
        column: &str,
        query: &[f32],
        limit: usize,
        metric: Similarity,
    ) -> Result<Vec<VectorMatch>>;
}

/// Validate that `query` matches the store's declared dimension.
pub fn validate_dimension(query: &[f32], expected: usize) -> Result<()> {
    if query.len() != expected {
        return Err(VectorSearchError::DimensionMismatch {
            expected,
            actual: query.len(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::metric::{cosine, l2};
    use super::*;
    use crate::embeddings::stub_embedding;

    struct StubSearch {
        dim: usize,
        docs: Vec<VectorDocument>,
    }

    #[async_trait::async_trait]
    impl VectorSearch for StubSearch {
        async fn where_vector_similar_to(
            &self,
            _table: &str,
            _column: &str,
            query: &[f32],
            limit: usize,
            metric: Similarity,
        ) -> Result<Vec<VectorMatch>> {
            validate_dimension(query, self.dim)?;
            let mut scored: Vec<(f32, &VectorDocument)> = self
                .docs
                .iter()
                .map(|d| {
                    let distance = match metric {
                        Similarity::Cosine => 1.0 - cosine(query, &d.vector),
                        Similarity::L2 => l2(query, &d.vector),
                        Similarity::InnerProduct => {
                            -query.iter().zip(&d.vector).map(|(a, b)| a * b).sum::<f32>()
                        }
                    };
                    (distance, d)
                })
                .collect();
            scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            Ok(scored
                .into_iter()
                .take(limit)
                .map(|(distance, d)| VectorMatch {
                    id: d.id.clone(),
                    distance,
                })
                .collect())
        }
    }

    fn docs() -> Vec<VectorDocument> {
        (0..6)
            .map(|i| VectorDocument {
                id: format!("doc-{i}"),
                vector: stub_embedding(&format!("doc {i}"), 4),
            })
            .collect()
    }

    #[tokio::test]
    async fn returns_5_nearest() {
        let store = StubSearch {
            dim: 4,
            docs: docs(),
        };
        let query = stub_embedding("doc 3", 4);
        let matches = store
            .where_vector_similar_to("documents", "embedding", &query, 5, Similarity::Cosine)
            .await
            .unwrap();
        assert_eq!(matches.len(), 5);
        // The exact row is the nearest neighbour.
        assert_eq!(matches[0].id, "doc-3");
    }

    #[tokio::test]
    async fn dimension_mismatch_is_rejected() {
        let store = StubSearch {
            dim: 4,
            docs: docs(),
        };
        let err = store
            .where_vector_similar_to("documents", "embedding", &[0.0; 3], 5, Similarity::Cosine)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            VectorSearchError::DimensionMismatch {
                expected: 4,
                actual: 3
            }
        ));
    }
}
