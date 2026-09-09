//! Queued notifications — `#[deleteWhenMissingModels]` suppression (FR-605).
//!
//! A queued notification carries a model reference (e.g. `User`) in its
//! serialized payload. When the notification opts into missing-model
//! suppression (`#[deleteWhenMissingModels]` in Laravel terms), the queue
//! worker resolves the referenced model before sending: if the model was
//! deleted or soft-deleted before the worker ran, the job is **skipped, not
//! retried**, and a `NotificationSkipped { reason: MissingModel }` diagnostic
//! is recorded (US-M6-04, FS-M6-04, TC-M6-11/12).

use serde::{Deserialize, Serialize};

/// Reason a queued notification was not delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationSkipReason {
    /// The referenced model is missing or soft-deleted.
    MissingModel,
    /// The notification did not resolve to a sendable recipient.
    NoRecipient,
}

impl std::fmt::Display for NotificationSkipReason {
    /// Render the skip reason as its variant name (`MissingModel`).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            NotificationSkipReason::MissingModel => "MissingModel",
            NotificationSkipReason::NoRecipient => "NoRecipient",
        };
        f.write_str(name)
    }
}

/// Skipped-notification diagnostic record (domain event E-06).
///
/// Mirrors the `NotificationSkipped { reason: MissingModel }` log line from
/// `domain.md BC-6`: the worker records one of these instead of dispatching
/// the mail when suppression applies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationSkipped {
    /// Stable notification/job type name.
    pub notification: String,
    /// Identifier of the referenced model that was missing.
    pub model_id: String,
    /// Why delivery was suppressed.
    pub reason: NotificationSkipReason,
    /// UTC instant the skip was recorded.
    pub skipped_at: chrono::DateTime<chrono::Utc>,
}

impl NotificationSkipped {
    /// Convenience constructor recording a `MissingModel` skip.
    pub fn missing_model(notification: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self {
            notification: notification.into(),
            model_id: model_id.into(),
            reason: NotificationSkipReason::MissingModel,
            skipped_at: chrono::Utc::now(),
        }
    }
}

/// In-memory sink for `NotificationSkipped` diagnostics.
///
/// The real surface forwards to structured logs / a dead-letter queue; this
/// sink keeps the suppression contract assertable in-process (TC-M6-11).
static SKIPPED: std::sync::OnceLock<std::sync::Mutex<Vec<NotificationSkipped>>> =
    std::sync::OnceLock::new();

/// Record a skipped-notification diagnostic.
pub(crate) fn record_skipped(record: NotificationSkipped) {
    let list = SKIPPED.get_or_init(|| std::sync::Mutex::new(Vec::new()));
    if let Ok(mut guard) = list.lock() {
        guard.push(record);
    }
}

/// Snapshot the recorded skip diagnostics.
pub fn skipped_notifications() -> Vec<NotificationSkipped> {
    let Some(list) = SKIPPED.get() else {
        return Vec::new();
    };
    let Ok(guard) = list.lock() else {
        return Vec::new();
    };
    guard.clone()
}

/// Guard applied by the worker before dispatching a queued notification.
///
/// Implementors answer "does the referenced model still exist?" from the
/// notification payload. The guard is consulted only for jobs whose
/// `Job::delete_when_missing_models()` returns true (the
/// `#[deleteWhenMissingModels]` marker).
pub trait NotificationGuard: Send + Sync + 'static {
    /// Resolve the referenced model id (e.g. `"user:9"`) from the job payload.
    fn model_id(&self, payload: &serde_json::Value) -> Option<String>;

    /// Whether the model exists (and is not soft-deleted) right now.
    fn model_exists(&self, model_id: &str) -> bool;
}

/// Evaluate whether a notification job must be suppressed.
///
/// Returns `Ok(true)` (skip) when the job declares missing-model suppression,
/// the payload names a referenced model, and that model no longer exists.
/// `Ok(false)` means deliver normally. `Err` only when the guard or payload
/// contract is misconfigured (`model_id` matched no known reference shape).
pub fn should_suppress(
    notification_type: &str,
    payload: &serde_json::Value,
    guard: Option<&dyn NotificationGuard>,
) -> std::result::Result<bool, crate::QueueError> {
    let Some(guard) = guard else {
        // No guard registered: nothing can be resolved, so nothing is skipped.
        return Ok(false);
    };
    let Some(model_id) = guard.model_id(payload) else {
        return Err(crate::QueueError::Unrouted(Box::leak(
            format!("{notification_type}:no-model-reference").into_boxed_str(),
        )));
    };
    Ok(!guard.model_exists(&model_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct UserGuard {
        /// Live user ids the guard can see.
        live: Vec<String>,
    }

    impl NotificationGuard for UserGuard {
        fn model_id(&self, payload: &serde_json::Value) -> Option<String> {
            payload
                .get("user")
                .and_then(|u| u.get("id"))
                .and_then(|id| id.as_str())
                .map(|id| format!("user:{id}"))
        }

        fn model_exists(&self, model_id: &str) -> bool {
            self.live.iter().any(|live| live == model_id)
        }
    }

    #[test]
    fn missing_model_records_skip_with_reason() {
        let record = NotificationSkipped::missing_model("welcome", "user:9");
        assert_eq!(record.reason, NotificationSkipReason::MissingModel);
        assert_eq!(record.reason.to_string(), "MissingModel");
    }

    #[test]
    fn guard_suppresses_when_model_is_deleted() {
        let guard = UserGuard {
            live: vec!["user:1".to_string()],
        };
        let payload = json!({ "user": { "id": "9" } });
        assert!(should_suppress("welcome", &payload, Some(&guard)).unwrap());
    }

    #[test]
    fn guard_allows_when_model_still_exists() {
        let guard = UserGuard {
            live: vec!["user:9".to_string()],
        };
        let payload = json!({ "user": { "id": "9" } });
        assert!(!should_suppress("welcome", &payload, Some(&guard)).unwrap());
    }
}
