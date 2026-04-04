//! Ranked fuzzy path search for workspace files.

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    error::ToolError,
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use ignore::WalkBuilder;
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Config, Matcher, Utf32Str,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

const MAX_LIMIT: usize = 100;

/// Arguments for fuzzy file search.
#[derive(Debug, Deserialize)]
pub struct FuzzyFindFilesArgs {
    /// Approximate query string.
    pub query: String,
    /// Base path to search from.
    #[serde(default)]
    pub base_path: Option<String>,
    /// Maximum results to return.
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Whether to respect ignore files.
    #[serde(default = "default_respect_ignore_files")]
    pub respect_ignore_files: bool,
    /// Whether to include directories in results.
    #[serde(default)]
    pub include_directories: bool,
}

fn default_limit() -> usize {
    20
}

fn default_respect_ignore_files() -> bool {
    true
}

/// A single fuzzy match result.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FuzzyMatch {
    /// Workspace-relative path.
    pub path: String,
    /// Relevance score (higher is better).
    pub score: u32,
}

/// Output from a fuzzy file search.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FuzzyFindFilesOutput {
    /// Original query.
    pub query: String,
    /// Number of results returned.
    pub count: usize,
    /// Ranked matches.
    pub matches: Vec<FuzzyMatch>,
}

/// Tool to find workspace files by approximate query.
pub struct FuzzyFindFilesTool;

impl Tool for FuzzyFindFilesTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("fs", "fuzzy_find_files"),
            description: "Search for workspace files using an approximate query when the exact path is not known. Returns ranked candidates.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Approximate filename or path fragment to search for"
                    },
                    "base_path": {
                        "type": "string",
                        "description": "Path to search from (default: workspace root)"
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 100,
                        "description": "Maximum results to return (default: 20)"
                    },
                    "respect_ignore_files": {
                        "type": "boolean",
                        "description": "Whether .gitignore and related ignore files are respected (default: true)",
                        "default": true
                    },
                    "include_directories": {
                        "type": "boolean",
                        "description": "Whether directory paths may be returned (default: false)"
                    }
                },
                "required": ["query"]
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

            let args: FuzzyFindFilesArgs =
                crate::infrastructure::tools::error::parse_args("fs_fuzzy_find_files", arguments)?;

            if args.query.trim().is_empty() {
                return Err(ToolError::invalid_args("Query cannot be empty"));
            }

            let limit = args.limit.min(MAX_LIMIT);
            let base_path = args.base_path.clone().unwrap_or_else(|| ".".to_string());
            let absolute_base = resolve_workspace_path(&workspace, &base_path)?;

            let output = fuzzy_search(&workspace, &absolute_base, &args, limit)?;
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
            let value = relative.to_string_lossy().replace('\\', "/");
            if value.is_empty() {
                ".".to_string()
            } else {
                value
            }
        },
    )
}

