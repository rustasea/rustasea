//! `#[middleware]` and `#[authorize]` attribute macro — expansion checks
//! (FS-M3-06).
//!
//! Both macros re-emit the annotated handler unchanged and emit doc-hidden
//! metadata consts: `__RUSTASEA_MIDDLEWARE_<Fn>` carries the middleware name
//! list and `__RUSTASEA_AUTHORIZE_<Fn>` carries the `(ability, target)` pair.
//! The runtime resolves the enforcing guard from the middleware spec and
//! rejects with `GuardMismatch` before the body when the guard is unknown
//! (TC-M3-02).

use rustasea::macros::{authorize, middleware};

#[middleware("auth:jwt")]
async fn profile() -> &'static str {
    "profile"
}

#[middleware("auth:jwt", "throttle:60,1")]
async fn login() -> &'static str {
    "logged in"
}

#[authorize("update", User)]
async fn update_user() -> &'static str {
    "updated"
}

#[authorize("view")]
async fn view_user() -> &'static str {
    "viewed"
}

/// A minimal type the target path in `#[authorize]` refers to.
pub struct User;

/// The handler bodies survive the macros untouched.
#[test]
fn handler_macros_preserve_functions() {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    assert_eq!(rt.block_on(profile()), "profile");
    assert_eq!(rt.block_on(login()), "logged in");
    assert_eq!(rt.block_on(update_user()), "updated");
    assert_eq!(rt.block_on(view_user()), "viewed");
}

/// Middleware metadata consts exist with the parsed name list.
#[test]
fn middleware_macro_emits_name_list() {
    assert_eq!(__RUSTASEA_MIDDLEWARE_profile, &["auth:jwt"][..]);
    assert_eq!(
        __RUSTASEA_MIDDLEWARE_login,
        &["auth:jwt", "throttle:60,1"][..]
    );
}

/// Authorize metadata consts exist with `(ability, target)`; a missing
/// target is recorded as an empty string.
#[test]
fn authorize_macro_emits_ability_and_target() {
    assert_eq!(__RUSTASEA_AUTHORIZE_update_user, ("update", "User"));
    assert_eq!(__RUSTASEA_AUTHORIZE_view_user, ("view", ""));
}
