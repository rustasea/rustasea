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
/// [`PathError`] with no filesystem access performed. When `candidate` does
/// not exist yet (a pending write), the deepest existing ancestor is
/// canonicalized and checked first — so a symlinked parent directory that
/// points outside `root` is rejected instead of being silently resolved
/// underneath `root` (audit S5 S5).
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
        Err(_) => {
            // The candidate does not exist yet (write path). Canonicalize the
            // deepest existing ancestor so a symlinked parent is resolved
            // before the containment check; reject when that ancestor falls
            // outside the root.
            let Some(anchor) = deepest_existing_ancestor(candidate) else {
                return Ok(PathOutcome::Confined(root.join(candidate)));
            };
            match anchor.canonicalize() {
                Ok(abs) if abs.starts_with(&root) => {
                    Ok(PathOutcome::Confined(root.join(candidate)))
                }
                Ok(_) => Ok(PathOutcome::Traversal),
                Err(_) => Ok(PathOutcome::Confined(root.join(candidate))),
            }
        }
    }
}

/// Deepest existing ancestor of `path` (itself if it exists), else `None`.
///
/// Walks `path` towards its parents until an existing entry is found; `None`
/// means no ancestor exists at all (root included).
fn deepest_existing_ancestor(path: &Path) -> Option<PathBuf> {
    if path.exists() {
        return Some(path.to_path_buf());
    }
    path.parent()
        .filter(|parent| parent.as_os_str() != path.as_os_str())
        .and_then(deepest_existing_ancestor)
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
            "candidate \"../../etc/passwd\" escapes root \"/tmp/rustavel-store\""
        );
    }

    #[test]
    fn normal_relative_path_is_confined() {
        let outcome = confine_path(Path::new("."), Path::new("Cargo.toml")).unwrap();
        assert!(matches!(outcome, PathOutcome::Confined(_)));
    }

    #[test]
    fn symlinked_parent_outside_root_is_traversal() {
        // S5 regression: `escape -> /etc` is a symlink inside the root pointing
        // outside. A pending write under the link must be rejected — resolving
        // it to `root.join("escape/rustavel-escape")` would write into `/etc`.
        let base =
            std::env::temp_dir().join(format!("rustavel-path-symlink-{}", std::process::id()));
        std::fs::remove_dir_all(&base).ok();
        std::fs::create_dir_all(&base).unwrap();

        let root = base.join("root");
        std::fs::create_dir_all(&root).unwrap();
        let link = root.join("escape");
        #[cfg(unix)]
        std::os::unix::fs::symlink("/etc", &link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir("/etc", &link).unwrap();

        let outcome = confine_path(&root, &link.join("rustavel-escape"))
            .expect("escaped anchor must surface as Traversal, not an error");
        assert_eq!(outcome, PathOutcome::Traversal);

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn missing_path_under_safe_symlink_is_confined() {
        // A symlink that stays inside the root must still allow writes through
        // it once the anchor is verified under the root.
        let base =
            std::env::temp_dir().join(format!("rustavel-path-symlink-in-{}", std::process::id()));
        std::fs::remove_dir_all(&base).ok();
        std::fs::create_dir_all(&base).unwrap();

        let root = base.join("root");
        std::fs::create_dir_all(root.join("real")).unwrap();
        let link = root.join("alias");
        #[cfg(unix)]
        std::os::unix::fs::symlink(root.join("real"), &link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(root.join("real"), &link).unwrap();

        let outcome = confine_path(&root, &link.join("new-file.txt")).unwrap();
        assert!(matches!(outcome, PathOutcome::Confined(_)));

        std::fs::remove_dir_all(&base).ok();
    }
}
