//! Per-directory workspace listing for the UI's Files panel.
//!
//! The UI renders a lazy, VS Code-style tree: only the children of an
//! expanded directory are fetched. Each call walks **one** directory
//! non-recursively, which keeps the initial session load cheap and
//! removes the old whole-workspace walk that was the dominant cost of
//! agent-run UI stalls.
//!
//! # Constraints
//! - symlinks are skipped (no infinite loops, no workspace escapes)
//! - `.git/` is hidden at the root (binary plumbing, no display value)
//! - results are capped per directory — see [`MAX_ENTRIES_PER_DIR`]
//! - `path` is a forward-slash relative path under `root`; any segment
//!   that escapes the workspace (`..`, absolute, Windows prefix) is
//!   rejected before any I/O
//!
//! Paths are returned relative to `root`, using `/` separators on every
//! platform for parity with touched-file events.

use serde::Serialize;
use std::path::{Component, Path, PathBuf};

/// Per-directory cap on returned entries.
///
/// Picked to keep a single expanded folder's Svelte render under ~25 ms
/// without virtualization. Real repos contain pathological folders
/// (e.g. `target/debug/deps` with tens of thousands of entries); the
/// cap truncates those after a locale-aware sort so users still see a
/// meaningful alphabetical slice. Raise if user reports demand it;
/// lowering after adoption is harder.
pub const MAX_ENTRIES_PER_DIR: usize = 1_000;

/// One entry in a directory listing.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct WorkspaceEntry {
    /// Path relative to the workspace root, using `/` separators.
    pub path: String,
    /// Kind of entry: `"file"` or `"directory"`.
    pub kind: &'static str,
}

/// Response for `list_workspace_dir`.
#[derive(Debug, Serialize)]
pub struct WorkspaceDirResponse {
    /// Immediate children of the requested directory, sorted
    /// directories-first then case-insensitive by name.
    pub entries: Vec<WorkspaceEntry>,
    /// `true` when the directory contained more than
    /// [`MAX_ENTRIES_PER_DIR`] entries and the listing was truncated.
    pub truncated: bool,
}

/// List the immediate children of one directory under `root`.
///
/// `path` is a forward-slash relative path; pass an empty string to
/// list the workspace root itself.
///
/// # Errors
/// - `root` does not exist or is not a directory
/// - `path` escapes `root` (contains `..`, is absolute, or otherwise
///   resolves outside the workspace)
/// - the resolved directory does not exist or is not a directory
#[tauri::command]
pub async fn list_workspace_dir(
    root: String,
    path: String,
) -> Result<WorkspaceDirResponse, String> {
    let root_path = PathBuf::from(&root);
    let target = resolve_target(&root_path, &path)?;
    read_dir_sorted(&root_path, &target, MAX_ENTRIES_PER_DIR)
}

/// Validate `rel` and join it under `root`.
///
/// Rejects any component that could escape the workspace before any
/// disk access happens.
fn resolve_target(root: &Path, rel: &str) -> Result<PathBuf, String> {
    if !root.exists() {
        return Err(format!("Workspace root does not exist: {}", root.display()));
    }
    let root_meta = std::fs::symlink_metadata(root)
        .map_err(|e| format!("Failed to stat workspace root: {e}"))?;
    if !root_meta.is_dir() {
        return Err(format!(
            "Workspace root is not a directory: {}",
            root.display()
        ));
    }

    let trimmed = rel.trim_end_matches('/');
    if trimmed.is_empty() {
        return Ok(root.to_path_buf());
    }

    // Validate: forward-slash-only, no empty segments, no `..`, no absolute
    // components. Reject anything exotic before touching the filesystem.
    // A leading `/` surfaces as `Component::RootDir` and is rejected here.
    let rel_path = Path::new(trimmed);
    for component in rel_path.components() {
        match component {
            Component::Normal(_) => {}
            _ => {
                return Err(format!("Path escapes workspace root: {trimmed}"));
            }
        }
    }

    let target = root.join(rel_path);
    if !target.exists() {
        return Err(format!("Directory does not exist: {trimmed}"));
    }
    let target_meta = std::fs::symlink_metadata(&target)
        .map_err(|e| format!("Failed to stat directory: {e}"))?;
    if target_meta.file_type().is_symlink() {
        return Err(format!("Path is a symlink: {trimmed}"));
    }
    if !target_meta.is_dir() {
        return Err(format!("Path is not a directory: {trimmed}"));
    }

    Ok(target)
}

