//! Tauri commands for tool-owned credentials (web search today; future tools
//! land next to this one).
//!
//! The backend deliberately keeps `tool_id` as a plain string discriminator
//! rather than a wire-level enum so adding a new tool's credentials only
//! requires adding a branch in [`set_tool_credentials_impl`] — no schema
//! changes, no new commands.

use serde::{Deserialize, Serialize};

use crate::commands::config::state::ConfigState;
use crate::core::config::{ToolCredentialsRepository, ValidationResult};
use crate::infrastructure::tools::search::web_search::credentials::{
    self, StoredCredentials, WEB_SEARCH_TOOL_ID,
};
use crate::infrastructure::tools::search::web_search::provider::google::GoogleProvider;
use crate::infrastructure::tools::search::web_search::provider::{
    SearchProvider, SearchProviderError, SearchRequest,
};

/// Wire shape for web-search credentials on `get` / `set`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WebSearchCredentialsPayload {
    pub api_key: String,
    pub cx: String,
}

/// Inner validation + persistence logic. Split out so unit tests can call it
/// against an in-memory repository without constructing a Tauri `State`.
pub(crate) fn set_tool_credentials_impl(
    repo: &dyn ToolCredentialsRepository,
    tool_id: &str,
    api_key: &str,
    cx: &str,
) -> Result<(), String> {
    if tool_id != WEB_SEARCH_TOOL_ID {
        return Err(format!("unsupported tool_id: {tool_id}"));
    }
    let api_key_t = api_key.trim();
    let cx_t = cx.trim();
    if api_key_t.is_empty() {
        return Err("api_key must not be empty".to_string());
    }
    if cx_t.is_empty() {
        return Err("cx must not be empty".to_string());
    }
    let blob = credentials::serialize(&StoredCredentials {
        api_key: api_key_t.to_string(),
        cx: cx_t.to_string(),
    })
    .map_err(|e| e.to_string())?;
    repo.set_tool_credentials(tool_id, &blob)
        .map_err(|e| e.to_string())
}

pub(crate) fn get_tool_credentials_impl(
    repo: &dyn ToolCredentialsRepository,
    tool_id: &str,
) -> Result<Option<WebSearchCredentialsPayload>, String> {
    if tool_id != WEB_SEARCH_TOOL_ID {
        return Err(format!("unsupported tool_id: {tool_id}"));
    }
    let Some(blob) = repo
        .get_tool_credentials(tool_id)
        .map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };
    let parsed: StoredCredentials = serde_json::from_str(&blob)
        .map_err(|e| format!("stored credentials are malformed: {e}"))?;
    Ok(Some(WebSearchCredentialsPayload {
        api_key: parsed.api_key,
        cx: parsed.cx,
    }))
}

pub(crate) fn delete_tool_credentials_impl(
    repo: &dyn ToolCredentialsRepository,
    tool_id: &str,
) -> Result<(), String> {
    if tool_id != WEB_SEARCH_TOOL_ID {
        return Err(format!("unsupported tool_id: {tool_id}"));
    }
    repo.delete_tool_credentials(tool_id)
        .map_err(|e| e.to_string())
}

pub(crate) fn has_tool_credentials_impl(
    repo: &dyn ToolCredentialsRepository,
    tool_id: &str,
) -> Result<bool, String> {
    if tool_id != WEB_SEARCH_TOOL_ID {
        return Err(format!("unsupported tool_id: {tool_id}"));
    }
    repo.has_tool_credentials(tool_id)
        .map_err(|e| e.to_string())
}

/// Upserts credentials for `tool_id`.
#[tauri::command]
pub async fn set_tool_credentials(
    tool_id: String,
    api_key: String,
    cx: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    set_tool_credentials_impl(&*state.repository, &tool_id, &api_key, &cx)
}

