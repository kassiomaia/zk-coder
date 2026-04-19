//! Replace exact text in an existing workspace file.

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    error::ToolError,
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::path::{Path, PathBuf};
use tauri::AppHandle;

/// Maximum number of characters from the submitted `old` snippet we
/// echo back in the enriched error payload. Keeps the tool result from
/// ballooning when the model submits a large drifted snippet.
const SUBMITTED_OLD_PREVIEW_CHARS: usize = 200;

/// Threshold below which we suppress the `nearest_match` hint. The
/// fuzzy match is only useful when it plausibly reflects the model's
/// intent; emitting a `0.3` overlap would mislead more than it helps.
const NEAREST_MATCH_MIN_SIMILARITY: f64 = 0.5;

/// Context window (lines before + after the best-match region) echoed
/// back in the nearest-match payload. Keeps the hint terse.
const NEAREST_MATCH_CONTEXT_LINES: usize = 2;

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

    let canonical = candidate
        .canonicalize()
        .map_err(|e| ToolError::failed(format!("Failed to resolve path '{trimmed}': {e}")))?;

    if !canonical.starts_with(workspace_root) {
        return Err(ToolError::path_traversal(&canonical, workspace_root));
    }

    if !canonical.is_file() {
        return Err(ToolError::failed(format!("Path is not a file: {trimmed}")));
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
            return Err(build_not_found_error(
                &args.path,
                &replacement.old,
                &updated_content,
            ));
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
                "Expected {expected} replacement(s), but applied {applied_replacements}"
            )));
        }
    }

    Ok(ReplacementPlan {
        changed: updated_content != content,
        updated_content,
        applied_replacements,
    })
}

/// Describes the best fuzzy hit for a drifted `old` snippet. Emitted as
/// the `nearest_match` object on a miss when the similarity clears
/// [`NEAREST_MATCH_MIN_SIMILARITY`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NearestMatch {
    /// Line-level similarity ratio in `[0.0, 1.0]`. Always passes
    /// [`NEAREST_MATCH_MIN_SIMILARITY`] when this struct is surfaced.
    pub similarity: f64,
    /// 5-line (by default) context window centered on the best hit.
    pub context: String,
    /// 1-indexed line number of `context`'s first line.
    pub starting_line: usize,
}

/// Builds the enriched `fs_replace_in_file` "not found" error. Shape
/// is documented in Workstream γ.1; the `nearest_match` key is only
/// present when the fuzzy similarity exceeds
/// [`NEAREST_MATCH_MIN_SIMILARITY`] — below that, the hint would point
/// the model at an arbitrary stretch of the file and confuse more than
/// it clarifies.
pub(crate) fn build_not_found_error(
    path: &str,
    submitted_old: &str,
    file_content: &str,
) -> ToolError {
    let mut payload = json!({
        "submitted_old": truncate_for_preview(submitted_old, SUBMITTED_OLD_PREVIEW_CHARS),
        "hint": "File contents may have changed since you last read it. Call \
                 fs_read_file first and retry with the current content.",
    });

    if let Some(hit) = find_nearest_match(submitted_old, file_content) {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert(
                "nearest_match".to_string(),
                json!({
                    "similarity": round_similarity(hit.similarity),
                    "context": hit.context,
                    "starting_line": hit.starting_line,
                }),
            );
        }
    }

    ToolError::structured(
        format!("Replacement text not found in '{path}'"),
        payload,
    )
}

/// Truncates `text` to at most `max_chars` characters, appending an
/// ellipsis (`…`) when truncation actually occurs. Works on character
/// (not byte) boundaries so multibyte content (e.g. emoji in a code
/// comment) is never split mid-codepoint. When `text` fits within the
/// budget, returns it unchanged with no ellipsis.
fn truncate_for_preview(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max_chars).collect();
    out.push('…');
    out
}

