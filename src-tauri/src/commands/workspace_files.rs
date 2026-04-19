//! Workspace file enumeration command for the UI.
//!
//! Powers the Files panel in `AgentPanel`. Walks the workspace root
//! recursively and returns a flat list of entries (files and
//! directories) with paths relative to the root, using forward-slash
//! separators. Does not follow symlinks and skips the `.git/` subtree
//! (binary plumbing with no display value). A hard cap protects the UI
//! from stalling on very large trees.

use serde::Serialize;
use std::collections::VecDeque;
use std::path::{Component, Path, PathBuf};

/// Hard cap on returned entries — protects the UI from stalling on
/// large trees.
///
/// The Files panel does not honor `.gitignore` by product decision, so
/// the walk can hit `node_modules` / `target` in real repos. Ten
/// thousand entries keeps Svelte rendering responsive while still
/// covering typical projects in full.
pub const MAX_ENTRIES: usize = 10_000;

/// One entry in the workspace listing.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct WorkspaceEntry {
    /// Path relative to the workspace root, using `/` separators.
    pub path: String,
    /// Kind of entry: `"file"` or `"directory"`.
    pub kind: &'static str,
}

/// Response for `list_workspace_files`.
#[derive(Debug, Serialize)]
pub struct WorkspaceFilesResponse {
    /// Flat list of all entries under the root (breadth-first order).
    pub entries: Vec<WorkspaceEntry>,
    /// `true` when the walk stopped after `MAX_ENTRIES` to protect the UI.
    pub truncated: bool,
}

/// Enumerate every file and directory under `root`.
///
/// See module-level docs for constraints: symlinks are not followed,
/// `.git/` is excluded, and results are capped at [`MAX_ENTRIES`].
///
/// # Errors
/// Returns an error when `root` does not exist or is not a directory.
#[tauri::command]
pub async fn list_workspace_files(root: String) -> Result<WorkspaceFilesResponse, String> {
    let root_path = PathBuf::from(&root);
    walk(&root_path, MAX_ENTRIES)
}

