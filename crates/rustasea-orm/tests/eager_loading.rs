//! Eager-loading and relation serde round-trip tests (M2-E).
//!
//! Exercises `QueryBuilder::with(&[...])` against a real in-memory SQLite pool:
//! `HasMany`, `BelongsTo`, and `ManyToMany` each hydrate their `relations` map
//! with exactly one query per relation. Also covers the `Relations` depth-capped
//! serde round-trip (FS-M2-02, Laravel #13).

use rustasea_orm::{
    DbPool, Model, ModelOps, OrderDirection, OrmError, QueryBuilder, Relation, Relations, Value,
};
use serde_json::{json, Value as JsonValue};
use uuid::Uuid;

/// Derived model mapped to `users` carrying an eager `relations` map.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, rustasea_macros::Model)]
struct User {
    id: Uuid,
    name: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    relations: Relations,
}

/// Build a fresh pool with `users`, `posts`, `roles`, and the `role_user` pivot.
async fn pool_with_relations() -> DbPool {
    let pool = DbPool::connect("sqlite::memory:").await.unwrap();
    pool.execute_script(
        "CREATE TABLE users (
            id BLOB PRIMARY KEY,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT
        );
        CREATE TABLE posts (
            id BLOB PRIMARY KEY,
            user_id BLOB,
            title TEXT NOT NULL
        );
        CREATE TABLE roles (
            id BLOB PRIMARY KEY,
            name TEXT NOT NULL
        );
        CREATE TABLE role_user (
            user_id BLOB NOT NULL,
            role_id BLOB NOT NULL
        );",
    )
    .await
    .unwrap();
    pool
}

/// Create a user row and return its id.
async fn insert_user(pool: &DbPool, name: &str) -> Uuid {
    let now = chrono::Utc::now();
    let user = User {
        id: Uuid::now_v7(),
        name: name.to_string(),
        created_at: now,
        updated_at: now,
        deleted_at: None,
        relations: Relations::new(),
    };
    let id = user.id;
    User::create(pool, user).await.unwrap();
    id
}

/// Insert a post for `user_id` directly (no `Post` model needed).
async fn insert_post(pool: &DbPool, user_id: Uuid, title: &str) {
    pool.execute_bind(
        "INSERT INTO posts (id, user_id, title) VALUES ($1, $2, $3)",
        &[
            Value::Uuid(Uuid::now_v7()),
            Value::Uuid(user_id),
            Value::Text(title.to_string()),
        ],
    )
    .await
    .unwrap();
}

/// Insert a role and link it to `user_id` in the pivot table.
async fn insert_role(pool: &DbPool, user_id: Uuid, name: &str) {
    let role_id = Uuid::now_v7();
    pool.execute_bind(
        "INSERT INTO roles (id, name) VALUES ($1, $2)",
        &[Value::Uuid(role_id), Value::Text(name.to_string())],
    )
    .await
    .unwrap();
    pool.execute_bind(
        "INSERT INTO role_user (user_id, role_id) VALUES ($1, $2)",
        &[Value::Uuid(user_id), Value::Uuid(role_id)],
    )
    .await
    .unwrap();
}

/// Verifies `with("posts")` eager-loads HasMany and groups per parent.
#[tokio::test]
async fn with_has_many_fills_relations() {
    let pool = pool_with_relations().await;
    let ada = insert_user(&pool, "Ada").await;
    let alan = insert_user(&pool, "Alan").await;
    insert_post(&pool, ada, "First").await;
    insert_post(&pool, ada, "Second").await;
    insert_post(&pool, alan, "Only").await;

    let relation = Relation::has_many("posts", "posts", "User");
    let rows = QueryBuilder::table("users")
        .with_relations(vec![relation])
        .with(&["posts"])
        .order_by("name", OrderDirection::Asc)
        .get_eager(&pool)
        .await
        .unwrap();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["name"], "Ada");
    assert_eq!(rows[0]["relations"]["posts"].as_array().unwrap().len(), 2);
    assert_eq!(rows[1]["relations"]["posts"].as_array().unwrap().len(), 1);
}

/// Verifies an empty HasMany relation is `[]`, never `null`.
#[tokio::test]
async fn empty_has_many_is_empty_array() {
    let pool = pool_with_relations().await;
    insert_user(&pool, "Lonely").await;
    let relation = Relation::has_many("posts", "posts", "User");
    let rows = QueryBuilder::table("users")
        .with_relations(vec![relation])
        .with(&["posts"])
        .get_eager(&pool)
        .await
        .unwrap();
    assert_eq!(rows[0]["relations"]["posts"], json!([]));
}

/// Verifies HasMany still attaches `relations[name] = []` when no parent keys exist.
#[tokio::test]
async fn has_many_with_empty_keys_attaches_empty_array() {
    let pool = pool_with_relations().await;
    insert_user(&pool, "Ada").await;
    let relation = Relation::has_many("posts", "posts", "User");
    let rows = QueryBuilder::table("users")
        .select(&["name"])
        .with_relations(vec![relation])
        .with(&["posts"])
        .get_eager(&pool)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["relations"]["posts"], json!([]));
}

/// Verifies `with("author")` eager-loads a BelongsTo into one object.
#[tokio::test]
async fn with_belongs_to_fills_relations() {
    let pool = pool_with_relations().await;
    let ada = insert_user(&pool, "Ada").await;
    insert_post(&pool, ada, "First").await;

    let relation = Relation::belongs_to("author", "users");
    let rows = QueryBuilder::table("posts")
        .with_relations(vec![relation])
        .with(&["author"])
        .get_eager(&pool)
        .await
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["relations"]["author"]["name"], "Ada");
}

