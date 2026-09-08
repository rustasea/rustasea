//! Vector index management: `VectorIndex` stub + drop contract.

use thiserror::Error;

/// Alias for results produced by index operations.
pub type Result<T> = std::result::Result<T, IndexError>;

/// Errors produced while managing vector indexes.
#[derive(Debug, Error)]
pub enum IndexError {
    /// The index does not exist.
    #[error("vector index not found: {0}")]
    NotFound(String),
    /// The index could not be created/dropped.
    #[error("vector index operation failed: {0}")]
    Operation(String),
}

/// A named vector index over one column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorIndex {
    /// Index name (SQL identifier).
    pub name: String,
    /// Table the index belongs to.
    pub table: String,
    /// Column carrying the vectors.
    pub column: String,
}

impl VectorIndex {
    /// Create a named index reference.
    pub fn new(
        name: impl Into<String>,
        table: impl Into<String>,
        column: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            table: table.into(),
            column: column.into(),
        }
    }

    /// Qualified SQL name (`schema.index` form is caller-controlled).
    pub fn qualified_name(&self) -> String {
        self.name.clone()
    }
}

/// Index lifecycle operations over a vector store.
///
/// `drop` removes the index (FS-M6-02 contract); creating indexes is the
/// engine's concern (M6-full) so only the drop half ships as a stub surface.
pub trait VectorIndexOps: Send + Sync + 'static {
    /// Drop the index, erroring when it does not exist.
    fn drop(&self, index: &VectorIndex) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoopIndex;

    impl VectorIndexOps for NoopIndex {
        fn drop(&self, index: &VectorIndex) -> Result<()> {
            Err(IndexError::NotFound(index.name.clone()))
        }
    }

    #[test]
    fn drop_missing_index_errors() {
        let ops = NoopIndex;
        let idx = VectorIndex::new("idx_docs_embedding", "documents", "embedding");
        assert!(matches!(
            ops.drop(&idx),
            Err(IndexError::NotFound(name)) if name == "idx_docs_embedding"
        ));
    }
}
