//! Eager-loading methods on [`QueryBuilder`] — `with`, `with_relations`,
//! `get_eager`, and `first_eager`.
//!
//! Split out of [`crate::builder`] to keep the builder module within the
//! file-size limit. Execution runs one query per requested relation via
//! [`crate::eager::eager_load`].

use super::{Executor, QueryBuilder};
use crate::error::Result;
use crate::model::Relation;

impl QueryBuilder {
    /// Request relations to eager-load (`with("posts")`).
    ///
    /// Names are resolved against the metadata attached by
    /// [`QueryBuilder::with_relations`]; execution is deferred to
    /// [`QueryBuilder::get_eager`], which runs one query per relation.
    pub fn with(mut self, names: &[&str]) -> Self {
        self.eager = names.iter().map(|name| (*name).to_string()).collect();
        self
    }

    /// Attach the declared relation metadata used to resolve `with` names.
    ///
    /// [`crate::model::Model::query`] calls this with `Self::relations()`.
    pub fn with_relations(mut self, relations: Vec<Relation>) -> Self {
        self.eager_declared = relations;
        self
    }

    /// The requested eager relation names.
    pub fn eager_names(&self) -> &[String] {
        &self.eager
    }

    /// Whether eager loading was requested.
    pub fn has_eager(&self) -> bool {
        !self.eager.is_empty()
    }

    /// Execute the query and eager-load every requested relation.
    ///
    /// Runs the base SELECT plus one query per requested relation, attaching
    /// results into each row's `relations` map. Returns the hydrated rows.
    pub async fn get_eager<'a>(
        self,
        executor: impl Into<Executor<'a>>,
    ) -> Result<Vec<serde_json::Value>> {
        let mut executor = executor.into();
        let sql = self.to_sql()?;
        let bindings = self.bindings().to_vec();
        let mut rows = executor.fetch_json(&sql, &bindings).await?;
        if self.eager.is_empty() {
            return Ok(rows);
        }
        let names: Vec<&str> = self.eager.iter().map(String::as_str).collect();
        let plan = crate::eager::EagerPlan::new()
            .request(&names)
            .declared(&self.eager_declared);
        crate::eager::eager_load(&mut rows, &mut executor, &plan).await?;
        Ok(rows)
    }

    /// Execute the query and eager-load relations, returning the first row.
    pub async fn first_eager<'a>(
        self,
        executor: impl Into<Executor<'a>>,
    ) -> Result<Option<serde_json::Value>> {
        Ok(self.limit(1).get_eager(executor).await?.into_iter().next())
    }
}
