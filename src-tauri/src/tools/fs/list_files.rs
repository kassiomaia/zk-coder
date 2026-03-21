//! List files in workspace tool.

use crate::tools::constants::{DEFAULT_LIST_DEPTH, DEFAULT_LIST_LIMIT, MAX_LIST_DEPTH, MAX_LIST_LIMIT};
use crate::tools::{
    context::ToolExecutionContext, error::ToolError, traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

/// Arguments for listing workspace files.
#[derive(Debug, Deserialize)]
pub struct ListFilesArgs {
    /// Path to directory (default: workspace root).
    #[serde(default)]
    pub path: Option<String>,
    /// Maximum directory traversal depth (default: 3, max: 8).
    #[serde(default = "default_depth")]
    pub max_depth: usize,
    /// Maximum files to return (default: 200, max: 500).
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_depth() -> usize {
    DEFAULT_LIST_DEPTH
}

fn default_limit() -> usize {
    DEFAULT_LIST_LIMIT
}

/// Output from listing files.
#[derive(Debug, Serialize)]
pub struct ListFilesOutput {
    /// Workspace root path.
    pub root: String,
    /// Query path (relative to workspace root).
    pub path: String,
    /// Number of files returned.
    pub count: usize,
    /// List of file paths (relative to workspace root).
    pub files: Vec<String>,
}

/// Tool to list files under a workspace path with bounded recursion and limit.
pub struct ListFilesTool;

impl Tool for ListFilesTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("fs", "list_files"),
            description: "List files under a workspace path with bounded recursion and limit.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to directory (default: workspace root)"
                    },
                    "max_depth": {
                        "type": "integer",
                        "description": "Maximum directory traversal depth (default: 3, max: 8)",
                        "minimum": 0,
                        "maximum": MAX_LIST_DEPTH
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum files to return (default: 200, max: 500)",
                        "minimum": 1,
                        "maximum": MAX_LIST_LIMIT
                    }
                }
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

            let args: ListFilesArgs =
                crate::tools::error::parse_args("fs_list_files", arguments)?;

            let max_depth = args.max_depth.min(MAX_LIST_DEPTH);
            let limit = args.limit.clamp(1, MAX_LIST_LIMIT);
            let base_path = args.path.unwrap_or_else(|| ".".to_string());

            let absolute_path = Self::resolve_workspace_path(&workspace, &base_path)?;

            if !absolute_path.is_dir() {
                return Err(ToolError::failed(format!(
                    "Path is not a directory: {}",
                    base_path
                )));
            }

            let mut files = Vec::with_capacity(limit.min(128));
            Self::collect_workspace_files(
                &workspace,
                &absolute_path,
                0,
                max_depth,
                limit,
                &mut files,
            );
            files.sort();

            let output = ListFilesOutput {
                root: workspace.to_string_lossy().to_string(),
                path: ListFilesTool::workspace_relative_path(&workspace, &absolute_path),
                count: files.len(),
                files,
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}

impl ListFilesTool {
    /// Resolve a workspace path, ensuring it stays within workspace bounds.
    fn resolve_workspace_path(workspace_root: &Path, raw_path: &str) -> Result<PathBuf, ToolError> {
        let trimmed = raw_path.trim();
        if trimmed.is_empty() {
            return Ok(workspace_root.to_path_buf());
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

    /// Check if a directory should be skipped.
    fn should_skip_directory(name: &str) -> bool {
        matches!(
            name,
            ".git" | "node_modules" | "target" | "dist" | "build" | ".next"
        )
    }

    /// Recursively collect files from workspace directory.
    fn collect_workspace_files(
        workspace_root: &Path,
        current_path: &Path,
        depth: usize,
        max_depth: usize,
        limit: usize,
        out: &mut Vec<String>,
    ) {
        if out.len() >= limit || depth > max_depth {
            return;
        }

        let entries = match std::fs::read_dir(current_path) {
            Ok(entries) => entries,
            Err(err) => {
                log::warn!(
                    "[tools] Skipping unreadable directory '{}': {}",
                    current_path.display(),
                    err
                );
                return;
            }
        };

        for entry in entries {
            if out.len() >= limit {
                break;
            }
            let Ok(entry) = entry else {
                continue;
            };
            let Ok(file_type) = entry.file_type() else {
                continue;
            };

            let path = entry.path();
            if file_type.is_dir() {
                let directory_name = entry.file_name().to_string_lossy().to_string();
                if Self::should_skip_directory(&directory_name) {
                    continue;
                }
                Self::collect_workspace_files(
                    workspace_root,
                    &path,
                    depth + 1,
                    max_depth,
                    limit,
                    out,
                );
                continue;
            }
            if file_type.is_file() {
                out.push(Self::workspace_relative_path(workspace_root, &path));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_skip_directory() {
        assert!(ListFilesTool::should_skip_directory(".git"));
        assert!(ListFilesTool::should_skip_directory("node_modules"));
        assert!(ListFilesTool::should_skip_directory("target"));
        assert!(!ListFilesTool::should_skip_directory("src"));
        assert!(!ListFilesTool::should_skip_directory("lib"));
    }
}
