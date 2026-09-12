//! SMTP transport backed by `lettre` (opt-in `smtp` feature).

use async_trait::async_trait;
use lettre::message::{header::ContentType, Mailbox, MultiPart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::address::MailAddress;
use crate::error::{MailError, Result};
use crate::mailer::Mailer;
use crate::message::MailMessage;

/// SMTP mailer built over lettre's async transport.
pub struct SmtpMailer {
    /// Underlying lettre transport.
    transport: AsyncSmtpTransport<Tokio1Executor>,
    /// Sender used when a message has no `from`.
    default_from: Option<MailAddress>,
}

impl SmtpMailer {
    /// Create an SMTP mailer from a STARTTLS relay host, port, and credentials.
    pub fn relay(
        host: impl Into<String>,
        port: u16,
        credentials: Option<(String, String)>,
    ) -> Result<Self> {
        let host = host.into();
        let mut builder = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)
            .map_err(|e| MailError::Transport(e.to_string()))?
            .port(port);
        if let Some((user, password)) = credentials {
            builder = builder.credentials(Credentials::new(user, password));
        }
        Ok(Self {
            transport: builder.build(),
            default_from: None,
        })
    }

    /// Set the sender used when a message has no `from`.
    pub fn with_default_from(mut self, from: impl Into<MailAddress>) -> Self {
        self.default_from = Some(from.into());
        self
    }

    /// Build a lettre message from the crate's message model.
    fn build_lettre(&self, message: &MailMessage) -> Result<Message> {
        let from = message
            .from
            .clone()
            .or_else(|| self.default_from.clone())
            .ok_or_else(|| MailError::InvalidRecipient("missing sender".into()))?;
        let mut builder = Message::builder().from(to_mailbox(&from)?);
        if let Some(reply_to) = &message.reply_to {
            builder = builder.reply_to(to_mailbox(reply_to)?);
        }
        for address in &message.to {
            builder = builder.to(to_mailbox(address)?);
        }
        for address in &message.cc {
            builder = builder.cc(to_mailbox(address)?);
        }
        for address in &message.bcc {
            builder = builder.bcc(to_mailbox(address)?);
        }
        let builder = builder.subject(message.subject.clone());

        let email = match (&message.html, &message.text) {
            (Some(html), Some(text)) => builder.multipart(MultiPart::alternative_plain_html(
                text.clone(),
                html.clone(),
            )),
            (Some(html), None) => builder.header(ContentType::TEXT_HTML).body(html.clone()),
            (None, Some(text)) => builder.header(ContentType::TEXT_PLAIN).body(text.clone()),
            (None, None) => builder.body(String::new()),
        };
        email.map_err(|e| MailError::Render(e.to_string()))
    }
}

#[async_trait]
impl Mailer for SmtpMailer {
    /// Validate, render, and deliver `message` over SMTP.
    async fn send(&self, message: MailMessage) -> Result<()> {
        message.validate()?;
        let email = self.build_lettre(&message)?;
        self.transport
            .send(email)
            .await
            .map(|_| ())
            .map_err(|e| MailError::Transport(e.to_string()))
    }
}

/// Convert a [`MailAddress`] into a lettre `Mailbox`.
fn to_mailbox(address: &MailAddress) -> Result<Mailbox> {
    address
        .to_string()
        .parse::<Mailbox>()
        .map_err(|e| MailError::InvalidRecipient(e.to_string()))
}
