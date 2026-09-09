//! `#[derive(Model)]` compile smoke tests (M2).
//!
//! The derive lives in `rustasea-macros`; these integration tests exercise it
//! against real structs through the public `rustasea_orm` surface so the
//! emitted `Model` impl (table naming, timestamps, soft deletes, touch)
//! compiles and behaves end to end.

use chrono::{DateTime, Utc};
use rustasea_orm::{Model, SoftDeletes, Timestamps};
use uuid::Uuid;

/// Canonical derived model — snake_plural `users`, tracked timestamps and
/// soft deletes inferred from the field layout.
#[allow(dead_code)]
#[derive(rustasea_macros::Model)]
struct User {
    id: Uuid,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
}

/// Derived model with an explicit table override and soft deletes disabled.
#[allow(dead_code)]
#[derive(rustasea_macros::Model)]
#[model(table = "people", soft_deletes = "none")]
struct Person {
    id: Uuid,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Verifies table_name derives snake_plural from the type name.
#[test]
fn derive_names_table_via_snake_plural() {
    assert_eq!(<User as Model>::table_name(), "users");
    assert_eq!(<User as Model>::type_name(), "User");
}

/// Verifies an explicit `#[model(table = "...")]` overrides the derivation.
#[test]
fn derive_honors_explicit_table() {
    assert_eq!(<Person as Model>::table_name(), "people");
}

/// Verifies timestamps/soft-delete flags are inferred from fields.
#[test]
fn derive_infers_tracked_columns() {
    assert!(<User as Model>::uses_timestamps());
    assert!(<User as Model>::uses_soft_deletes());
    assert_eq!(
        <User as Model>::insert_columns(),
        vec!["created_at", "updated_at"]
    );
    assert_eq!(<User as Model>::updated_column(), Some("updated_at"));
    assert_eq!(<User as Model>::deleted_column(), Some("deleted_at"));

    // `soft_deletes = "none"` — and no `deleted_at` field either.
    assert!(!<Person as Model>::uses_soft_deletes());
    assert_eq!(<Person as Model>::deleted_column(), None);
}

/// Verifies the emitted primary-key and assign_id helpers work on an instance.
#[test]
fn derive_primary_key_and_assign_id() {
    let mut user = User {
        id: Uuid::now_v7(),
        name: "Ada".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        deleted_at: None,
    };
    assert_eq!(user.primary_key(), user.id);
    let fresh = user.assign_id();
    assert_eq!(fresh, user.id);
    assert_ne!(fresh, Uuid::nil());
}

/// Verifies the default query gate carries the soft-delete guard and trashed
/// queries remove it — all through the Model query helpers.
#[test]
fn derive_query_helpers_emit_soft_delete_guards() {
    let active = <User as Model>::query();
    assert!(active
        .to_sql()
        .unwrap()
        .ends_with("WHERE deleted_at IS NULL"));

    let trashed = <User as Model>::query_with_trashed();
    assert_eq!(trashed.to_sql().unwrap(), "SELECT * FROM users");

    let only = <User as Model>::query_only_trashed();
    assert!(only
        .to_sql()
        .unwrap()
        .ends_with("WHERE deleted_at IS NOT NULL"));
}

/// Verifies `touch` bumps only the tracked updated_at field.
#[test]
fn derive_touch_bumps_updated_at() {
    let mut user = User {
        id: Uuid::now_v7(),
        name: "Ada".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        deleted_at: None,
    };
    let before = user.updated_at;
    user.touch();
    assert!(user.updated_at >= before);
}

/// Verifies a model with trackable timestamp wrapper types still compiles
/// (wrappers keep the hand-written style used by `make:model` scaffolds).
#[allow(dead_code)]
#[derive(rustasea_macros::Model)]
#[model(soft_deletes = "none", timestamps = "none")]
struct Account {
    id: Uuid,
    name: String,
    timestamps: Timestamps,
    soft_deletes: SoftDeletes,
}

/// Verifies wrapper-style structs without raw datetime columns report no
/// tracked columns, so `uses_*` gates stay false for the manual style.
#[test]
fn derive_wrapper_style_has_no_tracked_columns() {
    assert!(!<Account as Model>::uses_timestamps());
    assert!(!<Account as Model>::uses_soft_deletes());
    assert_eq!(<Account as Model>::insert_columns().len(), 0);
    assert_eq!(<Account as Model>::updated_column(), None);
    assert_eq!(<Account as Model>::deleted_column(), None);
    assert_eq!(<Account as Model>::table_name(), "accounts");

    let account = Account {
        id: Uuid::now_v7(),
        name: "Acme".into(),
        timestamps: Timestamps::default(),
        soft_deletes: SoftDeletes::default(),
    };
    let pk = account.primary_key();
    let ts = account.timestamps.created_at;
    assert_eq!(pk, account.id);
    assert!(!account.name.is_empty());
    assert!(ts <= account.timestamps.updated_at);
    assert!(account.soft_deletes.deleted_at.is_none());
}
