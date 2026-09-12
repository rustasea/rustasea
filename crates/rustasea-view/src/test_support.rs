//! Test-only fixtures shared by the crate's unit tests.
//!
//! Compiled exclusively under `#[cfg(test)]`; nothing here is part of the
//! public API.

#[cfg(feature = "runtime-templates")]
use std::path::{Path, PathBuf};
#[cfg(feature = "runtime-templates")]
use std::sync::atomic::{AtomicU64, Ordering};

/// Askama template used to exercise compile-time rendering and auto-escaping.
///
/// Resolves through this crate's `askama.toml`, proving template discovery is
/// rooted at `resources/views/` (ADR-0002 §6).
#[derive(askama::Template, serde::Deserialize)]
#[template(path = "hello.html")]
pub(crate) struct Hello {
    /// Value interpolated into the greeting.
    pub(crate) name: String,
}

/// Temporary views directory removed when dropped.
#[cfg(feature = "runtime-templates")]
pub(crate) struct TempViews {
    /// Root directory handed to the runtime engine.
    path: PathBuf,
}

#[cfg(feature = "runtime-templates")]
impl TempViews {
    /// Create an empty temp directory with a process-unique name.
    pub(crate) fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("rustasea-view-{}-{}", std::process::id(), unique));
        std::fs::create_dir_all(&path).expect("create temp views dir");
        Self { path }
    }

    /// Write `contents` to `name` inside the temp directory.
    pub(crate) fn write(&self, name: &str, contents: &str) {
        std::fs::write(self.path.join(name), contents).expect("write temp view");
    }

    /// Borrow the temp directory path.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(feature = "runtime-templates")]
impl Drop for TempViews {
    /// Remove the temp directory and all of its contents.
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
