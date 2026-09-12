//! The scaffold driver: turn a name + variant into a written application tree.

use std::path::{Path, PathBuf};

use crate::error::{ScaffoldError, ScaffoldResult};
use crate::name::AppName;
use crate::templates::{self, Placeholders};
use crate::variant::StarterKitVariant;

/// One file written (or about to be written) by a scaffold run.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Generated {
    /// Path relative to the application root (`app/models/user.rs`).
    pub path: String,
    /// Number of bytes written.
    pub bytes: usize,
}

/// A rendered file that has not yet been persisted (dry-run / tests).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedFile {
    /// Path relative to the application root.
    pub path: String,
    /// Fully substituted file contents.
    pub contents: String,
}

/// A pending `cargo rustasea new` request.
#[derive(Debug, Clone)]
pub struct Scaffold {
    app_name: String,
    variant: StarterKitVariant,
    force: bool,
}

impl Scaffold {
    /// Build a scaffold request for `app_name` and `variant`.
    pub fn new(app_name: impl Into<String>, variant: StarterKitVariant) -> Self {
        Self {
            app_name: app_name.into(),
            variant,
            force: false,
        }
    }

    /// Overwrite existing files instead of failing with
    /// [`ScaffoldError::AlreadyExists`].
    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    /// The presentation variant this request targets.
    pub fn variant(&self) -> StarterKitVariant {
        self.variant
    }

    /// Render every file for this request without touching the filesystem.
    ///
    /// Template substitution and name validation happen here, so callers get
    /// the same errors as [`Scaffold::generate`] without side effects.
    pub fn render(&self) -> ScaffoldResult<Vec<RenderedFile>> {
        let name = AppName::parse(&self.app_name)?;
        let vars = Placeholders::new(&name, self.variant);
        let files = templates::entries(self.variant)
            .into_iter()
            .map(|(path, template)| RenderedFile {
                path: path.to_string(),
                contents: templates::render(template, &vars),
            });
        Ok(files.collect())
    }

    /// Write the full application tree under `path`.
    ///
    /// The target directory is created if missing. Existing files are left in
    /// place unless [`Scaffold::with_force`] is set, in which case they are
    /// overwritten. Returns the list of written files in template order.
    pub fn generate(&self, path: &Path) -> ScaffoldResult<Vec<Generated>> {
        if path.exists() && !path.is_dir() {
            return Err(ScaffoldError::InvalidTarget {
                path: path.display().to_string(),
            });
        }
        let rendered = self.render()?;
        let mut generated = Vec::with_capacity(rendered.len());
        for file in rendered {
            let absolute = path.join(&file.path);
            if absolute.exists() && !self.force {
                return Err(ScaffoldError::AlreadyExists { path: file.path });
            }
            if let Some(parent) = absolute.parent() {
                std::fs::create_dir_all(parent).map_err(|source| ScaffoldError::Io {
                    path: file.path.clone(),
                    source,
                })?;
            }
            std::fs::write(&absolute, file.contents.as_bytes()).map_err(|source| {
                ScaffoldError::Io {
                    path: file.path.clone(),
                    source,
                }
            })?;
            generated.push(Generated {
                path: file.path,
                bytes: file.contents.len(),
            });
        }
        Ok(generated)
    }
}

/// Resolve the application root for a CLI-supplied name/path.
///
/// The last path component is the application name used for crate/module
/// identifiers; the whole value is the output directory.
pub fn resolve_target(name_or_path: &str) -> ScaffoldResult<(PathBuf, String)> {
    let target = PathBuf::from(name_or_path);
    let app_name = target
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.to_string())
        .ok_or_else(|| ScaffoldError::InvalidAppName {
            name: name_or_path.to_string(),
        })?;
    Ok((target, app_name))
}
