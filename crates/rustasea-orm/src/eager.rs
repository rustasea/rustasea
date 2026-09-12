//! Eager loading for `HasMany` / `BelongsTo` / `ManyToMany` relations.
//!
//! A relation is loaded with exactly one additional query (never one query per
//! parent row): the related rows are fetched for the whole parent set with a
//! single `IN (…)` (or pivot `JOIN`) and grouped back onto each parent under the
//! `relations` key. Only the `sqlx` runtime API is used — never `query!` macros.

use crate::builder::Executor;
use crate::error::{OrmError, Result};
use crate::model::{Relation, RelationKind};
use crate::types::Value;
use serde_json::{Map, Value as JsonValue};
use std::collections::HashMap;
use uuid::Uuid;

/// Requested relation names and the declared relation metadata to resolve them.
#[derive(Debug, Clone, Default)]
pub struct EagerPlan {
    /// Relation names requested via `with(&[...])`.
    names: Vec<String>,
    /// Declared relations available for resolution.
    declared: Vec<Relation>,
}

impl EagerPlan {
    /// Create an empty plan.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register the relation names requested by the caller.
    pub fn request(mut self, names: &[&str]) -> Self {
        self.names = names.iter().map(|n| (*n).to_string()).collect();
        self
    }

    /// Register the declared relations used to resolve the requests.
    pub fn declared(mut self, relations: &[Relation]) -> Self {
        self.declared = relations.to_vec();
        self
    }

    /// The requested relation names.
    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// Whether any relation was requested.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// Load the requested relations into every parent row under `relations`.
///
/// Runs one query per requested relation; each relation's rows are grouped by
/// foreign key and attached to the matching parent. A requested name with no
/// declared relation is a typed [`OrmError::InvalidState`].
pub async fn eager_load<'a>(
    rows: &mut [JsonValue],
    executor: &mut Executor<'a>,
    plan: &EagerPlan,
) -> Result<()> {
    if plan.is_empty() || rows.is_empty() {
        return Ok(());
    }
    for name in plan.names() {
        let relation = plan
            .declared
            .iter()
            .find(|relation| &relation.name == name)
            .ok_or_else(|| {
                OrmError::InvalidState(format!("relation `{name}` is not declared on the model"))
            })?;
        match relation.kind {
            RelationKind::HasMany => load_has_many(rows, executor, relation).await?,
            RelationKind::BelongsTo => load_belongs_to(rows, executor, relation).await?,
            RelationKind::ManyToMany => load_many_to_many(rows, executor, relation).await?,
        }
    }
    Ok(())
}

/// Eager-load a `HasMany`: related rows keyed by the foreign key.
async fn load_has_many<'a>(
    rows: &mut [JsonValue],
    executor: &mut Executor<'a>,
    relation: &Relation,
) -> Result<()> {
    let keys = parent_keys(rows, &relation.local_key);
    let mut grouped: HashMap<String, Vec<JsonValue>> = HashMap::new();
    if !keys.is_empty() {
        let builder = crate::builder::QueryBuilder::table(relation.related_table.clone())
            .where_in(&relation.foreign_key, keys.clone());
        let sql = builder.to_sql()?;
        let bindings = builder.bindings().to_vec();
        let related = executor.fetch_json(&sql, &bindings).await?;
        for row in related {
            if let Some(key) = row_key(&row, &relation.foreign_key) {
                grouped.entry(key).or_default().push(row);
            }
        }
    }
    attach_arrays(rows, &relation.local_key, &relation.name, grouped);
    Ok(())
}

/// Eager-load a `BelongsTo`: one parent object keyed by its primary key.
async fn load_belongs_to<'a>(
    rows: &mut [JsonValue],
    executor: &mut Executor<'a>,
    relation: &Relation,
) -> Result<()> {
    let keys = parent_keys(rows, &relation.foreign_key);
    let mut by_id: HashMap<String, JsonValue> = HashMap::new();
    if !keys.is_empty() {
        let builder = crate::builder::QueryBuilder::table(relation.related_table.clone())
            .where_in("id", keys.clone());
        let sql = builder.to_sql()?;
        let bindings = builder.bindings().to_vec();
        let related = executor.fetch_json(&sql, &bindings).await?;
        for row in related {
            if let Some(key) = row_key(&row, "id") {
                by_id.entry(key).or_insert(row);
            }
        }
    }
    for row in rows.iter_mut() {
        let payload = row_key(row, &relation.foreign_key)
            .and_then(|key| by_id.get(&key).cloned())
            .unwrap_or(JsonValue::Null);
        insert_relation(row, &relation.name, payload);
    }
    Ok(())
}

