//! Command trait, registry, and the Artisan facade.
//!
//! A console command is a [`Command`] implementor carrying a static
//! `signature` plus optional `usage`/`help`/`hidden` metadata (the M5
//! `#[usage]`/`#[help]`/`#[hidden]` attributes feed the same shape).
//! [`Artisan::call`] dispatches commands **in-process** — no subprocess —
//! which `TestCase` and the `migrate` command rely on (FR-502, FR-505).
//!
//! `run` is asynchronous so built-ins can await M4 facades (`queue:retry`,
//! `schedule:run`, …) without a subprocess runtime boundary.

use std::fmt::Write as _;

use async_trait::async_trait;

use crate::error::{CliError, CliResult};

/// Metadata describing one console command (contract `list --json`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommandMeta {
    /// Command name/signature, e.g. `make:controller`.
    pub name: String,
    /// Laravel-style usage signature, e.g. `make:controller {name} [--resource]`.
    #[serde(default)]
    pub usage: String,
    /// One-line description shown under the name in `list`.
    #[serde(default)]
    pub help: String,
    /// Whether the command is omitted from `list` unless `--all`.
    #[serde(default)]
    pub hidden: bool,
}

/// Buffered result of an in-process command run.
#[derive(Debug, Clone, Default)]
pub struct CommandOutput {
    /// Captured stdout lines.
    pub stdout: String,
    /// Captured stderr lines.
    pub stderr: String,
    /// Process-style exit code (`0` success).
    pub exit_code: i32,
}

impl CommandOutput {
    /// Append a formatted line to stdout.
    pub fn line(&mut self, text: impl AsRef<str>) {
        let _ = writeln!(self.stdout, "{}", text.as_ref());
    }

    /// Append a formatted line to stderr.
    pub fn error_line(&mut self, text: impl AsRef<str>) {
        let _ = writeln!(self.stderr, "{}", text.as_ref());
    }

    /// Whether the run finished successfully.
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Non-TTY-capable stdout abstraction (falls back to plain lines).
pub type Io = CommandOutput;

/// A runnable console command.
///
/// Implementors declare a static `signature()`; the optional trait methods
/// mirror `#[usage]`/`#[help]`/`#[hidden]` for commands registered
/// programmatically, keeping the enumeration contract uniform.
#[async_trait]
pub trait Command: Send + Sync {
    /// Command name/signature, e.g. `make:controller`.
    fn signature(&self) -> &'static str;

    /// Optional usage signature rendered by `list` and `--help`.
    fn usage(&self) -> Option<&'static str> {
        None
    }

    /// Optional one-line description.
    fn help(&self) -> Option<&'static str> {
        None
    }

    /// Whether the command is hidden from `list` without `--all`.
    fn hidden(&self) -> bool {
        false
    }

    /// Execute the command with raw string arguments, appending to `io`.
    async fn run(&self, args: Vec<String>, io: &mut Io) -> CliResult<()>;
}

/// The Artisan facade — in-process command invocation (FR-502, FR-505).
///
/// Backed by the process-wide registry singleton; no subprocess is spawned.
pub struct Artisan;

impl Artisan {
    /// Run a command in-process with raw arguments, returning its output.
    ///
    /// Unknown commands surface [`CliError::UnknownCommand`] with a `did you
    /// mean?` suggestion; hidden commands refuse direct dispatch via
    /// [`CliError::HiddenCommand`]. A command's own error is captured into
    /// the output's `stderr` + typed exit code rather than propagated, so
    /// callers can assert on both channels.
    pub async fn call(signature: &str, args: Vec<String>) -> CliResult<CommandOutput> {
        let registered = {
            let registry = crate::registry();
            let guard = registry
                .read()
                .map_err(|_| CliError::Domain("command registry lock poisoned".into()))?;
            guard.resolve(signature).ok_or_else(|| {
                CliError::UnknownCommand(match guard.suggestion(signature) {
                    Some(s) => format!("{signature}. Did you mean {s}?"),
                    None => signature.to_string(),
                })
            })?
        };
        if registered.meta.hidden {
            return Err(CliError::HiddenCommand(signature.to_string()));
        }
        let mut out = CommandOutput::default();
        match registered.command().run(args, &mut out).await {
            Ok(()) => out.exit_code = 0,
            Err(e) => {
                out.exit_code = crate::error::exit_code(&e);
                out.error_line(e.to_string());
            }
        }
        Ok(out)
    }

    /// List registered commands with optional hidden inclusion.
    pub fn list(include_hidden: bool) -> Vec<CommandMeta> {
        let registry = crate::registry();
        let Ok(guard) = registry.read() else {
            return Vec::new();
        };
        guard
            .list()
            .into_iter()
            .filter(|r| include_hidden || !r.meta.hidden)
            .map(|r| r.meta.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test command with fixed metadata.
    struct Hello;

    #[async_trait]
    impl Command for Hello {
        fn signature(&self) -> &'static str {
            "hello"
        }
        fn usage(&self) -> Option<&'static str> {
            Some("hello {name}")
        }
        fn help(&self) -> Option<&'static str> {
            Some("Say hello")
        }
        async fn run(&self, args: Vec<String>, io: &mut Io) -> CliResult<()> {
            let name = args.first().map(String::as_str).unwrap_or("world");
            io.line(format!("hello, {name}"));
            Ok(())
        }
    }

    /// Hidden test command.
    struct Secret;

    #[async_trait]
    impl Command for Secret {
        fn signature(&self) -> &'static str {
            "internal:debug"
        }
        fn hidden(&self) -> bool {
            true
        }
        async fn run(&self, _args: Vec<String>, io: &mut Io) -> CliResult<()> {
            io.line("debug");
            Ok(())
        }
    }

    /// Verifies call dispatch, output capture, and unknown-command suggestion.
    #[tokio::test]
    async fn artisan_call_in_process() {
        let registry = crate::registry();
        {
            let mut reg = registry.write().expect("registry lock poisoned");
            reg.clear();
            reg.register(Hello);
            reg.register(Secret);
        }

        let out = Artisan::call("hello", vec!["Ada".into()]).await.unwrap();
        assert!(out.is_success());
        assert_eq!(out.stdout, "hello, Ada\n");

        let err = Artisan::call("helo", vec![]).await.unwrap_err();
        assert!(
            matches!(err, CliError::UnknownCommand(_)),
            "expected UnknownCommand, got {err:?}"
        );
        assert!(
            err.to_string().contains("hello"),
            "suggestion missing: {err}"
        );

        let hidden = Artisan::call("internal:debug", vec![]).await.unwrap_err();
        assert!(matches!(hidden, CliError::HiddenCommand(_)));
    }

    /// Verifies list hides hidden commands unless --all.
    #[test]
    fn list_filters_hidden() {
        let registry = crate::registry();
        {
            let mut reg = registry.write().expect("registry lock poisoned");
            reg.clear();
            reg.register(Hello);
            reg.register(Secret);
        }

        let visible = Artisan::list(false);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].name, "hello");

        let all = Artisan::list(true);
        assert_eq!(all.len(), 2);
        assert!(all.iter().any(|m| m.hidden && m.name == "internal:debug"));
    }
}
