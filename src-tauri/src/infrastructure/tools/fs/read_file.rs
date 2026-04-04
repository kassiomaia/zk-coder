//! Read file from workspace tool.

use crate::infrastructure::tools::constants::{DEFAULT_READ_MAX_BYTES, MAX_READ_MAX_BYTES};
use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    error::ToolError,
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
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
    /// Maximum bytes to read (default: `16_000`, max: `200_000`).
    #[serde(default = "default_max_bytes")]
    pub max_bytes: usize,
}

const fn default_max_bytes() -> usize {
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
                crate::infrastructure::tools::error::parse_args("fs_read_file", arguments)?;

            let absolute_path = Self::resolve_workspace_path(&workspace, &args.path)?;

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
            ToolError::failed(format!("Failed to resolve path '{trimmed}': {e}"))
        })?;

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
}

#[cfg(all(test, feature = "integration-test"))]
mod tests {
    use super::*;
    use crate::infrastructure::tools::context::ToolExecutionContext;
    use std::fs;
    use tempfile::TempDir;

    // === Workspace relative path tests ===

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
        assert_eq!(ReadFileTool::workspace_relative_path(workspace, abs), ".");
    }

    #[test]
    fn test_workspace_relative_path_nested() {
        let workspace = Path::new("/home/user/project");
        let abs = Path::new("/home/user/project/src/components/ui/button.rs");
        assert_eq!(
            ReadFileTool::workspace_relative_path(workspace, abs),
            "src/components/ui/button.rs"
        );
    }

    #[test]
    fn test_workspace_relative_path_outside_workspace() {
        let workspace = Path::new("/home/user/project");
        let abs = Path::new("/etc/passwd");
        // Should return absolute path when outside workspace
        assert_eq!(
            ReadFileTool::workspace_relative_path(workspace, abs),
            "/etc/passwd"
        );
    }

    // === Tool metadata tests ===

    #[test]
    fn test_tool_metadata() {
        let tool = ReadFileTool;
        let meta = tool.meta();

        assert_eq!(meta.id.to_string(), "fs_read_file");
        assert!(meta.description.contains("UTF-8"));
        assert!(meta.requires_workspace);
        assert!(!meta.experimental);
        assert!(meta.feature_flag.is_none());
    }

    #[test]
    fn test_input_schema() {
        let tool = ReadFileTool;
        let meta = tool.meta();
        let schema = &meta.input_schema;

        assert_eq!(schema["type"], "object");
        assert!(schema["additionalProperties"] == false);

        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("path")));

        // Check max_bytes constraints
        let max_bytes_prop = &schema["properties"]["max_bytes"];
        assert_eq!(max_bytes_prop["type"], "integer");
        assert_eq!(max_bytes_prop["minimum"], 1);
        assert_eq!(max_bytes_prop["maximum"], MAX_READ_MAX_BYTES);
    }

    // === Path resolution tests ===

    #[test]
    fn test_resolve_workspace_path_empty_path() {
        let workspace = Path::new("/home/user/project");
        let result = ReadFileTool::resolve_workspace_path(workspace, "");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_workspace_path_whitespace_only() {
        let workspace = Path::new("/home/user/project");
        let result = ReadFileTool::resolve_workspace_path(workspace, "   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_workspace_path_nonexistent() {
        let workspace = Path::new("/nonexistent/path");
        let result = ReadFileTool::resolve_workspace_path(workspace, "file.txt");
        assert!(result.is_err());
    }

    // === Arguments parsing tests ===

    #[test]
    fn test_parse_args_valid() {
        let args = serde_json::json!({
            "path": "src/main.rs",
            "max_bytes": 1000
        });

        let parsed: ReadFileArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.path, "src/main.rs");
        assert_eq!(parsed.max_bytes, 1000);
    }

    #[test]
    fn test_parse_args_defaults() {
        let args = serde_json::json!({
            "path": "src/main.rs"
        });

        let parsed: ReadFileArgs = serde_json::from_value(args).unwrap();
        assert_eq!(parsed.path, "src/main.rs");
        assert_eq!(parsed.max_bytes, DEFAULT_READ_MAX_BYTES);
    }

    #[test]
    fn test_parse_args_missing_path() {
        let args = serde_json::json!({
            "max_bytes": 1000
        });

        let parsed: Result<ReadFileArgs, _> = serde_json::from_value(args);
        assert!(parsed.is_err());
    }

    // === Integration tests with temporary directory ===

    #[tokio::test]
    async fn test_read_file_success() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, b"Hello, World!").unwrap();

        let tool = ReadFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "path": temp_dir.path().join("test.txt").to_str().unwrap(),
            "max_bytes": 1000
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
        assert_eq!(output["size_bytes"], 13);
        assert_eq!(output["truncated"], false);
        assert_eq!(output["content"], "Hello, World!");
    }

    #[tokio::test]
    async fn test_read_file_truncation() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("large.txt");
        let large_content = "x".repeat(50_000);
        fs::write(&file_path, &large_content).unwrap();

        let tool = ReadFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "path": temp_dir.path().join("large.txt").to_str().unwrap(),
            "max_bytes": 1000
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
        assert_eq!(output["size_bytes"], 50_000);
        assert_eq!(output["truncated"], true);
        assert!(output["content"].as_str().unwrap().len() == 1000);
    }

    #[tokio::test]
    async fn test_read_file_not_found() {
        let temp_dir = TempDir::new().unwrap();

        let tool = ReadFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "path": temp_dir.path().join("nonexistent.txt").to_str().unwrap()
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
    async fn test_read_file_no_workspace() {
        let tool = ReadFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "path": "test.txt"
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_file_path_is_directory() {
        let temp_dir = TempDir::new().unwrap();

        let tool = ReadFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "path": temp_dir.path().to_str().unwrap()
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
        assert!(err.to_string().contains("not a file"));
    }

    #[tokio::test]
    async fn test_read_file_max_bytes_clamping() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, b"test").unwrap();

        let tool = ReadFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        // Test that max_bytes larger than MAX_READ_MAX_BYTES is clamped
        let args = serde_json::json!({
            "path": temp_dir.path().join("test.txt").to_str().unwrap(),
            "max_bytes": MAX_READ_MAX_BYTES + 100_000
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
        // Should not be truncated since file is small
        assert_eq!(output["truncated"], false);
    }

    #[tokio::test]
    async fn test_read_file_relative_path() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("subdir/test.txt");
        fs::create_dir_all(temp_dir.path().join("subdir")).unwrap();
        fs::write(&file_path, b"relative path test").unwrap();

        let tool = ReadFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        // Use relative path
        let args = serde_json::json!({
            "path": "subdir/test.txt"
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
        assert_eq!(output["path"], "subdir/test.txt");
        assert_eq!(output["content"], "relative path test");
    }

    #[tokio::test]
    async fn test_read_file_utf8_content() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("utf8.txt");
        let utf8_content = "Hello 世界 🌍";
        fs::write(&file_path, utf8_content.as_bytes()).unwrap();

        let tool = ReadFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "path": temp_dir.path().join("utf8.txt").to_str().unwrap()
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
        assert_eq!(output["content"], utf8_content);
    }

    #[tokio::test]
    async fn test_read_file_binary_content() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("binary.bin");
        let binary_content: Vec<u8> = vec![0x00, 0xFF, 0xFE, 0xFD, 0x80, 0x81];
        fs::write(&file_path, &binary_content).unwrap();

        let tool = ReadFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "path": temp_dir.path().join("binary.bin").to_str().unwrap()
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
        // Binary content should be converted to UTF-8 with replacement characters
        let content = output["content"].as_str().unwrap();
        assert!(!content.is_empty());
    }

    // === Edge case tests ===

    #[tokio::test]
    async fn test_read_file_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty.txt");
        fs::write(&file_path, b"").unwrap();

        let tool = ReadFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "path": temp_dir.path().join("empty.txt").to_str().unwrap()
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
        assert_eq!(output["size_bytes"], 0);
        assert_eq!(output["content"], "");
        assert_eq!(output["truncated"], false);
    }

    #[tokio::test]
    async fn test_read_file_max_bytes_zero_is_clamped() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, b"content").unwrap();

        let tool = ReadFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "path": temp_dir.path().join("test.txt").to_str().unwrap(),
            "max_bytes": 0
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
        // max_bytes should be clamped to minimum of 1
        let output = result.unwrap();
        assert_eq!(output["content"].as_str().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_read_file_negative_max_bytes() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, b"content").unwrap();

        let tool = ReadFileTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        // serde_json parses numbers as signed, so -1 would be deserialized
        let args = serde_json::json!({
            "path": temp_dir.path().join("test.txt").to_str().unwrap(),
            "max_bytes": -1
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
        // Negative should be clamped to 1
        let output = result.unwrap();
        assert_eq!(output["content"].as_str().unwrap().len(), 1);
    }
}
