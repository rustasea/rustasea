//! Mailer transports and the process-wide mailer registry.

use std::sync::{Arc, Mutex, OnceLock, RwLock};

use async_trait::async_trait;

use crate::error::Result;
use crate::message::MailMessage;

/// A transport that delivers fully-built messages.
#[async_trait]
pub trait Mailer: Send + Sync + 'static {
    /// Deliver `message`, or return a transport error.
    async fn send(&self, message: MailMessage) -> Result<()>;
}

/// In-memory mailer that records every delivered message.
///
/// Useful for tests and local development: no network is touched, and the
/// delivered messages can be asserted through [`ArrayMailer::sent`].
#[derive(Clone, Default)]
pub struct ArrayMailer {
    /// Messages delivered so far, in order.
    sent: Arc<Mutex<Vec<MailMessage>>>,
}

impl ArrayMailer {
    /// Create an empty recording mailer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of every delivered message, in order.
    pub fn sent(&self) -> Vec<MailMessage> {
        self.sent.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    /// Number of messages delivered.
    pub fn count(&self) -> usize {
        self.sent.lock().unwrap_or_else(|p| p.into_inner()).len()
    }

    /// Most recently delivered message, if any.
    pub fn last(&self) -> Option<MailMessage> {
        self.sent
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .last()
            .cloned()
    }

    /// Drop every recorded message.
    pub fn clear(&self) {
        self.sent.lock().unwrap_or_else(|p| p.into_inner()).clear();
    }
}

#[async_trait]
impl Mailer for ArrayMailer {
    /// Validate and record `message` in memory.
    async fn send(&self, message: MailMessage) -> Result<()> {
        message.validate()?;
        self.sent
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(message);
        Ok(())
    }
}

/// Mailer that formats messages to a log sink instead of delivering them.
#[derive(Clone, Default)]
pub struct LogMailer {
    /// Formatted lines captured so far.
    lines: Arc<Mutex<Vec<String>>>,
    /// Whether captured lines are also printed to stdout.
    echo: bool,
}

impl LogMailer {
    /// Create a logging mailer that records formatted lines.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a logging mailer that also prints lines to stdout.
    pub fn with_echo(echo: bool) -> Self {
        Self {
            lines: Arc::default(),
            echo,
        }
    }

    /// Formatted log lines captured so far.
    pub fn lines(&self) -> Vec<String> {
        self.lines.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    /// Drop every captured line.
    pub fn clear(&self) {
        self.lines.lock().unwrap_or_else(|p| p.into_inner()).clear();
    }

    /// Render `message` as a single diagnostic log line.
    pub fn format(message: &MailMessage) -> String {
        let recipients = message
            .recipients()
            .map(|address| address.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let body = message
            .html
            .as_deref()
            .or(message.text.as_deref())
            .unwrap_or("");
        format!(
            "mail to [{recipients}] subject={:?} bytes={}",
            message.subject,
            body.len()
        )
    }
}

#[async_trait]
impl Mailer for LogMailer {
    /// Validate, format, and capture `message` (optionally echoing it).
    async fn send(&self, message: MailMessage) -> Result<()> {
        message.validate()?;
        let line = Self::format(&message);
        if self.echo {
            println!("{line}");
        }
        self.lines
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(line);
        Ok(())
    }
}

/// Registry slot holding the process-wide mailer.
static MAILER: OnceLock<RwLock<Option<Arc<dyn Mailer>>>> = OnceLock::new();

/// Access the lazily-initialized mailer registry slot.
fn slot() -> &'static RwLock<Option<Arc<dyn Mailer>>> {
    MAILER.get_or_init(|| RwLock::new(None))
}

/// Install the process-wide mailer used by [`Mail`](crate::Mail) and queued
/// notifications.
pub fn set_mailer(mailer: Arc<dyn Mailer>) {
    *slot().write().unwrap_or_else(|p| p.into_inner()) = Some(mailer);
}

/// The process-wide mailer, when one has been installed.
pub fn mailer() -> Option<Arc<dyn Mailer>> {
    slot().read().unwrap_or_else(|p| p.into_inner()).clone()
}
