//! Match workspace files using glob patterns.

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    error::ToolError,
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

/// Arguments for globbing workspace files.
#[derive(Debug, Deserialize)]
pub struct GlobFilesArgs {
    /// Base path to search from.
    #[serde(default)]
    pub base_path: Option<String>,
    /// One or more include patterns.
    pub patterns: Vec<String>,
    /// Optional exclude patterns.
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
    /// Maximum traversal depth.
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    /// Maximum matches to return.
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Whether to include directories in results.
    #[serde(default)]
    pub include_directories: bool,
    /// Whether to respect ignore files.
    #[serde(default = "default_respect_ignore_files")]
    pub respect_ignore_files: bool,
}

const fn default_max_depth() -> usize {
    8
}

const fn default_limit() -> usize {
    200
}

const fn default_respect_ignore_files() -> bool {
    true
}

/// Output from globbing workspace files.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobFilesOutput {
    /// Base path resolved relative to the workspace root.
    pub base_path: String,
    /// Number of matches returned.
    pub count: usize,
    /// Whether the result set hit the configured limit.
    pub truncated: bool,
    /// Relative matched paths.
    pub matches: Vec<String>,
}

/// Tool to glob files inside the workspace.
pub struct GlobFilesTool;

impl Tool for GlobFilesTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("fs", "glob_files"),
            description: "Match files under a workspace path using one or more glob patterns.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "base_path": {
                        "type": "string",
                        "description": "Path to search from (default: workspace root)"
                    },
                    "patterns": {
                        "type": "array",
                        "items": { "type": "string" },
                        "minItems": 1,
                        "description": "One or more glob patterns to include"
                    },
                    "exclude_patterns": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Glob patterns to exclude from results"
                    },
                    "max_depth": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 32,
                        "description": "Maximum traversal depth (default: 8)"
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 1000,
                        "description": "Maximum matches to return (default: 200)"
                    },
                    "include_directories": {
                        "type": "boolean",
                        "description": "Whether directory paths may be returned"
                    },
                    "respect_ignore_files": {
                        "type": "boolean",
                        "description": "Whether .gitignore and related ignore files are respected (default: true)",
                        "default": true
                    }
                },
                "required": ["patterns"]
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

            let args: GlobFilesArgs =
                crate::infrastructure::tools::error::parse_args("fs_glob_files", arguments)?;
            let base_path = args.base_path.clone().unwrap_or_else(|| ".".to_string());
            let absolute_base = resolve_workspace_path(&workspace, &base_path)?;
            let output = glob_workspace_files(&workspace, &absolute_base, &args)?;
            Ok(serde_json::to_value(output)?)
        })
    }
}

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

    if !canonical.starts_with(workspace_root) {
        return Err(ToolError::path_traversal(&canonical, workspace_root));
    }

    Ok(canonical)
}

fn workspace_relative_path(workspace_root: &Path, absolute_path: &Path) -> String {
    absolute_path.strip_prefix(workspace_root).map_or_else(
        |_| absolute_path.to_string_lossy().to_string(),
        |relative| {
            let value = relative.to_string_lossy().replace('\\', "/");
            if value.is_empty() {
                ".".to_string()
            } else {
                value
            }
        },
    )
}

fn glob_workspace_files(
    workspace_root: &Path,
    absolute_base: &Path,
    args: &GlobFilesArgs,
) -> Result<GlobFilesOutput, ToolError> {
    if args.patterns.is_empty() {
        return Err(ToolError::invalid_args(
            "At least one glob pattern is required",
        ));
    }

    let include_set = compile_glob_set(&args.patterns)?;
    let exclude_set = if args.exclude_patterns.is_empty() {
        None
    } else {
        Some(compile_glob_set(&args.exclude_patterns)?)
    };

    let mut builder = WalkBuilder::new(absolute_base);
    builder.hidden(false);
    builder.max_depth(Some(args.max_depth));
    builder.parents(true);

    if args.respect_ignore_files {
        builder.standard_filters(true);
        builder.ignore(true);
        builder.git_ignore(true);
        builder.git_global(true);
        builder.git_exclude(true);
        builder.require_git(false);
    } else {
        builder.standard_filters(false);
        builder.ignore(false);
        builder.git_ignore(false);
        builder.git_global(false);
        builder.git_exclude(false);
        builder.require_git(false);
    }

    let mut matches = Vec::new();
    let mut truncated = false;

    for entry in builder.build() {
        let Ok(entry) = entry else {
            continue;
        };

        let Some(file_type) = entry.file_type() else {
            continue;
        };

        let is_directory = file_type.is_dir();
        if !args.include_directories && is_directory {
            continue;
        }
        if !is_directory && !file_type.is_file() {
            continue;
        }

        let relative = workspace_relative_path(workspace_root, entry.path());
        if relative == "." {
            continue;
        }

        if !include_set.is_match(&relative) {
            continue;
        }

        if exclude_set
            .as_ref()
            .is_some_and(|patterns| patterns.is_match(&relative))
        {
            continue;
        }

        if matches.len() >= args.limit {
            truncated = true;
            break;
        }

        matches.push(relative);
    }

    matches.sort();

    Ok(GlobFilesOutput {
        base_path: workspace_relative_path(workspace_root, absolute_base),
        count: matches.len(),
        truncated,
        matches,
    })
}

