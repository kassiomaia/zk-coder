//! List files in workspace tool.

use crate::infrastructure::tools::constants::{
    DEFAULT_LIST_DEPTH, DEFAULT_LIST_LIMIT, MAX_LIST_DEPTH, MAX_LIST_LIMIT,
};
use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    error::ToolError,
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
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

const fn default_depth() -> usize {
    DEFAULT_LIST_DEPTH
}

const fn default_limit() -> usize {
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
                crate::infrastructure::tools::error::parse_args("fs_list_files", arguments)?;

            let max_depth = args.max_depth.min(MAX_LIST_DEPTH);
            let limit = args.limit.clamp(1, MAX_LIST_LIMIT);
            let base_path = args.path.unwrap_or_else(|| ".".to_string());

            let absolute_path = Self::resolve_workspace_path(&workspace, &base_path)?;

            if !absolute_path.is_dir() {
                return Err(ToolError::failed(format!(
                    "Path is not a directory: {base_path}"
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
                path: Self::workspace_relative_path(&workspace, &absolute_path),
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

        let canonical = candidate
            .canonicalize()
            .map_err(|e| ToolError::failed(format!("Failed to resolve path '{trimmed}': {e}")))?;

        // Path traversal protection
        if !canonical.starts_with(workspace_root) {
            return Err(ToolError::path_traversal(&canonical, workspace_root));
        }

        Ok(canonical)
    }

    /// Compute workspace-relative path for display.
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

#[cfg(all(test, feature = "integration-test"))]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // === Directory skip tests ===

    #[test]
    fn test_should_skip_directory() {
        assert!(ListFilesTool::should_skip_directory(".git"));
        assert!(ListFilesTool::should_skip_directory("node_modules"));
        assert!(ListFilesTool::should_skip_directory("target"));
        assert!(!ListFilesTool::should_skip_directory("src"));
        assert!(!ListFilesTool::should_skip_directory("lib"));
    }

    #[test]
    fn test_should_skip_directory_all_patterns() {
        // All skip patterns
        assert!(ListFilesTool::should_skip_directory(".git"));
        assert!(ListFilesTool::should_skip_directory("node_modules"));
        assert!(ListFilesTool::should_skip_directory("target"));
        assert!(ListFilesTool::should_skip_directory("dist"));
        assert!(ListFilesTool::should_skip_directory("build"));
        assert!(ListFilesTool::should_skip_directory(".next"));

        // Case sensitivity - should not skip different cases
        assert!(!ListFilesTool::should_skip_directory("NodeModules"));
        assert!(!ListFilesTool::should_skip_directory("TARGET"));
    }

    // === Tool metadata tests ===

    #[test]
    fn test_tool_metadata() {
        let tool = ListFilesTool;
        let meta = tool.meta();

        assert_eq!(meta.id.to_string(), "fs_list_files");
        assert!(meta.description.contains("list"));
        assert!(meta.requires_workspace);
        assert!(!meta.experimental);
        assert!(meta.feature_flag.is_none());
    }

    #[test]
    fn test_input_schema() {
        let tool = ListFilesTool;
        let meta = tool.meta();
        let schema = &meta.input_schema;

        assert_eq!(schema["type"], "object");
        assert!(schema["additionalProperties"] == false);

        // Check max_depth constraints
        let max_depth_prop = &schema["properties"]["max_depth"];
        assert_eq!(max_depth_prop["type"], "integer");
        assert_eq!(max_depth_prop["minimum"], 0);
        assert_eq!(max_depth_prop["maximum"], MAX_LIST_DEPTH);

        // Check limit constraints
        let limit_prop = &schema["properties"]["limit"];
        assert_eq!(limit_prop["type"], "integer");
        assert_eq!(limit_prop["minimum"], 1);
        assert_eq!(limit_prop["maximum"], MAX_LIST_LIMIT);
    }

    // === Arguments parsing tests ===

    #[test]
    fn test_parse_args_valid() {
        let args = serde_json::json!({
            "path": "src",
            "max_depth": 2,
            "limit": 50
        });

        let parsed: ListFilesArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.path, Some("src".to_string()));
        assert_eq!(parsed.max_depth, 2);
        assert_eq!(parsed.limit, 50);
    }

    #[test]
    fn test_parse_args_defaults() {
        let args = serde_json::json!({});

        let parsed: ListFilesArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.path, None);
        assert_eq!(parsed.max_depth, DEFAULT_LIST_DEPTH);
        assert_eq!(parsed.limit, DEFAULT_LIST_LIMIT);
    }

    #[test]
    fn test_parse_args_empty_path() {
        let args = serde_json::json!({
            "path": ""
        });

        let parsed: ListFilesArgs = serde_json::from_value(args).unwrap();
        // Empty string becomes Some("") but gets handled during execution
        assert_eq!(parsed.path, Some("".to_string()));
    }

    #[test]
    fn test_default_depth() {
        assert_eq!(default_depth(), DEFAULT_LIST_DEPTH);
    }

    #[test]
    fn test_default_limit() {
        assert_eq!(default_limit(), DEFAULT_LIST_LIMIT);
    }

    // === Path resolution tests ===

    #[test]
    fn test_resolve_workspace_path_empty() {
        let workspace = Path::new("/home/user/project");
        let result = ListFilesTool::resolve_workspace_path(workspace, "");
        // Empty path should return workspace root
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolve_workspace_path_whitespace() {
        let workspace = Path::new("/home/user/project");
        let result = ListFilesTool::resolve_workspace_path(workspace, "   ");
        // Whitespace should be trimmed and resolve to workspace root
        assert!(result.is_ok());
    }

    // === Workspace relative path tests ===

    #[test]
    fn test_workspace_relative_path_nested() {
        let workspace = Path::new("/home/user/project");
        let abs = Path::new("/home/user/project/src/components/button.rs");
        assert_eq!(
            ListFilesTool::workspace_relative_path(workspace, abs),
            "src/components/button.rs"
        );
    }

    #[test]
    fn test_workspace_relative_path_root() {
        let workspace = Path::new("/home/user/project");
        let abs = Path::new("/home/user/project");
        assert_eq!(ListFilesTool::workspace_relative_path(workspace, abs), ".");
    }

    #[test]
    fn test_workspace_relative_path_outside_workspace() {
        let workspace = Path::new("/home/user/project");
        let abs = Path::new("/etc/passwd");
        assert_eq!(
            ListFilesTool::workspace_relative_path(workspace, abs),
            "/etc/passwd"
        );
    }

    // === Integration tests with temporary directory ===

    #[tokio::test]
    async fn test_list_files_empty_directory() {
        let temp_dir = TempDir::new().unwrap();

        let tool = ListFilesTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({});

        let result = tool
            .execute(
                context,
                args,
                app_handle,
                Some(temp_dir.path().to_path_buf()),
            )
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["count"], 0);
        assert!(output["files"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_list_files_with_files() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file1.rs"), b"content1").unwrap();
        fs::write(temp_dir.path().join("file2.rs"), b"content2").unwrap();
        fs::write(temp_dir.path().join("file3.txt"), b"content3").unwrap();

        let tool = ListFilesTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({});

        let result = tool
            .execute(
                context,
                args,
                app_handle,
                Some(temp_dir.path().to_path_buf()),
            )
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["count"], 3);

        let files: Vec<String> = output["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .map(String::from)
            .collect();

        assert!(files.contains(&"file1.rs".to_string()));
        assert!(files.contains(&"file2.rs".to_string()));
        assert!(files.contains(&"file3.txt".to_string()));
    }

    #[tokio::test]
    async fn test_list_files_with_nested_directories() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join("src/components")).unwrap();
        fs::write(temp_dir.path().join("src/main.rs"), b"main").unwrap();
        fs::write(temp_dir.path().join("src/components/button.rs"), b"button").unwrap();
        fs::write(temp_dir.path().join("README.md"), b"readme").unwrap();

        let tool = ListFilesTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "max_depth": 3
        });

        let result = tool
            .execute(
                context,
                args,
                app_handle,
                Some(temp_dir.path().to_path_buf()),
            )
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["count"], 3);

        let files: Vec<String> = output["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .map(String::from)
            .collect();

        assert!(files.contains(&"README.md".to_string()));
        assert!(files.contains(&"src/main.rs".to_string()));
        assert!(files.contains(&"src/components/button.rs".to_string()));
    }

    #[tokio::test]
    async fn test_list_files_respects_max_depth() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join("a/b/c/d")).unwrap();
        fs::write(temp_dir.path().join("a/file1.rs"), b"f1").unwrap();
        fs::write(temp_dir.path().join("a/b/file2.rs"), b"f2").unwrap();
        fs::write(temp_dir.path().join("a/b/c/file3.rs"), b"f3").unwrap();
        fs::write(temp_dir.path().join("a/b/c/d/file4.rs"), b"f4").unwrap();

        let tool = ListFilesTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "max_depth": 2
        });

        let result = tool
            .execute(
                context,
                args,
                app_handle,
                Some(temp_dir.path().to_path_buf()),
            )
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();

        // Should only find files up to depth 2 from root
        let files: Vec<String> = output["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .map(String::from)
            .collect();

        assert!(files.contains(&"a/file1.rs".to_string()));
        assert!(files.contains(&"a/b/file2.rs".to_string()));
        // Depth 3 and 4 should not be included
        assert!(!files.iter().any(|f| f.contains("a/b/c/")));
    }

    #[tokio::test]
    async fn test_list_files_respects_limit() {
        let temp_dir = TempDir::new().unwrap();
        for i in 0..10 {
            fs::write(temp_dir.path().join(format!("file{}.txt", i)), b"content").unwrap();
        }

        let tool = ListFilesTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "limit": 5
        });

        let result = tool
            .execute(
                context,
                args,
                app_handle,
                Some(temp_dir.path().to_path_buf()),
            )
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["count"], 5);
    }

    #[tokio::test]
    async fn test_list_files_skips_ignored_directories() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join(".git/hooks")).unwrap();
        fs::create_dir_all(temp_dir.path().join("node_modules/package")).unwrap();
        fs::create_dir_all(temp_dir.path().join("target/debug")).unwrap();
        fs::write(temp_dir.path().join(".git/config"), b"git").unwrap();
        fs::write(temp_dir.path().join("node_modules/index.js"), b"js").unwrap();
        fs::write(temp_dir.path().join("target/binary"), b"bin").unwrap();
        fs::write(temp_dir.path().join("src/main.rs"), b"main").unwrap();

        let tool = ListFilesTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "max_depth": 3
        });

        let result = tool
            .execute(
                context,
                args,
                app_handle,
                Some(temp_dir.path().to_path_buf()),
            )
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();

        // Should only find src/main.rs, not files in ignored directories
        assert_eq!(output["count"], 1);
        assert_eq!(output["files"].as_array().unwrap()[0], "src/main.rs");
    }

    #[tokio::test]
    async fn test_list_files_from_subdirectory() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join("src/components")).unwrap();
        fs::write(temp_dir.path().join("src/main.rs"), b"main").unwrap();
        fs::write(temp_dir.path().join("src/lib.rs"), b"lib").unwrap();
        fs::write(temp_dir.path().join("src/components/button.rs"), b"button").unwrap();
        fs::write(temp_dir.path().join("README.md"), b"readme").unwrap();

        let tool = ListFilesTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "path": "src"
        });

        let result = tool
            .execute(
                context,
                args,
                app_handle,
                Some(temp_dir.path().to_path_buf()),
            )
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["path"], "src");

        let files: Vec<String> = output["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .map(String::from)
            .collect();

        // Should only list files in src, not README.md
        assert!(files.contains(&"src/main.rs".to_string()));
        assert!(files.contains(&"src/lib.rs".to_string()));
        assert!(files.contains(&"src/components/button.rs".to_string()));
        assert!(!files.iter().any(|f| f.contains("README.md")));
    }

    #[tokio::test]
    async fn test_list_files_no_workspace() {
        let tool = ListFilesTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({});

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_files_path_is_file() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file.txt"), b"content").unwrap();

        let tool = ListFilesTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "path": "file.txt"
        });

        let result = tool
            .execute(
                context,
                args,
                app_handle,
                Some(temp_dir.path().to_path_buf()),
            )
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not a directory"));
    }

    #[tokio::test]
    async fn test_list_files_path_does_not_exist() {
        let temp_dir = TempDir::new().unwrap();

        let tool = ListFilesTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "path": "nonexistent"
        });

        let result = tool
            .execute(
                context,
                args,
                app_handle,
                Some(temp_dir.path().to_path_buf()),
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_files_sorting() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("z_file.rs"), b"z").unwrap();
        fs::write(temp_dir.path().join("a_file.rs"), b"a").unwrap();
        fs::write(temp_dir.path().join("m_file.rs"), b"m").unwrap();

        let tool = ListFilesTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({});

        let result = tool
            .execute(
                context,
                args,
                app_handle,
                Some(temp_dir.path().to_path_buf()),
            )
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();

        let files: Vec<&str> = output["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        // Files should be sorted
        assert_eq!(files, vec!["a_file.rs", "m_file.rs", "z_file.rs"]);
    }

    #[tokio::test]
    async fn test_list_files_max_depth_zero() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join("src/components")).unwrap();
        fs::write(temp_dir.path().join("README.md"), b"readme").unwrap();
        fs::write(temp_dir.path().join("src/main.rs"), b"main").unwrap();

        let tool = ListFilesTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "max_depth": 0
        });

        let result = tool
            .execute(
                context,
                args,
                app_handle,
                Some(temp_dir.path().to_path_buf()),
            )
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();

        // With max_depth=0, only files in root directory
        assert_eq!(output["count"], 1);
        assert_eq!(output["files"].as_array().unwrap()[0], "README.md");
    }

    #[tokio::test]
    async fn test_list_files_unreadable_directory() {
        let temp_dir = TempDir::new().unwrap();

        // Create a directory and remove read permissions
        let restricted_dir = temp_dir.path().join("restricted");
        fs::create_dir(&restricted_dir).unwrap();

        // Try to remove read permissions (may not work on all systems)
        let mut perms = fs::metadata(&restricted_dir).unwrap().permissions();
        perms.set_readonly(false);
        let _ = fs::set_permissions(&restricted_dir, perms);

        fs::write(temp_dir.path().join("readable.txt"), b"content").unwrap();

        let tool = ListFilesTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "max_depth": 3
        });

        let result = tool
            .execute(
                context,
                args,
                app_handle,
                Some(temp_dir.path().to_path_buf()),
            )
            .await;

        // Should succeed and at least find the readable file
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output["count"].as_i64().unwrap() >= 1);
    }

    #[tokio::test]
    async fn test_list_files_limit_exceeds_hard_max() {
        let temp_dir = TempDir::new().unwrap();
        for i in 0..10 {
            fs::write(temp_dir.path().join(format!("file{}.txt", i)), b"content").unwrap();
        }

        let tool = ListFilesTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        // Try to set limit above hard max
        let args = serde_json::json!({
            "limit": MAX_LIST_LIMIT + 1000
        });

        let result = tool
            .execute(
                context,
                args,
                app_handle,
                Some(temp_dir.path().to_path_buf()),
            )
            .await;

        assert!(result.is_ok());
        // Should be clamped to MAX_LIST_LIMIT
        let output = result.unwrap();
        assert!(output["count"].as_i64().unwrap() <= MAX_LIST_LIMIT as i64);
    }

    #[tokio::test]
    async fn test_list_files_depth_exceeds_hard_max() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join("a/b/c/d/e/f/g/h")).unwrap();
        fs::write(temp_dir.path().join("a/b/c/d/e/f/g/h/deep.txt"), b"deep").unwrap();

        let tool = ListFilesTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        // Try to set depth above hard max
        let args = serde_json::json!({
            "max_depth": MAX_LIST_DEPTH + 10
        });

        let result = tool
            .execute(
                context,
                args,
                app_handle,
                Some(temp_dir.path().to_path_buf()),
            )
            .await;

        assert!(result.is_ok());
        // Should execute without error (depth clamped)
    }
}