/// Eager-load a `ManyToMany` through its pivot table in one join query.
async fn load_many_to_many<'a>(
    rows: &mut [JsonValue],
    executor: &mut Executor<'a>,
    relation: &Relation,
) -> Result<()> {
    let pivot = relation.pivot_table.as_deref().ok_or_else(|| {
        OrmError::InvalidState(format!(
            "ManyToMany relation `{}` is missing a pivot table",
            relation.name
        ))
    })?;
    let related_key = relation.related_key.as_deref().ok_or_else(|| {
        OrmError::InvalidState(format!(
            "ManyToMany relation `{}` is missing a related key",
            relation.name
        ))
    })?;
    let keys = parent_keys(rows, &relation.local_key);
    let mut grouped: HashMap<String, Vec<JsonValue>> = HashMap::new();
    if !keys.is_empty() {
        let pivot_alias = "__pivot_parent";
        let placeholders: Vec<String> = (1..=keys.len()).map(|i| format!("${i}")).collect();
        let sql = format!(
            "SELECT r.*, p.{foreign_key} AS {pivot_alias} FROM {related} r \
             JOIN {pivot} p ON p.{related_key} = r.id WHERE p.{foreign_key} IN ({in_list})",
            foreign_key = relation.foreign_key,
            related = relation.related_table,
            pivot = pivot,
            in_list = placeholders.join(", "),
        );
        let related = executor.fetch_json(&sql, &keys).await?;

        for mut row in related {
            let key = row_key(&row, pivot_alias);
            if let Some(object) = row.as_object_mut() {
                object.remove(pivot_alias);
            }
            if let Some(key) = key {
                grouped.entry(key).or_default().push(row);
            }
        }
    }
    attach_arrays(rows, &relation.local_key, &relation.name, grouped);
    Ok(())
}

/// Collect the distinct parent key bind values for `column` across `rows`.
fn parent_keys(rows: &[JsonValue], column: &str) -> Vec<Value> {
    let mut seen: HashMap<String, Value> = HashMap::new();
    for row in rows {
        if let Some(key) = row_key(row, column) {
            seen.entry(key.clone())
                .or_insert_with(|| key_to_value(&key));
        }
    }
    seen.into_values().collect()
}

/// Read a row's key column as a string, when present and non-null.
fn row_key(row: &JsonValue, column: &str) -> Option<String> {
    match row.get(column)? {
        JsonValue::String(text) => Some(text.clone()),
        JsonValue::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

/// Bind a string key as a UUID when it parses, else as text.
fn key_to_value(key: &str) -> Value {
    match Uuid::parse_str(key) {
        Ok(id) => Value::Uuid(id),
        Err(_) => Value::Text(key.to_string()),
    }
}

/// Attach grouped arrays onto each parent row under `relations[name]`.
fn attach_arrays(
    rows: &mut [JsonValue],
    local_key: &str,
    name: &str,
    grouped: HashMap<String, Vec<JsonValue>>,
) {
    for row in rows.iter_mut() {
        let payload = row_key(row, local_key)
            .and_then(|key| grouped.get(&key).cloned())
            .unwrap_or_default();
        let payload = JsonValue::Array(payload);
        insert_relation(row, name, payload);
    }
}

/// Insert a payload into the row's `relations` map (empty relation → `[]`).
fn insert_relation(row: &mut JsonValue, name: &str, payload: JsonValue) {
    let Some(object) = row.as_object_mut() else {
        return;
    };
    let entry = object
        .entry("relations".to_string())
        .or_insert_with(|| JsonValue::Object(Map::new()));
    if let JsonValue::Object(map) = entry {
        map.insert(name.to_string(), payload);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Verifies parent key collection dedupes and types UUIDs natively.
    #[test]
    fn collects_distinct_parent_keys() {
        let id = "f3c1f3c1-0000-4000-8000-000000000000";
        let rows = vec![
            json!({ "id": id }),
            json!({ "id": id }),
            json!({ "id": "not-a-uuid" }),
        ];
        let keys = parent_keys(&rows, "id");
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&Value::Uuid(Uuid::parse_str(id).unwrap())));
        assert!(keys.contains(&Value::Text("not-a-uuid".into())));
    }

    /// Verifies arrays attach under `relations[name]`, defaulting to `[]`.
    #[test]
    fn attaches_arrays_and_defaults_empty() {
        let mut rows = vec![json!({ "id": "p1" }), json!({ "id": "p2" })];
        let mut grouped: HashMap<String, Vec<JsonValue>> = HashMap::new();
        grouped.insert("p1".to_string(), vec![json!({ "id": "c1" })]);
        attach_arrays(&mut rows, "id", "comments", grouped);
        assert_eq!(
            rows[0]["relations"]["comments"].as_array().unwrap().len(),
            1
        );
        assert_eq!(
            rows[1]["relations"]["comments"].as_array().unwrap().len(),
            0
        );
    }
}