fn compile_glob_set(patterns: &[String]) -> Result<GlobSet, ToolError> {
    let mut builder = GlobSetBuilder::new();

    for pattern in patterns {
        let trimmed = pattern.trim();
        if trimmed.is_empty() {
            return Err(ToolError::invalid_args(
                "Glob patterns cannot contain empty strings",
            ));
        }

        let glob = Glob::new(trimmed).map_err(|e| {
            ToolError::invalid_args(format!("Invalid glob pattern '{trimmed}': {e}"))
        })?;
        builder.add(glob);
    }

    builder
        .build()
        .map_err(|e| ToolError::invalid_args(format!("Failed to compile glob patterns: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_tool_metadata() {
        let tool = GlobFilesTool;
        let meta = tool.meta();

        assert_eq!(meta.id.to_string(), "fs_glob_files");
        assert!(meta.description.contains("glob"));
        assert!(meta.requires_workspace);
    }

    #[test]
    fn test_resolve_workspace_path_empty_defaults_to_workspace() {
        let workspace = Path::new("/tmp/workspace");
        let resolved = resolve_workspace_path(workspace, "").unwrap();
        assert_eq!(resolved, workspace);
    }

    #[test]
    fn test_workspace_relative_path_normalizes_separators() {
        let workspace = Path::new("/tmp/workspace");
        let absolute = Path::new("/tmp/workspace/src/lib.rs");
        assert_eq!(workspace_relative_path(workspace, absolute), "src/lib.rs");
    }

    #[test]
    fn test_glob_workspace_files_matches_single_pattern() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        std::fs::write(temp.path().join("src/lib.rs"), "lib").unwrap();
        std::fs::write(temp.path().join("src/main.rs"), "main").unwrap();
        std::fs::write(temp.path().join("README.md"), "readme").unwrap();

        let args = GlobFilesArgs {
            base_path: Some("src".to_string()),
            patterns: vec!["**/*.rs".to_string()],
            exclude_patterns: Vec::new(),
            max_depth: 8,
            limit: 200,
            include_directories: false,
            respect_ignore_files: true,
        };

        let output = glob_workspace_files(temp.path(), &temp.path().join("src"), &args).unwrap();

        assert_eq!(output.base_path, "src");
        assert_eq!(output.matches, vec!["src/lib.rs", "src/main.rs"]);
        assert_eq!(output.count, 2);
        assert!(!output.truncated);
    }

    #[test]
    fn test_glob_workspace_files_applies_exclude_patterns() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        std::fs::write(temp.path().join("src/lib.rs"), "lib").unwrap();
        std::fs::write(temp.path().join("src/lib.test.rs"), "test").unwrap();

        let args = GlobFilesArgs {
            base_path: None,
            patterns: vec!["**/*.rs".to_string()],
            exclude_patterns: vec!["**/*.test.rs".to_string()],
            max_depth: 8,
            limit: 200,
            include_directories: false,
            respect_ignore_files: true,
        };

        let output = glob_workspace_files(temp.path(), temp.path(), &args).unwrap();
        assert_eq!(output.matches, vec!["src/lib.rs"]);
    }

    #[test]
    fn test_glob_workspace_files_respects_limit_and_marks_truncated() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        for index in 0..3 {
            std::fs::write(temp.path().join(format!("src/file{index}.rs")), "x").unwrap();
        }

        let args = GlobFilesArgs {
            base_path: None,
            patterns: vec!["**/*.rs".to_string()],
            exclude_patterns: Vec::new(),
            max_depth: 8,
            limit: 2,
            include_directories: false,
            respect_ignore_files: true,
        };

        let output = glob_workspace_files(temp.path(), temp.path(), &args).unwrap();
        assert_eq!(output.count, 2);
        assert_eq!(output.matches.len(), 2);
        assert!(output.truncated);
    }

    #[test]
    fn test_glob_workspace_files_respects_ignore_files() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        std::fs::create_dir_all(temp.path().join("target")).unwrap();
        std::fs::write(temp.path().join(".gitignore"), "target/\n").unwrap();
        std::fs::write(temp.path().join("src/main.rs"), "main").unwrap();
        std::fs::write(temp.path().join("target/generated.rs"), "generated").unwrap();

        let args = GlobFilesArgs {
            base_path: None,
            patterns: vec!["**/*.rs".to_string()],
            exclude_patterns: Vec::new(),
            max_depth: 8,
            limit: 200,
            include_directories: false,
            respect_ignore_files: true,
        };

        let output = glob_workspace_files(temp.path(), temp.path(), &args).unwrap();
        assert_eq!(output.matches, vec!["src/main.rs"]);
    }
}