/// Rounds similarity to two decimal places so the emitted payload is
/// stable across platforms and regressions on formatting do not
/// generate diff noise. Out-of-range values are clamped to `[0.0, 1.0]`.
fn round_similarity(value: f64) -> f64 {
    let clamped = value.clamp(0.0, 1.0);
    (clamped * 100.0).round() / 100.0
}

/// Finds the best fuzzy window for `submitted_old` inside
/// `file_content`. Returns `None` when the file is empty, the submitted
/// snippet is empty, or the best similarity is at or below
/// [`NEAREST_MATCH_MIN_SIMILARITY`].
///
/// Algorithm: both inputs are split into lines and each line is
/// whitespace-normalized (trim + collapse internal whitespace). A
/// sliding window the size of the normalized `submitted_old` walks the
/// file and scores per-line equality. The score is the fraction of
/// matching normalized lines in the window. O(file_lines * old_lines)
/// in the worst case, but `old_lines` is typically small (the model
/// submits a few lines at most).
fn find_nearest_match(submitted_old: &str, file_content: &str) -> Option<NearestMatch> {
    let old_lines: Vec<&str> = submitted_old.lines().collect();
    let file_lines: Vec<&str> = file_content.lines().collect();
    if old_lines.is_empty() || file_lines.is_empty() {
        return None;
    }

    let normalized_old: Vec<String> = old_lines.iter().map(|l| normalize_line(l)).collect();
    let normalized_file: Vec<String> = file_lines.iter().map(|l| normalize_line(l)).collect();

    let window = normalized_old.len().min(normalized_file.len());
    if window == 0 {
        return None;
    }

    let mut best_score = 0.0_f64;
    let mut best_start = 0_usize;
    for start in 0..=(normalized_file.len() - window) {
        let mut matches = 0usize;
        for offset in 0..window {
            if normalized_file[start + offset] == normalized_old[offset]
                && !normalized_old[offset].is_empty()
            {
                matches += 1;
            }
        }
        let score = matches as f64 / window as f64;
        if score > best_score {
            best_score = score;
            best_start = start;
        }
    }

    if best_score <= NEAREST_MATCH_MIN_SIMILARITY {
        return None;
    }

    let (context, starting_line) =
        build_context_window(&file_lines, best_start, window, NEAREST_MATCH_CONTEXT_LINES);
    Some(NearestMatch {
        similarity: best_score,
        context,
        starting_line,
    })
}

/// Normalizes a line for fuzzy comparison: trim leading/trailing
/// whitespace and collapse runs of internal whitespace to a single
/// space. This is what makes the matcher indent-insensitive — models
/// routinely submit `old` snippets whose indentation drifted after a
/// prior edit.
fn normalize_line(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(trimmed.len());
    let mut prev_space = false;
    for ch in trimmed.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out
}

