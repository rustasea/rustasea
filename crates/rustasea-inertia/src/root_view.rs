//! HTML root document rendering for non-Inertia (first-load) requests.

use serde_json::Value;

use crate::{InertiaError, Page};

/// Renders the HTML root document that boots the WASM client.
///
/// The default implementation is [`HtmlShell`]. A custom implementation can
/// delegate to a template engine (for example `rustasea-view`) while keeping
/// the `data-page` embedding contract.
pub trait RootView: Send + Sync {
    /// Render the root document with `page` embedded for hydration.
    fn render(&self, page: &Page<Value>) -> Result<String, InertiaError>;
}

/// Default root document: a minimal HTML shell embedding `data-page`.
///
/// ```rust
/// use rustasea_inertia::HtmlShell;
///
/// let shell = HtmlShell::new("My App").app_id("root");
/// assert_eq!(shell.title(), "My App");
/// ```
#[derive(Debug, Clone)]
pub struct HtmlShell {
    title: String,
    app_id: String,
    head: Vec<String>,
    body: Vec<String>,
}

impl HtmlShell {
    /// Create a shell with the given document title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            app_id: "app".to_string(),
            head: Vec::new(),
            body: Vec::new(),
        }
    }

    /// Set the id of the mount element (default `"app"`).
    pub fn app_id(mut self, app_id: impl Into<String>) -> Self {
        self.app_id = app_id.into();
        self
    }

    /// Append a raw HTML fragment to the document `<head>`.
    pub fn head(mut self, fragment: impl Into<String>) -> Self {
        self.head.push(fragment.into());
        self
    }

    /// Append a raw HTML fragment before the mount element.
    pub fn body(mut self, fragment: impl Into<String>) -> Self {
        self.body.push(fragment.into());
        self
    }

    /// Document title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Mount element id.
    pub fn app_id_value(&self) -> &str {
        &self.app_id
    }
}

impl Default for HtmlShell {
    /// Create a shell titled `"RustaSea"`.
    fn default() -> Self {
        Self::new("RustaSea")
    }
}

impl RootView for HtmlShell {
    fn render(&self, page: &Page<Value>) -> Result<String, InertiaError> {
        let json =
            serde_json::to_string(page).map_err(|source| InertiaError::SerializePage { source })?;
        let escaped = escape_html_attribute(&json);
        let title = escape_html_text(&self.title);
        let head = self.head.join("\n    ");
        let body = self.body.join("\n    ");

        Ok(format!(
            "<!DOCTYPE html>\n\
             <html lang=\"en\">\n  \
             <head>\n    \
             <meta charset=\"utf-8\" />\n    \
             <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n    \
             <title>{title}</title>\n    \
             {head}\n  \
             </head>\n  \
             <body>\n    \
             {body}\n    \
             <div id=\"{app_id}\" data-page=\"{escaped}\"></div>\n  \
             </body>\n\
             </html>\n",
            app_id = self.app_id,
        ))
    }
}

/// Escape a string for safe interpolation into a double-quoted HTML attribute.
///
/// Escapes `&`, `<`, `>`, `"`, and `'`. The JSON page object always contains
/// double quotes, so this is required before embedding it in `data-page`.
pub fn escape_html_attribute(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#x27;"),
            other => output.push(other),
        }
    }
    output
}

/// Escape a string for interpolation into HTML text content.
fn escape_html_text(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            other => output.push(other),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The shell embeds the escaped page JSON in `data-page`.
    #[test]
    fn embeds_escaped_page_json() {
        let page = Page::new("dashboard", json!({ "user": "Ada" }), "/dashboard", "v1");
        let html = HtmlShell::new("App")
            .app_id("root")
            .render(&page)
            .expect("render shell");

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("id=\"root\""));
        assert!(html.contains("data-page=\""));
        assert!(html.contains("&quot;component&quot;"));
        // Raw JSON quotes must never appear inside the attribute.
        assert!(!html.contains("data-page=\"{\""));
    }

    /// The attribute escaper neutralizes every HTML metacharacter.
    #[test]
    fn escapes_attribute_metacharacters() {
        let escaped = escape_html_attribute("<a href=\"x\">&'");
        assert_eq!(escaped, "&lt;a href=&quot;x&quot;&gt;&amp;&#x27;");
    }

    /// A custom title is text-escaped in the document head.
    #[test]
    fn escapes_title_text() {
        let page = Page::new("dashboard", json!({}), "/", "v1");
        let html = HtmlShell::new("<script>alert(1)</script>")
            .render(&page)
            .expect("render shell");

        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(!html.contains("<script>alert(1)</script>"));
    }
}