/// Verifies a null BelongsTo foreign key still attaches `relations[name] = null`.
#[tokio::test]
async fn belongs_to_with_null_foreign_key_attaches_null() {
    let pool = pool_with_relations().await;
    pool.execute_bind(
        "INSERT INTO posts (id, user_id, title) VALUES ($1, $2, $3)",
        &[
            Value::Uuid(Uuid::now_v7()),
            Value::Null,
            Value::Text("Orphan".to_string()),
        ],
    )
    .await
    .unwrap();

    let relation = Relation::belongs_to("author", "users");
    let rows = QueryBuilder::table("posts")
        .with_relations(vec![relation])
        .with(&["author"])
        .get_eager(&pool)
        .await
        .unwrap();

    assert_eq!(rows.len(), 1);
    let relations = rows[0].get("relations").expect("relations map present");
    assert!(relations.get("author").is_some(), "author key must be present");
    assert_eq!(rows[0]["relations"]["author"], JsonValue::Null);
}

/// Verifies `with("roles")` eager-loads a ManyToMany through the pivot.
#[tokio::test]
async fn with_many_to_many_fills_relations() {
    let pool = pool_with_relations().await;
    let ada = insert_user(&pool, "Ada").await;
    insert_user(&pool, "Alan").await;
    insert_role(&pool, ada, "admin").await;
    insert_role(&pool, ada, "editor").await;

    let relation = Relation::many_to_many("roles", "roles", "role_user", "User", "Role");
    let rows = QueryBuilder::table("users")
        .with_relations(vec![relation])
        .with(&["roles"])
        .get_eager(&pool)
        .await
        .unwrap();

    let ada_row = rows.iter().find(|row| row["name"] == "Ada").unwrap();
    let roles = ada_row["relations"]["roles"].as_array().unwrap();
    assert_eq!(roles.len(), 2);
    // The pivot helper column must not leak into the related payload.
    assert!(roles[0].get("__pivot_parent").is_none());
}

/// Verifies ManyToMany still attaches `relations[name] = []` when no parent keys exist.
#[tokio::test]
async fn many_to_many_with_empty_keys_attaches_empty_array() {
    let pool = pool_with_relations().await;
    insert_user(&pool, "Ada").await;
    let relation = Relation::many_to_many("roles", "roles", "role_user", "User", "Role");
    let rows = QueryBuilder::table("users")
        .select(&["name"])
        .with_relations(vec![relation])
        .with(&["roles"])
        .get_eager(&pool)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["relations"]["roles"], json!([]));
}

/// Verifies an undeclared relation name is a typed error, not a silent no-op.
#[tokio::test]
async fn undeclared_relation_is_typed_error() {
    let pool = pool_with_relations().await;
    insert_user(&pool, "Ada").await;
    let error = QueryBuilder::table("users")
        .with(&["nope"])
        .get_eager(&pool)
        .await
        .unwrap_err();
    assert!(matches!(error, OrmError::InvalidState(_)), "got {error:?}");
}

/// Verifies `Model::query()` auto-attaches declared relations for `with`.
#[tokio::test]
async fn model_query_attaches_declared_relations() {
    let pool = pool_with_relations().await;
    let ada = insert_user(&pool, "Ada").await;
    insert_post(&pool, ada, "First").await;

    // `User` declares no relations, so a declared name must be supplied; the
    // auto-attached set is empty and the request resolves against `with_relations`.
    let relation = Relation::has_many("posts", "posts", "User");
    let rows = <User as Model>::query()
        .with_relations(vec![relation])
        .with(&["posts"])
        .get_eager(&pool)
        .await
        .unwrap();
    assert_eq!(rows[0]["relations"]["posts"].as_array().unwrap().len(), 1);
}

/// Verifies relations survive a typed serde serialize → deserialize round-trip.
#[tokio::test]
async fn serde_round_trip_preserves_relations() {
    let pool = pool_with_relations().await;
    let ada = insert_user(&pool, "Ada").await;
    insert_post(&pool, ada, "First").await;
    insert_post(&pool, ada, "Second").await;

    let relation = Relation::has_many("posts", "posts", "User");
    let rows = QueryBuilder::table("users")
        .with_relations(vec![relation])
        .with(&["posts"])
        .get_eager(&pool)
        .await
        .unwrap();

    let user: User = serde_json::from_value(rows[0].clone()).unwrap();
    assert!(user.relations.relation_loaded("posts"));

    let encoded = serde_json::to_string(&user).unwrap();
    assert!(encoded.contains("\"relations\""), "{encoded}");
    let decoded: User = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.id, user.id);
    assert_eq!(decoded.relations, user.relations);
    assert_eq!(decoded.relations.get("posts").unwrap().as_array().unwrap().len(), 2);
}

/// Verifies the `Relations` depth cap prevents an infinite `User→Post→User`.
#[test]
fn relations_depth_cap_prevents_cycles() {
    let deep = json!({
        "relations": {
            "posts": [{
                "id": "p1",
                "relations": {
                    "author": {
                        "id": "u2",
                        "relations": {
                            "posts": [{
                                "id": "p2",
                                "relations": { "author": { "id": "u3" } }
                            }]
                        }
                    }
                }
            }]
        }
    });
    let mut relations = Relations::new();
    relations.insert("user", deep);
    let encoded = serde_json::to_string(&relations).unwrap();
    let decoded: Relations = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.depth(), rustasea_orm::relations::MAX_DEPTH);
    let stripped: JsonValue = serde_json::from_str(&encoded).unwrap();
    assert_eq!(
        stripped["user"]["relations"]["posts"][0]["relations"]["author"]["relations"],
        JsonValue::Null
    );
}
