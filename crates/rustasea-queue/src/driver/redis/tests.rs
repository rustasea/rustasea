//! Unit and live tests for the redis queue driver.

use super::*;
use chrono::TimeZone;

/// An inert driver surfaces typed store errors rather than fake success.
#[tokio::test]
async fn disabled_driver_reports_store_unavailable() {
    let driver = RedisDriver::disabled();
    assert!(!driver.is_enabled());
    let payload = JobPayload::new("podcasts", "redis", None, serde_json::json!({}));
    assert!(matches!(
        driver.push(payload).await,
        Err(QueueError::StoreUnavailable(_))
    ));
    assert!(matches!(
        driver.pending_size("podcasts").await,
        Err(QueueError::StoreUnavailable(_))
    ));
    assert!(matches!(
        driver.delayed_size("podcasts").await,
        Err(QueueError::StoreUnavailable(_))
    ));
    assert!(matches!(
        driver.reserved_size("podcasts").await,
        Err(QueueError::StoreUnavailable(_))
    ));
    assert!(matches!(
        driver.creation_time_of_oldest_pending_job("podcasts").await,
        Err(QueueError::StoreUnavailable(_))
    ));
}

/// The pending-time index key mirrors the queue key prefix scheme.
#[test]
fn pending_times_key_mirrors_queue_prefix() {
    let driver = RedisDriver::disabled().with_prefix("rustasea:test:");
    assert_eq!(
        driver.pending_times_key("podcasts"),
        "rustasea:test:podcasts:pending_times"
    );
}

/// Millisecond scores convert back exactly; non-finite scores yield `None`.
#[test]
fn millis_to_datetime_round_trips_and_rejects_non_finite() {
    let at = chrono::Utc
        .timestamp_millis_opt(1_700_000_000_123)
        .single()
        .expect("valid instant");
    assert_eq!(millis_to_datetime(1_700_000_000_123.0), Some(at));
    assert_eq!(millis_to_datetime(f64::NAN), None);
    assert_eq!(millis_to_datetime(f64::INFINITY), None);
    assert_eq!(millis_to_datetime(f64::NEG_INFINITY), None);
}

/// An unreachable server yields a typed error, never a panic.
#[tokio::test]
async fn unreachable_redis_returns_typed_error() {
    let driver = RedisDriver::from_url(Some("redis://127.0.0.1:1")).expect("valid url");
    assert!(driver.is_enabled());
    assert!(matches!(
        driver.pending_size("podcasts").await,
        Err(QueueError::StoreUnavailable(_))
    ));
}

/// Redis URL for live tests (defaults to a local server).
fn live_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
}

/// A unique prefix so live tests never collide with other data.
fn test_prefix() -> String {
    format!("rustasea:test:{}:", uuid::Uuid::new_v4())
}

