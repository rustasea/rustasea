//! PostgreSQL/pgvector-backed vector store (`pgvector` feature).
//!
//! [`PgVectorStore`] runs native `pgvector` similarity queries through the ORM
//! query builder: the embedding binds as a `vector` (no text literal or manual
//! `::vector` cast), Postgres orders by distance, and the distance is projected
//! back as a `distance` column. [`crate::MemoryVectorStore`] remains the
//! default/test backend.

use rustasea_orm::{DbPool, QueryBuilder, Value, VectorMetric};
use serde_json::Value as JsonValue;

use crate::index::{validate_identifier, IndexError, VectorIndex, VectorIndexOps};
use crate::vector::VectorSearchError;
use crate::vector::{validate_dimension, Result as VectorResult, VectorMatch, VectorSearch};
use crate::Similarity;

/// A pgvector-backed [`VectorSearch`] store over one table.
///
/// The store is bound to the table/column/id column supplied at construction;
/// the `table`/`column` arguments of [`VectorSearch::where_vector_similar_to`]
/// are advisory (parity with [`crate::MemoryVectorStore`]) so callers cannot
/// inject an unchecked identifier at query time.
#[derive(Debug, Clone)]
pub struct PgVectorStore {
    /// Postgres connection pool used for every query.
    pool: DbPool,
    /// Table carrying the vectors.
    table: String,
    /// Column carrying the vectors.
    column: String,
    /// Column carrying the row primary key.
    id_column: String,
    /// Declared vector dimension for this store.
    dim: usize,
}

impl PgVectorStore {
    /// Create a store over `table.column`, reading ids from `table.id_column`.
    ///
    /// Every identifier is validated and the declared dimension must be
    /// non-zero; a bad name or zero dimension surfaces as
    /// [`VectorSearchError::InvalidConfiguration`] before any round-trip.
    pub fn new(
        pool: DbPool,
        table: impl Into<String>,
        column: impl Into<String>,
        id_column: impl Into<String>,
        dim: usize,
    ) -> VectorResult<Self> {
        let store = Self {
            pool,
            table: table.into(),
            column: column.into(),
            id_column: id_column.into(),
            dim,
        };
        if store.dim == 0 {
            return Err(VectorSearchError::InvalidConfiguration(
                "vector dimension must be greater than zero".into(),
            ));
        }
        validate_identifier(&store.table).map_err(config_error)?;
        validate_identifier(&store.column).map_err(config_error)?;
        validate_identifier(&store.id_column).map_err(config_error)?;
        Ok(store)
    }

    /// The backing connection pool.
    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    /// The declared embedding dimension.
    pub fn dim(&self) -> usize {
        self.dim
    }
}

/// Map an identifier validation failure onto the store configuration error.
fn config_error(error: IndexError) -> VectorSearchError {
    VectorSearchError::InvalidConfiguration(error.to_string())
}

/// Map the search [`Similarity`] onto the ORM's distance metric.
fn to_orm_metric(similarity: Similarity) -> VectorMetric {
    match similarity {
        Similarity::Cosine => VectorMetric::Cosine,
        Similarity::L2 => VectorMetric::L2,
        Similarity::InnerProduct => VectorMetric::InnerProduct,
    }
}

/// Read one JSON row into a [`VectorMatch`] from its `id` and `distance` columns.
fn row_to_match(row: &JsonValue) -> VectorResult<VectorMatch> {
    let id = match row.get("id") {
        Some(JsonValue::String(value)) => value.clone(),
        Some(JsonValue::Number(value)) => value.to_string(),
        Some(other) => other.to_string(),
        None => {
            return Err(VectorSearchError::StoreUnavailable(
                "similarity row is missing the `id` column".into(),
            ))
        }
    };
    let distance = row
        .get("distance")
        .and_then(JsonValue::as_f64)
        .ok_or_else(|| {
            VectorSearchError::StoreUnavailable(
                "similarity row is missing the `distance` column".into(),
            )
        })?;
    Ok(VectorMatch {
        id,
        distance: distance as f32,
    })
}

