//! Vector extension support (pgvector) — `vector` feature.
//!
//! Sprint 03 provides the query shape (`ORDER BY col <=> $1 LIMIT k`), the
//! `VECTOR(n)` column type, and typed dimension errors. Execution against a real
//! pgvector-enabled Postgres lands with the sqlx wiring and testcontainers suite.

use crate::error::{OrmError, Result};

/// Maximum embedding dimension accepted by the ORM.
pub const MAX_DIMENSION: u32 = 16_000;

/// A validated embedding vector plus similarity configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorSimilarity {
    /// The query embedding; dimension must match the column's `VECTOR(n)`.
    pub embedding: Vec<f32>,
    /// Distance metric emitted by `whereVectorSimilarTo`.
    pub metric: VectorMetric,
    /// Expected column dimension, when known — mismatch is a typed error.
    pub expected_dimension: Option<u32>,
}

/// Distance operator for similarity ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorMetric {
    /// `<=>` cosine distance (default).
    Cosine,
    /// `<->` L2 distance.
    L2,
    /// `<#>` negative inner product.
    InnerProduct,
}

impl VectorMetric {
    /// pgvector distance operator.
    pub fn operator(&self) -> &'static str {
        match self {
            VectorMetric::Cosine => "<=>",
            VectorMetric::L2 => "<->",
            VectorMetric::InnerProduct => "<#>",
        }
    }
}

impl VectorSimilarity {
    /// Validate and construct a similarity query payload.
    ///
    /// Returns `VectorDimensionMismatch` when `expected_dimension` disagrees
    /// with the embedding length, and rejects empty/oversized vectors.
    pub fn new(embedding: Vec<f32>) -> Result<Self> {
        if embedding.is_empty() {
            return Err(OrmError::Vector("embedding must be non-empty".into()));
        }
        if embedding.len() as u32 > MAX_DIMENSION {
            return Err(OrmError::Vector(format!(
                "embedding dimension {} exceeds pgvector maximum {MAX_DIMENSION}",
                embedding.len()
            )));
        }
        Ok(Self {
            embedding,
            metric: VectorMetric::Cosine,
            expected_dimension: None,
        })
    }

    /// Set the expected column dimension and validate immediately.
    pub fn with_dimension(mut self, dim: u32) -> Result<Self> {
        if dim == 0 || dim > MAX_DIMENSION {
            return Err(OrmError::Vector(format!("invalid column dimension {dim}")));
        }
        if self.embedding.len() as u32 != dim {
            return Err(OrmError::Vector(format!(
                "dimension mismatch: column VECTOR({dim}) but embedding has {} dimensions",
                self.embedding.len()
            )));
        }
        self.expected_dimension = Some(dim);
        Ok(self)
    }

    /// Select the distance metric.
    pub fn with_metric(mut self, metric: VectorMetric) -> Self {
        self.metric = metric;
        self
    }
}

/// Render a literal pgvector array bind (`'[0.1,0.2]'`) for `toRawSql` display.
pub fn to_vector_literal(embedding: &[f32]) -> String {
    let parts: Vec<String> = embedding.iter().map(|f| f.to_string()).collect();
    format!("'[{}]'::vector", parts.join(","))
}

/// Check whether the `vector` extension is available (guarded in migrations).
///
/// Real implementation executes `SELECT 1 FROM pg_extension WHERE extname='vector'`;
/// the stub returns the would-be SQL for the Migrator to run.
pub fn has_extension_sql() -> &'static str {
    "SELECT 1 FROM pg_extension WHERE extname = 'vector'"
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies dimension mismatch produces a typed vector error.
    #[test]
    fn dimension_mismatch_is_typed() {
        let sim = VectorSimilarity::new(vec![0.1, 0.2]).unwrap();
        let err = sim.with_dimension(1536).unwrap_err();
        match err {
            OrmError::Vector(msg) => assert!(msg.contains("mismatch")),
            other => panic!("expected Vector error, got {other:?}"),
        }
    }

    /// Verifies empty embeddings are rejected.
    #[test]
    fn empty_embedding_rejected() {
        assert!(VectorSimilarity::new(vec![]).is_err());
    }
}
