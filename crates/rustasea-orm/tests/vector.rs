//! pgvector tests (feature `vector`).
//!
//! Shape tests run without a database; the live nearest-neighbour test is
//! gated on both the `vector` and `postgres` features plus a `DATABASE_URL`
//! whose server has the `vector` extension, and is skipped otherwise.

#![cfg(feature = "vector")]

#[cfg(any(feature = "sqlite", feature = "postgres"))]
use rustasea_orm::DbPool;
use rustasea_orm::{
    vector::MAX_DIMENSION, Blueprint, OrmError, QueryBuilder, VectorMetric, VectorSimilarity,
};

/// Verifies `Blueprint::vector` emits a `VECTOR(dim)` column.
#[test]
fn blueprint_vector_column() {
    let sql = Blueprint::create("products")
        .column("id UUID PRIMARY KEY")
        .vector("embedding", 1536)
        .to_sql();
    assert!(sql.contains("embedding VECTOR(1536)"), "{sql}");
}

/// Verifies `drop_vector_index` emits the conventional DROP INDEX DDL.
#[test]
fn blueprint_drop_vector_index_ddl() {
    let sql = Blueprint::create("products")
        .drop_vector_index("embedding")
        .to_sql();
    assert_eq!(sql, "DROP INDEX IF EXISTS products_embedding_hnsw");
}

/// Verifies the HNSW index DDL targets the opclass.
#[test]
fn blueprint_vector_index_ddl() {
    let sql = Blueprint::create("products")
        .vector_index("embedding", "vector_cosine_ops")
        .to_sql();
    assert!(
        sql.contains("USING hnsw (embedding vector_cosine_ops)"),
        "{sql}"
    );
}

/// Verifies cosine is the default metric and its operator is `<=>`.
#[test]
fn default_metric_is_cosine() {
    let sim = VectorSimilarity::new(vec![0.1, 0.2]).unwrap();
    assert_eq!(sim.metric, VectorMetric::Cosine);
    assert_eq!(sim.metric.operator(), "<=>");
    assert_eq!(VectorMetric::L2.operator(), "<->");
    assert_eq!(VectorMetric::InnerProduct.operator(), "<#>");
}

/// Verifies `where_vector_similar_to` orders by cosine and limits to top-k.
#[test]
fn where_vector_similar_to_orders_cosine() {
    let qb = QueryBuilder::table("products")
        .where_vector_similar_to("embedding", &[0.1, 0.2, 0.3], 10)
        .unwrap();
    let sql = qb.to_sql().unwrap();
    assert!(sql.contains("embedding IS NOT NULL"), "{sql}");
    assert!(sql.contains("ORDER BY embedding <=> $1 ASC"), "{sql}");
    assert!(sql.contains("LIMIT 10"), "{sql}");
    let where_clause = sql.split("ORDER BY").next().unwrap_or_default();
    assert!(!where_clause.contains("embedding <=> $1"), "{sql}");
}

/// Verifies `order_by_distance` supports L2 and inner product operators.
#[test]
fn order_by_distance_supports_l2_and_ip() {
    let l2 = QueryBuilder::table("products")
        .order_by_distance("embedding", &[0.1, 0.2], VectorMetric::L2)
        .unwrap();
    assert!(l2
        .to_sql()
        .unwrap()
        .contains("ORDER BY embedding <-> $1 ASC"));

    let ip = QueryBuilder::table("products")
        .order_by_distance("embedding", &[0.1, 0.2], VectorMetric::InnerProduct)
        .unwrap();
    assert!(ip
        .to_sql()
        .unwrap()
        .contains("ORDER BY embedding <#> $1 ASC"));
}

/// Verifies a dimension mismatch is a typed `VectorDimensionMismatch`.
#[test]
fn dimension_mismatch_is_typed() {
    let sim = VectorSimilarity::new(vec![0.1, 0.2]).unwrap();
    let error = sim.with_dimension(1536).unwrap_err();
    assert!(matches!(
        error,
        OrmError::VectorDimensionMismatch {
            expected: 1536,
            actual: 2
        }
    ));
}

/// Verifies the builder surfaces a typed error for a mismatched column dim.
#[test]
fn builder_dimension_mismatch_is_typed() {
    let error = QueryBuilder::table("products")
        .where_vector_similar_to_dim("embedding", &[0.1; 768], 10, 1536, VectorMetric::Cosine)
        .err()
        .expect("dimension mismatch must be an error");
    assert!(matches!(
        error,
        OrmError::VectorDimensionMismatch {
            expected: 1536,
            actual: 768
        }
    ));
}

/// Verifies empty and oversized embeddings are rejected before any round-trip.
#[test]
fn invalid_embeddings_rejected() {
    assert!(matches!(
        VectorSimilarity::new(vec![]),
        Err(OrmError::InvalidValue(_))
    ));
    let oversize = vec![0.0_f32; MAX_DIMENSION as usize + 1];
    assert!(matches!(
        VectorSimilarity::new(oversize),
        Err(OrmError::InvalidValue(_))
    ));
}

/// Verifies the migration guard surfaces `ExtensionMissing` when `vector` is absent.
#[cfg(feature = "sqlite")]
#[tokio::test]
async fn require_extension_missing_is_typed() {
    let pool = DbPool::connect("sqlite::memory:").await.unwrap();
    assert!(!rustasea_orm::vector::has_extension(&pool).await.unwrap());
    let error = rustasea_orm::vector::require_extension(&pool)
        .await
        .expect_err("sqlite has no pgvector");
    assert!(matches!(
        error,
        OrmError::Migration(rustasea_orm::MigrationError::ExtensionMissing { ref extension, .. })
            if extension == "vector"
    ));
    pool.close().await;
}

/// Live nearest-neighbour check — skipped unless Postgres + pgvector is present.
///
/// Set `DATABASE_URL` (e.g. `postgres://user:pass@localhost/rustasea`) with the
/// `vector` extension installed to exercise the real round-trip; without it the
/// test returns early so the suite stays green on machines without Postgres.
#[cfg(feature = "postgres")]
#[tokio::test]
async fn where_vector_similar_to_returns_ordered_top_k() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        return;
    };
    let pool = match DbPool::connect(&url).await {
        Ok(pool) => pool,
        Err(_) => return,
    };
    if !rustasea_orm::vector::has_extension(&pool)
        .await
        .unwrap_or(false)
    {
        return;
    }

    pool.execute_script(
        "DROP TABLE IF EXISTS vec_items;
         CREATE TABLE vec_items (id INT PRIMARY KEY, embedding VECTOR(3));",
    )
    .await
    .unwrap();
    for (id, embedding) in [
        (1, "[1.0,0.0,0.0]"),
        (2, "[0.0,1.0,0.0]"),
        (3, "[0.0,0.0,1.0]"),
    ] {
        pool.execute_bind(
            "INSERT INTO vec_items (id, embedding) VALUES ($1, $2::vector)",
            &[
                rustasea_orm::Value::Int(id),
                rustasea_orm::Value::Text(embedding.to_string()),
            ],
        )
        .await
        .unwrap();
    }

    let rows = QueryBuilder::table("vec_items")
        .where_vector_similar_to("embedding", &[1.0, 0.0, 0.0], 2)
        .get(&pool)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["id"], 1, "closest vector must come first");
    assert_eq!(rows[1]["id"], 2);

    pool.execute_script("DROP TABLE vec_items;").await.unwrap();
    pool.close().await;
}
