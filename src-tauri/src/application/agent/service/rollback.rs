use std::path::Path;

use crate::core::agent::entities::AgentDomainError;
use crate::core::agent::repositories::RepositoryResult;
use crate::core::agent::services::{FileAction, FileChange};

use super::AgentServiceImpl;

impl AgentServiceImpl {
    pub(super) fn get_file_snapshots_impl(
        &self,
        session_id: &str,
    ) -> RepositoryResult<Vec<FileChange>> {
        Ok(self
            .file_snapshot_registry
            .get_session_snapshots(session_id)
            .into_iter()
            .map(|snapshot| FileChange {
                path: snapshot.path.display().to_string(),
                action: if snapshot.original_content.is_some() {
                    FileAction::Write
                } else {
                    FileAction::Read
                },
                diff: None,
            })
            .collect())
    }

    pub(super) fn rollback_file_impl(
        &self,
        session_id: &str,
        path: &str,
    ) -> RepositoryResult<bool> {
        self.file_snapshot_registry
            .rollback_file(session_id, Path::new(path))
            .map_err(|e| AgentDomainError::InvalidInput {
                reason: format!("Failed to rollback file: {}", e),
            })
    }

    pub(super) fn rollback_session_impl(&self, session_id: &str) -> RepositoryResult<Vec<String>> {
        let paths = self
            .file_snapshot_registry
            .rollback_session(session_id)
            .map_err(|e| AgentDomainError::InvalidInput {
                reason: format!("Failed to rollback session: {}", e),
            })?;

        Ok(paths
            .into_iter()
            .map(|path| path.display().to_string())
            .collect())
    }
}
