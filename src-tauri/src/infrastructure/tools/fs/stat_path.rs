//! Inspect filesystem metadata for a workspace path.

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    error::ToolError,
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

/// Arguments for statting a workspace path.
#[derive(Debug, Deserialize)]
pub struct StatPathArgs {
    /// Relative or absolute path inside the workspace.
    pub path: String,
    /// Whether to follow symlinks when reading metadata.
    #[serde(default = "default_follow_symlinks")]
    pub follow_symlinks: bool,
}

fn default_follow_symlinks() -> bool {
    true
}

/// Platform-safe permission information.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathPermissions {
    /// Whether the path is readonly for the current process.
    pub readonly: bool,
    /// Unix mode bits when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unix_mode: Option<u32>,
}

/// Output from statting a path.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatPathOutput {
    /// Original path string provided to the tool.
    pub path: String,
    /// Path relative to the workspace root when possible.
    pub resolved_path: String,
    /// Whether the path exists.
    pub exists: bool,
    /// Path type: file, directory, symlink, or other.
    pub file_type: String,
    /// File size in bytes. Directories may report platform-specific values.
    pub size_bytes: u64,
    /// Whether the path itself is a symlink.
    pub is_symlink: bool,
    /// Permission information.
    pub permissions: PathPermissions,
    /// Last modification timestamp if available.
    pub modified_at: Option<String>,
    /// Last access timestamp if available.
    pub accessed_at: Option<String>,
    /// Creation timestamp if available.
    pub created_at: Option<String>,
}

/// Tool to inspect filesystem metadata without reading file content.
pub struct StatPathTool;

impl Tool for StatPathTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("fs", "stat_path"),
            description: "Inspect filesystem metadata for a path inside the workspace, including type, size, timestamps, and permissions.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path relative to workspace root, or absolute path within workspace"
                    },
                    "follow_symlinks": {
                        "type": "boolean",
                        "description": "Whether to follow symlinks when reading metadata (default: true)",
                        "default": true
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

            let args: StatPathArgs =
                crate::infrastructure::tools::error::parse_args("fs_stat_path", arguments)?;

            let absolute_path = resolve_workspace_path(&workspace, &args.path)?;
            let symlink_metadata = std::fs::symlink_metadata(&absolute_path).map_err(|e| {
                ToolError::failed(format!("Failed to inspect path '{}': {}", args.path, e))
            })?;
            let metadata = if args.follow_symlinks {
                std::fs::metadata(&absolute_path).map_err(|e| {
                    ToolError::failed(format!("Failed to inspect path '{}': {}", args.path, e))
                })?
            } else {
                symlink_metadata.clone()
            };

            let file_type = if symlink_metadata.file_type().is_symlink() {
                "symlink"
            } else if metadata.is_file() {
                "file"
            } else if metadata.is_dir() {
                "directory"
            } else {
                "other"
            }
            .to_string();

            let output = StatPathOutput {
                path: args.path,
                resolved_path: workspace_relative_path(&workspace, &absolute_path),
                exists: true,
                file_type,
                size_bytes: metadata.len(),
                is_symlink: symlink_metadata.file_type().is_symlink(),
                permissions: PathPermissions {
                    readonly: metadata.permissions().readonly(),
                    unix_mode: unix_mode(&metadata.permissions()),
                },
                modified_at: format_system_time(metadata.modified().ok()),
                accessed_at: format_system_time(metadata.accessed().ok()),
                created_at: format_system_time(metadata.created().ok()),
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}

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

    if !canonical.starts_with(workspace_root) {
        return Err(ToolError::path_traversal(&canonical, workspace_root));
    }

    Ok(canonical)
}

