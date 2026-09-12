//! Unit tests for the Redis-backed cache store (inert and live paths).

use super::*;

/// Every operation on an inert store must fail, never fake success.
#[tokio::test]
async fn inert_store_never_reports_fake_success() {
    let store = RedisStore::new("test");
    assert!(!store.is_enabled());
    assert_eq!(store.name(), "test");
    assert!(matches!(
        store.get("k").await,
        Err(CacheError::StoreUnavailable(_))
    ));
    assert!(matches!(
        store.put("k", vec![1], Duration::from_secs(1)).await,
        Err(CacheError::StoreUnavailable(_))
    ));
    assert!(matches!(
        store
            .put_if_absent("k", vec![1], Duration::from_secs(1))
            .await,
        Err(CacheError::StoreUnavailable(_))
    ));
    assert!(matches!(
        store.compare_and_delete("k", b"x").await,
        Err(CacheError::StoreUnavailable(_))
    ));
    assert!(matches!(
        store.touch("k", Duration::from_secs(1)).await,
        Err(CacheError::StoreUnavailable(_))
    ));
    assert!(matches!(
        store.forget("k").await,
        Err(CacheError::StoreUnavailable(_))
    ));
    assert!(matches!(
        store.flush().await,
        Err(CacheError::StoreUnavailable(_))
    ));
    assert!(matches!(
        store.increment("k", 1).await,
        Err(CacheError::StoreUnavailable(_))
    ));
    assert!(matches!(
        store.decrement("k", 1).await,
        Err(CacheError::StoreUnavailable(_))
    ));
}

/// Redis URL for live tests (defaults to a local server).
#[cfg(feature = "redis")]
fn live_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
}

/// A unique prefix so live tests never collide with other data.
#[cfg(feature = "redis")]
fn test_prefix() -> String {
    format!("rustasea:test:{}:", uuid::Uuid::new_v4())
}

/// A live round-trip stores bytes and `touch` extends the TTL.
#[cfg(feature = "redis")]
#[tokio::test]
#[ignore = "requires a live Redis at REDIS_URL (default redis://127.0.0.1:6379)"]
async fn round_trip_and_touch_extends_ttl() {
    let store = RedisStore::from_url("test", live_url())
        .expect("valid redis url")
        .with_prefix(test_prefix());
    store
        .put("k", b"v".to_vec(), Duration::from_secs(30))
        .await
        .expect("put");
    assert_eq!(store.get("k").await.expect("get"), Some(b"v".to_vec()));
    assert!(store
        .touch("k", Duration::from_secs(120))
        .await
        .expect("touch"));
    assert_eq!(store.get("k").await.expect("get"), Some(b"v".to_vec()));
    assert!(store.forget("k").await.is_ok());
    assert_eq!(store.get("k").await.expect("get"), None);
    assert!(!store
        .touch("missing", Duration::from_secs(5))
        .await
        .unwrap());
    store.flush().await.expect("flush");
}

/// A held lock blocks a second acquirer and releases by owner token.
#[cfg(feature = "redis")]
#[tokio::test]
#[ignore = "requires a live Redis at REDIS_URL (default redis://127.0.0.1:6379)"]
async fn lock_is_atomic_over_redis() {
    use std::sync::Arc;
    let store: Arc<dyn Store> = Arc::new(
        RedisStore::from_url("test", live_url())
            .expect("valid redis url")
            .with_prefix(test_prefix()),
    );
    let lock = crate::Lock::new(store, "lock", Duration::from_secs(30));
    let mut guard = lock.get().await.expect("acquire").expect("first wins");
    assert!(
        lock.get().await.expect("contend").is_none(),
        "second acquire must fail while held"
    );
    guard.release().await.expect("release");
    assert!(
        lock.get().await.expect("reacquire").is_some(),
        "released lock must be re-acquirable"
    );
}

/// An unreachable server yields a typed error, never a fake success.
#[cfg(feature = "redis")]
#[tokio::test]
async fn unreachable_redis_returns_typed_error() {
    let store = RedisStore::from_url("test", "redis://127.0.0.1:1").expect("valid redis url");
    assert!(store.is_enabled());
    assert!(matches!(
        store.get("k").await,
        Err(CacheError::StoreUnavailable(_))
    ));
    assert!(matches!(
        store.put("k", vec![1], Duration::from_secs(1)).await,
        Err(CacheError::StoreUnavailable(_))
    ));
    assert!(matches!(
        store
            .put_if_absent("k", vec![1], Duration::from_secs(1))
            .await,
        Err(CacheError::StoreUnavailable(_))
    ));
    assert!(matches!(
        store.compare_and_delete("k", b"x").await,
        Err(CacheError::StoreUnavailable(_))
    ));
    assert!(matches!(
        store.touch("k", Duration::from_secs(1)).await,
        Err(CacheError::StoreUnavailable(_))
    ));
}
