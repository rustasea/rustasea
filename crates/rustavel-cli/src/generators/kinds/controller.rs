//! `make:controller` template — app/http/controllers/<snake>.rs.
//!
//! Plain scaffolding yields a `handle` skeleton; `--resource` adds the seven
//! REST methods (index/create/store/show/edit/update/destroy).

use std::path::Path;

use crate::error::CliResult;
use crate::generator::Generated;
use crate::generators::kinds::{slug, write_scaffold};
use crate::generators::MakeOptions;

/// Render and write the controller file.
pub fn scaffold(root: &Path, opts: &MakeOptions) -> CliResult<Generated> {
    let rel = format!("app/http/controllers/{}.rs", slug(&opts.name));
    let source = if opts.resource {
        resource_source(&opts.name)
    } else {
        plain_source(&opts.name)
    };
    write_scaffold(root, rel, source, opts.force)
}

/// Plain controller with a single `handle` method.
fn plain_source(name: &str) -> String {
    format!(
        r#"//! HTTP controller scaffold — {name}.
//!
//! Register routes against these handlers in `routes/web.rs`; add `--resource`
//! on `make:controller` to scaffold the full REST method set.

use rustavel::http::{{IntoResponse, Json}};
use rustavel::http::Request;

/// Handles {kind} HTTP requests.
pub struct {name};

impl {name} {{
    /// Respond to the primary route for this controller.
    pub async fn handle(request: Request) -> Json<serde_json::Value> {{
        let _ = request;
        Json(serde_json::json!({{ "controller": "{kind}" }}))
    }}
}}

/// Fallback response used when a handler is still a stub.
pub fn ok() -> impl IntoResponse {{
    Json(serde_json::json!({{ "status": "ok" }}))
}}
"#,
        kind = slug(name),
        name = name,
    )
}

/// Resource controller with the seven REST methods.
fn resource_source(name: &str) -> String {
    format!(
        r#"//! Resource controller scaffold — {name}.
//!
//! Full REST method set produced by `make:controller {name} --resource`.

use rustavel::http::{{IntoResponse, Json}};
use rustavel::http::Request;

/// Handles {kind} resource HTTP requests.
pub struct {name};

impl {name} {{
    /// GET /{kind} — list rows.
    pub async fn index(_request: Request) -> impl IntoResponse {{
        Json(serde_json::json!({{ "rows": [] }}))
    }}

    /// GET /{kind}/new — render the create form.
    pub async fn create(_request: Request) -> impl IntoResponse {{
        Json(serde_json::json!({{ "form": "create" }}))
    }}

    /// POST /{kind} — persist a new row.
    pub async fn store(_request: Request) -> impl IntoResponse {{
        Json(serde_json::json!({{ "stored": true }}))
    }}

    /// GET /{kind}/{{id}} — show one row.
    pub async fn show(_request: Request) -> impl IntoResponse {{
        Json(serde_json::json!({{ "row": null }}))
    }}

    /// GET /{kind}/{{id}}/edit — render the edit form.
    pub async fn edit(_request: Request) -> impl IntoResponse {{
        Json(serde_json::json!({{ "form": "edit" }}))
    }}

    /// PUT/PATCH /{kind}/{{id}} — persist changes.
    pub async fn update(_request: Request) -> impl IntoResponse {{
        Json(serde_json::json!({{ "updated": true }}))
    }}

    /// DELETE /{kind}/{{id}} — remove a row (soft delete when applicable).
    pub async fn destroy(_request: Request) -> impl IntoResponse {{
        Json(serde_json::json!({{ "deleted": true }}))
    }}
}}
"#,
        kind = slug(name),
        name = name,
    )
}
