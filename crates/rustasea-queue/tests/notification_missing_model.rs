//! FS-M6-04 / FR-605 — queued notifications with `#[deleteWhenMissingModels]`.
//!
//! Mirrors TC-M6-11/12: a queued `WelcomeNotification` whose referenced user
//! was deleted before the worker ran must be skipped — not retried — with a
//! `NotificationSkipped { reason: MissingModel }` diagnostic; a live user
//! delivers normally.

use rustasea_queue::{
    skipped_notifications, Job, JobError, JobOutcome, NotificationSkipped, Queue,
};
use serde::Serialize;

/// A sentinel the guard uses to learn which users "exist" right now.
static LIVE_USERS: std::sync::OnceLock<std::sync::Mutex<Vec<String>>> = std::sync::OnceLock::new();

fn live_users() -> &'static std::sync::Mutex<Vec<String>> {
    LIVE_USERS.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

/// Serialized sentinel shared with the test (skipped via `#[serde(skip)]`).
#[derive(Default, Clone)]
struct SentFlag(std::sync::Arc<std::sync::atomic::AtomicBool>);

impl SentFlag {
    /// Create a fresh false flag.
    fn new() -> Self {
        Self::default()
    }

    /// Whether the mail body was dispatched.
    fn was_sent(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Record a dispatch.
    fn mark_sent(&self) {
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Queued welcome notification targeting one user (mail #17 parity).
#[derive(Serialize)]
struct WelcomeNotification {
    /// Serialized user reference; deleted → suppress send.
    user_id: String,
    /// Dispatch sentinel, excluded from the queue payload.
    #[serde(skip)]
    sent: SentFlag,
}

impl WelcomeNotification {
    /// Declares the `#[deleteWhenMissingModels]` marker.
    fn missing_model_suppressed(&self) -> bool {
        let guard = live_users().lock().unwrap_or_else(|p| p.into_inner());
        !guard.iter().any(|id| id == &self.user_id)
    }
}

#[async_trait::async_trait]
impl Job for WelcomeNotification {
    /// Send the notification (only reached when the user exists).
    async fn handle(self) -> Result<(), JobError> {
        self.sent.mark_sent();
        Ok(())
    }

    /// `#[deleteWhenMissingModels]` marker on.
    fn delete_when_missing_models(&self) -> bool {
        true
    }

    /// Suppression probe: user deleted before the worker ran?
    fn is_missing_model_suppressed(&self) -> bool {
        self.missing_model_suppressed()
    }

    /// `NotificationSkipped { reason: MissingModel }` diagnostic.
    fn skipped_notification(&self) -> Option<NotificationSkipped> {
        if self.delete_when_missing_models() {
            Some(NotificationSkipped::missing_model(
                "welcome",
                format!("user:{}", self.user_id),
            ))
        } else {
            None
        }
    }
}

#[tokio::test]
async fn tc_m6_11_skipped_when_missing_no_retry() {
    let sent = SentFlag::new();
    let notification = WelcomeNotification {
        user_id: "9".to_string(),
        sent: sent.clone(),
    };
    Queue::route_sync::<WelcomeNotification>("notifications").unwrap();

    // User 9 was deleted before the queue processes the job.
    let outcome = Queue::dispatch_sync(notification).await.unwrap();

    assert_eq!(outcome, JobOutcome::Skipped, "skipped, not retried");
    assert!(!sent.was_sent(), "skipped notification is never dispatched");
    let diagnostics = skipped_notifications();
    assert!(
        diagnostics
            .iter()
            .any(|d| d.notification == "welcome" && d.model_id == "user:9"),
        "NotificationSkipped{{MissingModel}} recorded"
    );
}

#[tokio::test]
async fn tc_m6_12_notification_delivered_when_user_exists() {
    // Clear prior diagnostics and restore the live set. Scope the guard so it
    // is never held across the dispatch await (`clippy::await_holding_lock`).
    {
        let mut live = live_users().lock().unwrap_or_else(|p| p.into_inner());
        live.clear();
        live.push("7".to_string());
    }

    let sent = SentFlag::new();
    let notification = WelcomeNotification {
        user_id: "7".to_string(),
        sent: sent.clone(),
    };

    let outcome = Queue::dispatch_sync(notification).await.unwrap();
    assert_eq!(outcome, JobOutcome::Succeeded, "delivered normally");
    assert!(sent.was_sent(), "mail body dispatched for a live user");
    let check = skipped_notifications();
    assert!(
        !check.iter().any(|d| d.model_id == "user:7"),
        "no skip diagnostic for a live user"
    );
}
