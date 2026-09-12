//! Default view engine backed by askama's compile-time templates.

use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::ViewError;
use crate::response::ViewResponse;
use crate::ViewEngine;

/// Boxed renderer that materializes a template from JSON and renders it.
type Renderer = Box<dyn Fn(&Value) -> Result<String, ViewError> + Send + Sync>;

/// Compile-time view engine built on askama.
///
/// askama templates are Rust types, not runtime paths, so the engine keeps a
/// `name -> renderer` registry populated at startup with
/// [`AskamaEngine::register`]. Once registered, rendering is name-based and
/// satisfies the [`ViewEngine`] contract while retaining askama's build-time
/// template checking.
///
/// ```rust,ignore
/// #[derive(askama::Template, serde::Deserialize)]
/// #[template(path = "auth/login.html")]
/// struct LoginView {
///     email: String,
/// }
///
/// let mut engine = AskamaEngine::new();
/// engine.register::<LoginView>("auth/login");
/// let page = engine.render("auth/login", &serde_json::json!({ "email": "a@b.c" }))?;
/// ```
#[derive(Default)]
pub struct AskamaEngine {
    renderers: HashMap<String, Renderer>,
}

impl AskamaEngine {
    /// Create an engine with no registered templates.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register the askama template type `T` under `name`.
    ///
    /// `T` must be deserializable from the JSON context so the engine can
    /// rebuild the template for each render. Returns `&mut Self` for chaining.
    pub fn register<T>(&mut self, name: impl Into<String>) -> &mut Self
    where
        T: askama::Template + DeserializeOwned + 'static,
    {
        let name = name.into();
        let error_name = name.clone();
        let renderer = move |value: &Value| {
            let template: T = serde_json::from_value(value.clone())
                .map_err(|source| ViewError::data(&error_name, source))?;
            template
                .render()
                .map_err(|source| ViewError::render(&error_name, source))
        };
        self.renderers.insert(name, Box::new(renderer));
        self
    }

    /// Number of registered templates.
    pub fn len(&self) -> usize {
        self.renderers.len()
    }

    /// Whether no templates are registered.
    pub fn is_empty(&self) -> bool {
        self.renderers.is_empty()
    }
}

impl ViewEngine for AskamaEngine {
    fn render_value(&self, name: &str, data: &Value) -> Result<ViewResponse, ViewError> {
        let renderer = self
            .renderers
            .get(name)
            .ok_or_else(|| ViewError::template_not_found(name))?;
        renderer(data).map(ViewResponse::html)
    }

    fn contains(&self, name: &str) -> bool {
        self.renderers.contains_key(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Hello;

    /// A registered askama template renders from a JSON context.
    #[test]
    fn renders_a_registered_template() {
        let mut engine = AskamaEngine::new();
        engine.register::<Hello>("hello");

        let rendered = engine
            .render_value("hello", &serde_json::json!({ "name": "Ada" }))
            .expect("render registered template");

        assert!(rendered.body().contains("Hello Ada!"));
        assert!(engine.contains("hello"));
    }

    /// The typed `render` convenience serializes its argument to JSON.
    #[test]
    fn typed_render_serializes_the_context() {
        let mut engine = AskamaEngine::new();
        engine.register::<Hello>("hello");

        let rendered = engine
            .render("hello", &serde_json::json!({ "name": "Grace" }))
            .expect("typed render");

        assert!(rendered.body().contains("Hello Grace!"));
    }

    /// Rendering an unregistered name is a typed `TemplateNotFound` error.
    #[test]
    fn unknown_template_is_a_typed_error() {
        let engine = AskamaEngine::new();

        let error = engine
            .render_value("missing", &serde_json::json!({}))
            .expect_err("missing template must fail");

        match error {
            ViewError::TemplateNotFound { name } => assert_eq!(name, "missing"),
            other => panic!("expected TemplateNotFound, got {other:?}"),
        }
    }
}
