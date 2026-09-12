//! Vue variant WASM frontend — Leptos + the shared Inertia contract.
//!
//! The entrypoint installs the generated [`ComponentRegistry`] and mounts the
//! Leptos app; each Inertia component key maps to a page module in
//! `resources/js/pages`.

use super::TemplateFile;

/// Vue (Leptos) frontend templates.
pub fn entries() -> Vec<TemplateFile> {
    vec![
        ("resources/js/Cargo.toml", CARGO),
        ("resources/js/main.rs", MAIN),
        ("resources/js/pages/mod.rs", PAGES_MOD),
        ("resources/js/pages/dashboard.rs", DASHBOARD),
        ("resources/js/pages/auth_login.rs", AUTH_LOGIN),
        ("resources/js/pages/auth_register.rs", AUTH_REGISTER),
        ("resources/js/pages/settings_profile.rs", SETTINGS_PROFILE),
        ("resources/js/pages/settings_password.rs", SETTINGS_PASSWORD),
    ]
}

const CARGO: &str = r##"[package]
name = "@@app_name@@-ui"
version = "0.1.0"
edition = "2021"
rust-version = "1.88"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
rustasea-inertia-adapters = { version = "0.1", features = ["vue"] }
leptos = { version = "0.8", features = ["csr"] }
wasm-bindgen = "0.2"
serde_json = "1"
"##;

const MAIN: &str = r##"//! Leptos WASM entry point for the vue variant.

use leptos::prelude::*;
use rustasea_inertia_adapters::{install_registry, LeptosRouterProvider, RouterState};

mod pages;

/// Install the generated registry and mount the Leptos app.
fn main() {
    install_registry(Box::new(pages::AppRegistry::new()));
    mount_to_body(App);
}

/// Root component — owns the router state hydrated from `data-page`.
#[component]
fn App() -> impl IntoView {
    let state = RouterState::new();
    view! {
        <LeptosRouterProvider state=state/>
    }
}
"##;

const PAGES_MOD: &str = r##"//! Generated Inertia component registry (Leptos).
//!
//! WASM has no reflection, so the scaffolder emits this `match` explicitly
//! (ADR-0002 decision 4).

use rustasea_inertia_adapters::inertia_registry;

pub mod auth_login;
pub mod auth_register;
pub mod dashboard;
pub mod settings_password;
pub mod settings_profile;

inertia_registry!(AppRegistry {
    "auth/login" => auth_login::mount,
    "auth/register" => auth_register::mount,
    "dashboard" => dashboard::mount,
    "settings/password" => settings_password::mount,
    "settings/profile" => settings_profile::mount,
});
"##;

const DASHBOARD: &str = r##"//! `dashboard` Inertia component.

use leptos::prelude::*;
use rustasea_inertia_adapters::{ClientError, Value};

/// Mount the dashboard with the Inertia page props.
pub fn mount(_props: &Value) -> Result<(), ClientError> {
    mount_to_body(Dashboard);
    Ok(())
}

/// Dashboard screen.
#[component]
fn Dashboard() -> impl IntoView {
    view! { <h1>"Dashboard"</h1> }
}
"##;

const AUTH_LOGIN: &str = r##"//! `auth/login` Inertia component.

use leptos::prelude::*;
use rustasea_inertia_adapters::{ClientError, Value};

/// Mount the login screen with the Inertia page props.
pub fn mount(_props: &Value) -> Result<(), ClientError> {
    mount_to_body(Login);
    Ok(())
}

/// Login screen.
#[component]
fn Login() -> impl IntoView {
    view! { <h1>"Log in"</h1> }
}
"##;

const AUTH_REGISTER: &str = r##"//! `auth/register` Inertia component.

use leptos::prelude::*;
use rustasea_inertia_adapters::{ClientError, Value};

/// Mount the registration screen with the Inertia page props.
pub fn mount(_props: &Value) -> Result<(), ClientError> {
    mount_to_body(Register);
    Ok(())
}

/// Registration screen.
#[component]
fn Register() -> impl IntoView {
    view! { <h1>"Register"</h1> }
}
"##;

const SETTINGS_PROFILE: &str = r##"//! `settings/profile` Inertia component.

use leptos::prelude::*;
use rustasea_inertia_adapters::{ClientError, Value};

/// Mount the profile settings screen with the Inertia page props.
pub fn mount(_props: &Value) -> Result<(), ClientError> {
    mount_to_body(Profile);
    Ok(())
}

/// Profile settings screen.
#[component]
fn Profile() -> impl IntoView {
    view! { <h1>"Profile"</h1> }
}
"##;

const SETTINGS_PASSWORD: &str = r##"//! `settings/password` Inertia component.

use leptos::prelude::*;
use rustasea_inertia_adapters::{ClientError, Value};

/// Mount the password settings screen with the Inertia page props.
pub fn mount(_props: &Value) -> Result<(), ClientError> {
    mount_to_body(Password);
    Ok(())
}

/// Password settings screen.
#[component]
fn Password() -> impl IntoView {
    view! { <h1>"Password"</h1> }
}
"##;
