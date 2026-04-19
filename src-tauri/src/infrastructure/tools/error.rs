//! Tool error types.
//!
//! Provides structured error handling for tool execution.

use serde_json::{json, Value as JsonValue};
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

    /// Structured failure. The tool produced a rich payload (e.g.
    /// `fs_replace_in_file` nearest-match hints) that the runtime must
    /// surface verbatim to the model instead of collapsing to the
    /// message string. `message` stays in the `error` field for
    /// backward-compatibility; `payload` overlays additional keys.
    #[error("{message}")]
    Structured {
        /// Short, model-facing error message. Also surfaced as `error`
        /// in the emitted JSON payload.
        message: String,
        /// Extra JSON fields merged on top of the default
        /// `{ "error": message }` object. Must serialize as a JSON
        /// object; non-object payloads are ignored by
        /// [`Self::into_tool_output`].
        payload: JsonValue,
    },
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

    /// Create a structured error carrying both a human-readable
    /// `message` and a JSON `payload` with additional fields the tool
    /// wants to expose to the LLM (e.g. fuzzy-match hints on a replace
    /// miss). See [`Self::Structured`] for the serialization shape.
    #[must_use]
    pub fn structured(message: impl Into<String>, payload: JsonValue) -> Self {
        Self::Structured {
            message: message.into(),
            payload,
        }
    }

    /// Converts the error into the JSON object the registry surfaces as
    /// the tool result output. For most variants this is the thin
    /// `{ "error": "<message>" }` shape preserved from before; for
    /// [`Self::Structured`], the payload object's fields are merged on
    /// top of `{ "error": message }` so callers see both the legacy
    /// `error` field and the richer hints.
    #[must_use]
    pub fn into_tool_output(self) -> JsonValue {
        match self {
            Self::Structured { message, payload } => {
                let mut base = json!({ "error": message });
                if let (Some(base_obj), Some(extra_obj)) =
                    (base.as_object_mut(), payload.as_object())
                {
                    for (key, value) in extra_obj {
                        if key == "error" {
                            continue;
                        }
                        base_obj.insert(key.clone(), value.clone());
                    }
                }
                base
            }
            other => json!({ "error": other.to_string() }),
        }
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
    serde_json::from_value(args)
        .map_err(|e| ToolError::InvalidArguments(format!("{tool_name}: {e}")))
}
