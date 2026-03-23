//! File rollback system for agent operations.
//!
//! Tracks original file contents before agent writes, enabling
//! users to revert changes made during an agent session.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A snapshot of a file's state before modification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSnapshot {
    /// Absolute path to the file.
    pub path: PathBuf,
    /// Original content (None if file didn't exist before write).
    pub original_content: Option<String>,
    /// Timestamp when the snapshot was taken.
    pub created_at: DateTime<Utc>,
    /// Whether the file has been rolled back.
    pub rolled_back: bool,
}

/// Tracks file snapshots per session for rollback support.
pub struct RollbackStore {
    /// session_id -> (file_path -> snapshot)
    snapshots: Mutex<HashMap<String, HashMap<PathBuf, FileSnapshot>>>,
}

impl RollbackStore {
    /// Create a new rollback store.
    pub fn new() -> Self {
        Self {
            snapshots: Mutex::new(HashMap::new()),
        }
    }

    /// Capture the current state of a file before modification.
    ///
    /// Only captures the first snapshot per file per session (preserves
    /// the true original state across multiple writes to the same file).
    pub fn capture(&self, session_id: &str, path: &Path) {
        let mut store = self.snapshots.lock().unwrap_or_else(|e| e.into_inner());
        let session_map = store.entry(session_id.to_string()).or_default();

        // Only capture if we haven't already snapshot this file
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

    /// Roll back a single file to its original state.
    ///
    /// Returns Ok(true) if the file was reverted, Ok(false) if already rolled back.
    pub fn rollback_file(
        &self,
        session_id: &str,
        path: &Path,
    ) -> Result<bool, RollbackError> {
        let mut store = self.snapshots.lock().unwrap_or_else(|e| e.into_inner());
        let session_map = store
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
                    RollbackError::IoError(format!(
                        "Failed to restore '{}': {}",
                        path.display(),
                        e
                    ))
                })?;
            }
            None => {
                // File didn't exist before - remove it
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

    /// Roll back all files modified in a session.
    ///
    /// Returns the list of files that were reverted.
    pub fn rollback_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<PathBuf>, RollbackError> {
        let mut store = self.snapshots.lock().unwrap_or_else(|e| e.into_inner());
        let session_map = store
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

    /// Get all tracked file snapshots for a session.
    pub fn get_session_snapshots(&self, session_id: &str) -> Vec<FileSnapshot> {
        let store = self.snapshots.lock().unwrap_or_else(|e| e.into_inner());
        store
            .get(session_id)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Clear all snapshots for a session (e.g., after session ends).
    pub fn clear_session(&self, session_id: &str) {
        let mut store = self.snapshots.lock().unwrap_or_else(|e| e.into_inner());
        store.remove(session_id);
    }

    /// Get the original content from a snapshot for diff generation.
    ///
    /// Returns the original file content if a snapshot exists, None otherwise.
    pub fn get_snapshot_content(&self, session_id: &str, path: &str) -> Option<String> {
        let store = self.snapshots.lock().ok()?;
        let session_map = store.get(session_id)?;
        // Try to find by exact path match first
        session_map.get(Path::new(path))
            .or_else(|| {
                // Try as relative path - check all snapshots for matching filename
                session_map.values().find(|s| {
                    s.path.ends_with(path) || s.path.file_name().map_or(false, |n| n.to_string_lossy() == path)
                })
            })
            .and_then(|s| s.original_content.clone())
    }
}

impl Default for RollbackStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Rollback operation errors.
#[derive(Debug, thiserror::Error)]
pub enum RollbackError {
    /// Session has no tracked files.
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    /// File was not tracked by the rollback system.
    #[error("File not tracked: {0}")]
    FileNotTracked(String),
    /// I/O error during rollback.
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

        let store = RollbackStore::new();
        store.capture("sess-1", &file);

        // Simulate agent write
        fs::write(&file, "modified content").unwrap();

        // Rollback
        let result = store.rollback_file("sess-1", &file).unwrap();
        assert!(result);
        assert_eq!(fs::read_to_string(&file).unwrap(), "original content");
    }

    #[test]
    fn test_capture_and_rollback_new_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("new_file.rs");

        let store = RollbackStore::new();
        store.capture("sess-1", &file);

        // Simulate agent creating the file
        fs::write(&file, "new content").unwrap();
        assert!(file.exists());

        // Rollback should remove the file
        let result = store.rollback_file("sess-1", &file).unwrap();
        assert!(result);
        assert!(!file.exists());
    }

    #[test]
    fn test_double_capture_preserves_original() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.rs");
        fs::write(&file, "v1").unwrap();

        let store = RollbackStore::new();
        store.capture("sess-1", &file);

        // Modify and capture again
        fs::write(&file, "v2").unwrap();
        store.capture("sess-1", &file);

        // Modify again
        fs::write(&file, "v3").unwrap();

        // Should roll back to v1, not v2
        store.rollback_file("sess-1", &file).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "v1");
    }

    #[test]
    fn test_rollback_session() {
        let dir = TempDir::new().unwrap();
        let file_a = dir.path().join("a.rs");
        let file_b = dir.path().join("b.rs");
        fs::write(&file_a, "original a").unwrap();
        fs::write(&file_b, "original b").unwrap();

        let store = RollbackStore::new();
        store.capture("sess-1", &file_a);
        store.capture("sess-1", &file_b);

        fs::write(&file_a, "changed a").unwrap();
        fs::write(&file_b, "changed b").unwrap();

        let reverted = store.rollback_session("sess-1").unwrap();
        assert_eq!(reverted.len(), 2);
        assert_eq!(fs::read_to_string(&file_a).unwrap(), "original a");
        assert_eq!(fs::read_to_string(&file_b).unwrap(), "original b");
    }

    #[test]
    fn test_double_rollback_returns_false() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.rs");
        fs::write(&file, "original").unwrap();

        let store = RollbackStore::new();
        store.capture("sess-1", &file);
        fs::write(&file, "changed").unwrap();

        assert!(store.rollback_file("sess-1", &file).unwrap());
        assert!(!store.rollback_file("sess-1", &file).unwrap());
    }
}
