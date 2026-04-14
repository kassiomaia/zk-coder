//! Shared constants for tool configuration.
//!
//! Centralizes limits, defaults, and environment variable names.

// =============================================================================
// File System Tool Limits
// =============================================================================

/// Default maximum bytes to read from a file.
pub const DEFAULT_READ_MAX_BYTES: usize = 16_000;

/// Maximum allowed bytes for file reads (hard limit).
pub const MAX_READ_MAX_BYTES: usize = 200_000;

/// Default maximum directory traversal depth.
pub const DEFAULT_LIST_DEPTH: usize = 3;

/// Maximum allowed directory traversal depth (hard limit).
pub const MAX_LIST_DEPTH: usize = 8;

/// Default maximum number of files to list.
pub const DEFAULT_LIST_LIMIT: usize = 200;

/// Maximum allowed files to list (hard limit).
pub const MAX_LIST_LIMIT: usize = 500;

// =============================================================================
// Conversation Tool Limits
// =============================================================================

/// Default maximum number of messages to search.
pub const DEFAULT_MESSAGE_SEARCH_LIMIT: usize = 20;

/// Maximum allowed messages to search (hard limit).
pub const MAX_MESSAGE_SEARCH_LIMIT: usize = 100;

// =============================================================================
// Web Search Tool Limits
// =============================================================================

/// Default number of search results to return.
pub const DEFAULT_WEB_SEARCH_LIMIT: usize = 5;

/// Maximum number of results the Google Custom Search API will return in one call.
pub const MAX_WEB_SEARCH_LIMIT: usize = 10;

/// Maximum length of the user-supplied query in characters (post-trim).
pub const MAX_WEB_SEARCH_QUERY_LEN: usize = 512;

// =============================================================================
// Orchestrator Constants
// =============================================================================

/// Maximum number of orchestration steps before stopping.
pub const ORCHESTRATOR_MAX_STEPS: usize = 6;

/// Maximum retries for LLM calls during orchestration.
pub const ORCHESTRATOR_LLM_MAX_RETRIES: usize = 2;

/// Base delay in milliseconds for LLM retry backoff.
pub const ORCHESTRATOR_LLM_RETRY_BASE_MS: u64 = 500;

// =============================================================================
// Environment Variable Names
// =============================================================================

/// Enable the orchestrated conversation runner.
pub const ENV_ORCHESTRATOR_ENABLED: &str = "CODER_LLM_ORCHESTRATOR";

/// Enable the tool registry for real tool execution.
pub const ENV_TOOL_REGISTRY_ENABLED: &str = "CODER_LLM_TOOL_REGISTRY";

/// Override workspace root for tool execution.
pub const ENV_WORKSPACE_ROOT: &str = "CODER_LLM_TOOL_WORKSPACE_ROOT";

/// Enable the Google scrape search tool (dev/test only).
pub const ENV_GOOGLE_SCRAPE_ENABLED: &str = "CODER_LLM_TOOL_GOOGLE_SCRAPE";

/// Enable the production web search tool (Google Custom Search JSON API).
pub const ENV_WEB_SEARCH_ENABLED: &str = "CODER_LLM_TOOL_WEB_SEARCH";

/// Developer override for the web-search API key; consulted by live integration
/// tests only. Production credentials live in `coder_config.db`.
pub const ENV_WEB_SEARCH_API_KEY: &str = "CODER_LLM_TOOL_WEB_SEARCH_API_KEY";

/// Developer override for the web-search Custom Search Engine id (`cx`); used
/// by live integration tests only. Production credentials live in `coder_config.db`.
pub const ENV_WEB_SEARCH_CX: &str = "CODER_LLM_TOOL_WEB_SEARCH_CX";

// =============================================================================
// Helper Functions
// =============================================================================

/// Parse a boolean environment variable flag.
///
/// Returns `true` if the env var is set to "1" or "true" (case-insensitive).
#[must_use]
pub fn parse_env_flag(value: Option<&str>) -> bool {
    match value {
        Some("1" | "true" | "TRUE" | "True") => true,
        _ => false,
    }
}

/// Check if an environment variable flag is enabled.
#[must_use]
pub fn is_env_enabled(var: &str) -> bool {
    parse_env_flag(std::env::var(var).ok().as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_env_flag() {
        assert!(parse_env_flag(Some("1")));
        assert!(parse_env_flag(Some("true")));
        assert!(parse_env_flag(Some("TRUE")));
        assert!(parse_env_flag(Some("True")));
        assert!(!parse_env_flag(Some("0")));
        assert!(!parse_env_flag(Some("false")));
        assert!(!parse_env_flag(Some("")));
        assert!(!parse_env_flag(None));
    }
}
