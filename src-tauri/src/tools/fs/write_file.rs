//! Write file to workspace tool.
//!
//! Writes content to a file inside the workspace root.
//! Creates parent directories as needed. Requires approval
//! via the agent permission system.

use crate::tools::{
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
                crate::tools::error::parse_args("fs_write_file", arguments)?;

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
    let workspace_canonical = workspace_root.canonicalize().unwrap_or_else(|_| workspace_root.to_path_buf());

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

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
}
