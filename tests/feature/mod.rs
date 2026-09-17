//! Feature tests — HTTP-level behaviour of the generated routes.
//!
//! This module is the crate root of the `feature` test target; `Cargo.toml`
//! wires it with an explicit `[[test]]` entry because cargo does not
//! auto-discover `tests/feature/mod.rs`.

pub mod auth_test;
pub mod routes_test;

use rustasea::auth::FortifyConfig;
use rustasea::config::ConfigLoader;

/// Whether the generated `[fortify.features]` toggle `feature` is enabled.
///
/// Mirrors Laravel Fortify's `Features::enabled()` gate behind the starter
/// kit's `skipUnlessFortifyHas(...)` helper: the value is read from
/// `config/fortify.toml` through the real [`ConfigLoader`]. A missing config
/// file or `[fortify]` table falls back to the enabled default so a test fails
/// open rather than silently skipping.
///
/// Unknown feature names are reported as disabled (fail closed), matching the
/// kit's "only the named features gate a test" behaviour.
pub fn fortify_feature_enabled(feature: &str) -> bool {
    let Ok(loader) = ConfigLoader::load() else {
        return true;
    };
    let Ok(config) = FortifyConfig::from_loader(&loader) else {
        return true;
    };
    match feature {
        "registration" => config.features.registration,
        "reset_passwords" => config.features.reset_passwords,
        "email_verification" => config.features.email_verification,
        _ => false,
    }
}

/// Skip the current test when the named Fortify feature is disabled.
///
/// Mirrors the starter kit's `skipUnlessFortifyHas('registration')`: call it at
/// the top of a test to make that test conditional on `config/fortify.toml`.
#[macro_export]
macro_rules! skip_unless_fortify_has {
    ($feature:expr) => {
        if !$crate::fortify_feature_enabled($feature) {
            return;
        }
    };
}
