//! File rollback support for agent operations.
//!
//! Tracks original file contents before agent writes, enabling users to revert
//! changes made during an agent session.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A snapshot of a file's state before modification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSnapshot {
    /// Path to the file.
    pub path: PathBuf,
    /// Original file content (None if file didn't exist).
    pub original_content: Option<String>,
    /// When the snapshot was created.
    pub created_at: DateTime<Utc>,
    /// Whether the file has been rolled back.
    pub rolled_back: bool,
}

/// Tracks file snapshots per session for rollback support.
pub struct FileSnapshotRegistry {
    snapshots: Mutex<HashMap<String, HashMap<PathBuf, FileSnapshot>>>,
}

/// Best-effort snapshot stats for workspace capture.
#[derive(Debug, Clone, Copy)]
pub struct WorkspaceSnapshotStats {
    /// Number of files scanned.
    pub scanned_files: usize,
    /// Number of files captured.
    pub captured_files: usize,
    /// Number of files skipped due to size.
    pub skipped_large_files: usize,
    /// Whether the capture was truncated due to limits.
    pub truncated: bool,
}

impl FileSnapshotRegistry {
    /// Creates a new file snapshot registry.
    #[must_use] 
    pub fn new() -> Self {
        Self {
            snapshots: Mutex::new(HashMap::new()),
        }
    }

    /// Capture a snapshot of a file before modification.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session ID
    /// * `path` - Path to the file to capture
    pub fn capture(&self, session_id: &str, path: &Path) {
        let mut snapshots = self.snapshots.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let session_map = snapshots.entry(session_id.to_string()).or_default();
        if session_map.contains_key(path) {
            return;
        }

        let original_content = std::fs::read_to_string(path).ok();
        session_map.insert(
            path.to_path_buf(),
            FileSnapshot {
                path: path.to_path_buf(),
                original_content,
                created_at: Utc::now(),
                rolled_back: false,
            },
        );
    }

