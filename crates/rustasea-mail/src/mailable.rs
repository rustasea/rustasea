//! The `Mailable` contract — a renderable mail that knows its recipients.

use crate::address::MailAddress;
use crate::message::MailMessage;

/// A renderable mail that knows its recipients and bodies.
///
/// Implementors provide `subject`/`to`/`html_body` (and optionally
/// `text_body`, `cc`, `bcc`, `sender`, `reply_to`); [`Mailable::build`]
/// assembles them into a transport-ready [`MailMessage`].
pub trait Mailable: Send + Sync + 'static {
    /// Subject line.
    fn subject(&self) -> String;

    /// Primary recipients.
    fn to(&self) -> Vec<MailAddress>;

    /// HTML body.
    fn html_body(&self) -> String;

    /// Optional plain-text body.
    fn text_body(&self) -> Option<String> {
        None
    }

    /// Carbon-copy recipients (default empty).
    fn cc(&self) -> Vec<MailAddress> {
        Vec::new()
    }

    /// Blind carbon-copy recipients (default empty).
    fn bcc(&self) -> Vec<MailAddress> {
        Vec::new()
    }

    /// Optional sender override.
    fn sender(&self) -> Option<MailAddress> {
        None
    }

    /// Optional `Reply-To` override.
    fn reply_to(&self) -> Option<MailAddress> {
        None
    }

    /// Assemble the concrete [`MailMessage`] for this mailable.
    fn build(&self) -> MailMessage {
        let mut message = MailMessage::new(self.subject());
        message.from = self.sender();
        message.reply_to = self.reply_to();
        message.to = self.to();
        message.cc = self.cc();
        message.bcc = self.bcc();
        message.html = Some(self.html_body());
        message.text = self.text_body();
        message
    }
}
