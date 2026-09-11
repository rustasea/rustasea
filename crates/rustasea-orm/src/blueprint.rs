//! Schema blueprint helpers for migration DDL — columns, indexes, and the
//! feature-gated pgvector column/index operations.
//!
//! A [`Blueprint`] accumulates a `CREATE TABLE` body plus any standalone DDL
//! statements (indexes, drops) and renders them as one `;`-separated script for
//! [`crate::migration::Migration::up`]. Vector methods are available only with
//! the `vector` feature.

/// Builder for a single table's migration DDL.
///
/// ```rust,ignore
/// let sql = Blueprint::create("products")
///     .vector("embedding", 1536)
///     .vector_index("embedding", "vector_cosine_ops")
///     .to_sql();
/// ```
#[derive(Debug, Clone, Default)]
pub struct Blueprint {
    /// Table the blueprint targets (drives index naming).
    table: String,
    /// Accumulated column definitions for the `CREATE TABLE` body.
    columns: Vec<String>,
    /// Standalone statements emitted after the table (indexes, drops).
    statements: Vec<String>,
}

impl Blueprint {
    /// Start a blueprint for `table`.
    pub fn create(table: &str) -> Self {
        Self {
            table: table.to_string(),
            ..Self::default()
        }
    }

    /// The target table name.
    pub fn table(&self) -> &str {
        &self.table
    }

    /// Add a raw column definition (e.g. `id UUID PRIMARY KEY`).
    pub fn column(mut self, definition: &str) -> Self {
        self.columns.push(definition.to_string());
        self
    }

    /// Add a pgvector `VECTOR(dim)` column.
    #[cfg(feature = "vector")]
    pub fn vector(mut self, name: &str, dim: u32) -> Self {
        self.columns.push(format!("{name} VECTOR({dim})"));
        self
    }

    /// Emit a `CREATE INDEX … USING hnsw` for a vector column.
    #[cfg(feature = "vector")]
    pub fn vector_index(mut self, column: &str, opclass: &str) -> Self {
        let index = self.index_name(column);
        self.statements.push(format!(
            "CREATE INDEX IF NOT EXISTS {index} ON {} USING hnsw ({column} {opclass})",
            self.table
        ));
        self
    }

    /// Emit `DROP INDEX IF EXISTS <table>_<column>_hnsw` for a vector column.
    #[cfg(feature = "vector")]
    pub fn drop_vector_index(mut self, column: &str) -> Self {
        let index = self.index_name(column);
        self.statements
            .push(format!("DROP INDEX IF EXISTS {index}"));
        self
    }

    /// The conventional HNSW index name for a vector `column`.
    #[cfg(feature = "vector")]
    pub fn index_name(&self, column: &str) -> String {
        format!("{}_{column}_hnsw", self.table)
    }

    /// Render the accumulated DDL as one `;`-separated script.
    pub fn to_sql(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if !self.columns.is_empty() {
            parts.push(format!(
                "CREATE TABLE {} ({})",
                self.table,
                self.columns.join(", ")
            ));
        }
        parts.extend(self.statements.iter().cloned());
        parts.join(";\n")
    }
}

#[cfg(all(test, feature = "vector"))]
mod tests {
    use super::*;

    /// Verifies a vector column renders as `VECTOR(dim)`.
    #[test]
    fn vector_column_renders() {
        let sql = Blueprint::create("products")
            .column("id UUID PRIMARY KEY")
            .vector("embedding", 1536)
            .to_sql();
        assert!(sql.contains("embedding VECTOR(1536)"), "{sql}");
    }

    /// Verifies `drop_vector_index` emits the conventional DROP INDEX DDL.
    #[test]
    fn drop_vector_index_emits_ddl() {
        let sql = Blueprint::create("products")
            .drop_vector_index("embedding")
            .to_sql();
        assert_eq!(sql, "DROP INDEX IF EXISTS products_embedding_hnsw");
    }

    /// Verifies the HNSW index DDL targets the table and opclass.
    #[test]
    fn vector_index_emits_hnsw() {
        let sql = Blueprint::create("products")
            .vector_index("embedding", "vector_cosine_ops")
            .to_sql();
        assert!(
            sql.contains(
                "CREATE INDEX IF NOT EXISTS products_embedding_hnsw ON products USING hnsw (embedding vector_cosine_ops)"
            ),
            "{sql}"
        );
    }
}
