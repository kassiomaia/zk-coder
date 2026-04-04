//! Replace exact text in an existing workspace file.

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    error::ToolError,
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

/// A single replacement operation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReplacementSpec {
    /// Existing text to match.
    pub old: String,
    /// Replacement text.
    pub new: String,
}

/// Arguments for replacing text in a file.
#[derive(Debug, Clone, Deserialize)]
pub struct ReplaceInFileArgs {
    /// Relative or absolute path inside the workspace.
    pub path: String,
    /// Replacement operations to apply.
    pub replacements: Vec<ReplacementSpec>,
    /// Optional total replacement-count guard.
    #[serde(default)]
    pub expected_replacements: Option<usize>,
    /// Whether multiple matches per replacement are allowed.
    #[serde(default)]
    pub allow_multiple_occurrences: bool,
}

/// Output from replacing text in a file.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceInFileOutput {
    /// Path of the updated file.
    pub path: String,
    /// Total replacements applied.
    pub applied_replacements: usize,
    /// Bytes written to disk.
    pub bytes_written: usize,
    /// Whether file content changed.
    pub changed: bool,
}

#[derive(Debug)]
struct ReplacementPlan {
    updated_content: String,
    applied_replacements: usize,
    changed: bool,
}

/// Tool to perform deterministic replacements in an existing file.
pub struct ReplaceInFileTool;

impl Tool for ReplaceInFileTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("fs", "replace_in_file"),
            description: "Replace exact text in an existing file inside the workspace without rewriting the full file body in the tool call.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path relative to workspace root, or absolute path within workspace"
                    },
                    "replacements": {
                        "type": "array",
                        "minItems": 1,
                        "items": {
                            "type": "object",
                            "additionalProperties": false,
                            "properties": {
                                "old": { "type": "string" },
                                "new": { "type": "string" }
                            },
                            "required": ["old", "new"]
                        }
                    },
                    "expected_replacements": {
                        "type": "integer",
                        "minimum": 0
                    },
                    "allow_multiple_occurrences": {
                        "type": "boolean",
                        "default": false
                    }
                },
                "required": ["path", "replacements"]
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
            let args: ReplaceInFileArgs =
                crate::infrastructure::tools::error::parse_args("fs_replace_in_file", arguments)?;
            let target_path = resolve_existing_workspace_file(&workspace, &args.path)?;
            let plan = build_replacement_plan(&target_path, &args)?;

            let bytes_written = if plan.changed {
                std::fs::write(&target_path, &plan.updated_content).map_err(|e| {
                    ToolError::failed(format!("Failed to write file '{}': {}", args.path, e))
                })?;
                plan.updated_content.len()
            } else {
                0
            };

            Ok(serde_json::to_value(ReplaceInFileOutput {
                path: args.path,
                applied_replacements: plan.applied_replacements,
                bytes_written,
                changed: plan.changed,
            })?)
        })
    }
}

/// Build a diff preview for a pending replacement request.
pub fn generate_preview_diff(
    workspace_root: &Path,
    arguments: &JsonValue,
) -> Result<Option<String>, ToolError> {
    let args: ReplaceInFileArgs =
        crate::infrastructure::tools::error::parse_args("fs_replace_in_file", arguments.clone())?;
    let target_path = resolve_existing_workspace_file(workspace_root, &args.path)?;
    let plan = build_replacement_plan(&target_path, &args)?;
    if !plan.changed {
        return Ok(None);
    }

    Ok(crate::infrastructure::tools::fs::write_file::generate_diff(
        workspace_root,
        &args.path,
        &plan.updated_content,
    ))
}

fn resolve_existing_workspace_file(
    workspace_root: &Path,
    raw_path: &str,
) -> Result<PathBuf, ToolError> {
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

    if !canonical.is_file() {
        return Err(ToolError::failed(format!("Path is not a file: {}", trimmed)));
    }

    Ok(canonical)
}

