//! Tauri commands for the web-search per-workspace domain policy.

use serde::{Deserialize, Serialize};

use crate::commands::config::state::ConfigState;
use crate::core::config::WorkspaceSearchPolicyRepository;

/// Wire shape returned by `get_workspace_search_policy`. Always populated —
/// when no row exists, the command returns a policy with empty lists.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePolicyDto {
    /// List of allowed domains for web search.
    pub allowlist: Vec<String>,
    /// List of denied domains for web search.
    pub denylist: Vec<String>,
}

pub(crate) fn get_workspace_search_policy_impl(
    repo: &dyn WorkspaceSearchPolicyRepository,
    workspace_path: &str,
) -> Result<WorkspacePolicyDto, String> {
    let stored = repo
        .get_workspace_search_policy(workspace_path)
        .map_err(|e| e.to_string())?;
    Ok(stored
        .map(|s| WorkspacePolicyDto {
            allowlist: s.allowlist,
            denylist: s.denylist,
        })
        .unwrap_or_default())
}

/// Inner logic for setting workspace search policy. Returns an empty policy
/// with empty lists when no row exists in storage.
pub(crate) fn set_workspace_search_policy_impl(
    repo: &dyn WorkspaceSearchPolicyRepository,
    workspace_path: &str,
    allowlist: &[String],
    denylist: &[String],
) -> Result<(), String> {
    repo.set_workspace_search_policy(workspace_path, allowlist, denylist)
        .map_err(|e| e.to_string())
}

/// Inner logic for deleting workspace search policy.
pub(crate) fn delete_workspace_search_policy_impl(
    repo: &dyn WorkspaceSearchPolicyRepository,
    workspace_path: &str,
) -> Result<(), String> {
    repo.delete_workspace_search_policy(workspace_path)
        .map_err(|e| e.to_string())
}

/// Tauri command to retrieve the web-search policy for a workspace.
/// Returns a policy with empty lists when no policy is configured.
#[tauri::command]
pub async fn get_workspace_search_policy(
    workspace_path: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<WorkspacePolicyDto, String> {
    get_workspace_search_policy_impl(&*state.repository, &workspace_path)
}

/// Tauri command to update the web-search policy for a workspace.
#[tauri::command]
pub async fn set_workspace_search_policy(
    workspace_path: String,
    allowlist: Vec<String>,
    denylist: Vec<String>,
    state: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    set_workspace_search_policy_impl(&*state.repository, &workspace_path, &allowlist, &denylist)
}

/// Tauri command to delete the web-search policy for a workspace.
#[tauri::command]
pub async fn delete_workspace_search_policy(
    workspace_path: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    delete_workspace_search_policy_impl(&*state.repository, &workspace_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::sqlite::repositories::config::SqliteConfigRepository;
    use std::sync::Arc;

    fn repo() -> Arc<SqliteConfigRepository> {
        Arc::new(SqliteConfigRepository::in_memory().expect("in-memory"))
    }

    #[test]
    fn get_workspace_policy_returns_empty_for_unknown_path() {
        let r = repo();
        let policy = get_workspace_search_policy_impl(r.as_ref(), "/nothing").unwrap();
        assert!(policy.allowlist.is_empty());
        assert!(policy.denylist.is_empty());
    }

    #[test]
    fn set_persists_allowlist_and_denylist() {
        let r = repo();
        set_workspace_search_policy_impl(
            r.as_ref(),
            "/ws",
            &["github.com".to_string()],
            &["pinterest.com".to_string()],
        )
        .unwrap();
        let got = get_workspace_search_policy_impl(r.as_ref(), "/ws").unwrap();
        assert_eq!(got.allowlist, vec!["github.com"]);
        assert_eq!(got.denylist, vec!["pinterest.com"]);
    }

    #[test]
    fn delete_workspace_policy_removes_entry() {
        let r = repo();
        set_workspace_search_policy_impl(
            r.as_ref(),
            "/ws",
            &["github.com".to_string()],
            &[],
        )
        .unwrap();
        delete_workspace_search_policy_impl(r.as_ref(), "/ws").unwrap();
        let got = get_workspace_search_policy_impl(r.as_ref(), "/ws").unwrap();
        assert!(got.allowlist.is_empty());
        assert!(got.denylist.is_empty());
    }

    #[test]
    fn empty_string_workspace_path_is_global_bucket() {
        let r = repo();
        set_workspace_search_policy_impl(
            r.as_ref(),
            "",
            &[],
            &["bad.example".to_string()],
        )
        .unwrap();
        let got = get_workspace_search_policy_impl(r.as_ref(), "").unwrap();
        assert_eq!(got.denylist, vec!["bad.example"]);
    }
}
