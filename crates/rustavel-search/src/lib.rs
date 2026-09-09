//! Rustavel Search — vector similarity search contracts.
//!
//! Sprint 07 (M6) scope: the `VectorSearch` trait with
//! `where_vector_similar_to`, the `Str::to_embeddings` extension stub, and
//! `drop_vector_index`. Engine integration (pgvector HNSW/IVFFLAT) is M6-full;
//! this crate ships compilable stubs plus the `VectorDocument` contract.

pub mod embeddings;
pub mod index;
pub mod store;
pub mod vector;

pub use embeddings::{EmbeddingError, Str, VectorEmbeddings};
pub use index::{IndexError, VectorIndex, VectorIndexOps};
pub use store::MemoryVectorStore;
pub use vector::{VectorDocument, VectorMatch, VectorSearch, VectorSearchError};

/// Similarity ranking strategy for a vector search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Similarity {
    /// Cosine distance (default for [OI]-style embeddings).
    Cosine,
    /// Euclidean (L2) distance.
    L2,
    /// Inner-product distance.
    InnerProduct,
}
