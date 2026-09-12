//! Opt-in runtime view engine backed by minijinja.
//!
//! Enabled with the `runtime-templates` cargo feature. Templates are loaded
//! from disk on demand — for development hot-reload and user-authored
//! templates — while HTML auto-escaping remains ON for `.html` templates.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::ViewError;
use crate::response::ViewResponse;
use crate::{ViewEngine, VIEWS_DIR};

/// Runtime template engine rooted at a views directory.
///
/// The minijinja path loader confines template names to `root` and rejects
/// traversal outside it. Auto-escaping follows minijinja's default: `.html`,
/// `.htm`, and `.xml` templates are HTML-escaped.
pub struct MinijinjaEngine {
    env: minijinja::Environment<'static>,
}

impl MinijinjaEngine {
    /// Create an engine rooted at `root`.
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root: PathBuf = root.as_ref().to_path_buf();
        let mut env = minijinja::Environment::new();
        env.set_loader(minijinja::path_loader(root));
        Self { env }
    }

    /// Create an engine rooted at [`VIEWS_DIR`] (`resources/views`).
    pub fn from_default_root() -> Self {
        Self::new(VIEWS_DIR)
    }
}

impl ViewEngine for MinijinjaEngine {
    fn render_value(&self, name: &str, data: &Value) -> Result<ViewResponse, ViewError> {
        let template = self.env.get_template(name).map_err(|source| {
            if source.kind() == minijinja::ErrorKind::TemplateNotFound {
                ViewError::template_not_found(name)
            } else {
                ViewError::runtime(name, source)
            }
        })?;
        let html = template
            .render(data)
            .map_err(|source| ViewError::runtime(name, source))?;
        Ok(ViewResponse::html(html))
    }

    fn contains(&self, name: &str) -> bool {
        self.env.get_template(name).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempViews;

    /// A template present on disk renders from a JSON context.
    #[test]
    fn renders_a_runtime_template() {
        let views = TempViews::new();
        views.write("hello.html", "Hello {{ name }}!");
        let engine = MinijinjaEngine::new(views.path());

        let rendered = engine
            .render_value("hello.html", &serde_json::json!({ "name": "Ada" }))
            .expect("render runtime template");

        assert!(rendered.body().contains("Hello Ada!"));
        assert!(engine.contains("hello.html"));
    }

    /// HTML templates auto-escape interpolated data by default.
    #[test]
    fn html_templates_auto_escape() {
        let views = TempViews::new();
        views.write("hello.html", "Hello {{ name }}!");
        let engine = MinijinjaEngine::new(views.path());

        let rendered = engine
            .render_value("hello.html", &serde_json::json!({ "name": "<b>Ada</b>" }))
            .expect("render runtime template");

        assert!(rendered.body().contains("&lt;b&gt;Ada"));
        assert!(rendered.body().contains("Ada"));
        assert!(
            !rendered.body().contains('<'),
            "auto-escaping must be ON for .html templates"
        );
    }

    /// A missing template is a typed `TemplateNotFound` error.
    #[test]
    fn missing_template_is_a_typed_error() {
        let views = TempViews::new();
        let engine = MinijinjaEngine::new(views.path());

        let error = engine
            .render_value("nope.html", &serde_json::json!({}))
            .expect_err("missing template must fail");

        match error {
            ViewError::TemplateNotFound { name } => assert_eq!(name, "nope.html"),
            other => panic!("expected TemplateNotFound, got {other:?}"),
        }
    }

    /// Path traversal outside the views root is rejected by the loader.
    #[test]
    fn path_traversal_is_rejected() {
        let views = TempViews::new();
        let engine = MinijinjaEngine::new(views.path());

        let error = engine
            .render_value("../secret.html", &serde_json::json!({}))
            .expect_err("traversal must be rejected");

        assert!(matches!(error, ViewError::TemplateNotFound { .. }));
    }
}
