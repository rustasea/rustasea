//! React variant WASM frontend — Dioxus + the shared Inertia contract.
//!
//! The entrypoint installs the generated [`ComponentRegistry`] and launches the
//! Dioxus app; each Inertia component key maps to a page module in
//! `resources/js/pages`.

use super::TemplateFile;

/// React (Dioxus) frontend templates.
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
rustasea-inertia-adapters = { version = "0.1", features = ["react"] }
dioxus = { version = "0.7", features = ["web"] }
wasm-bindgen = "0.2"
serde_json = "1"
"##;

const MAIN: &str = r##"//! Dioxus WASM entry point for the react variant.

use dioxus::prelude::*;
use rustasea_inertia_adapters::{install_registry, DioxusRouterProvider, RouterState};

mod pages;

/// Install the generated registry and launch the Dioxus app.
fn main() {
    install_registry(Box::new(pages::AppRegistry::new()));
    dioxus::launch(App);
}

/// Root component — owns the router state hydrated from `data-page`.
#[component]
fn App() -> Element {
    let state = use_signal(RouterState::new);
    rsx! {
        DioxusRouterProvider { state }
    }
}
"##;

const PAGES_MOD: &str = r##"//! Generated Inertia component registry (Dioxus).
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

use dioxus::prelude::*;
use rustasea_inertia_adapters::{ClientError, Value};

/// Mount the dashboard with the Inertia page props.
pub fn mount(_props: &Value) -> Result<(), ClientError> {
    dioxus::launch(Dashboard);
    Ok(())
}

/// Dashboard screen.
#[component]
fn Dashboard() -> Element {
    rsx! { h1 { "Dashboard" } }
}
"##;

const AUTH_LOGIN: &str = r##"//! `auth/login` Inertia component.

use dioxus::prelude::*;
use rustasea_inertia_adapters::{ClientError, Value};

/// Mount the login screen with the Inertia page props.
pub fn mount(_props: &Value) -> Result<(), ClientError> {
    dioxus::launch(Login);
    Ok(())
}

/// Login screen.
#[component]
fn Login() -> Element {
    rsx! { h1 { "Log in" } }
}
"##;

const AUTH_REGISTER: &str = r##"//! `auth/register` Inertia component.

use dioxus::prelude::*;
use rustasea_inertia_adapters::{ClientError, Value};

/// Mount the registration screen with the Inertia page props.
pub fn mount(_props: &Value) -> Result<(), ClientError> {
    dioxus::launch(Register);
    Ok(())
}

/// Registration screen.
#[component]
fn Register() -> Element {
    rsx! { h1 { "Register" } }
}
"##;

const SETTINGS_PROFILE: &str = r##"//! `settings/profile` Inertia component.

use dioxus::prelude::*;
use rustasea_inertia_adapters::{ClientError, Value};

/// Mount the profile settings screen with the Inertia page props.
pub fn mount(_props: &Value) -> Result<(), ClientError> {
    dioxus::launch(Profile);
    Ok(())
}

/// Profile settings screen.
#[component]
fn Profile() -> Element {
    rsx! { h1 { "Profile" } }
}
"##;

const SETTINGS_PASSWORD: &str = r##"//! `settings/password` Inertia component.

use dioxus::prelude::*;
use rustasea_inertia_adapters::{ClientError, Value};

/// Mount the password settings screen with the Inertia page props.
pub fn mount(_props: &Value) -> Result<(), ClientError> {
    dioxus::launch(Password);
    Ok(())
}

/// Password settings screen.
#[component]
fn Password() -> Element {
    rsx! { h1 { "Password" } }
}
"##;
