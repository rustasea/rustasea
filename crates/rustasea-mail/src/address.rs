//! Mail address value type and parsing.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{MailError, Result};

/// A single mail recipient or sender.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailAddress {
    /// Optional display name (`Ada Lovelace`).
    pub name: Option<String>,
    /// Bare email address (`ada@example.com`).
    pub email: String,
}

impl MailAddress {
    /// Create an address from an email and an optional display name.
    pub fn new(email: impl Into<String>, name: Option<String>) -> Self {
        Self {
            name,
            email: email.into(),
        }
    }

    /// Create a bare address without a display name.
    pub fn from_email(email: impl Into<String>) -> Self {
        Self {
            name: None,
            email: email.into(),
        }
    }

    /// Parse `"Name <email>"` or a bare `email`, validating the result.
    pub fn parse(value: &str) -> Result<Self> {
        let trimmed = value.trim();
        let address = match trimmed.split_once('<') {
            Some((name, rest)) => {
                let email = rest.trim_end_matches('>').trim().to_string();
                let name = name.trim().trim_matches('"').trim();
                Self {
                    name: (!name.is_empty()).then(|| name.to_string()),
                    email,
                }
            }
            None => Self::from_email(trimmed),
        };
        address.validate()?;
        Ok(address)
    }

    /// Validate the bare email shape (non-empty local/domain, single `@`).
    pub fn validate(&self) -> Result<()> {
        let invalid = || MailError::InvalidRecipient(self.email.clone());
        if self.email.is_empty() || self.email.chars().any(char::is_whitespace) {
            return Err(invalid());
        }
        let mut parts = self.email.split('@');
        let local = parts.next().unwrap_or_default();
        let domain = parts.next().unwrap_or_default();
        if local.is_empty() || domain.is_empty() || parts.next().is_some() {
            return Err(invalid());
        }
        Ok(())
    }
}

impl From<&str> for MailAddress {
    /// Best-effort parse; malformed input becomes a bare address.
    fn from(value: &str) -> Self {
        Self::parse(value).unwrap_or_else(|_| Self::from_email(value.trim()))
    }
}

impl From<String> for MailAddress {
    /// Best-effort parse; malformed input becomes a bare address.
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl FromStr for MailAddress {
    type Err = MailError;

    /// Strict parse; malformed input returns [`MailError::InvalidRecipient`].
    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

impl fmt::Display for MailAddress {
    /// Render as `Name <email>` or the bare email.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.name {
            Some(name) => write!(f, "{name} <{}>", self.email),
            None => f.write_str(&self.email),
        }
    }
}
