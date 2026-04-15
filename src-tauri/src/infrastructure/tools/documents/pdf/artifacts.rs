//! Artifact path resolution and one-time legacy migration for PDF
//! finalization.
//!
//! Before T1.7 finalized PDFs were written under `$HOME/.coder/artifacts/`
//! — a location outside Tauri's sandbox, unportable across platforms,
//! and invisible to any uninstall flow. T1.7 moves the canonical
//! artifact location under `AppHandle::path().app_local_data_dir()` and
//! migrates any previously written files exactly once.
//!
//! The two concerns are split so each can be tested without a live
//! Tauri runtime:
//!
//! * [`resolve_artifacts_dir`] requires an `AppHandle` and is exercised
//!   by integration tests.
//! * [`migrate_legacy_artifacts`] takes plain `&Path`s and is unit-tested
//!   here with `tempfile` fixtures.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::infrastructure::tools::error::ToolError;

/// Directory name (under `app_local_data_dir`) that holds finalized
/// artifacts.
const ARTIFACTS_DIR_NAME: &str = "artifacts";

/// Resolves the canonical artifacts directory under the app's local
/// data directory and creates it if missing.
///
/// # Errors
/// Returns [`ToolError::failed`] if Tauri cannot resolve
/// `app_local_data_dir()` or if directory creation fails.
pub fn resolve_artifacts_dir(app: &AppHandle) -> Result<PathBuf, ToolError> {
    let base = app
        .path()
        .app_local_data_dir()
        .map_err(|e| ToolError::failed(format!("resolve app_local_data_dir: {e}")))?;
    let dir = base.join(ARTIFACTS_DIR_NAME);
    std::fs::create_dir_all(&dir)
        .map_err(|e| ToolError::failed(format!("create artifacts dir: {e}")))?;
    Ok(dir)
}

/// Moves every regular file from `legacy` into `current`. Directories
/// inside `legacy` are skipped — callers should only have flat files
/// there from the pre-T1.7 layout.
///
/// Returns the number of files migrated. Safe to call when `legacy`
/// does not exist (returns `Ok(0)`). A second call after a successful
/// migration is a no-op because `legacy` is then empty (all files were
/// renamed away).
///
/// # Errors
/// Returns an [`std::io::Error`] if reading `legacy` or renaming a
/// file fails. Partial progress is kept — a failure mid-run leaves
/// already-moved files under `current`.
pub fn migrate_legacy_artifacts(legacy: &Path, current: &Path) -> std::io::Result<usize> {
    if !legacy.exists() {
        return Ok(0);
    }
    std::fs::create_dir_all(current)?;
    let mut moved = 0_usize;
    for entry in std::fs::read_dir(legacy)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let dest = current.join(entry.file_name());
            std::fs::rename(&path, &dest)?;
            moved += 1;
        }
    }
    Ok(moved)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn scratch() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn touch(dir: &Path, name: &str, body: &[u8]) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn migrate_moves_every_regular_file() {
        let td = scratch();
        let legacy = td.path().join("legacy");
        let current = td.path().join("current");
        touch(&legacy, "one.pdf", b"one");
        touch(&legacy, "two.pdf", b"two");

        let moved = migrate_legacy_artifacts(&legacy, &current).unwrap();
        assert_eq!(moved, 2);

        assert!(current.join("one.pdf").exists());
        assert!(current.join("two.pdf").exists());
        assert_eq!(std::fs::read(current.join("one.pdf")).unwrap(), b"one");
        assert!(!legacy.join("one.pdf").exists());
        assert!(!legacy.join("two.pdf").exists());
    }

    #[test]
    fn migrate_is_idempotent_on_second_run() {
        let td = scratch();
        let legacy = td.path().join("legacy");
        let current = td.path().join("current");
        touch(&legacy, "report.pdf", b"payload");

        let first = migrate_legacy_artifacts(&legacy, &current).unwrap();
        let second = migrate_legacy_artifacts(&legacy, &current).unwrap();
        assert_eq!(first, 1);
        assert_eq!(second, 0, "second run must be a noop");
        assert!(current.join("report.pdf").exists());
    }

    #[test]
    fn migrate_is_noop_when_legacy_missing() {
        let td = scratch();
        let legacy = td.path().join("never-existed");
        let current = td.path().join("current");
        let moved = migrate_legacy_artifacts(&legacy, &current).unwrap();
        assert_eq!(moved, 0);
        // `current` should not have been created either.
        assert!(!current.exists());
    }

    #[test]
    fn migrate_skips_subdirectories() {
        let td = scratch();
        let legacy = td.path().join("legacy");
        let current = td.path().join("current");
        touch(&legacy, "top.pdf", b"flat");
        std::fs::create_dir_all(legacy.join("nested")).unwrap();
        std::fs::write(legacy.join("nested").join("deep.pdf"), b"deep").unwrap();

        let moved = migrate_legacy_artifacts(&legacy, &current).unwrap();
        assert_eq!(moved, 1);
        assert!(current.join("top.pdf").exists());
        assert!(!current.join("deep.pdf").exists());
        assert!(legacy.join("nested").join("deep.pdf").exists());
    }
}
