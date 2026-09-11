//! Dialect-neutral decoded row.
//!
//! [`DbRow`] pairs ordered column names with [`Value`]s so callers never depend
//! on a concrete driver's row type. It is produced by the decoders in
//! [`super::codec`] after a query returns.

use crate::types::Value;

/// A decoded row: ordered column names plus dialect-neutral values.
#[derive(Debug, Clone, PartialEq)]
pub struct DbRow {
    columns: Vec<String>,
    values: Vec<Value>,
}

impl DbRow {
    /// Build a row from ordered column names and values.
    pub fn new(columns: Vec<String>, values: Vec<Value>) -> Self {
        Self { columns, values }
    }

    /// Column names in result order.
    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    /// Values in result order (parallel to [`DbRow::columns`]).
    pub fn values(&self) -> &[Value] {
        &self.values
    }

    /// Number of columns in the row.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether the row has no columns.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Look up a value by column name.
    pub fn get(&self, column: &str) -> Option<&Value> {
        self.columns
            .iter()
            .position(|name| name == column)
            .and_then(|index| self.values.get(index))
    }

    /// Look up a value by column index.
    pub fn get_index(&self, index: usize) -> Option<&Value> {
        self.values.get(index)
    }
}
