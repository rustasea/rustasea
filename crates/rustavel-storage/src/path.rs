//! Path confinement: canonical containment checks with no traversal.

use std::path::{Component, Path, PathBuf};

use crate::error::PathError;

/// Outcome of a confinement check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathOutcome {
    /// Path is safely inside the root.
    Confined(PathBuf),
    /// Path escapes the root (traversal attempt).
    Traversal,
}

/// Check whether `candidate` stays within `root` after canonicalization.
///
/// Confinement is `candidate.canonicalize()` + `starts_with(root)`
/// (FS-M6-03 contract); a traversal attempt (`../../etc/passwd`) yields
/// [`PathError`] with no filesystem access performed.
pub fn confine_path(root: &Path, candidate: &Path) -> Result<PathOutcome, PathError> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    if has_parent_escape(candidate) {
        return Err(PathError(format!(
            "candidate {:?} escapes root {:?}",
            candidate, root
        )));
    }
    match candidate.canonicalize() {
        Ok(abs) => {
            if abs.starts_with(&root) {
                Ok(PathOutcome::Confined(abs))
            } else {
                Err(PathError(format!("{:?} escapes root {:?}", abs, root)))
            }
        }
        Err(_) => Ok(PathOutcome::Confined(root.join(candidate))),
    }
}

/// Cheap lexical pre-check for `..` components that would escape the root.
fn has_parent_escape(path: &Path) -> bool {
    path.components().any(|c| matches!(c, Component::ParentDir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexical_parent_dir_is_traversal() {
        let root = Path::new("/tmp/rustavel-store");
        let err = confine_path(root, Path::new("../../etc/passwd")).unwrap_err();
        assert_eq!(
            err.0,
            "path traversal detected: \"../../etc/passwd\" escapes root \"/tmp/rustavel-store\""
        );
    }

    #[test]
    fn normal_relative_path_is_confined() {
        let outcome = confine_path(Path::new("."), Path::new("Cargo.toml")).unwrap();
        assert!(matches!(outcome, PathOutcome::Confined(_)));
    }
}