/// Returns stored credentials for `tool_id`, or `None` when absent.
#[tauri::command]
pub async fn get_tool_credentials(
    tool_id: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<Option<WebSearchCredentialsPayload>, String> {
    get_tool_credentials_impl(&*state.repository, &tool_id)
}

/// Removes stored credentials for `tool_id`.
#[tauri::command]
pub async fn delete_tool_credentials(
    tool_id: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    delete_tool_credentials_impl(&*state.repository, &tool_id)
}

/// Reflects whether credentials are persisted for `tool_id`.
#[tauri::command]
pub async fn has_tool_credentials(
    tool_id: String,
    state: tauri::State<'_, ConfigState>,
) -> Result<bool, String> {
    has_tool_credentials_impl(&*state.repository, &tool_id)
}

/// Runs a canned query against the currently-stored web-search credentials
/// and reports whether Google accepted them.
///
/// The IPC-level `Result` only fails on infrastructure errors; credential
/// rejection surfaces as `ValidationResult::failure(...)` with a
/// human-readable message so the Settings UI can render it inline.
#[tauri::command]
pub async fn test_web_search_credentials(
    state: tauri::State<'_, ConfigState>,
) -> Result<ValidationResult, String> {
    let resolved = match credentials::resolve(&*state.repository) {
        Ok(r) => r,
        Err(failure) => {
            return Ok(ValidationResult::failure(failure.human_message()));
        }
    };

    let provider = GoogleProvider::new(resolved.api_key, resolved.cx);
    let request = SearchRequest {
        query: "rust programming".to_string(),
        num_results: 1,
        language: None,
        site: None,
    };

    match provider.search(request).await {
        Ok(_) => Ok(ValidationResult::success(
            "Credentials verified against Google Custom Search.".to_string(),
        )),
        Err(SearchProviderError::Unauthorized) => Ok(ValidationResult::failure(
            "Google rejected the credentials. Double-check the API key and Engine ID.".to_string(),
        )),
        Err(SearchProviderError::RateLimited) => Ok(ValidationResult::failure(
            "Quota exhausted — credentials appear valid but the daily limit has been reached."
                .to_string(),
        )),
        Err(SearchProviderError::Upstream(msg)) => Ok(ValidationResult::failure(format!(
            "Provider error while validating: {msg}"
        ))),
        Err(SearchProviderError::Network(msg)) => Ok(ValidationResult::failure(format!(
            "Network error while validating: {msg}"
        ))),
    }
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
    fn set_tool_credentials_rejects_empty_api_key() {
        let r = repo();
        let err = set_tool_credentials_impl(r.as_ref(), WEB_SEARCH_TOOL_ID, "  ", "cx")
            .expect_err("empty api_key must fail");
        assert!(err.contains("api_key"));
    }

    #[test]
    fn set_tool_credentials_rejects_empty_cx() {
        let r = repo();
        let err = set_tool_credentials_impl(r.as_ref(), WEB_SEARCH_TOOL_ID, "sk-abc", "   ")
            .expect_err("empty cx must fail");
        assert!(err.contains("cx"));
    }

    #[test]
    fn set_tool_credentials_rejects_unknown_tool_id() {
        let r = repo();
        let err = set_tool_credentials_impl(r.as_ref(), "search_other_tool", "k", "c")
            .expect_err("unknown tool_id must fail");
        assert!(err.to_lowercase().contains("unsupported"));
    }

    #[test]
    fn set_trims_whitespace_before_persisting() {
        let r = repo();
        set_tool_credentials_impl(r.as_ref(), WEB_SEARCH_TOOL_ID, "  sk-abc ", "  cx-1 ").unwrap();
        let stored = get_tool_credentials_impl(r.as_ref(), WEB_SEARCH_TOOL_ID)
            .unwrap()
            .expect("stored");
        assert_eq!(stored.api_key, "sk-abc");
        assert_eq!(stored.cx, "cx-1");
    }

    #[test]
    fn get_tool_credentials_returns_none_when_absent() {
        let r = repo();
        let stored = get_tool_credentials_impl(r.as_ref(), WEB_SEARCH_TOOL_ID).unwrap();
        assert!(stored.is_none());
    }

    #[test]
    fn get_tool_credentials_returns_stored_value() {
        let r = repo();
        set_tool_credentials_impl(r.as_ref(), WEB_SEARCH_TOOL_ID, "sk-abc", "cx-1").unwrap();
        let got = get_tool_credentials_impl(r.as_ref(), WEB_SEARCH_TOOL_ID)
            .unwrap()
            .expect("stored");
        assert_eq!(got.api_key, "sk-abc");
        assert_eq!(got.cx, "cx-1");
    }

    #[test]
    fn delete_tool_credentials_removes_the_entry() {
        let r = repo();
        set_tool_credentials_impl(r.as_ref(), WEB_SEARCH_TOOL_ID, "sk-abc", "cx-1").unwrap();
        delete_tool_credentials_impl(r.as_ref(), WEB_SEARCH_TOOL_ID).unwrap();
        let got = get_tool_credentials_impl(r.as_ref(), WEB_SEARCH_TOOL_ID).unwrap();
        assert!(got.is_none());
    }

    #[test]
    fn has_tool_credentials_reflects_presence() {
        let r = repo();
        assert!(!has_tool_credentials_impl(r.as_ref(), WEB_SEARCH_TOOL_ID).unwrap());
        set_tool_credentials_impl(r.as_ref(), WEB_SEARCH_TOOL_ID, "sk", "cx").unwrap();
        assert!(has_tool_credentials_impl(r.as_ref(), WEB_SEARCH_TOOL_ID).unwrap());
    }

    #[tokio::test]
    async fn test_web_search_credentials_returns_failure_when_unconfigured() {
        // Without credentials in the DB, the command must surface a
        // validation failure (not a Tauri-level error).
        let r = repo();
        let result = (|| -> Result<ValidationResult, String> {
            match credentials::resolve(r.as_ref()) {
                Ok(_) => panic!("should be unconfigured"),
                Err(failure) => Ok(ValidationResult::failure(failure.human_message())),
            }
        })()
        .unwrap();
        assert!(!result.valid);
        assert!(result.message.to_lowercase().contains("not configured"));
    }
}