#[async_trait::async_trait]
impl VectorSearch for PgVectorStore {
    /// Run `ORDER BY column <op> $1 LIMIT k` and return scored rows.
    ///
    /// The query embedding is validated against the store dimension first, so a
    /// mismatch returns [`VectorSearchError::DimensionMismatch`] without
    /// touching the database.
    async fn where_vector_similar_to(
        &self,
        _table: &str,
        _column: &str,
        query: &[f32],
        limit: usize,
        metric: Similarity,
    ) -> VectorResult<Vec<VectorMatch>> {
        validate_dimension(query, self.dim)?;
        let limit = u32::try_from(limit).unwrap_or(u32::MAX);
        let rows = QueryBuilder::table(&self.table)
            .where_vector_similar_to_scored(
                &self.column,
                query,
                limit,
                to_orm_metric(metric),
                &self.id_column,
            )
            .map_err(|error| VectorSearchError::StoreUnavailable(error.to_string()))?
            .get(&self.pool)
            .await
            .map_err(|error| VectorSearchError::StoreUnavailable(error.to_string()))?;
        rows.iter().map(row_to_match).collect()
    }
}

#[async_trait::async_trait]
impl VectorIndexOps for PgVectorStore {
    /// Report that a database-backed store must use [`Self::drop_index`].
    fn drop(&self, index: &VectorIndex) -> Result<(), IndexError> {
        Err(IndexError::Operation(format!(
            "PgVectorStore drops indexes asynchronously; call drop_index for `{}`",
            index.name
        )))
    }

    /// Drop `index`, returning [`IndexError::NotFound`] when it does not exist.
    async fn drop_index(&self, index: &VectorIndex) -> Result<(), IndexError> {
        validate_identifier(&index.name)?;
        let existing = self
            .pool
            .fetch_json(
                "SELECT 1 AS present FROM pg_indexes WHERE indexname = $1",
                &[Value::Text(index.name.clone())],
            )
            .await
            .map_err(|error| IndexError::Operation(error.to_string()))?;
        if existing.is_empty() {
            return Err(IndexError::NotFound(index.name.clone()));
        }
        self.pool
            .execute_script(&format!("DROP INDEX IF EXISTS {}", index.name))
            .await
            .map_err(|error| IndexError::Operation(error.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build an in-memory SQLite pool for pre-flight (no round-trip) tests.
    async fn sqlite_pool() -> DbPool {
        DbPool::connect("sqlite::memory:")
            .await
            .expect("sqlite pool")
    }

    /// Verifies a dimension mismatch is a typed error before any DB access.
    #[tokio::test]
    async fn dimension_mismatch_is_typed() {
        let store = PgVectorStore::new(sqlite_pool().await, "documents", "embedding", "id", 4)
            .expect("valid store");
        let error = store
            .where_vector_similar_to("documents", "embedding", &[0.0; 3], 5, Similarity::Cosine)
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            VectorSearchError::DimensionMismatch {
                expected: 4,
                actual: 3
            }
        ));
    }

    /// Verifies unsafe identifiers are rejected at construction time.
    #[tokio::test]
    async fn invalid_identifier_is_rejected() {
        let error = PgVectorStore::new(
            sqlite_pool().await,
            "documents; DROP TABLE users",
            "embedding",
            "id",
            4,
        )
        .expect_err("injection-shaped table name must be rejected");
        assert!(matches!(error, VectorSearchError::InvalidConfiguration(_)));
    }

    /// Verifies a zero dimension is rejected at construction time.
    #[tokio::test]
    async fn zero_dimension_is_rejected() {
        let error = PgVectorStore::new(sqlite_pool().await, "documents", "embedding", "id", 0)
            .expect_err("zero dimension must be rejected");
        assert!(matches!(error, VectorSearchError::InvalidConfiguration(_)));
    }

    /// Verifies the async drop delegates to the sync marker on in-memory stores.
    #[tokio::test]
    async fn memory_drop_index_delegates_to_sync_drop() {
        let mut store = crate::MemoryVectorStore::new(4);
        store.install_index("documents_embedding_hnsw", "documents", "embedding");
        let index = store.index.clone().unwrap();
        assert!(store.drop_index(&index).await.is_ok());
    }
}
