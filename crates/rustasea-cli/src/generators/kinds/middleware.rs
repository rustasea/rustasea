//! `make:middleware` template — app/http/middleware/<snake>.rs.
//!
//! Emits an axum 0.7 function middleware built on `axum::middleware::from_fn`,
//! mirroring the repo's Inertia middleware (`axum::extract::Request` +
//! `axum::middleware::Next`). The starter kits depend on `axum` directly but not
//! on `tower`, so the skeleton deliberately stays on axum's re-exported surface
//! to compile unchanged inside a scaffolded app.

use std::path::Path;

use crate::error::CliResult;
use crate::generator::Generated;
use crate::generators::kinds::{slug, write_scaffold};
use crate::generators::MakeOptions;

/// Render and write the middleware file.
pub fn scaffold(root: &Path, opts: &MakeOptions) -> CliResult<Generated> {
    let rel = format!("app/http/middleware/{}.rs", slug(&opts.name));
    write_scaffold(root, rel, source(&opts.name), opts.force)
}

/// Pre-formatted middleware source for `name`.
fn source(name: &str) -> String {
    format!(
        r#"//! HTTP middleware scaffold — {name}.
//!
//! Apply with `axum::middleware::from_fn({name}::handle)` on a router or route
//! group. The function runs before the wrapped handler and may short-circuit by
//! returning a response without calling `next.run(request)`.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

/// {name} middleware.
pub struct {name};

impl {name} {{
    /// Inspect the request, then delegate to the wrapped handler.
    pub async fn handle(request: Request, next: Next) -> Response {{
        // Read request headers/extensions here; return a response early to
        // short-circuit the handler.
        next.run(request).await
    }}
}}
"#,
        name = name,
    )
}