/// Read `target`'s immediate children, sort, and truncate to `cap`.
///
/// `root` is the workspace root; returned paths are relative to it
/// with forward-slash separators. `target` must be a resolved
/// descendant of `root`.
fn read_dir_sorted(
    root: &Path,
    target: &Path,
    cap: usize,
) -> Result<WorkspaceDirResponse, String> {
    let read = std::fs::read_dir(target)
        .map_err(|e| format!("Failed to read directory: {e}"))?;

    let at_root = target == root;

    let mut entries: Vec<WorkspaceEntry> = Vec::new();
    for dir_entry in read.flatten() {
        let file_type = match dir_entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };

        // Skip symlinks — avoids loops and workspace escapes.
        if file_type.is_symlink() {
            continue;
        }

        let Some(name) = dir_entry.file_name().into_string().ok() else {
            continue;
        };

        // Hide `.git` only at the workspace root — inside the tree
        // we don't descend into it because we'd never be asked to.
        if at_root && name == ".git" {
            continue;
        }

        let full = dir_entry.path();
        let Ok(rel) = full.strip_prefix(root) else {
            continue;
        };
        let rel_str = to_forward_slash(rel);
        if rel_str.is_empty() {
            continue;
        }

        if file_type.is_dir() {
            entries.push(WorkspaceEntry {
                path: rel_str,
                kind: "directory",
            });
        } else if file_type.is_file() {
            entries.push(WorkspaceEntry {
                path: rel_str,
                kind: "file",
            });
        }
    }

    entries.sort_by(compare_entries);

    let truncated = entries.len() > cap;
    if truncated {
        entries.truncate(cap);
    }

    Ok(WorkspaceDirResponse { entries, truncated })
}

