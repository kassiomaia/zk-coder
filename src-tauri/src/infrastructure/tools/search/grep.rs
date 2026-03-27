//! Local workspace grep-style search tool.

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    error::ToolError,
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

const MAX_RESULTS_HARD_LIMIT: usize = 200;
const SKIP_DIRS: [&str; 6] = [".git", "node_modules", "target", "dist", "build", "out"];

#[derive(Debug, Deserialize)]
pub struct GrepArgs {
    pub query: String,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub path: Option<String>,
}

fn default_max_results() -> usize {
    50
}

#[derive(Debug, Serialize)]
pub struct GrepMatch {
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct GrepOutput {
    pub query: String,
    pub count: usize,
    pub truncated: bool,
    pub matches: Vec<GrepMatch>,
}

pub struct GrepTool;

impl Tool for GrepTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("search", "grep"),
            description: "Search text in workspace files using regex. Returns matching file path, line, column, and line text.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Regex pattern to search for"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of matches to return (default: 50, max: 200)",
                        "minimum": 1,
                        "maximum": MAX_RESULTS_HARD_LIMIT
                    },
                    "case_sensitive": {
                        "type": "boolean",
                        "description": "Whether matching is case-sensitive (default: false)"
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional subpath inside workspace to limit search scope"
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

            let args: GrepArgs =
                crate::infrastructure::tools::error::parse_args("search_grep", arguments)?;

            let query = args.query.trim().to_string();
            if query.is_empty() {
                return Err(ToolError::invalid_args("Query cannot be empty"));
            }

            let max_results = args.max_results.clamp(1, MAX_RESULTS_HARD_LIMIT);
            let regex = RegexBuilder::new(&query)
                .case_insensitive(!args.case_sensitive)
                .build()
                .map_err(|e| ToolError::invalid_args(&format!("Invalid regex pattern: {}", e)))?;

            let root = resolve_search_root(&workspace, args.path.as_deref())?;
            let mut matches = Vec::new();
            collect_matches(&workspace, &root, &regex, max_results, &mut matches)?;

            let output = GrepOutput {
                query,
                count: matches.len(),
                truncated: matches.len() >= max_results,
                matches,
            };

            Ok(serde_json::to_value(output)?)
        })
    }
}

fn resolve_search_root(workspace: &Path, path: Option<&str>) -> Result<PathBuf, ToolError> {
    let Some(path) = path else {
        return Ok(workspace.to_path_buf());
    };

    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Ok(workspace.to_path_buf());
    }

    let candidate = if Path::new(trimmed).is_absolute() {
        PathBuf::from(trimmed)
    } else {
        workspace.join(trimmed)
    };

    let canonical = candidate.canonicalize().map_err(|e| {
        ToolError::failed(format!(
            "Failed to resolve search path '{}': {}",
            trimmed, e
        ))
    })?;
    if !canonical.starts_with(workspace) {
        return Err(ToolError::path_traversal(&canonical, workspace));
    }
    Ok(canonical)
}

fn collect_matches(
    workspace: &Path,
    root: &Path,
    regex: &regex::Regex,
    max_results: usize,
    out: &mut Vec<GrepMatch>,
) -> Result<(), ToolError> {
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        if out.len() >= max_results {
            break;
        }

        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            if out.len() >= max_results {
                break;
            }

            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if SKIP_DIRS.contains(&name) {
                        continue;
                    }
                }
                stack.push(path);
                continue;
            }

            let content = match std::fs::read_to_string(&path) {
                Ok(content) => content,
                Err(_) => continue,
            };

            for (line_idx, line) in content.lines().enumerate() {
                if out.len() >= max_results {
                    break;
                }
                if let Some(m) = regex.find(line) {
                    let rel_path = path.strip_prefix(workspace).map_or_else(
                        |_| path.to_string_lossy().to_string(),
                        |p| p.to_string_lossy().to_string(),
                    );
                    out.push(GrepMatch {
                        path: rel_path,
                        line: line_idx + 1,
                        column: m.start() + 1,
                        text: line.to_string(),
                    });
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_tool_metadata() {
        let tool = GrepTool;
        let meta = tool.meta();
        assert_eq!(meta.id.full_name(), "search_grep");
        assert!(meta.requires_workspace);
    }

    #[test]
    fn test_resolve_search_root_rejects_traversal() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path().canonicalize().unwrap();
        let result = resolve_search_root(&workspace, Some("/etc"));
        assert!(result.is_err());
    }

    #[test]
    fn test_collect_matches_basic() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path();
        let file = workspace.join("main.rs");
        fs::write(&file, "fn main() {\nprintln!(\"hello\");\n}\n").unwrap();
        let regex = RegexBuilder::new("println").build().unwrap();
        let mut out = Vec::new();
        collect_matches(workspace, workspace, &regex, 10, &mut out).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].line, 2);
    }
}
