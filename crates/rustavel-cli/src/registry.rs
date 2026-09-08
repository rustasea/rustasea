//! Process-wide command registry backing [`crate::Artisan`].
//!
//! Commands are stored behind `Arc` so callers can clone a handle out of the
//! registry lock and execute it lock-free (no reentrancy deadlock when a
//! command dispatches another command). `list` reads metadata clones.

use std::sync::Arc;

use crate::artisan::{Command, CommandMeta};

/// A registered command: metadata plus the executable implementation.
#[derive(Clone)]
pub struct Registered {
    /// Command metadata.
    pub meta: CommandMeta,
    cmd: Arc<dyn Command>,
}

impl Registered {
    /// Create a registered entry from a command implementation.
    pub fn new(cmd: Box<dyn Command>) -> Self {
        let meta = CommandMeta {
            name: cmd.signature().to_string(),
            usage: cmd
                .usage()
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| cmd.signature().to_string()),
            help: cmd.help().unwrap_or("").to_string(),
            hidden: cmd.hidden(),
        };
        Self {
            meta,
            cmd: Arc::from(cmd),
        }
    }

    /// Access the executable command.
    pub fn command(&self) -> &dyn Command {
        self.cmd.as_ref()
    }
}

/// Registry of console commands shared by the clap shim and `Artisan::call`.
#[derive(Default)]
pub struct CommandRegistry {
    commands: Vec<Registered>,
}

impl CommandRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a command (later registrations replace earlier same-name ones).
    pub fn register(&mut self, command: impl Command + 'static) {
        let registered = Registered::new(Box::new(command));
        self.commands
            .retain(|r| r.meta.name != registered.meta.name);
        self.commands.push(registered);
    }

    /// Resolve a command by exact signature, cloning its handle.
    pub fn resolve(&self, signature: &str) -> Option<Registered> {
        self.commands
            .iter()
            .find(|r| r.meta.name == signature)
            .cloned()
    }

    /// List all registered entries in registration order (cloned handles).
    pub fn list(&self) -> Vec<Registered> {
        self.commands.clone()
    }

    /// Count registered commands.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Whether no command is registered.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Return the closest registered signature to `input`, if any.
    ///
    /// Candidates farther than half the input's length (minimum 3 edits) are
    /// ignored so suggestions stay relevant.
    pub fn suggestion(&self, input: &str) -> Option<String> {
        let best = self
            .commands
            .iter()
            .map(|r| r.meta.name.as_str())
            .min_by_key(|name| strsim::levenshtein(input, name))?;
        let distance = strsim::levenshtein(input, best);
        let threshold = (input.len() / 2).max(3);
        if distance <= threshold {
            Some(best.to_string())
        } else {
            None
        }
    }

    /// Clear every registration (test isolation).
    pub fn clear(&mut self) {
        self.commands.clear();
    }
}
