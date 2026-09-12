//! Inertia variant resources shared by react and vue.
//!
//! The HTML shell is rendered by the app's `RootView` (default `HtmlShell`);
//! Trunk compiles the WASM frontend in `resources/js` and bundles it with the
//! shell. The framework-specific entrypoint and page registry live in
//! [`super::inertia_react`] (Dioxus) and [`super::inertia_vue`] (Leptos).

use crate::variant::StarterKitVariant;

use super::TemplateFile;

/// Inertia templates for `variant` (react or vue).
pub fn entries(variant: StarterKitVariant) -> Vec<TemplateFile> {
    let mut files = vec![
        ("resources/views/app.html", APP_SHELL),
        ("Trunk.toml", TRUNK_TOML),
        ("index.html", INDEX),
    ];
    match variant {
        StarterKitVariant::React => files.extend(super::inertia_react::entries()),
        StarterKitVariant::Vue => files.extend(super::inertia_vue::entries()),
        StarterKitVariant::Blade | StarterKitVariant::Livewire => {}
    }
    files
}

const APP_SHELL: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>@@app_pascal@@</title>
</head>
<body>
  <!-- `__INERTIA_PAGE__` is replaced by the configured RootView with the
       escaped Page<T> JSON that hydrates the WASM client. -->
  <div id="app" data-page="__INERTIA_PAGE__"></div>
</body>
</html>
"##;

const TRUNK_TOML: &str = r##"[build]
target = "index.html"
dist = "public"

[watch]
watch = ["resources/js"]
"##;

const INDEX: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <link data-trunk rel="rust" href="resources/js/Cargo.toml" data-bin="@@app_name@@" />
</head>
<body>
  <div id="app"></div>
</body>
</html>
"##;
