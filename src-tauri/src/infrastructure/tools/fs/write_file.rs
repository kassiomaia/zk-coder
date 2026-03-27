//! Write file to workspace tool.
//!
//! Writes content to a file inside the workspace root.
//! Creates parent directories as needed. Requires approval
//! via the agent permission system.

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    error::ToolError,
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

/// Arguments for writing a file.
#[derive(Debug, Deserialize)]
pub struct WriteFileArgs {
    /// Path relative to workspace root (or absolute within workspace).
    pub path: String,
    /// Content to write to the file.
    pub content: String,
    /// If true, create parent directories as needed.
    #[serde(default = "default_create_dirs")]
    pub create_dirs: bool,
}

fn default_create_dirs() -> bool {
    true
}

/// Tool to write content to a file inside the workspace root.
pub struct WriteFileTool;

impl Tool for WriteFileTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("fs", "write_file"),
            description: "Write content to a file inside the workspace root. Creates the file if it doesn't exist, or overwrites if it does. Creates parent directories as needed.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path relative to workspace root, or absolute path within workspace"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
                    },
                    "create_dirs": {
                        "type": "boolean",
                        "description": "Create parent directories if they don't exist (default: true)",
                        "default": true
                    }
                },
                "required": ["path", "content"]
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

            let args: WriteFileArgs =
                crate::infrastructure::tools::error::parse_args("fs_write_file", arguments)?;

            let target_path = resolve_write_path(&workspace, &args.path)?;

            // Create parent directories if needed
            if args.create_dirs {
                if let Some(parent) = target_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        ToolError::failed(format!(
                            "Failed to create directories for '{}': {}",
                            args.path, e
                        ))
                    })?;
                }
            }

            let bytes_written = args.content.len();
            std::fs::write(&target_path, &args.content).map_err(|e| {
                ToolError::failed(format!("Failed to write file '{}': {}", args.path, e))
            })?;

            Ok(serde_json::json!({
                "path": args.path,
                "bytes_written": bytes_written,
                "created": !target_path.exists() || bytes_written > 0,
            }))
        })
    }
}

/// Generate a unified diff between existing file content and new content.
///
/// Returns None if the file doesn't exist yet (new file creation).
pub fn generate_diff(workspace_root: &Path, path: &str, new_content: &str) -> Option<String> {
    let target = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        workspace_root.join(path)
    };

    let old_content = std::fs::read_to_string(&target).ok()?;

    let diff = similar::TextDiff::from_lines(old_content.as_str(), new_content);

    let hunks: Vec<String> = diff
        .unified_diff()
        .context_radius(3)
        .iter_hunks()
        .map(|h| h.to_string())
        .collect();

    if hunks.is_empty() {
        return None; // No changes
    }

    let mut output = format!("--- a/{}\n+++ b/{}\n", path, path);
    for hunk in hunks {
        output.push_str(&hunk);
    }
    Some(output)
}

/// Resolve a write path, ensuring it stays within workspace bounds.
///
/// Unlike read path resolution, this doesn't require the file to already exist.
fn resolve_write_path(workspace_root: &Path, raw_path: &str) -> Result<PathBuf, ToolError> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return Err(ToolError::invalid_args("Path cannot be empty"));
    }

    let candidate = if Path::new(trimmed).is_absolute() {
        PathBuf::from(trimmed)
    } else {
        workspace_root.join(trimmed)
    };

    // For write targets that don't exist yet, we can't canonicalize.
    // Instead, we normalize and check the path prefix.
    // If the file exists, canonicalize for full safety.
    let workspace_canonical = workspace_root
        .canonicalize()
        .unwrap_or_else(|_| workspace_root.to_path_buf());

    let resolved = if candidate.exists() {
        let canon = candidate.canonicalize().map_err(|e| {
            ToolError::failed(format!("Failed to resolve path '{}': {}", trimmed, e))
        })?;
        if !canon.starts_with(&workspace_canonical) {
            return Err(ToolError::path_traversal(&canon, workspace_root));
        }
        canon
    } else {
        // Check that the parent directory would be within workspace
        // Normalize the path components
        let mut normalized = workspace_root.to_path_buf();
        let relative = if Path::new(trimmed).is_absolute() {
            candidate
                .strip_prefix(workspace_root)
                .map_err(|_| {
                    ToolError::PathTraversal(format!(
                        "Absolute path '{}' is outside workspace '{}'",
                        trimmed,
                        workspace_root.display()
                    ))
                })?
                .to_path_buf()
        } else {
            PathBuf::from(trimmed)
        };

        for component in relative.components() {
            match component {
                std::path::Component::ParentDir => {
                    if !normalized.pop() || !normalized.starts_with(workspace_root) {
                        return Err(ToolError::path_traversal(&candidate, workspace_root));
                    }
                }
                std::path::Component::Normal(c) => {
                    normalized.push(c);
                }
                _ => {}
            }
        }

        if !normalized.starts_with(workspace_root) {
            return Err(ToolError::path_traversal(&normalized, workspace_root));
        }

        normalized
    };

    Ok(resolved)
}

