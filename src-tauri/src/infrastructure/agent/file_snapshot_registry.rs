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
    pub path: PathBuf,
    pub original_content: Option<String>,
    pub created_at: DateTime<Utc>,
    pub rolled_back: bool,
}

/// Tracks file snapshots per session for rollback support.
pub struct FileSnapshotRegistry {
    snapshots: Mutex<HashMap<String, HashMap<PathBuf, FileSnapshot>>>,
}

impl FileSnapshotRegistry {
    pub fn new() -> Self {
        Self {
            snapshots: Mutex::new(HashMap::new()),
        }
    }

    pub fn capture(&self, session_id: &str, path: &Path) {
        let mut snapshots = self.snapshots.lock().unwrap_or_else(|e| e.into_inner());
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

    pub fn rollback_file(&self, session_id: &str, path: &Path) -> Result<bool, RollbackError> {
        let mut snapshots = self.snapshots.lock().unwrap_or_else(|e| e.into_inner());
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

    pub fn rollback_session(&self, session_id: &str) -> Result<Vec<PathBuf>, RollbackError> {
        let mut snapshots = self.snapshots.lock().unwrap_or_else(|e| e.into_inner());
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

    pub fn get_session_snapshots(&self, session_id: &str) -> Vec<FileSnapshot> {
        let snapshots = self.snapshots.lock().unwrap_or_else(|e| e.into_inner());
        snapshots
            .get(session_id)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    pub fn clear_session(&self, session_id: &str) {
        let mut snapshots = self.snapshots.lock().unwrap_or_else(|e| e.into_inner());
        snapshots.remove(session_id);
    }

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

#[derive(Debug, thiserror::Error)]
pub enum RollbackError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("File not tracked: {0}")]
    FileNotTracked(String),
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
}
