//! Mail rendering, transports, and queued-notification delivery tests.

use std::sync::Arc;

use rustasea_mail::{
    ArrayMailer, LogMailer, Mail, MailAddress, MailError, MailMessage, MailNotification, Mailable,
    Mailer, QueuedNotification,
};
use rustasea_queue::{skipped_notifications, JobOutcome, Queue};
use serde::{Deserialize, Serialize};

/// Minimal mailable used across the rendering tests.
struct WelcomeMail {
    /// Recipient display name.
    name: String,
}

impl Mailable for WelcomeMail {
    /// Build the subject line.
    fn subject(&self) -> String {
        format!("Welcome, {}", self.name)
    }

    /// Address the mail to Ada.
    fn to(&self) -> Vec<MailAddress> {
        vec![MailAddress::parse("Ada <ada@example.com>").unwrap()]
    }

    /// HTML body.
    fn html_body(&self) -> String {
        format!("<h1>Hello {}</h1>", self.name)
    }

    /// Plain-text body.
    fn text_body(&self) -> Option<String> {
        Some(format!("Hello {}", self.name))
    }
}

#[test]
fn mailable_builds_message() {
    let message = WelcomeMail { name: "Ada".into() }.build();
    assert_eq!(message.subject, "Welcome, Ada");
    assert_eq!(message.to.len(), 1);
    assert_eq!(message.to[0].email, "ada@example.com");
    assert_eq!(message.to[0].name.as_deref(), Some("Ada"));
    assert_eq!(message.html.as_deref(), Some("<h1>Hello Ada</h1>"));
    assert_eq!(message.text.as_deref(), Some("Hello Ada"));
}

#[tokio::test]
async fn array_mailer_records_messages() {
    let mailer = ArrayMailer::new();
    mailer
        .send(WelcomeMail { name: "Ada".into() }.build())
        .await
        .unwrap();
    assert_eq!(mailer.count(), 1);
    assert_eq!(mailer.last().unwrap().subject, "Welcome, Ada");
    mailer.clear();
    assert_eq!(mailer.count(), 0);
}

#[tokio::test]
async fn log_mailer_captures_formatted_line() {
    let mailer = LogMailer::new();
    mailer
        .send(WelcomeMail { name: "Ada".into() }.build())
        .await
        .unwrap();
    let lines = mailer.lines();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("ada@example.com"), "got {}", lines[0]);
    assert!(lines[0].contains("Welcome, Ada"), "got {}", lines[0]);
}

#[tokio::test]
async fn invalid_recipient_is_rejected() {
    let mailer = ArrayMailer::new();
    let message = MailMessage::new("Bad").to(MailAddress::from_email("not-an-email"));
    let err = mailer.send(message).await.unwrap_err();
    assert!(matches!(err, MailError::InvalidRecipient(_)), "got {err:?}");
    assert_eq!(mailer.count(), 0);
}

/// Serializable notification whose model liveness is controlled by a flag.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WelcomeNotification {
    /// Recipient name.
    name: String,
    /// Referenced user id, when any.
    user_id: Option<String>,
    /// Whether the referenced user still exists.
    live: bool,
}

impl Mailable for WelcomeNotification {
    /// Build the subject line.
    fn subject(&self) -> String {
        format!("Welcome, {}", self.name)
    }

    /// Address the notification to Ada.
    fn to(&self) -> Vec<MailAddress> {
        vec![MailAddress::from_email("ada@example.com")]
    }

    /// HTML body.
    fn html_body(&self) -> String {
        format!("<p>Hi {}</p>", self.name)
    }
}

impl MailNotification for WelcomeNotification {
    /// Expose the referenced user id.
    fn model_id(&self) -> Option<String> {
        self.user_id.clone()
    }

    /// Report whether the referenced user still exists.
    fn model_exists(&self) -> bool {
        self.live
    }
}

#[tokio::test]
async fn queued_notification_delivers_live_and_skips_missing_model() {
    let mailer = Arc::new(ArrayMailer::new());
    Mail::set_mailer(mailer.clone());
    let _ = Queue::route_sync::<QueuedNotification<WelcomeNotification>>("notifications").unwrap();

    // Live model → delivered inline through the sync connection.
    let live = WelcomeNotification {
        name: "Ada".into(),
        user_id: Some("user:1".into()),
        live: true,
    };
    let _ = Mail::queue(live).await.unwrap();
    assert_eq!(mailer.count(), 1);
    assert_eq!(mailer.last().unwrap().subject, "Welcome, Ada");

    // Deleted model → skipped, not retried, and diagnosed.
    let missing = WelcomeNotification {
        name: "Grace".into(),
        user_id: Some("user:9".into()),
        live: false,
    };
    let outcome = Queue::dispatch_sync(QueuedNotification::new(missing))
        .await
        .unwrap();
    assert_eq!(outcome, JobOutcome::Skipped);
    assert_eq!(
        mailer.count(),
        1,
        "suppressed notification must not be delivered"
    );
    assert!(
        skipped_notifications()
            .iter()
            .any(|record| record.model_id == "user:9"),
        "missing-model skip must be recorded"
    );
}