    /// Best-effort capture of workspace files before command execution.
    ///
    /// This is intentionally bounded to avoid large memory usage. It skips large
    /// files and stops after reaching the file or total byte limits.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session ID
    /// * `workspace_root` - Root directory of the workspace
    pub fn capture_workspace_best_effort(
        &self,
        session_id: &str,
        workspace_root: &Path,
    ) -> WorkspaceSnapshotStats {
        const MAX_FILES: usize = 2000;
        const MAX_TOTAL_BYTES: u64 = 5 * 1024 * 1024;
        const MAX_FILE_BYTES: u64 = 256 * 1024;
        const SKIP_DIRS: [&str; 6] = [".git", "node_modules", "target", "dist", "build", "out"];

        let mut stats = WorkspaceSnapshotStats {
            scanned_files: 0,
            captured_files: 0,
            skipped_large_files: 0,
            truncated: false,
        };
        let mut total_bytes: u64 = 0;

        let mut stack: Vec<PathBuf> = vec![workspace_root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let entries = match std::fs::read_dir(&dir) {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if SKIP_DIRS.contains(&name) {
                            continue;
                        }
                    }
                    stack.push(path);
                    continue;
                }

                stats.scanned_files += 1;
                if stats.captured_files >= MAX_FILES || total_bytes >= MAX_TOTAL_BYTES {
                    stats.truncated = true;
                    return stats;
                }

                let meta = match entry.metadata() {
                    Ok(meta) => meta,
                    Err(_) => continue,
                };
                if !meta.is_file() {
                    continue;
                }

                let size = meta.len();
                if size > MAX_FILE_BYTES {
                    stats.skipped_large_files += 1;
                    continue;
                }

                self.capture(session_id, &path);
                total_bytes = total_bytes.saturating_add(size);
                stats.captured_files += 1;
            }
        }

        stats
    }

    /// Rollback a single file to its original state.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session ID
    /// * `path` - Path to the file to rollback
    pub fn rollback_file(&self, session_id: &str, path: &Path) -> Result<bool, RollbackError> {
        let mut snapshots = self.snapshots.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let session_map = snapshots
            .get_mut(session_id)
            .ok_or_else(|| RollbackError::SessionNotFound(session_id.to_string()))?;

        let snapshot = session_map
            .get_mut(path)
            .ok_or_else(|| RollbackError::FileNotTracked(path.display().to_string()))?;

        if snapshot.rolled_back {
            return Ok(false);
        }

        match &snapshot.original_content {
            Some(content) => {
                std::fs::write(path, content).map_err(|e| {
                    RollbackError::IoError(format!("Failed to restore '{}': {}", path.display(), e))
                })?;
            }
            None => {
                if path.exists() {
                    std::fs::remove_file(path).map_err(|e| {
                        RollbackError::IoError(format!(
                            "Failed to remove '{}': {}",
                            path.display(),
                            e
                        ))
                    })?;
                }
            }
        }

        snapshot.rolled_back = true;
        Ok(true)
    }

    /// Rollback all files in a session to their original state.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session ID
    pub fn rollback_session(&self, session_id: &str) -> Result<Vec<PathBuf>, RollbackError> {
        let mut snapshots = self.snapshots.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let session_map = snapshots
            .get_mut(session_id)
            .ok_or_else(|| RollbackError::SessionNotFound(session_id.to_string()))?;

        let mut reverted = Vec::new();
        for (path, snapshot) in session_map.iter_mut() {
            if snapshot.rolled_back {
                continue;
            }

            let result = match &snapshot.original_content {
                Some(content) => std::fs::write(path, content),
                None => {
                    if path.exists() {
                        std::fs::remove_file(path)
                    } else {
                        Ok(())
                    }
                }
            };

            if result.is_ok() {
                snapshot.rolled_back = true;
                reverted.push(path.clone());
            }
        }

        Ok(reverted)
    }

    /// Get all snapshots for a session.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session ID
    pub fn get_session_snapshots(&self, session_id: &str) -> Vec<FileSnapshot> {
        let snapshots = self.snapshots.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        snapshots
            .get(session_id)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Clear all snapshots for a session.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session ID
    pub fn clear_session(&self, session_id: &str) {
        let mut snapshots = self.snapshots.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        snapshots.remove(session_id);
    }

    /// Get the original content of a snapshot.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session ID
    /// * `path` - Path to the file (exact match or filename)
    pub fn get_snapshot_content(&self, session_id: &str, path: &str) -> Option<String> {
        let snapshots = self.snapshots.lock().ok()?;
        let session_map = snapshots.get(session_id)?;
        session_map
            .get(Path::new(path))
            .or_else(|| {
                session_map.values().find(|snapshot| {
                    snapshot.path.ends_with(path)
                        || snapshot
                            .path
                            .file_name()
                            .is_some_and(|name| name.to_string_lossy() == path)
                })
            })
            .and_then(|snapshot| snapshot.original_content.clone())
    }
}

impl Default for FileSnapshotRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during rollback operations.
#[derive(Debug, thiserror::Error)]
pub enum RollbackError {
    /// Session was not found.
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    /// File is not being tracked.
    #[error("File not tracked: {0}")]
    FileNotTracked(String),
    /// I/O error occurred during rollback.
    #[error("I/O error: {0}")]
    IoError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_capture_and_rollback_existing_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.rs");
        fs::write(&file, "original content").unwrap();

        let registry = FileSnapshotRegistry::new();
        registry.capture("sess-1", &file);
        fs::write(&file, "modified content").unwrap();

        let result = registry.rollback_file("sess-1", &file).unwrap();
        assert!(result);
        assert_eq!(fs::read_to_string(&file).unwrap(), "original content");
    }

    #[test]
    fn test_capture_and_rollback_new_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("new_file.rs");

        let registry = FileSnapshotRegistry::new();
        registry.capture("sess-1", &file);

        fs::write(&file, "new content").unwrap();
        assert!(file.exists());

        let result = registry.rollback_file("sess-1", &file).unwrap();
        assert!(result);
        assert!(!file.exists());
    }

    #[test]
    fn test_capture_workspace_best_effort() {
        let dir = TempDir::new().unwrap();
        let file_a = dir.path().join("a.txt");
        let file_b = dir.path().join("b.txt");
        fs::write(&file_a, "a").unwrap();
        fs::write(&file_b, "b").unwrap();

        let registry = FileSnapshotRegistry::new();
        let stats = registry.capture_workspace_best_effort("sess-2", dir.path());

        assert!(stats.scanned_files >= 2);
        assert!(stats.captured_files >= 2);
    }
}