/// A live queue reports real counts and the earliest pending instant.
#[tokio::test]
#[ignore = "requires a live Redis at REDIS_URL (default redis://127.0.0.1:6379)"]
async fn live_metrics_track_enqueue_and_oldest() {
    let driver = RedisDriver::from_url(Some(&live_url()))
        .expect("valid redis url")
        .with_prefix(test_prefix());

    // Empty queue → zeros and no oldest instant.
    assert_eq!(driver.pending_size("podcasts").await.unwrap(), 0);
    assert_eq!(driver.delayed_size("podcasts").await.unwrap(), 0);
    assert_eq!(driver.reserved_size("podcasts").await.unwrap(), 0);
    assert_eq!(
        driver
            .creation_time_of_oldest_pending_job("podcasts")
            .await
            .unwrap(),
        None
    );

    let first = chrono::Utc::now();
    driver
        .push(JobPayload::new(
            "podcasts",
            "redis",
            None,
            serde_json::json!({"n": 1}),
        ))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(5)).await;
    driver
        .push(JobPayload::new(
            "podcasts",
            "redis",
            None,
            serde_json::json!({"n": 2}),
        ))
        .await
        .unwrap();

    assert_eq!(driver.pending_size("podcasts").await.unwrap(), 2);
    let oldest = driver
        .creation_time_of_oldest_pending_job("podcasts")
        .await
        .unwrap()
        .expect("oldest pending");
    assert!(
        oldest >= first - chrono::Duration::seconds(1),
        "oldest {oldest} predates push"
    );
    assert!(
        oldest <= chrono::Utc::now(),
        "oldest {oldest} is in the future"
    );

    // A future-dated push counts as delayed, not pending.
    let mut delayed = JobPayload::new("podcasts", "redis", None, serde_json::json!({"n": 3}));
    delayed.available_at = Some(chrono::Utc::now() + chrono::Duration::hours(1));
    driver.push(delayed).await.unwrap();
    assert_eq!(driver.pending_size("podcasts").await.unwrap(), 2);
    assert_eq!(driver.delayed_size("podcasts").await.unwrap(), 1);

    // Pop reserves one and clears its pending-time score.
    let popped = driver
        .pop("podcasts", Duration::from_millis(50))
        .await
        .unwrap()
        .expect("job");
    assert_eq!(driver.pending_size("podcasts").await.unwrap(), 1);
    assert_eq!(driver.reserved_size("podcasts").await.unwrap(), 1);
    let after_pop = driver
        .creation_time_of_oldest_pending_job("podcasts")
        .await
        .unwrap()
        .expect("oldest pending");
    assert!(
        after_pop >= oldest,
        "remaining oldest must not predate the first"
    );

    // Ack clears the reservation.
    driver.ack(&popped).await.unwrap();
    assert_eq!(driver.reserved_size("podcasts").await.unwrap(), 0);

    // Drain the last pending job → empty again.
    let last = driver
        .pop("podcasts", Duration::from_millis(50))
        .await
        .unwrap()
        .expect("job");
    driver.ack(&last).await.unwrap();
    assert_eq!(driver.pending_size("podcasts").await.unwrap(), 0);
    assert_eq!(
        driver
            .creation_time_of_oldest_pending_job("podcasts")
            .await
            .unwrap(),
        None
    );
}

/// Release re-indexes the pending time; dead letter leaves no stale score.
#[tokio::test]
#[ignore = "requires a live Redis at REDIS_URL (default redis://127.0.0.1:6379)"]
async fn live_release_and_dead_letter_clear_pending_times() {
    let driver = RedisDriver::from_url(Some(&live_url()))
        .expect("valid redis url")
        .with_prefix(test_prefix());

    driver
        .push(JobPayload::new(
            "emails",
            "redis",
            None,
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    let popped = driver
        .pop("emails", Duration::from_millis(50))
        .await
        .unwrap()
        .expect("job");
    assert_eq!(driver.pending_size("emails").await.unwrap(), 0);

    // release(delay = 0) re-enqueues immediately and re-indexes the time.
    driver.release(&popped, Duration::ZERO).await.unwrap();
    assert_eq!(driver.pending_size("emails").await.unwrap(), 1);
    assert!(driver
        .creation_time_of_oldest_pending_job("emails")
        .await
        .unwrap()
        .is_some());

    // dead_letter + ack leaves the queue empty with no stale score.
    let again = driver
        .pop("emails", Duration::from_millis(50))
        .await
        .unwrap()
        .expect("job");
    let failed = FailedJob::new(
        "redis",
        "emails",
        serde_json::to_value(&again).unwrap(),
        "boom",
    );
    driver.dead_letter(failed).await.unwrap();
    driver.ack(&again).await.unwrap();
    assert_eq!(driver.pending_size("emails").await.unwrap(), 0);
    assert_eq!(driver.reserved_size("emails").await.unwrap(), 0);
    assert_eq!(
        driver
            .creation_time_of_oldest_pending_job("emails")
            .await
            .unwrap(),
        None
    );
}
