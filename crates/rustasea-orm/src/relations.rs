//! Eager-loaded relation payloads with a depth-limited `serde` round-trip.
//!
//! [`Relations`] is the map attached to a fetched model under the `relations`
//! key. It serializes as a plain JSON object and survives `serde` round-trips;
//! nested relation maps are capped at [`MAX_DEPTH`] levels so a `User -> Post ->
//! User` cycle can never serialize forever (FS-M2-02, Laravel #13).

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::collections::HashMap;

/// Maximum nested `relations` depth preserved by serialization.
pub const MAX_DEPTH: usize = 3;

/// Eager-loaded relation payloads attached to a fetched model.
///
/// Serializes as `{ "<relation>": <payload> }`; deserialization restores the
/// map and re-applies the depth cap. Payloads are `serde_json::Value` so the
/// ORM stays model-agnostic.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Relations {
    /// Relation name to eager payload (`array` for HasMany/ManyToMany, object
    /// or `null` for BelongsTo).
    entries: HashMap<String, Value>,
}

impl Relations {
    /// Create an empty relations map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a relation payload, returning any previous value.
    pub fn insert(&mut self, name: impl Into<String>, payload: Value) -> Option<Value> {
        self.entries.insert(name.into(), payload)
    }

    /// Borrow a relation payload by name.
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.entries.get(name)
    }

    /// Whether a relation name is present (even if its payload is `null`).
    pub fn contains_key(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    /// Whether a relation was eager-loaded with a non-null payload.
    pub fn relation_loaded(&self, name: &str) -> bool {
        matches!(self.entries.get(name), Some(value) if !value.is_null())
    }

    /// Number of loaded relations.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no relations were loaded.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over the loaded relation payloads.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.entries.iter()
    }

    /// Consume the map into its backing [`HashMap`].
    pub fn into_map(self) -> HashMap<String, Value> {
        self.entries
    }

    /// The current maximum nested `relations` depth (0 when flat).
    pub fn depth(&self) -> usize {
        self.entries
            .values()
            .map(|value| measure(value, 1))
            .max()
            .unwrap_or(0)
    }

    /// Remove `relations` maps nested deeper than [`MAX_DEPTH`].
    pub fn truncate_to_max_depth(&mut self) {
        for value in self.entries.values_mut() {
            strip(value, 1, MAX_DEPTH);
        }
    }
}

impl Serialize for Relations {
    /// Emit the map as a JSON object, capped at [`MAX_DEPTH`] nested levels.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut capped = self.entries.clone();
        for value in capped.values_mut() {
            strip(value, 1, MAX_DEPTH);
        }
        capped.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Relations {
    /// Restore the map from a JSON object and re-apply the depth cap.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let map = HashMap::<String, Value>::deserialize(deserializer)?;
        let mut relations = Self { entries: map };
        if relations.depth() > MAX_DEPTH {
            return Err(D::Error::custom(format!(
                "relations depth {} exceeds maximum {MAX_DEPTH}",
                relations.depth()
            )));
        }
        relations.truncate_to_max_depth();
        Ok(relations)
    }
}

/// Recursively strip nested `relations` maps beyond `max` levels.
///
/// `depth` is the level of the current `relations` map (1-based).
fn strip(value: &mut Value, depth: usize, max: usize) {
    match value {
        Value::Object(map) => {
            let keys: Vec<String> = map.keys().cloned().collect();
            for key in keys {
                if key == "relations" {
                    if depth >= max {
                        map.remove("relations");
                        continue;
                    }
                    if let Some(nested) = map.get_mut("relations") {
                        strip(nested, depth + 1, max);
                    }
                } else if let Some(nested) = map.get_mut(&key) {
                    strip(nested, depth, max);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                strip(item, depth, max);
            }
        }
        _ => {}
    }
}

/// Measure the nested `relations` depth of a payload value.
fn measure(value: &Value, depth: usize) -> usize {
    match value {
        Value::Object(map) => {
            let mut max = depth;
            for (key, nested) in map {
                if key == "relations" {
                    max = max.max(measure(nested, depth + 1));
                } else {
                    max = max.max(measure(nested, depth));
                }
            }
            max
        }
        Value::Array(items) => items
            .iter()
            .map(|item| measure(item, depth))
            .max()
            .unwrap_or(depth),
        _ => depth,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Verifies a flat relations map round-trips through JSON unchanged.
    #[test]
    fn flat_relations_round_trip() {
        let mut relations = Relations::new();
        relations.insert("posts", json!([{"id": "p1"}, {"id": "p2"}]));
        relations.insert("profile", Value::Null);
        assert_eq!(relations.len(), 2);
        assert!(relations.relation_loaded("posts"));
        assert!(!relations.relation_loaded("profile"));
        assert_eq!(relations.depth(), 1);

        let encoded = serde_json::to_string(&relations).unwrap();
        let decoded: Relations = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, relations);
        assert_eq!(decoded.get("posts").unwrap().as_array().unwrap().len(), 2);
    }

    /// Verifies nested relations are capped at MAX_DEPTH during serialization.
    #[test]
    fn nested_relations_capped_at_max_depth() {
        let deep = json!({
            "relations": {
                "user": {
                    "id": "u1",
                    "relations": {
                        "posts": [{
                            "id": "p1",
                            "relations": {
                                "user": {
                                    "id": "u2",
                                    "relations": { "posts": [{"id": "p2"}] }
                                }
                            }
                        }]
                    }
                }
            }
        });
        let mut relations = Relations::new();
        relations.insert("profile", deep);
        assert!(relations.depth() > MAX_DEPTH);

        let encoded = serde_json::to_string(&relations).unwrap();
        let decoded: Relations = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.depth(), MAX_DEPTH);
        assert_eq!(
            decoded,
            serde_json::from_str::<Relations>(&encoded).unwrap()
        );
    }

    /// Verifies deserialization rejects payloads deeper than MAX_DEPTH.
    #[test]
    fn over_deep_payload_is_rejected() {
        let encoded = format!(
            "{{\"a\":{}}}",
            "{\"relations\":".repeat(MAX_DEPTH + 1) + "{}" + &"}".repeat(MAX_DEPTH + 1)
        );
        let error = serde_json::from_str::<Relations>(&encoded).unwrap_err();
        assert!(error.to_string().contains("exceeds maximum"), "{error}");
    }

    /// Verifies the empty map reports zero depth and no loaded relations.
    #[test]
    fn empty_map_defaults() {
        let relations = Relations::default();
        assert!(relations.is_empty());
        assert_eq!(relations.depth(), 0);
        assert!(!relations.relation_loaded("posts"));
    }
}
