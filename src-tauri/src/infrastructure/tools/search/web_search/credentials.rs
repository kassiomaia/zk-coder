//! Resolves web-search credentials from the `tool_credentials` row into a
//! type-safe pair of `(api_key, cx)`.
//!
//! Both the "row absent" and "row present but malformed" cases collapse to
//! the same `WebSearchFailure::NotConfigured` — from the user's perspective
//! the fix is identical: open Settings and paste valid credentials. Keeping
//! the external error shape uniform also prevents a malformed row from
//! leaking its contents to the LLM.

use serde::{Deserialize, Serialize};

use crate::core::config::ToolCredentialsRepository;

use super::error::WebSearchFailure;
use super::secret::RedactedSecret;

/// Stable `tool_id` under which the web-search tool's credentials live.
pub const WEB_SEARCH_TOOL_ID: &str = "search_web_search";

/// JSON shape persisted inside the `credentials` blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredCredentials {
    /// Google API key.
    pub api_key: String,
    /// Google Programmable Search Engine id (`cx`).
    pub cx: String,
}

/// Credentials ready to be handed to `GoogleProvider`.
#[derive(Debug)]
pub struct ResolvedCredentials {
    /// API key, wrapped so `Debug` / `Display` never leak it.
    pub api_key: RedactedSecret,
    /// Search engine id. Not a secret; kept as a plain string so the
    /// Settings UI can display it.
    pub cx: String,
}

/// Reads and validates the stored blob. All failure cases collapse to
/// [`WebSearchFailure::NotConfigured`] so the tool surfaces a single
/// actionable error code (`not_configured`) regardless of root cause.
///
/// # Errors
///
/// Returns [`WebSearchFailure::NotConfigured`] when the row is missing, the
/// blob fails to deserialize as [`StoredCredentials`], or either the
/// `api_key` or `cx` is empty after trimming.
pub fn resolve(
    repo: &dyn ToolCredentialsRepository,
) -> Result<ResolvedCredentials, WebSearchFailure> {
    let Some(blob) = repo
        .get_tool_credentials(WEB_SEARCH_TOOL_ID)
        .map_err(|_| WebSearchFailure::NotConfigured)?
    else {
        return Err(WebSearchFailure::NotConfigured);
    };

    let parsed: StoredCredentials =
        serde_json::from_str(&blob).map_err(|_| WebSearchFailure::NotConfigured)?;

    let api_key = parsed.api_key.trim();
    let cx = parsed.cx.trim();
    if api_key.is_empty() || cx.is_empty() {
        return Err(WebSearchFailure::NotConfigured);
    }

    Ok(ResolvedCredentials {
        api_key: RedactedSecret::new(api_key.to_string()),
        cx: cx.to_string(),
    })
}

/// Serializes a `StoredCredentials` back to the JSON shape on disk.
/// Useful for the Tauri `set_tool_credentials` command (Phase 7) so the
/// payload the frontend sends matches the shape this resolver expects.
///
/// # Errors
///
/// Returns `serde_json::Error` if serialization fails — practically
/// impossible for this shape but preserved for type clarity.
pub fn serialize(creds: &StoredCredentials) -> Result<String, serde_json::Error> {
    serde_json::to_string(creds)
}