/// Builds the context window surrounding a best-match region: up to
/// `pad` lines before, the matched `window` lines, and up to `pad`
/// lines after — clipped at file boundaries. Returns the joined string
/// and the 1-indexed starting line number of the first rendered line.
fn build_context_window(
    lines: &[&str],
    match_start: usize,
    window: usize,
    pad: usize,
) -> (String, usize) {
    let start = match_start.saturating_sub(pad);
    let end = (match_start + window + pad).min(lines.len());
    let rendered = lines[start..end].join("\n");
    (rendered, start + 1)
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

    // ------------------------------------------------------------------
    // γ.1 — nearest-match helper and enriched error payload
    // ------------------------------------------------------------------

    #[test]
    fn nearest_match_returns_high_similarity_on_exact_lines() {
        // The submitted snippet matches three adjacent lines of the
        // file verbatim; similarity must be 1.0 and the context must
        // include the matched region plus padding.
        let file = "line a\nfn foo() {\n    println!(\"hi\");\n}\nline b\n";
        let hit = find_nearest_match("fn foo() {\n    println!(\"hi\");\n}", file)
            .expect("should find a near-perfect match");
        assert!(
            (hit.similarity - 1.0).abs() < 1e-9,
            "exact match should be similarity 1.0, got {}",
            hit.similarity
        );
        assert!(hit.context.contains("fn foo()"));
        assert!(hit.context.contains("println!"));
        // Padding pulls in the surrounding "line a" / "line b" labels.
        assert!(hit.context.contains("line a"));
    }

    #[test]
    fn nearest_match_normalizes_indentation_drift() {
        // Model submits the right lines but with a stale indent. After
        // normalization the similarity must still clear the threshold.
        let file = "fn main() {\n        let x = 1;\n        let y = 2;\n}\n";
        let submitted = "    let x = 1;\n    let y = 2;";
        let hit = find_nearest_match(submitted, file)
            .expect("normalized match must survive indent drift");
        assert!(
            hit.similarity > 0.8,
            "indent-drifted match should stay > 0.8, got {}",
            hit.similarity
        );
    }

    #[test]
    fn nearest_match_returns_none_when_content_diverges() {
        // Completely unrelated content sits below the suppression
        // threshold; the payload must omit the hint rather than point
        // at an arbitrary window.
        let file = "alpha beta\ngamma delta\nepsilon zeta\n";
        let submitted = "import React from 'react';\nexport const App = () => null;";
        assert!(find_nearest_match(submitted, file).is_none());
    }

    #[test]
    fn nearest_match_clips_to_file_boundaries_for_small_files() {
        // File has fewer lines than the padding budget; the window
        // logic must clip rather than panic, and the starting_line
        // must stay a valid 1-indexed position.
        let file = "only line\n";
        let hit = find_nearest_match("only line", file);
        let hit = hit.expect("single-line file must still produce a match");
        assert_eq!(hit.starting_line, 1);
        assert!(hit.context.contains("only line"));
    }

    #[test]
    fn build_not_found_error_shape_matches_spec() {
        // When similarity passes the threshold, the payload carries
        // error, submitted_old, nearest_match, and hint. The
        // structured error renders exactly one `error` key at the top
        // level.
        let file = "fn foo() {\n    println!(\"hi\");\n}\n";
        let err = build_not_found_error("src/lib.rs", "fn foo() {\n  println!(\"hi\");\n}", file);
        let output = err.into_tool_output();
        assert_eq!(
            output.get("error").and_then(|v| v.as_str()),
            Some("Replacement text not found in 'src/lib.rs'"),
        );
        assert!(output.get("submitted_old").is_some());
        assert!(output.get("hint").is_some());
        let nearest = output
            .get("nearest_match")
            .expect("indent-drifted snippet must surface a nearest_match");
        assert!(nearest.get("similarity").is_some());
        assert!(nearest.get("context").is_some());
        assert!(nearest.get("starting_line").is_some());
    }

    #[test]
    fn build_not_found_error_omits_nearest_match_on_divergence() {
        let file = "totally unrelated text\nnothing like the submitted snippet\n";
        let err = build_not_found_error(
            "notes.md",
            "import { foo } from './bar';\nconsole.log('baz');",
            file,
        );
        let output = err.into_tool_output();
        assert!(
            output.get("nearest_match").is_none(),
            "diverged content must suppress nearest_match to avoid misleading hints"
        );
        // The hint and preview must still be present — those help even
        // when the fuzzy search gives up.
        assert!(output.get("hint").is_some());
        assert!(output.get("submitted_old").is_some());
    }

    #[test]
    fn submitted_old_preview_truncates_at_200_chars_with_ellipsis() {
        // A 210-char snippet becomes 200 chars + ellipsis.
        let big = "x".repeat(210);
        let preview = truncate_for_preview(&big, SUBMITTED_OLD_PREVIEW_CHARS);
        let char_count = preview.chars().count();
        assert_eq!(char_count, SUBMITTED_OLD_PREVIEW_CHARS + 1);
        assert!(preview.ends_with('…'));
    }

    #[test]
    fn submitted_old_preview_passes_through_when_under_budget() {
        let short = "hello";
        assert_eq!(
            truncate_for_preview(short, SUBMITTED_OLD_PREVIEW_CHARS),
            "hello"
        );
    }
}