fn workspace_relative_path(workspace_root: &Path, absolute_path: &Path) -> String {
    absolute_path.strip_prefix(workspace_root).map_or_else(
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

fn format_system_time(time: Option<std::time::SystemTime>) -> Option<String> {
    time.map(|value| DateTime::<Utc>::from(value).to_rfc3339())
}

#[cfg(unix)]
fn unix_mode(permissions: &std::fs::Permissions) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;

    Some(permissions.mode())
}

#[cfg(not(unix))]
fn unix_mode(_permissions: &std::fs::Permissions) -> Option<u32> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "integration-test")]
    use tempfile::TempDir;

    #[test]
    fn test_tool_metadata() {
        let tool = StatPathTool;
        let meta = tool.meta();

        assert_eq!(meta.id.to_string(), "fs_stat_path");
        assert!(meta.description.contains("metadata"));
        assert!(meta.requires_workspace);
    }

    #[test]
    fn test_resolve_workspace_path_rejects_empty() {
        let workspace = Path::new("/tmp/workspace");
        let err = resolve_workspace_path(workspace, "").unwrap_err();
        assert!(err.to_string().contains("Path cannot be empty"));
    }

    #[test]
    fn test_format_system_time_returns_iso8601() {
        let formatted = format_system_time(Some(std::time::SystemTime::UNIX_EPOCH)).unwrap();
        assert!(formatted.starts_with("1970-01-01T00:00:00"));
    }

    #[test]
    fn test_workspace_relative_path_inside_workspace() {
        let workspace = Path::new("/tmp/workspace");
        let absolute = Path::new("/tmp/workspace/src/main.rs");
        assert_eq!(workspace_relative_path(workspace, absolute), "src/main.rs");
    }

    #[cfg(all(test, feature = "integration-test"))]
    mod integration_tests {
        use super::*;

        #[test]
        fn test_execute_returns_file_metadata() {
            let temp = TempDir::new().unwrap();
            let workspace = temp.path().to_path_buf();
            let file_path = workspace.join("sample.txt");
            std::fs::write(&file_path, "hello").unwrap();

            let tool = StatPathTool;
            let context = ToolExecutionContext::new("conversation");
            let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);
            let args = serde_json::json!({
                "path": "sample.txt"
            });

            let output = futures::executor::block_on(tool.execute(
                context,
                args,
                app_handle,
                Some(workspace),
            ))
            .unwrap();

            assert_eq!(output["fileType"], "file");
            assert_eq!(output["resolvedPath"], "sample.txt");
            assert_eq!(output["sizeBytes"], 5);
            assert_eq!(output["isSymlink"], false);
            assert_eq!(output["exists"], true);
        }

        #[test]
        fn test_execute_returns_directory_metadata() {
            let temp = TempDir::new().unwrap();
            let workspace = temp.path().to_path_buf();
            let dir_path = workspace.join("nested");
            std::fs::create_dir(&dir_path).unwrap();

            let tool = StatPathTool;
            let context = ToolExecutionContext::new("conversation");
            let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);
            let args = serde_json::json!({
                "path": "nested"
            });

            let output = futures::executor::block_on(tool.execute(
                context,
                args,
                app_handle,
                Some(workspace),
            ))
            .unwrap();

            assert_eq!(output["fileType"], "directory");
            assert_eq!(output["resolvedPath"], "nested");
        }

        #[cfg(unix)]
        #[test]
        fn test_execute_symlink_behavior_respects_follow_symlinks() {
            use std::os::unix::fs::symlink;

            let temp = TempDir::new().unwrap();
            let workspace = temp.path().to_path_buf();
            let file_path = workspace.join("target.txt");
            let link_path = workspace.join("link.txt");
            std::fs::write(&file_path, "hello").unwrap();
            symlink(&file_path, &link_path).unwrap();

            let tool = StatPathTool;
            let context = ToolExecutionContext::new("conversation");
            let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

            let followed = futures::executor::block_on(tool.execute(
                context.clone(),
                serde_json::json!({"path": "link.txt", "follow_symlinks": true}),
                app_handle.clone(),
                Some(workspace.clone()),
            ))
            .unwrap();
            assert_eq!(followed["fileType"], "symlink");
            assert_eq!(followed["sizeBytes"], 5);
            assert_eq!(followed["isSymlink"], true);

            let not_followed = futures::executor::block_on(tool.execute(
                context,
                serde_json::json!({"path": "link.txt", "follow_symlinks": false}),
                app_handle,
                Some(workspace),
            ))
            .unwrap();
            assert_eq!(not_followed["fileType"], "symlink");
            assert_eq!(not_followed["isSymlink"], true);
        }

        #[test]
        fn test_execute_missing_path_fails() {
            let temp = TempDir::new().unwrap();
            let tool = StatPathTool;
            let context = ToolExecutionContext::new("conversation");
            let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

            let result = futures::executor::block_on(tool.execute(
                context,
                serde_json::json!({"path": "missing.txt"}),
                app_handle,
                Some(temp.path().to_path_buf()),
            ));

            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Failed to inspect path"));
        }

        #[test]
        fn test_execute_path_traversal_fails() {
            let temp = TempDir::new().unwrap();
            let outside = TempDir::new().unwrap();
            let outside_file = outside.path().join("secret.txt");
            std::fs::write(&outside_file, "secret").unwrap();

            let tool = StatPathTool;
            let context = ToolExecutionContext::new("conversation");
            let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

            let result = futures::executor::block_on(tool.execute(
                context,
                serde_json::json!({"path": outside_file.to_string_lossy().to_string()}),
                app_handle,
                Some(temp.path().to_path_buf()),
            ));

            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Path traversal"));
        }
    }
}
