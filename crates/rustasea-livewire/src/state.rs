//! Per-component JSON state and the shallow patches that mutate it.
//!
//! A livewire component owns a JSON object as its state. The browser holds a
//! copy and posts it back with every action (the Livewire request model), so
//! state arriving at the server is **untrusted input**: authorization runs
//! before it is read, and actions must validate the values they consume.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::StateError;

/// A shallow JSON merge patch applied to a component's state object.
///
/// Only top-level keys are merged; nested objects are replaced wholesale. This
/// keeps patch semantics predictable and avoids a deep-merge implementation
/// whose recursion/`null` handling is a common source of bugs.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StatePatch {
    /// Field values merged into the state object (top-level keys only).
    fields: Map<String, Value>,
}

impl StatePatch {
    /// Create an empty patch.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a field assignment to the patch, returning `self` for chaining.
    pub fn set(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    /// Whether the patch contains no fields.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Borrow the patch's field assignments.
    pub fn fields(&self) -> &Map<String, Value> {
        &self.fields
    }

    /// Build a patch from a JSON object value.
    pub fn from_value(value: Value) -> Result<Self, StateError> {
        match value {
            Value::Object(fields) => Ok(Self { fields }),
            _ => Err(StateError::PatchNotAnObject),
        }
    }
}

/// Mutable JSON state owned by one component instance.
///
/// Constructed only from a JSON object, so the object invariant holds for the
/// lifetime of the value and the accessors below cannot fail.
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentState {
    /// State object; guaranteed to be `Value::Object`.
    value: Value,
}

impl ComponentState {
    /// Wrap a JSON object as component state.
    pub fn new(value: Value) -> Result<Self, StateError> {
        if value.is_object() {
            Ok(Self { value })
        } else {
            Err(StateError::NotAnObject)
        }
    }

    /// Borrow the underlying JSON value.
    pub fn as_value(&self) -> &Value {
        &self.value
    }

    /// Borrow the state object.
    pub fn as_object(&self) -> &Map<String, Value> {
        self.value
            .as_object()
            .expect("component state invariant: JSON object")
    }

    /// Mutably borrow the state object.
    pub fn as_object_mut(&mut self) -> &mut Map<String, Value> {
        self.value
            .as_object_mut()
            .expect("component state invariant: JSON object")
    }

    /// Read a top-level field.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.as_object().get(key)
    }

    /// Set a top-level field.
    pub fn set(&mut self, key: impl Into<String>, value: Value) {
        self.as_object_mut().insert(key.into(), value);
    }

    /// Apply a shallow merge patch, replacing top-level fields.
    pub fn apply(&mut self, patch: &StatePatch) -> Result<(), StateError> {
        let object = self.value.as_object_mut().ok_or(StateError::NotAnObject)?;
        for (key, value) in patch.fields() {
            object.insert(key.clone(), value.clone());
        }
        Ok(())
    }

    /// Consume the state and return its JSON value.
    pub fn into_value(self) -> Value {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A shallow patch replaces only the fields it names.
    #[test]
    fn patch_replaces_named_fields_and_keeps_others() {
        let mut state = ComponentState::new(json!({ "count": 1, "label": "a" })).unwrap();
        let patch = StatePatch::new().set("count", json!(2));

        state.apply(&patch).unwrap();

        assert_eq!(state.get("count"), Some(&json!(2)));
        assert_eq!(state.get("label"), Some(&json!("a")));
    }

    /// Non-object state is rejected rather than coerced.
    #[test]
    fn non_object_state_is_rejected() {
        let error = ComponentState::new(json!([1, 2, 3])).unwrap_err();
        assert!(matches!(error, StateError::NotAnObject));
    }

    /// A non-object patch is rejected at construction.
    #[test]
    fn non_object_patch_is_rejected() {
        let error = StatePatch::from_value(json!("nope")).unwrap_err();
        assert!(matches!(error, StateError::PatchNotAnObject));
    }
}
