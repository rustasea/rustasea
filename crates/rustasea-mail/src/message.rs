//! Concrete mail message assembled from a [`Mailable`](crate::Mailable).

use serde::{Deserialize, Serialize};

use crate::address::MailAddress;
use crate::error::{MailError, Result};

/// A fully-rendered email ready for transport.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MailMessage {
    /// Optional sender; transports may fall back to a configured default.
    pub from: Option<MailAddress>,
    /// Optional `Reply-To` address.
    pub reply_to: Option<MailAddress>,
    /// Primary recipients.
    pub to: Vec<MailAddress>,
    /// Carbon-copy recipients.
    pub cc: Vec<MailAddress>,
    /// Blind carbon-copy recipients.
    pub bcc: Vec<MailAddress>,
    /// Subject line.
    pub subject: String,
    /// HTML body, when the message is HTML-capable.
    pub html: Option<String>,
    /// Plain-text body.
    pub text: Option<String>,
}

impl MailMessage {
    /// Create an empty message with `subject`.
    pub fn new(subject: impl Into<String>) -> Self {
        Self {
            subject: subject.into(),
            ..Self::default()
        }
    }

    /// Set the sender.
    pub fn sender(mut self, from: impl Into<MailAddress>) -> Self {
        self.from = Some(from.into());
        self
    }

    /// Set the `Reply-To` address.
    pub fn reply_to(mut self, reply_to: impl Into<MailAddress>) -> Self {
        self.reply_to = Some(reply_to.into());
        self
    }

    /// Append a primary recipient.
    pub fn to(mut self, to: impl Into<MailAddress>) -> Self {
        self.to.push(to.into());
        self
    }

    /// Append a carbon-copy recipient.
    pub fn cc(mut self, cc: impl Into<MailAddress>) -> Self {
        self.cc.push(cc.into());
        self
    }

    /// Append a blind carbon-copy recipient.
    pub fn bcc(mut self, bcc: impl Into<MailAddress>) -> Self {
        self.bcc.push(bcc.into());
        self
    }

    /// Set the HTML body.
    pub fn html(mut self, html: impl Into<String>) -> Self {
        self.html = Some(html.into());
        self
    }

    /// Set the plain-text body.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// All envelope recipients (`to` + `cc` + `bcc`).
    pub fn recipients(&self) -> impl Iterator<Item = &MailAddress> {
        self.to.iter().chain(self.cc.iter()).chain(self.bcc.iter())
    }

    /// Validate required fields and every address before transport.
    pub fn validate(&self) -> Result<()> {
        if self.to.is_empty() && self.cc.is_empty() && self.bcc.is_empty() {
            return Err(MailError::InvalidRecipient("no recipients".into()));
        }
        if let Some(from) = &self.from {
            from.validate()?;
        }
        if let Some(reply_to) = &self.reply_to {
            reply_to.validate()?;
        }
        for address in self.recipients() {
            address.validate()?;
        }
        Ok(())
    }
}
