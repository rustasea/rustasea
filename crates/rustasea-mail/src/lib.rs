//! RustaSea Mail — Mailable/Mailer contracts, transports, and queued
//! notifications.
//!
//! Sprint 07 (M6) scope per sprint-07.md S07-T05: the [`Mailable`] contract,
//! pluggable [`Mailer`] transports (SMTP/Log/Array), the [`MailMessage`] model,
//! and [`MailNotification`] delivery through `rustasea-queue` with
//! `#[deleteWhenMissingModels]` suppression (FR-605, US-M6-04).
//!
//! ```no_run
//! use std::sync::Arc;
//! use rustasea_mail::{ArrayMailer, Mail, MailAddress, Mailable};
//!
//! struct Welcome { name: String }
//! impl Mailable for Welcome {
//!     fn subject(&self) -> String { "Welcome".into() }
//!     fn to(&self) -> Vec<MailAddress> { vec![MailAddress::from("ada@example.com")] }
//!     fn html_body(&self) -> String { format!("<h1>Hi {}</h1>", self.name) }
//! }
//!
//! # async fn run() -> rustasea_mail::Result<()> {
//! Mail::set_mailer(Arc::new(ArrayMailer::new()));
//! Mail::send(&Welcome { name: "Ada".into() }).await?;
//! # Ok(())
//! # }
//! ```

pub mod address;
pub mod error;
pub mod mailable;
pub mod mailer;
pub mod message;
pub mod notification;
#[cfg(feature = "smtp")]
pub mod smtp;

use std::sync::Arc;

pub use address::MailAddress;
pub use error::{MailError, Result};
pub use mailable::Mailable;
pub use mailer::{mailer, set_mailer, ArrayMailer, LogMailer, Mailer};
pub use message::MailMessage;
pub use notification::{MailNotification, QueuedNotification};
#[cfg(feature = "smtp")]
pub use smtp::SmtpMailer;

/// Entry point for sending and queueing mail.
pub struct Mail;

impl Mail {
    /// Install the process-wide mailer used by [`Mail::send`] and queued
    /// notifications.
    pub fn set_mailer(mailer: Arc<dyn Mailer>) {
        mailer::set_mailer(mailer);
    }

    /// The installed process-wide mailer, when one has been set.
    pub fn mailer() -> Option<Arc<dyn Mailer>> {
        mailer::mailer()
    }

    /// Deliver a mailable immediately through the process-wide mailer.
    pub async fn send<M: Mailable>(mailable: &M) -> Result<()> {
        let mailer =
            mailer::mailer().ok_or_else(|| MailError::Transport("no mailer installed".into()))?;
        mailer.send(mailable.build()).await
    }

    /// Queue a notification for asynchronous delivery.
    ///
    /// The returned handle resolves through the `notifications` route; call
    /// [`rustasea_queue::Queue::route_sync`] (or `route`) at boot so dispatch
    /// can resolve the job type.
    pub async fn queue<M: MailNotification>(mailable: M) -> Result<rustasea_queue::JobId> {
        Ok(rustasea_queue::Queue::dispatch(QueuedNotification::new(mailable)).await?)
    }
}
