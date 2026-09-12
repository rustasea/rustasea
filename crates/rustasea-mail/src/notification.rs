//! Queued notification delivery with missing-model suppression.
//!
//! A [`MailNotification`] is a serializable [`Mailable`]. Wrapping it in a
//! [`QueuedNotification`] produces a [`Job`] that declares
//! `delete_when_missing_models` (Laravel's `#[deleteWhenMissingModels]`): when
//! the referenced model was deleted before the worker ran, the job is skipped,
//! not retried, and a `NotificationSkipped { reason: MissingModel }`
//! diagnostic is recorded (FR-605, US-M6-04).

use std::any::type_name;

use async_trait::async_trait;
use rustasea_queue::{Job, JobError, NotificationSkipped};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::mailable::Mailable;

/// A serializable mailable that can be delivered from the queue.
///
/// Implementors optionally declare the referenced model so the worker can
/// suppress delivery when that model no longer exists.
pub trait MailNotification: Mailable + Serialize + DeserializeOwned {
    /// Identifier of the model this notification targets, when any.
    fn model_id(&self) -> Option<String> {
        None
    }

    /// Whether the targeted model still exists.
    fn model_exists(&self) -> bool {
        true
    }

    /// Diagnostic name used when the notification is suppressed.
    fn notification_name(&self) -> &'static str {
        type_name::<Self>()
    }
}

/// Queue payload wrapping a [`MailNotification`] for asynchronous delivery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedNotification<M> {
    /// The notification to deliver.
    mailable: M,
}

impl<M: MailNotification> QueuedNotification<M> {
    /// Wrap `mailable` for queued delivery.
    pub fn new(mailable: M) -> Self {
        Self { mailable }
    }

    /// Borrow the wrapped notification.
    pub fn mailable(&self) -> &M {
        &self.mailable
    }
}

#[async_trait]
impl<M: MailNotification> Job for QueuedNotification<M> {
    /// Deliver the notification through the process-wide mailer.
    async fn handle(self) -> std::result::Result<(), JobError> {
        let mailer = crate::mailer::mailer()
            .ok_or_else(|| JobError::Exception("no mailer installed".into()))?;
        mailer
            .send(self.mailable.build())
            .await
            .map_err(|e| JobError::Exception(e.to_string()))
    }

    /// Declare `#[deleteWhenMissingModels]` semantics.
    fn delete_when_missing_models(&self) -> bool {
        true
    }

    /// Suppress when a referenced model is gone.
    fn is_missing_model_suppressed(&self) -> bool {
        self.mailable.model_id().is_some() && !self.mailable.model_exists()
    }

    /// Record which model was missing when suppression fires.
    fn skipped_notification(&self) -> Option<NotificationSkipped> {
        self.mailable
            .model_id()
            .map(|id| NotificationSkipped::missing_model(self.mailable.notification_name(), id))
    }
}
