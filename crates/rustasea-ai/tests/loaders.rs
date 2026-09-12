//! Integration tests for the deferred loaders backed by real engines.
//!
//! With the `search` feature a [`SimilaritySearch`] bound to a
//! [`rustasea_search::MemoryVectorStore`] returns distance-ranked matches; with
//! the `storage` feature a [`FileStorage`] bound to a local disk round-trips
//! bytes. When a feature is disabled the loader returns a typed capability
//! denial instead of panicking.

use rustasea_ai::loaders::{FileStorage, SimilaritySearch, ToolSearch};

/// Build a small in-memory vector store with two well-separated documents.
#[cfg(feature = "search")]
fn store() -> std::sync::Arc<rustasea_search::MemoryVectorStore> {
    use rustasea_search::{MemoryVectorStore, VectorDocument};

    let mut store = MemoryVectorStore::new(4);
    store.insert(VectorDocument {
        id: "near".to_string(),
        vector: vec![1.0, 0.0, 0.0, 0.0],
    });
    store.insert(VectorDocument {
        id: "far".to_string(),
        vector: vec![0.0, 1.0, 0.0, 0.0],
    });
    std::sync::Arc::new(store)
}

#[cfg(feature = "search")]
#[tokio::test]
async fn similarity_search_returns_ranked_matches() {
    let mut loader = SimilaritySearch::new("docs");
    loader.bind(store());
    assert!(loader.ready());

    let matches = loader
        .search(
            "docs",
            "embedding",
            &[1.0, 0.0, 0.0, 0.0],
            2,
            rustasea_search::Similarity::Cosine,
        )
        .await
        .expect("search should succeed against a bound store");

    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].id, "near");
    assert!(matches[0].distance <= matches[1].distance);
}

#[cfg(feature = "search")]
#[tokio::test]
async fn similarity_search_dimension_mismatch_is_backend_error() {
    let mut loader = SimilaritySearch::new("docs");
    loader.bind(store());
    let error = loader
        .search(
            "docs",
            "embedding",
            &[1.0, 0.0],
            1,
            rustasea_search::Similarity::Cosine,
        )
        .await
        .expect_err("dimension mismatch must surface as a typed error");
    assert!(matches!(error, rustasea_ai::AiError::Backend { .. }));
}

#[cfg(not(feature = "search"))]
#[tokio::test]
async fn similarity_search_without_backend_is_typed_error() {
    let loader = SimilaritySearch::new("docs");
    assert!(!loader.ready());
    let error = loader.handle().expect_err("no engine available");
    assert!(matches!(
        error,
        rustasea_ai::AiError::UnsupportedCapability { .. }
    ));
}

#[cfg(feature = "storage")]
#[tokio::test]
async fn file_storage_round_trips_bytes() {
    let dir = std::env::temp_dir().join(format!("rustasea-ai-{}", std::process::id()));
    let loader = FileStorage::local("assets", dir.clone());
    assert!(loader.ready());

    loader.put("greeting.txt", b"hello").await.expect("put");
    assert!(loader.exists("greeting.txt").await.expect("exists"));
    assert_eq!(loader.get("greeting.txt").await.expect("get"), b"hello");
    loader.delete("greeting.txt").await.expect("delete");
    assert!(!loader.exists("greeting.txt").await.expect("exists"));
    let _ = std::fs::remove_dir_all(dir);
}

#[cfg(not(feature = "storage"))]
#[tokio::test]
async fn file_storage_without_backend_is_typed_error() {
    let loader = FileStorage::new("assets");
    assert!(!loader.ready());
    let error = loader.handle().expect_err("no engine available");
    assert!(matches!(
        error,
        rustasea_ai::AiError::UnsupportedCapability { .. }
    ));
}

#[test]
fn tool_search_ranks_by_bm25_over_descriptions() {
    let mut loader = ToolSearch::new("find-tools");
    loader
        .with_query("search the documentation")
        .with_candidates([
            ("send_email", "Send an email message to a recipient"),
            (
                "search_docs",
                "Search the documentation corpus for a matching phrase",
            ),
            ("web_search", "Search the public web for a URL"),
        ]);

    let ranked = loader.search_candidates();
    assert_eq!(ranked.len(), 2);
    assert_eq!(ranked[0].name, "search_docs");
    // Descending relevance.
    assert!(ranked.windows(2).all(|pair| pair[0].score >= pair[1].score));
}