fn walk(root: &Path, cap: usize) -> Result<WorkspaceFilesResponse, String> {
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

    let mut entries: Vec<WorkspaceEntry> = Vec::new();
    // Breadth-first so shallow siblings get fully listed before we
    // descend. DFS would let one heavy subtree (e.g. `node_modules`)
    // eat the entire cap and leave sibling folders appearing empty.
    let mut queue: VecDeque<PathBuf> = VecDeque::from([root.to_path_buf()]);
    let mut truncated = false;

    while let Some(dir) = queue.pop_front() {
        // Skip unreadable directories (permissions etc.) rather than failing the whole walk.
        let Ok(read) = std::fs::read_dir(&dir) else {
            continue;
        };

        for dir_entry in read.flatten() {
            let path = dir_entry.path();
            let Ok(file_type) = dir_entry.file_type() else {
                continue;
            };

            // Do not traverse symlinks — avoids infinite loops and
            // escapes outside the workspace root.
            if file_type.is_symlink() {
                continue;
            }

            let Ok(rel) = path.strip_prefix(root) else {
                continue;
            };

            // Skip `.git/` (binary plumbing, not meaningful to display).
            if rel
                .components()
                .next()
                .and_then(|c| match c {
                    Component::Normal(n) => n.to_str(),
                    _ => None,
                })
                == Some(".git")
            {
                continue;
            }

            let rel_str = to_forward_slash(rel);
            if rel_str.is_empty() {
                continue;
            }

            if entries.len() >= cap {
                truncated = true;
                return Ok(WorkspaceFilesResponse {
                    entries,
                    truncated,
                });
            }

            if file_type.is_dir() {
                entries.push(WorkspaceEntry {
                    path: rel_str,
                    kind: "directory",
                });
                queue.push_back(path);
            } else if file_type.is_file() {
                entries.push(WorkspaceEntry {
                    path: rel_str,
                    kind: "file",
                });
            }
        }
    }

    Ok(WorkspaceFilesResponse {
        entries,
        truncated,
    })
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

    #[test]
    fn lists_all_files_and_dirs_relative_to_root() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "a.txt");
        touch(tmp.path(), "src/main.rs");
        touch(tmp.path(), "src/lib/mod.rs");

        let resp = walk(tmp.path(), MAX_ENTRIES).unwrap();

        let paths: Vec<&str> = resp.entries.iter().map(|e| e.path.as_str()).collect();
        assert!(paths.contains(&"a.txt"));
        assert!(paths.contains(&"src"));
        assert!(paths.contains(&"src/main.rs"));
        assert!(paths.contains(&"src/lib"));
        assert!(paths.contains(&"src/lib/mod.rs"));

        // Kinds are correct.
        let by_path = |p: &str| resp.entries.iter().find(|e| e.path == p).unwrap();
        assert_eq!(by_path("a.txt").kind, "file");
        assert_eq!(by_path("src").kind, "directory");
        assert_eq!(by_path("src/lib").kind, "directory");
        assert!(!resp.truncated);
    }

    #[test]
    fn excludes_dot_git_subtree() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), ".git/HEAD");
        touch(tmp.path(), ".git/objects/abc/def");
        touch(tmp.path(), "keep.txt");

        let resp = walk(tmp.path(), MAX_ENTRIES).unwrap();
        for e in &resp.entries {
            assert!(!e.path.starts_with(".git"), "unexpected entry: {}", e.path);
        }
        assert!(resp.entries.iter().any(|e| e.path == "keep.txt"));
    }

    #[test]
    fn handles_empty_directory() {
        let tmp = TempDir::new().unwrap();

        let resp = walk(tmp.path(), MAX_ENTRIES).unwrap();
        assert!(resp.entries.is_empty());
        assert!(!resp.truncated);
    }

    #[test]
    fn truncation_flag_set_when_over_cap() {
        let tmp = TempDir::new().unwrap();
        for i in 0..10 {
            touch(tmp.path(), &format!("f{i}.txt"));
        }

        let resp = walk(tmp.path(), 5).unwrap();
        assert_eq!(resp.entries.len(), 5);
        assert!(resp.truncated);
    }

    #[test]
    fn rejects_nonexistent_root() {
        let tmp = TempDir::new().unwrap();
        let missing = tmp.path().join("does-not-exist");

        let err = walk(&missing, MAX_ENTRIES).unwrap_err();
        assert!(err.contains("does not exist"));
    }

    #[test]
    fn rejects_non_directory_root() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("a.txt");
        fs::write(&file, "").unwrap();

        let err = walk(&file, MAX_ENTRIES).unwrap_err();
        assert!(err.contains("not a directory"));
    }

    #[cfg(unix)]
    #[test]
    fn does_not_follow_symlinks() {
        use std::os::unix::fs::symlink;

        let tmp = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        fs::write(outside.path().join("secret.txt"), "x").unwrap();

        // Symlink inside the workspace pointing outside it.
        symlink(outside.path(), tmp.path().join("link")).unwrap();
        touch(tmp.path(), "inside.txt");

        let resp = walk(tmp.path(), MAX_ENTRIES).unwrap();

        // `link` must not appear (or if it did, no child of it would —
        // we skip symlinks entirely, so it shouldn't appear at all).
        assert!(
            resp.entries.iter().all(|e| e.path != "link"
                && !e.path.starts_with("link/")
                && !e.path.contains("secret.txt")),
            "symlink traversal leaked: {:?}",
            resp.entries
        );
        assert!(resp.entries.iter().any(|e| e.path == "inside.txt"));
    }

    #[test]
    fn uses_forward_slashes_on_all_platforms() {
        let tmp = TempDir::new().unwrap();
        touch(tmp.path(), "a/b/c.txt");

        let resp = walk(tmp.path(), MAX_ENTRIES).unwrap();
        let paths: Vec<&str> = resp.entries.iter().map(|e| e.path.as_str()).collect();
        assert!(paths.contains(&"a/b/c.txt"));
        for p in paths {
            assert!(!p.contains('\\'), "expected forward slashes, got {p}");
        }
    }
}