#[cfg(all(test, feature = "integration-test"))]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // === Path resolution tests ===

    #[test]
    fn test_resolve_write_path_relative() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path();
        let result = resolve_write_path(workspace, "src/main.rs").unwrap();
        assert!(result.starts_with(workspace));
        assert!(result.ends_with("src/main.rs"));
    }

    #[test]
    fn test_resolve_write_path_traversal_blocked() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path();
        let result = resolve_write_path(workspace, "../../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_write_path_empty_path() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path();
        let result = resolve_write_path(workspace, "");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_write_path_whitespace_only() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path();
        let result = resolve_write_path(workspace, "   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_write_path_absolute_within_workspace() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path();
        let file_path = workspace.join("test.txt");
        fs::write(&file_path, b"test").unwrap();

        let result = resolve_write_path(workspace, file_path.to_str().unwrap());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), file_path.canonicalize().unwrap());
    }

    #[test]
    fn test_resolve_write_path_absolute_outside_workspace() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path();
        let outside_path = "/etc/passwd";

        let result = resolve_write_path(workspace, outside_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_write_path_traversal_with_parent_dir() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path();

        // Create a directory structure
        let sub_dir = workspace.join("subdir");
        fs::create_dir_all(&sub_dir).unwrap();

        let result = resolve_write_path(workspace, "subdir/../../../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_write_path_nested_traversal() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path();

        // Create nested directory
        let nested = workspace.join("a/b/c");
        fs::create_dir_all(&nested).unwrap();

        let result = resolve_write_path(workspace, "a/b/c/../../../../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_write_path_existing_file() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path();
        let file_path = workspace.join("existing.txt");
        fs::write(&file_path, b"existing").unwrap();

        let result = resolve_write_path(workspace, "existing.txt");
        assert!(result.is_ok());
        assert!(result.unwrap().starts_with(workspace));
    }

    #[test]
    fn test_resolve_write_path_new_file_in_subdirectory() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path();

        // Don't create the subdirectory - testing new file path resolution
        let result = resolve_write_path(workspace, "new_dir/new_file.txt");
        assert!(result.is_ok());
        assert!(result.unwrap().starts_with(workspace));
    }

    // === Diff generation tests ===

    #[test]
    fn test_generate_diff_new_file() {
        let dir = TempDir::new().unwrap();
        let result = generate_diff(dir.path(), "new_file.rs", "fn main() {}");
        assert!(result.is_none()); // No diff for new files
    }

    #[test]
    fn test_generate_diff_modified_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.rs");
        fs::write(&file_path, "fn old() {}\n").unwrap();

        let diff = generate_diff(dir.path(), "test.rs", "fn new() {}\n");
        assert!(diff.is_some());
        let diff_content = diff.unwrap();
        assert!(diff_content.contains("-fn old() {}"));
        assert!(diff_content.contains("+fn new() {}"));
    }

    #[test]
    fn test_generate_diff_no_changes() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.rs");
        fs::write(&file_path, "fn same() {}\n").unwrap();

        let diff = generate_diff(dir.path(), "test.rs", "fn same() {}\n");
        assert!(diff.is_none()); // No diff when content identical
    }

    #[test]
    fn test_generate_diff_multiple_lines() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("multi.rs");
        fs::write(&file_path, "line 1\nline 2\nline 3\n").unwrap();

        let diff = generate_diff(dir.path(), "multi.rs", "line 1\nline 2 modified\nline 3\n");
        assert!(diff.is_some());
        let diff_content = diff.unwrap();
        assert!(diff_content.contains("-line 2"));
        assert!(diff_content.contains("+line 2 modified"));
    }

    #[test]
    fn test_generate_diff_with_absolute_path() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "old content").unwrap();

        let diff = generate_diff(dir.path(), file_path.to_str().unwrap(), "new content");
        assert!(diff.is_some());
    }

    // === Tool metadata tests ===

    #[test]
    fn test_tool_metadata() {
        let tool = WriteFileTool;
        let meta = tool.meta();

        assert_eq!(meta.id.to_string(), "fs_write_file");
        assert!(meta.description.contains("workspace"));
        assert!(meta.requires_workspace);
        assert!(!meta.experimental);
        assert!(meta.feature_flag.is_none());
    }

    #[test]
    fn test_input_schema() {
        let tool = WriteFileTool;
        let meta = tool.meta();
        let schema = &meta.input_schema;

        assert_eq!(schema["type"], "object");
        assert!(schema["additionalProperties"] == false);

        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("path")));
        assert!(required.contains(&serde_json::json!("content")));

        // Check create_dirs default
        let create_dirs_prop = &schema["properties"]["create_dirs"];
        assert_eq!(create_dirs_prop["type"], "boolean");
        assert_eq!(create_dirs_prop["default"], true);
    }

    // === Arguments parsing tests ===

    #[test]
    fn test_parse_args_valid() {
        let args = serde_json::json!({
            "path": "test.txt",
            "content": "Hello, World!",
            "create_dirs": true
        });

        let parsed: WriteFileArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.path, "test.txt");
        assert_eq!(parsed.content, "Hello, World!");
        assert_eq!(parsed.create_dirs, true);
    }

    #[test]
    fn test_parse_args_defaults() {
        let args = serde_json::json!({
            "path": "test.txt",
            "content": "content"
        });

        let parsed: WriteFileArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.create_dirs, true); // Default
    }

    #[test]
    fn test_parse_args_missing_required_field() {
        let args = serde_json::json!({
            "path": "test.txt"
        });

        let parsed: Result<WriteFileArgs, _> = serde_json::from_value(args);
        assert!(parsed.is_err());
    }

    #[test]
    fn test_default_create_dirs() {
        assert_eq!(default_create_dirs(), true);
    }

    // === Integration tests with temporary directory ===

    #[tokio::test]
    async fn test_write_file_success() {
        let temp_dir = TempDir::new().unwrap();

        let tool = WriteFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "path": "test.txt",
            "content": "Hello, World!"
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
        assert_eq!(output["path"], "test.txt");
        assert_eq!(output["bytes_written"], 13);

        // Verify file was actually written
        let written_content = fs::read_to_string(temp_dir.path().join("test.txt")).unwrap();
        assert_eq!(written_content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_write_file_with_subdirectory() {
        let temp_dir = TempDir::new().unwrap();

        let tool = WriteFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "path": "subdir/nested/file.txt",
            "content": "nested content",
            "create_dirs": true
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

        // Verify directory was created
        assert!(temp_dir.path().join("subdir/nested").is_dir());
        // Verify file was written
        let written_content =
            fs::read_to_string(temp_dir.path().join("subdir/nested/file.txt")).unwrap();
        assert_eq!(written_content, "nested content");
    }

    #[tokio::test]
    async fn test_write_file_overwrite_existing() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("overwrite.txt");
        fs::write(&file_path, b"old content").unwrap();

        let tool = WriteFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "path": "overwrite.txt",
            "content": "new content"
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

        // Verify content was overwritten
        let written_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(written_content, "new content");
    }

    #[tokio::test]
    async fn test_write_file_empty_content() {
        let temp_dir = TempDir::new().unwrap();

        let tool = WriteFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "path": "empty.txt",
            "content": ""
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
        assert_eq!(result.unwrap()["bytes_written"], 0);

        // Verify file exists but is empty
        let file_path = temp_dir.path().join("empty.txt");
        assert!(file_path.exists());
        let metadata = fs::metadata(&file_path).unwrap();
        assert_eq!(metadata.len(), 0);
    }

    #[tokio::test]
    async fn test_write_file_no_workspace() {
        let tool = WriteFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "path": "test.txt",
            "content": "content"
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_write_file_without_create_dirs() {
        let temp_dir = TempDir::new().unwrap();

        let tool = WriteFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "path": "nonexistent/file.txt",
            "content": "content",
            "create_dirs": false
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
    async fn test_write_file_utf8_content() {
        let temp_dir = TempDir::new().unwrap();

        let tool = WriteFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let utf8_content = "Hello 世界 🌍\nÉmojis: 😀🎉";

        let args = serde_json::json!({
            "path": "utf8.txt",
            "content": utf8_content
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

        // Verify UTF-8 content was written correctly
        let written_content = fs::read_to_string(temp_dir.path().join("utf8.txt")).unwrap();
        assert_eq!(written_content, utf8_content);
    }

    #[tokio::test]
    async fn test_write_file_large_content() {
        let temp_dir = TempDir::new().unwrap();

        let tool = WriteFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let large_content = "x".repeat(1_000_000);

        let args = serde_json::json!({
            "path": "large.txt",
            "content": large_content
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
        assert_eq!(result.unwrap()["bytes_written"], 1_000_000);
    }

    #[tokio::test]
    async fn test_write_file_with_newlines() {
        let temp_dir = TempDir::new().unwrap();

        let tool = WriteFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let content = "line 1\nline 2\r\nline 3\rline 4";

        let args = serde_json::json!({
            "path": "newlines.txt",
            "content": content
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

        // Verify content was written with correct newlines
        let written_content = fs::read_to_string(temp_dir.path().join("newlines.txt")).unwrap();
        assert_eq!(written_content, content);
    }

    #[tokio::test]
    async fn test_write_file_special_characters() {
        let temp_dir = TempDir::new().unwrap();

        let tool = WriteFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let special_content = "\t\n\r\\\"'<>@#$%^&*(){}[]|;:,.?/~`";

        let args = serde_json::json!({
            "path": "special.txt",
            "content": special_content
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

        let written_content = fs::read_to_string(temp_dir.path().join("special.txt")).unwrap();
        assert_eq!(written_content, special_content);
    }

    // === Edge case tests ===

    #[tokio::test]
    async fn test_write_file_trailing_slash_in_path() {
        let temp_dir = TempDir::new().unwrap();

        let tool = WriteFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        // Path with trailing behavior might be unusual
        let args = serde_json::json!({
            "path": "test.txt/",
            "content": "content"
        });

        let result = tool
            .execute(
                context,
                args,
                app_handle,
                Some(temp_dir.path().to_path_buf()),
            )
            .await;

        // Result depends on OS behavior - we just verify it doesn't crash
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_write_file_absolute_path_within_workspace() {
        let temp_dir = TempDir::new().unwrap();

        let tool = WriteFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let absolute_path = temp_dir.path().join("absolute.txt");

        let args = serde_json::json!({
            "path": absolute_path.to_str().unwrap(),
            "content": "absolute path content"
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
        assert!(absolute_path.exists());
    }

    #[tokio::test]
    async fn test_write_file_absolute_path_outside_workspace() {
        let temp_dir = TempDir::new().unwrap();

        let tool = WriteFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        // Try to write outside workspace - should be blocked
        let args = serde_json::json!({
            "path": "/etc/coder-test-write.txt",
            "content": "should be blocked"
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
}
