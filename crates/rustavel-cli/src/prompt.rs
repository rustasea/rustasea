//! Interactive prompts — ask / secret / confirm / choice / multi_select.
//!
//! Wraps `dialoguer` behind a tiny non-TTY fallback: every prompt detects
//! whether stdin is a terminal and returns `None`/`false` on CI instead of
//! blocking forever (FS-M5-04 compliance: degrade in non-TTY).

use std::io::IsTerminal;

/// Answers that can be produced without a TTY (CI fallback).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonTty {
    /// Return the fallback value for this prompt kind.
    Fallback,
    /// Abort as if the user pressed Ctrl-C.
    Abort,
}

/// Environment the prompt layer runs under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptEnv {
    /// Real terminal — full dialoguer rendering.
    Interactive,
    /// Non-terminal — degraded fallbacks.
    NonInteractive,
}

impl PromptEnv {
    /// Detect the environment from stdin/stdout.
    pub fn detect() -> Self {
        if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
            PromptEnv::Interactive
        } else {
            PromptEnv::NonInteractive
        }
    }
}

/// Map a `dialoguer` interaction error into an `io::Error`.
fn io_err(e: dialoguer::Error) -> std::io::Error {
    std::io::Error::other(e)
}

/// Ask an open question and return the trimmed answer.
///
/// Returns `Ok(None)` when the user aborts or stdin is not a terminal.
pub fn ask(question: &str) -> std::io::Result<Option<String>> {
    if PromptEnv::detect() == PromptEnv::NonInteractive {
        return Ok(None);
    }
    let input = dialoguer::Input::<String>::new()
        .with_prompt(question)
        .interact_text()
        .map_err(io_err)?;
    let trimmed = input.trim().to_string();
    Ok((!trimmed.is_empty()).then_some(trimmed))
}

/// Ask a secret (echo-free) question and return the answer.
///
/// Returns `Ok(None)` when the user aborts or stdin is not a terminal.
pub fn secret(question: &str) -> std::io::Result<Option<String>> {
    if PromptEnv::detect() == PromptEnv::NonInteractive {
        return Ok(None);
    }
    let input = dialoguer::Password::new()
        .with_prompt(question)
        .interact()
        .map_err(io_err)?;
    Ok((!input.is_empty()).then_some(input))
}

/// Confirm a yes/no question.
///
/// `Ok(false)` means "no" — the caller aborts the operation. A non-terminal
/// stdin resolves to `Ok(false)` so CI pipelines halt before destructive
/// actions rather than guessing.
pub fn confirm(question: &str) -> std::io::Result<bool> {
    if PromptEnv::detect() == PromptEnv::NonInteractive {
        return Ok(false);
    }
    dialoguer::Confirm::new()
        .with_prompt(question)
        .default(false)
        .interact()
        .map_err(io_err)
}

/// Offer a single choice from `options` and return its index.
///
/// Returns `Ok(None)` when the user aborts or stdin is not a terminal.
pub fn choice(question: &str, options: &[&str]) -> std::io::Result<Option<usize>> {
    if PromptEnv::detect() == PromptEnv::NonInteractive || options.is_empty() {
        return Ok(None);
    }
    let selection = dialoguer::Select::new()
        .with_prompt(question)
        .items(options)
        .default(0)
        .interact()
        .map_err(io_err)?;
    Ok(Some(selection))
}

/// Offer a multi-select over `options` and return the chosen indices.
///
/// Returns `Ok(Vec::new())` when the user aborts or stdin is not a terminal.
pub fn multi_select(question: &str, options: &[&str]) -> std::io::Result<Vec<usize>> {
    if PromptEnv::detect() == PromptEnv::NonInteractive || options.is_empty() {
        return Ok(Vec::new());
    }
    let selections = dialoguer::MultiSelect::new()
        .with_prompt(question)
        .items(options)
        .interact()
        .map_err(io_err)?;
    Ok(selections)
}

/// Typed choice return: pick one option and map it to `T`.
///
/// Convenience for generator flows (`choice_kind("Kind?", &["controller","model"])`).
pub fn choice_value<T>(question: &str, options: &[(&str, T)]) -> std::io::Result<Option<T>>
where
    T: Clone,
{
    if options.is_empty() {
        return Ok(None);
    }
    let labels: Vec<&str> = options.iter().map(|(label, _)| *label).collect();
    match choice(question, &labels)? {
        Some(index) => Ok(options.get(index).map(|(_, value)| value.clone())),
        None => Ok(None),
    }
}

/// Whether prompts are interactive in the current environment.
pub fn interactive() -> bool {
    PromptEnv::detect() == PromptEnv::Interactive
}
