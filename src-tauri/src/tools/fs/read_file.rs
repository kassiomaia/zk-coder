//! Read file from workspace tool.

use crate::tools::constants::{DEFAULT_READ_MAX_BYTES, MAX_READ_MAX_BYTES};
use crate::tools::{
    context::ToolExecutionContext, error::ToolError, traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

/// Arguments for reading a workspace file.
#[derive(Debug, Deserialize)]
pub struct ReadFileArgs {
    /// Relative or absolute path to the file.
    pub path: String,
    /// Maximum bytes to read (default: 16_000, max: 200_000).
    #[serde(default = "default_max_bytes")]
    pub max_bytes: usize,
}

fn default_max_bytes() -> usize {
    DEFAULT_READ_MAX_BYTES
}

/// Output from reading a file.
#[derive(Debug, Serialize)]
pub struct ReadFileOutput {
    /// Path relative to workspace root.
    pub path: String,
    /// Total file size in bytes.
    pub size_bytes: usize,
    /// Whether content was truncated due to size limit.
    pub truncated: bool,
    /// File content (UTF-8).
    pub content: String,
}

/// Tool to read UTF-8 content from a file inside the workspace root.
pub struct ReadFileTool;

impl Tool for ReadFileTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("fs", "read_file"),
            description: "Read UTF-8 content from a file inside the workspace root.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path relative to workspace root, or absolute path within workspace"
                    },
                    "max_bytes": {
                        "type": "integer",
                        "description": "Maximum bytes to read (default: 16000, max: 200000)",
                        "minimum": 1,
                        "maximum": MAX_READ_MAX_BYTES
                    }
                },
                "required": ["path"]
            }),
            requires_workspace: true,
            experimental: false,
            feature_flag: None,
        }
    }

    fn execute(
        &self,
        _context: ToolExecutionContext,
        arguments: JsonValue,
        _app_handle: AppHandle,
        workspace_root: Option<PathBuf>,
    ) -> ToolFuture {
        Box::pin(async move {
            let workspace = workspace_root.ok_or_else(|| {
                ToolError::WorkspaceRequired("No workspace configured".to_string())
            })?;

            let args: ReadFileArgs =
                crate::tools::error::parse_args("fs_read_file", arguments)?;

            let absolute_path = ReadFileTool::resolve_workspace_path(&workspace, &args.path)?;

            if !absolute_path.is_file() {
                return Err(ToolError::failed(format!(
                    "Path is not a file: {}",
                    args.path
                )));
            }

            let max_bytes = args.max_bytes.clamp(1, MAX_READ_MAX_BYTES);
            let bytes = std::fs::read(&absolute_path)?;
            let size_bytes = bytes.len();
            let truncated = size_bytes > max_bytes;
            let content_bytes = if truncated {
                &bytes[..max_bytes]
            } else {
                &bytes
            };
            let content = String::from_utf8_lossy(content_bytes).into_owned();

            let output = ReadFileOutput {
                path: args.path,
                size_bytes,
                truncated,
                content,
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}

impl ReadFileTool {
    /// Resolve a workspace path, ensuring it stays within workspace bounds.
    fn resolve_workspace_path(workspace_root: &Path, raw_path: &str) -> Result<PathBuf, ToolError> {
        let trimmed = raw_path.trim();
        if trimmed.is_empty() {
            return Err(ToolError::invalid_args("Path cannot be empty"));
        }

        let candidate = if Path::new(trimmed).is_absolute() {
            PathBuf::from(trimmed)
        } else {
            workspace_root.join(trimmed)
        };

        let canonical = candidate.canonicalize().map_err(|e| {
            ToolError::failed(format!("Failed to resolve path '{}': {}", trimmed, e))
        })?;

        // Path traversal protection
        if !canonical.starts_with(workspace_root) {
            return Err(ToolError::path_traversal(&canonical, workspace_root));
        }

        Ok(canonical)
    }

    /// Compute workspace-relative path for display.
    fn workspace_relative_path(workspace_root: &Path, absolute_path: &Path) -> String {
        absolute_path
            .strip_prefix(workspace_root)
            .map_or_else(
                |_| absolute_path.to_string_lossy().to_string(),
                |relative| {
                    let value = relative.to_string_lossy().to_string();
                    if value.is_empty() {
                        ".".to_string()
                    } else {
                        value
                    }
                },
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::context::ToolExecutionContext;

    #[test]
    fn test_workspace_relative_path() {
        let workspace = Path::new("/home/user/project");
        let abs = Path::new("/home/user/project/src/main.rs");
        assert_eq!(
            ReadFileTool::workspace_relative_path(workspace, abs),
            "src/main.rs"
        );
    }

    #[test]
    fn test_workspace_relative_path_root() {
        let workspace = Path::new("/home/user/project");
        let abs = Path::new("/home/user/project");
        assert_eq!(
            ReadFileTool::workspace_relative_path(workspace, abs),
            "."
        );
    }
}