/// Sort order: directories before files, then case-insensitive by name.
fn compare_entries(a: &WorkspaceEntry, b: &WorkspaceEntry) -> std::cmp::Ordering {
    if a.kind != b.kind {
        return if a.kind == "directory" {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        };
    }
    let a_name = basename(&a.path);
    let b_name = basename(&b.path);
    a_name
        .to_lowercase()
        .cmp(&b_name.to_lowercase())
        .then_with(|| a_name.cmp(b_name))
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn to_forward_slash(path: &Path) -> String {
    path.components()
        .filter_map(|c| match c {
            Component::Normal(n) => n.to_str().map(String::from),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn touch(root: &Path, rel: &str) {
        let full = root.join(rel);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full, "").unwrap();
    }

    fn list(root: &Path, rel: &str) -> WorkspaceDirResponse {
        let target = resolve_target(root, rel).unwrap();
        read_dir_sorted(root, &target, MAX_ENTRIES_PER_DIR).unwrap()
    }

    #[test]
    fn lists_only_immediate_children() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "a.txt");
        touch(tmp.path(), "src/main.rs");
        touch(tmp.path(), "src/lib/mod.rs");

        let resp = list(tmp.path(), "");
        let paths: Vec<&str> = resp.entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths, vec!["src", "a.txt"]);

        let resp = list(tmp.path(), "src");
        let paths: Vec<&str> = resp.entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths, vec!["src/lib", "src/main.rs"]);
    }

    #[test]
    fn sorts_directories_before_files_then_case_insensitive() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "z.ts");
        touch(tmp.path(), "B.ts");
        touch(tmp.path(), "a.ts");
        fs::create_dir(tmp.path().join("delta")).unwrap();
        fs::create_dir(tmp.path().join("Alpha")).unwrap();

        let resp = list(tmp.path(), "");
        let paths: Vec<&str> = resp.entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths, vec!["Alpha", "delta", "a.ts", "B.ts", "z.ts"]);
    }

    #[test]
    fn hides_dot_git_at_root_only() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), ".git/HEAD");
        touch(tmp.path(), "keep.txt");
        touch(tmp.path(), "deep/.git/keepme");

        let root = list(tmp.path(), "");
        assert!(root.entries.iter().all(|e| e.path != ".git"));
        assert!(root.entries.iter().any(|e| e.path == "keep.txt"));

        // Nested `.git/` is only excluded at the workspace root — if a
        // descendant carries one, we list it normally.
        let deep = list(tmp.path(), "deep");
        assert!(deep.entries.iter().any(|e| e.path == "deep/.git"));
    }

    #[test]
    fn empty_directory_returns_empty_entries() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir(tmp.path().join("empty")).unwrap();

        let resp = list(tmp.path(), "empty");
        assert!(resp.entries.is_empty());
        assert!(!resp.truncated);
    }

    #[test]
    fn truncation_flag_set_when_over_cap() {
        let tmp = TempDir::new().unwrap();
        for i in 0..20 {
            touch(tmp.path(), &format!("f{i:03}.txt"));
        }

        let target = resolve_target(tmp.path(), "").unwrap();
        let resp = read_dir_sorted(tmp.path(), &target, 10).unwrap();
        assert_eq!(resp.entries.len(), 10);
        assert!(resp.truncated);
        // Sort-then-truncate semantics: we keep the alphabetically
        // first slice, not whatever FS order handed back.
        let paths: Vec<&str> = resp.entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths.first(), Some(&"f000.txt"));
        assert_eq!(paths.last(), Some(&"f009.txt"));
    }

    #[test]
    fn rejects_parent_dir_escape() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "a.txt");

        let err = resolve_target(tmp.path(), "..").unwrap_err();
        assert!(err.contains("escapes workspace root"));
        let err = resolve_target(tmp.path(), "src/../..").unwrap_err();
        assert!(err.contains("escapes workspace root"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_absolute_path() {
        let tmp = TempDir::new().unwrap();

        let err = resolve_target(tmp.path(), "/etc").unwrap_err();
        assert!(err.contains("escapes workspace root"));
    }

    #[test]
    fn rejects_nonexistent_directory() {
        let tmp = TempDir::new().unwrap();

        let err = resolve_target(tmp.path(), "nope").unwrap_err();
        assert!(err.contains("does not exist"));
    }

    #[test]
    fn rejects_file_as_directory() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "a.txt");

        let err = resolve_target(tmp.path(), "a.txt").unwrap_err();
        assert!(err.contains("not a directory"));
    }

    #[test]
    fn rejects_nonexistent_root() {
        let tmp = TempDir::new().unwrap();
        let missing = tmp.path().join("does-not-exist");

        let err = resolve_target(&missing, "").unwrap_err();
        assert!(err.contains("does not exist"));
    }

    #[cfg(unix)]
    #[test]
    fn does_not_follow_symlinks_inside_listing() {
        use std::os::unix::fs::symlink;

        let tmp = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        fs::write(outside.path().join("secret.txt"), "x").unwrap();

        symlink(outside.path(), tmp.path().join("link")).unwrap();
        touch(tmp.path(), "inside.txt");

        let resp = list(tmp.path(), "");
        assert!(resp.entries.iter().all(|e| e.path != "link"));
        assert!(resp.entries.iter().any(|e| e.path == "inside.txt"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_listing_through_symlink() {
        use std::os::unix::fs::symlink;

        let tmp = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        fs::write(outside.path().join("secret.txt"), "x").unwrap();
        symlink(outside.path(), tmp.path().join("link")).unwrap();

        let err = resolve_target(tmp.path(), "link").unwrap_err();
        assert!(err.contains("symlink"));
    }

    #[test]
    fn uses_forward_slashes_on_all_platforms() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "a/b/c.txt");

        let deep = list(tmp.path(), "a/b");
        let paths: Vec<&str> = deep.entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths, vec!["a/b/c.txt"]);
        for p in paths {
            assert!(!p.contains('\\'));
        }
    }

    #[test]
    fn trailing_slashes_in_path_are_tolerated() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "src/main.rs");

        let a = list(tmp.path(), "src");
        let b = list(tmp.path(), "src/");
        assert_eq!(
            a.entries.iter().map(|e| e.path.as_str()).collect::<Vec<_>>(),
            b.entries.iter().map(|e| e.path.as_str()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn rejects_leading_slash_as_absolute() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "src/main.rs");

        let err = resolve_target(tmp.path(), "/src").unwrap_err();
        assert!(err.contains("escapes workspace root"));
    }
}
