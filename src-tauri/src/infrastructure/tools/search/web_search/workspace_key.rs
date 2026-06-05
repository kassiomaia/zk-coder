//! Converts a `workspace_root: Option<&Path>` into the string key used to
//! look up per-workspace policy and cache entries.
//!
//! Sessions without a workspace fall back to the empty string, which acts as
//! the global bucket (see `workspace_search_policy` and `web_search_cache`
//! schemas). Canonicalization ensures that `./project` and `/home/me/project`
//! resolve to the same key.

use std::path::Path;

/// Returns a stable string key for the policy / cache lookups.
#[must_use]
pub fn canonical_workspace_key(path: Option<&Path>) -> String {
    let Some(path) = path else {
        return String::new();
    };
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    let as_string = canonical.to_string_lossy().into_owned();

    if cfg!(windows) {
        as_string.to_lowercase()
    } else {
        as_string
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn none_path_returns_empty_string() {
        assert_eq!(canonical_workspace_key(None), "");
    }

    #[test]
    fn existing_path_is_canonicalized_to_absolute() {
        let temp = TempDir::new().expect("temp dir");
        let key = canonical_workspace_key(Some(temp.path()));
        assert!(!key.is_empty());
        assert!(
            PathBuf::from(&key).is_absolute(),
            "canonical key should be an absolute path: {key}"
        );
    }

    #[test]
    fn nonexistent_path_still_produces_a_key_without_panicking() {
        let key =
            canonical_workspace_key(Some(Path::new("/definitely/does/not/exist/zzz-web-search")));
        // canonicalize fails → we fall back to the raw path, which is still a
        // valid string; just assert it isn't empty so the caller doesn't see
        // the "no workspace" sentinel by accident.
        assert!(!key.is_empty());
    }

    #[test]
    fn same_path_produces_same_key_twice() {
        let temp = TempDir::new().expect("temp dir");
        let a = canonical_workspace_key(Some(temp.path()));
        let b = canonical_workspace_key(Some(temp.path()));
        assert_eq!(a, b);
    }
}
