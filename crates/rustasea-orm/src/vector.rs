//! Vector extension support (pgvector) — `vector` feature.
//!
//! Provides the query shape (`ORDER BY col <=> $1 LIMIT k`), the `VECTOR(n)`
//! column type, the `order_by_distance` metric switch (L2 `<->` / inner product
//! `<#>`), typed dimension errors, and the `has_extension("vector")` migration
//! guard. Execution runs through the normal runtime `sqlx` path.

use crate::db::DbPool;
use crate::error::{OrmError, Result};
use crate::migration::MigrationError;
#[cfg(feature = "vector")]
use crate::types::Value;

/// Maximum embedding dimension accepted by the ORM.
pub const MAX_DIMENSION: u32 = 16_000;

/// Remediation hint surfaced with [`MigrationError::ExtensionMissing`].
pub const VECTOR_EXTENSION_HINT: &str = "CREATE EXTENSION IF NOT EXISTS vector;";


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
            return Err(OrmError::InvalidValue("embedding must be non-empty".into()));
        }
        if embedding.len() as u32 > MAX_DIMENSION {
            return Err(OrmError::InvalidValue(format!(
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
            return Err(OrmError::InvalidValue(format!(
                "invalid column dimension {dim}"
            )));
        }
        if self.embedding.len() as u32 != dim {
            return Err(OrmError::VectorDimensionMismatch {
                expected: dim as usize,
                actual: self.embedding.len(),
            });
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
/// Executes `SELECT 1 FROM pg_extension WHERE extname = 'vector'`; a missing
/// extension surfaces as [`MigrationError::ExtensionMissing`] carrying
/// [`VECTOR_EXTENSION_HINT`]. Non-Postgres pools have no `pg_extension`, so the
/// probe reports the extension as absent.
pub async fn has_extension(pool: &DbPool) -> Result<bool> {
    if pool.dialect() != "postgres" {
        return Ok(false);
    }
    let rows = pool
        .fetch_json(
            "SELECT 1 AS present FROM pg_extension WHERE extname = 'vector'",
            &[],
        )
        .await?;
    Ok(!rows.is_empty())
}

/// Fail with [`MigrationError::ExtensionMissing`] when `vector` is absent.
///
/// Call from a migration `up` body before emitting `VECTOR`/HNSW DDL.
pub async fn require_extension(pool: &DbPool) -> Result<()> {
    if has_extension(pool).await? {
        return Ok(());
    }
    Err(OrmError::Migration(MigrationError::ExtensionMissing {
        extension: "vector".to_string(),
        hint: VECTOR_EXTENSION_HINT.to_string(),
    }))
}

/// Check whether the `vector` extension is available (guarded in migrations).
///
/// Real implementation executes `SELECT 1 FROM pg_extension WHERE extname='vector'`;
/// the stub returns the would-be SQL for the Migrator to run.
pub fn has_extension_sql() -> &'static str {
    "SELECT 1 FROM pg_extension WHERE extname = 'vector'"
}

/// Render the pgvector bind literal for an embedding (`[a,b,c]::vector`).
///
/// Used when a similarity query must inline the embedding as a cast parameter.
#[cfg(feature = "vector")]
pub fn vector_param(embedding: &[f32]) -> Value {
    Value::Vector(embedding.to_vec())
}


#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies dimension mismatch produces a typed vector error.
    #[test]
    fn dimension_mismatch_is_typed() {
        let sim = VectorSimilarity::new(vec![0.1, 0.2]).unwrap();
        let err = sim.with_dimension(1536).unwrap_err();
        assert!(matches!(
            err,
            OrmError::VectorDimensionMismatch {
                expected: 1536,
                actual: 2,
            }
        ));
    }

    /// Verifies empty embeddings are rejected.
    #[test]
    fn empty_embedding_rejected() {
        assert!(matches!(
            VectorSimilarity::new(vec![]),
            Err(OrmError::InvalidValue(_))
        ));
    }
}
