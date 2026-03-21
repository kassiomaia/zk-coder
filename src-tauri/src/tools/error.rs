//! Tool error types.
//!
//! Provides structured error handling for tool execution.

use thiserror::Error;

/// Errors that can occur during tool execution.
#[derive(Debug, Error)]
pub enum ToolError {
    /// Invalid or malformed arguments passed to the tool.
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    /// Tool execution failed.
    #[error("{0}")]
    ExecutionFailed(String),

    /// Workspace access required but not available.
    #[error("Workspace access required: {0}")]
    WorkspaceRequired(String),

    /// Path escapes workspace boundaries.
    #[error("Path traversal attempt blocked: {0}")]
    PathTraversal(String),

    /// Tool is disabled by feature flag.
    #[error("Tool disabled: {0}")]
    Disabled(String),

    /// IO error during tool execution.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// HTTP request error.
    #[error("HTTP error: {0}")]
    Http(String),

    /// Generic error with message.
    #[error("{0}")]
    Other(String),
}

impl ToolError {
    /// Create an invalid arguments error.
    #[must_use]
    pub fn invalid_args(msg: impl Into<String>) -> Self {
        Self::InvalidArguments(msg.into())
    }

    /// Create an execution failed error.
    #[must_use]
    pub fn failed(msg: impl Into<String>) -> Self {
        Self::ExecutionFailed(msg.into())
    }

    /// Create a path traversal error.
    #[must_use]
    pub fn path_traversal(path: &std::path::Path, workspace: &std::path::Path) -> Self {
        Self::PathTraversal(format!(
            "Path '{}' escapes workspace '{}'",
            path.display(),
            workspace.display()
        ))
    }

    /// Create an HTTP error from a reqwest error.
    #[must_use]
    pub fn from_reqwest(err: reqwest::Error) -> Self {
        Self::Http(err.to_string())
    }
}

/// Helper for parsing tool arguments with clear error messages.
///
/// # Example
///
/// ```ignore
/// #[derive(Deserialize)]
/// struct MyArgs {
///     path: String,
/// }
///
/// let args: MyArgs = parse_args("my_tool", json_value)?;
/// ```
pub fn parse_args<T>(tool_name: &str, args: serde_json::Value) -> Result<T, ToolError>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(args).map_err(|e| ToolError::InvalidArguments(format!(
        "{}: {}",
        tool_name, e
    )))
}