fn fuzzy_search(
    workspace_root: &Path,
    absolute_base: &Path,
    args: &FuzzyFindFilesArgs,
    limit: usize,
) -> Result<FuzzyFindFilesOutput, ToolError> {
    let mut builder = WalkBuilder::new(absolute_base);
    builder.hidden(false);
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

    // Collect candidate paths
    let mut candidates: Vec<String> = Vec::new();
    for entry in builder.build() {
        let Ok(entry) = entry else {
            continue;
        };
        let Some(file_type) = entry.file_type() else {
            continue;
        };
        let is_dir = file_type.is_dir();
        if !args.include_directories && is_dir {
            continue;
        }
        if !is_dir && !file_type.is_file() {
            continue;
        }
        let relative = workspace_relative_path(workspace_root, entry.path());
        if relative == "." {
            continue;
        }
        candidates.push(relative);
    }

    // Score candidates using nucleo-matcher
    let mut matcher = Matcher::new(Config::DEFAULT);
    let pattern = Pattern::parse(
        args.query.trim(),
        CaseMatching::Ignore,
        Normalization::Smart,
    );

    let mut scored: Vec<FuzzyMatch> = candidates
        .into_iter()
        .filter_map(|path| {
            let mut buf = Vec::new();
            let haystack = Utf32Str::new(&path, &mut buf);
            pattern.score(haystack, &mut matcher).map(|score| FuzzyMatch { path, score })
        })
        .collect();

    // Sort by score descending, then path ascending for stable ordering
    scored.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));
    scored.truncate(limit);

    let count = scored.len();
    Ok(FuzzyFindFilesOutput {
        query: args.query.clone(),
        count,
        matches: scored,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_tool_metadata() {
        let tool = FuzzyFindFilesTool;
        let meta = tool.meta();

        assert_eq!(meta.id.to_string(), "fs_fuzzy_find_files");
        assert!(meta.description.contains("approximate"));
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
    fn test_fuzzy_search_returns_exact_match_highest() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("src/stores")).unwrap();
        std::fs::write(temp.path().join("src/stores/agentStore.ts"), "x").unwrap();
        std::fs::write(temp.path().join("src/stores/sessionStore.ts"), "x").unwrap();

        let args = FuzzyFindFilesArgs {
            query: "agentStore".to_string(),
            base_path: None,
            limit: 20,
            respect_ignore_files: false,
            include_directories: false,
        };

        let output = fuzzy_search(temp.path(), temp.path(), &args, 20).unwrap();
        assert!(!output.matches.is_empty());
        assert!(output.matches[0].path.contains("agentStore"));
    }

    #[test]
    fn test_fuzzy_search_respects_limit() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("src")).unwrap();
        for idx in 0..10 {
            std::fs::write(temp.path().join(format!("src/file{idx}.rs")), "x").unwrap();
        }

        let args = FuzzyFindFilesArgs {
            query: "file".to_string(),
            base_path: None,
            limit: 3,
            respect_ignore_files: false,
            include_directories: false,
        };

        let output = fuzzy_search(temp.path(), temp.path(), &args, 3).unwrap();
        assert!(output.matches.len() <= 3);
        assert_eq!(output.count, output.matches.len());
    }

    #[test]
    fn test_fuzzy_search_ignores_gitignore_when_disabled() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("target")).unwrap();
        std::fs::write(temp.path().join(".gitignore"), "target/\n").unwrap();
        std::fs::write(temp.path().join("target/generated.rs"), "x").unwrap();

        let args = FuzzyFindFilesArgs {
            query: "generated".to_string(),
            base_path: None,
            limit: 20,
            respect_ignore_files: false,
            include_directories: false,
        };

        let output = fuzzy_search(temp.path(), temp.path(), &args, 20).unwrap();
        assert!(output.matches.iter().any(|m| m.path.contains("generated")));
    }

    #[test]
    fn test_fuzzy_search_respects_gitignore_when_enabled() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("target")).unwrap();
        std::fs::write(temp.path().join(".gitignore"), "target/\n").unwrap();
        std::fs::write(temp.path().join("target/generated.rs"), "x").unwrap();
        std::fs::write(temp.path().join("src_generated.rs"), "x").unwrap();

        let args = FuzzyFindFilesArgs {
            query: "generated".to_string(),
            base_path: None,
            limit: 20,
            respect_ignore_files: true,
            include_directories: false,
        };

        let output = fuzzy_search(temp.path(), temp.path(), &args, 20).unwrap();
        assert!(!output.matches.iter().any(|m| m.path.contains("target/")));
    }

    #[test]
    fn test_fuzzy_search_stable_ordering_for_equal_scores() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("b_match.rs"), "x").unwrap();
        std::fs::write(temp.path().join("a_match.rs"), "x").unwrap();

        let args = FuzzyFindFilesArgs {
            query: "match".to_string(),
            base_path: None,
            limit: 20,
            respect_ignore_files: false,
            include_directories: false,
        };

        let output = fuzzy_search(temp.path(), temp.path(), &args, 20).unwrap();
        let paths: Vec<&str> = output.matches.iter().map(|m| m.path.as_str()).collect();

        // Verify that equal-score paths are in alphabetical order
        for window in paths.windows(2) {
            if output.matches.iter().find(|m| m.path == window[0]).map(|m| m.score)
                == output.matches.iter().find(|m| m.path == window[1]).map(|m| m.score)
            {
                assert!(window[0] <= window[1]);
            }
        }
    }
}