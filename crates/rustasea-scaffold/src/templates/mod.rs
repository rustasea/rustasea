//! Embedded starter-kit templates.
//!
//! Every generated file is a compile-time `&'static str` so scaffolding never
//! performs network or filesystem template lookups. [`render`] substitutes the
//! `@@…@@` placeholders derived from the requested application name; the
//! `@@` delimiters are chosen so generated askama `{{ }}` expressions survive
//! substitution untouched.

mod app_auth;
mod app_domain;
mod app_http;
mod blade;
mod config;
mod core;
mod database;
mod inertia;
mod inertia_react;
mod inertia_vue;
mod livewire;
mod manifest;
mod routes;
mod tests;

use crate::name::AppName;
use crate::variant::StarterKitVariant;

/// A template entry: application-relative path plus raw template contents.
pub type TemplateFile = (&'static str, &'static str);

/// Placeholder values substituted into every template.
pub struct Placeholders<'a> {
    /// Kebab-case application name (`my-app`).
    pub app_name: &'a str,
    /// Snake-case application name (`my_app`).
    pub app_snake: &'a str,
    /// PascalCase application name (`MyApp`).
    pub app_pascal: &'a str,
    /// Lowercase variant token (`blade`, `react`, `vue`, `livewire`).
    pub variant: &'a str,
}

impl<'a> Placeholders<'a> {
    /// Derive the placeholder set from a parsed name and variant.
    pub fn new(name: &'a AppName, variant: StarterKitVariant) -> Self {
        Self {
            app_name: &name.kebab,
            app_snake: &name.snake,
            app_pascal: &name.pascal,
            variant: variant.as_str(),
        }
    }
}

/// Substitute every placeholder in `template`.
pub fn render(template: &str, vars: &Placeholders<'_>) -> String {
    template
        .replace("@@app_name@@", vars.app_name)
        .replace("@@app_snake@@", vars.app_snake)
        .replace("@@app_pascal@@", vars.app_pascal)
        .replace("@@variant@@", vars.variant)
}

/// All template entries for `variant`, in deterministic write order.
///
/// Shared core templates come first, then the variant-specific `resources/`
/// layer. Duplicate paths are a programming error and are debug-asserted.
pub fn entries(variant: StarterKitVariant) -> Vec<TemplateFile> {
    let mut files: Vec<TemplateFile> = Vec::new();
    files.extend(manifest::entries(variant));
    files.extend(core::entries(variant));
    files.extend(app_domain::entries());
    files.extend(app_auth::entries());
    files.extend(app_http::entries(variant));
    files.extend(routes::entries());
    files.extend(config::entries(variant));
    files.extend(database::entries());
    files.extend(tests::entries());
    match variant {
        StarterKitVariant::Blade => files.extend(blade::entries()),
        StarterKitVariant::Livewire => files.extend(livewire::entries()),
        StarterKitVariant::React | StarterKitVariant::Vue => {
            files.extend(inertia::entries(variant))
        }
    }
    debug_assert_unique(&files);
    files
}

/// Debug-only guard against two templates targeting the same path.
fn debug_assert_unique(files: &[TemplateFile]) {
    #[cfg(debug_assertions)]
    {
        let mut seen = std::collections::HashSet::new();
        for (path, _) in files {
            debug_assert!(seen.insert(*path), "duplicate template path: {path}");
        }
    }
}