fn build_replacement_plan(
    target_path: &Path,
    args: &ReplaceInFileArgs,
) -> Result<ReplacementPlan, ToolError> {
    if args.replacements.is_empty() {
        return Err(ToolError::invalid_args(
            "At least one replacement must be provided",
        ));
    }

    let content = std::fs::read_to_string(target_path).map_err(|e| {
        ToolError::failed(format!(
            "Failed to read file '{}': {}",
            target_path.display(),
            e
        ))
    })?;

    let mut updated_content = content.clone();
    let mut applied_replacements = 0;

    for replacement in &args.replacements {
        if replacement.old.is_empty() {
            return Err(ToolError::invalid_args(
                "Replacement `old` value cannot be empty",
            ));
        }

        let match_count = updated_content.match_indices(&replacement.old).count();
        if match_count == 0 {
            return Err(ToolError::failed(format!(
                "Replacement text not found in '{}': {}",
                args.path, replacement.old
            )));
        }

        if match_count > 1 && !args.allow_multiple_occurrences {
            return Err(ToolError::failed(format!(
                "Replacement is ambiguous in '{}': multiple matches for '{}'",
                args.path, replacement.old
            )));
        }

        applied_replacements += match_count;
        updated_content = if args.allow_multiple_occurrences {
            updated_content.replace(&replacement.old, &replacement.new)
        } else {
            updated_content.replacen(&replacement.old, &replacement.new, 1)
        };
    }

    if let Some(expected) = args.expected_replacements {
        if applied_replacements != expected {
            return Err(ToolError::failed(format!(
                "Expected {} replacement(s), but applied {}",
                expected, applied_replacements
            )));
        }
    }

    Ok(ReplacementPlan {
        changed: updated_content != content,
        updated_content,
        applied_replacements,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_tool_metadata() {
        let tool = ReplaceInFileTool;
        let meta = tool.meta();

        assert_eq!(meta.id.to_string(), "fs_replace_in_file");
        assert!(meta.description.contains("Replace"));
        assert!(meta.requires_workspace);
    }

    #[test]
    fn test_resolve_existing_workspace_file_rejects_empty() {
        let workspace = Path::new("/tmp/workspace");
        let error = resolve_existing_workspace_file(workspace, "").unwrap_err();
        assert!(error.to_string().contains("Path cannot be empty"));
    }

    #[test]
    fn test_build_replacement_plan_applies_single_replacement() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("sample.txt");
        std::fs::write(&file_path, "hello world").unwrap();

        let args = ReplaceInFileArgs {
            path: "sample.txt".to_string(),
            replacements: vec![ReplacementSpec {
                old: "world".to_string(),
                new: "agent".to_string(),
            }],
            expected_replacements: Some(1),
            allow_multiple_occurrences: false,
        };

        let plan = build_replacement_plan(&file_path, &args).unwrap();
        assert_eq!(plan.updated_content, "hello agent");
        assert_eq!(plan.applied_replacements, 1);
        assert!(plan.changed);
    }

    #[test]
    fn test_build_replacement_plan_rejects_ambiguous_match_by_default() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("sample.txt");
        std::fs::write(&file_path, "foo foo").unwrap();

        let args = ReplaceInFileArgs {
            path: "sample.txt".to_string(),
            replacements: vec![ReplacementSpec {
                old: "foo".to_string(),
                new: "bar".to_string(),
            }],
            expected_replacements: None,
            allow_multiple_occurrences: false,
        };

        let error = build_replacement_plan(&file_path, &args).unwrap_err();
        assert!(error.to_string().contains("multiple matches"));
    }

    #[test]
    fn test_build_replacement_plan_allows_multiple_matches_when_enabled() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("sample.txt");
        std::fs::write(&file_path, "foo foo").unwrap();

        let args = ReplaceInFileArgs {
            path: "sample.txt".to_string(),
            replacements: vec![ReplacementSpec {
                old: "foo".to_string(),
                new: "bar".to_string(),
            }],
            expected_replacements: Some(2),
            allow_multiple_occurrences: true,
        };

        let plan = build_replacement_plan(&file_path, &args).unwrap();
        assert_eq!(plan.updated_content, "bar bar");
        assert_eq!(plan.applied_replacements, 2);
        assert!(plan.changed);
    }

    #[test]
    fn test_build_replacement_plan_returns_noop_for_identical_replacement() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("sample.txt");
        std::fs::write(&file_path, "same").unwrap();

        let args = ReplaceInFileArgs {
            path: "sample.txt".to_string(),
            replacements: vec![ReplacementSpec {
                old: "same".to_string(),
                new: "same".to_string(),
            }],
            expected_replacements: Some(1),
            allow_multiple_occurrences: false,
        };

        let plan = build_replacement_plan(&file_path, &args).unwrap();
        assert_eq!(plan.updated_content, "same");
        assert_eq!(plan.applied_replacements, 1);
        assert!(!plan.changed);
    }

    #[test]
    fn test_generate_preview_diff_returns_unified_diff() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("sample.txt"), "hello world\n").unwrap();

        let preview = generate_preview_diff(
            temp.path(),
            &serde_json::json!({
                "path": "sample.txt",
                "replacements": [
                    { "old": "world", "new": "agent" }
                ]
            }),
        )
        .unwrap()
        .unwrap();

        assert!(preview.contains("--- a/sample.txt"));
        assert!(preview.contains("+++ b/sample.txt"));
        assert!(preview.contains("-hello world"));
        assert!(preview.contains("+hello agent"));
    }
}
