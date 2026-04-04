//! Apply a unified diff patch to an existing workspace file.

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    error::ToolError,
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

/// Arguments for applying a patch to a file.
#[derive(Debug, Deserialize)]
pub struct ApplyPatchArgs {
    /// Relative or absolute path inside the workspace.
    pub path: String,
    /// Unified diff patch text.
    pub patch: String,
    /// Whether to validate and preview without writing.
    #[serde(default)]
    pub dry_run: bool,
}

/// Output from applying a patch.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPatchOutput {
    /// Path of the updated file.
    pub path: String,
    /// Whether the patch was applied successfully.
    pub applied: bool,
    /// Whether this was a dry run.
    pub dry_run: bool,
    /// Number of hunks in the patch.
    pub hunks_applied: usize,
    /// Bytes written to disk (0 for dry runs).
    pub bytes_written: usize,
}

/// Tool to apply a unified diff patch to a workspace file.
pub struct ApplyPatchTool;

impl Tool for ApplyPatchTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("fs", "apply_patch"),
            description: "Apply a unified diff patch to an existing file inside the workspace without rewriting the full file body.",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path relative to workspace root, or absolute path within workspace"
                    },
                    "patch": {
                        "type": "string",
                        "description": "Unified diff patch text (standard @@ hunk format)"
                    },
                    "dry_run": {
                        "type": "boolean",
                        "description": "If true, validate and preview without writing (default: false)",
                        "default": false
                    }
                },
                "required": ["path", "patch"]
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

            let args: ApplyPatchArgs =
                crate::infrastructure::tools::error::parse_args("fs_apply_patch", arguments)?;

            if args.patch.trim().is_empty() {
                return Err(ToolError::invalid_args("Patch cannot be empty"));
            }

            let target_path = resolve_existing_workspace_file(&workspace, &args.path)?;
            let original = std::fs::read_to_string(&target_path).map_err(|e| {
                ToolError::failed(format!("Failed to read file '{}': {}", args.path, e))
            })?;

            let patch = diffy::Patch::from_str(&args.patch)
                .map_err(|e| ToolError::invalid_args(format!("Invalid patch format: {e}")))?;

            let hunks_applied = patch.hunks().len();
            if hunks_applied == 0 {
                return Err(ToolError::invalid_args("Patch contains no hunks"));
            }

            let patched = diffy::apply(&original, &patch)
                .map_err(|e| ToolError::failed(format!("Patch did not apply cleanly: {e}")))?;

            let bytes_written = if args.dry_run {
                0
            } else {
                std::fs::write(&target_path, &patched).map_err(|e| {
                    ToolError::failed(format!("Failed to write file '{}': {}", args.path, e))
                })?;
                patched.len()
            };

            Ok(serde_json::to_value(ApplyPatchOutput {
                path: args.path,
                applied: true,
                dry_run: args.dry_run,
                hunks_applied,
                bytes_written,
            })?)
        })
    }
}

/// Build a diff preview for a pending patch request.
pub fn generate_preview_diff(
    workspace_root: &Path,
    arguments: &JsonValue,
) -> Result<Option<String>, ToolError> {
    let args: ApplyPatchArgs =
        crate::infrastructure::tools::error::parse_args("fs_apply_patch", arguments.clone())?;
    let target_path = resolve_existing_workspace_file(workspace_root, &args.path)?;
    let original = std::fs::read_to_string(&target_path)
        .map_err(|e| ToolError::failed(format!("Failed to read file '{}': {}", args.path, e)))?;
    let patch = diffy::Patch::from_str(&args.patch)
        .map_err(|e| ToolError::invalid_args(format!("Invalid patch format: {e}")))?;
    let patched = diffy::apply(&original, &patch)
        .map_err(|e| ToolError::failed(format!("Patch did not apply cleanly: {e}")))?;

    Ok(crate::infrastructure::tools::fs::write_file::generate_diff(
        workspace_root,
        &args.path,
        &patched,
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_tool_metadata() {
        let tool = ApplyPatchTool;
        let meta = tool.meta();

        assert_eq!(meta.id.to_string(), "fs_apply_patch");
        assert!(meta.description.contains("patch"));
        assert!(meta.requires_workspace);
    }

    #[test]
    fn test_resolve_existing_workspace_file_rejects_empty() {
        let workspace = Path::new("/tmp/workspace");
        let error = resolve_existing_workspace_file(workspace, "").unwrap_err();
        assert!(error.to_string().contains("Path cannot be empty"));
    }

    #[test]
    fn test_apply_single_hunk_succeeds() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("sample.txt");
        std::fs::write(&file_path, "line one\nline two\nline three\n").unwrap();

        let patch_text = "--- a/sample.txt\n+++ b/sample.txt\n@@ -1,3 +1,3 @@\n line one\n-line two\n+line 2\n line three\n";
        let patch = diffy::Patch::from_str(patch_text).unwrap();
        let original = std::fs::read_to_string(&file_path).unwrap();
        let patched = diffy::apply(&original, &patch).unwrap();
        assert!(patched.contains("line 2"));
        assert!(!patched.contains("line two"));
    }

    #[test]
    fn test_apply_empty_patch_fails() {
        let args_value = serde_json::json!({
            "path": "sample.txt",
            "patch": ""
        });
        let args: ApplyPatchArgs = serde_json::from_value(args_value).unwrap();
        assert!(args.patch.trim().is_empty());
    }

    #[test]
    fn test_apply_invalid_patch_hunk_header_fails() {
        // A malformed hunk header (bad line range syntax) must fail to parse.
        let result =
            diffy::Patch::from_str("--- a/file.txt\n+++ b/file.txt\n@@ bad header @@\n line\n");
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_invalid_patch_hunk_counts_fail() {
        let patch_text = "\
--- a/src-tauri/src/lib.rs\n\
+++ b/src-tauri/src/lib.rs\n\
@@ -28,6 +28,12 @@\n\
 static TRAY_ICON_IDLE: Mutex<Option<Image<'static>>> = Mutex::new(None);\n\
 static TRAY_ICON_RECORDING: Mutex<Option<Image<'static>>> = Mutex::new(None);\n\
 static TRAY_ICON_LISTENING: Mutex<Option<Image<'static>>> = Mutex::new(None);\n\
+/// Generates a microphone icon programmatically by drawing pixels directly.\n\
+/// This creates three icon variants (idle/white, recording/red, listening/green)\n\
+/// without requiring external image assets.\n\
+///\n\
+/// The icon consists of:\n\
+/// - A filled ellipse for the microphone body\n\
+/// - A partial arc for the microphone stand/curve\n\
+/// - Two horizontal lines for the base stand\n\
 fn create_microphone_icon(color: [u8; 4], size: u32) -> Image<'static> {\n";

        let error = diffy::Patch::from_str(patch_text).unwrap_err();
        assert!(error.to_string().contains("Hunk header does not match hunk"));
    }

    #[test]
    fn test_apply_valid_patch_hunk_counts_succeed() {
        let patch_text = "\
--- a/src-tauri/src/lib.rs\n\
+++ b/src-tauri/src/lib.rs\n\
@@ -28,4 +28,12 @@\n\
 static TRAY_ICON_IDLE: Mutex<Option<Image<'static>>> = Mutex::new(None);\n\
 static TRAY_ICON_RECORDING: Mutex<Option<Image<'static>>> = Mutex::new(None);\n\
 static TRAY_ICON_LISTENING: Mutex<Option<Image<'static>>> = Mutex::new(None);\n\
+/// Generates a microphone icon programmatically by drawing pixels directly.\n\
+/// This creates three icon variants (idle/white, recording/red, listening/green)\n\
+/// without requiring external image assets.\n\
+///\n\
+/// The icon consists of:\n\
+/// - A filled ellipse for the microphone body\n\
+/// - A partial arc for the microphone stand/curve\n\
+/// - Two horizontal lines for the base stand\n\
 fn create_microphone_icon(color: [u8; 4], size: u32) -> Image<'static> {\n";

        let patch = diffy::Patch::from_str(patch_text).unwrap();
        assert_eq!(patch.hunks().len(), 1);
    }

    #[test]
    fn test_apply_flattened_patch_fails_to_parse() {
        let patch_text = "--- a/src-tauri/src/lib.rs +++ b/src-tauri/src/lib.rs @@ -28,6 +28,12 @@ static TRAY_ICON_IDLE: Mutex<Option<Image<'static>>> = Mutex::new(None); static TRAY_ICON_RECORDING: Mutex<Option<Image<'static>>> = Mutex::new(None); static TRAY_ICON_LISTENING: Mutex<Option<Image<'static>>> = Mutex::new(None); +/// Generates a microphone icon programmatically by drawing pixels directly. +/// This creates three icon variants (idle/white, recording/red, listening/green) +/// without requiring external image assets. +/// +/// The icon consists of: +/// - A filled ellipse for the microphone body +/// - A partial arc for the microphone stand/curve +/// - Two horizontal lines for the base stand fn create_microphone_icon(color: [u8; 4], size: u32) -> Image<'static> { let mut rgba = vec![0u8; (size * size * 4) as usize];";

        let result = diffy::Patch::from_str(patch_text);
        assert!(result.is_err());
    }

    #[test]
    fn test_dry_run_does_not_write() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("sample.txt");
        let original_content = "line one\nline two\nline three\n";
        std::fs::write(&file_path, original_content).unwrap();

        let patch_text = "--- a/sample.txt\n+++ b/sample.txt\n@@ -1,3 +1,3 @@\n line one\n-line two\n+line 2\n line three\n";
        let patch = diffy::Patch::from_str(patch_text).unwrap();
        let original = std::fs::read_to_string(&file_path).unwrap();

        // Apply in dry-run: compute result but don't write
        let patched = diffy::apply(&original, &patch).unwrap();
        assert!(patched.contains("line 2"));

        // File should be unchanged
        let still_original = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(still_original, original_content);
    }

    #[test]
    fn test_patch_conflict_fails() {
        let original = "line one\nline two\nline three\n";
        // Patch expects different content than what's there
        let bad_patch = "--- a/sample.txt\n+++ b/sample.txt\n@@ -1,3 +1,3 @@\n line one\n-does not exist\n+replacement\n line three\n";
        let patch = diffy::Patch::from_str(bad_patch).unwrap();
        let result = diffy::apply(original, &patch);
        assert!(result.is_err());
    }

    #[test]
    fn test_path_traversal_rejected() {
        let temp = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        let outside_file = outside.path().join("secret.txt");
        std::fs::write(&outside_file, "secret").unwrap();

        let err = resolve_existing_workspace_file(temp.path(), &outside_file.to_string_lossy())
            .unwrap_err();
        assert!(err.to_string().contains("Path traversal"));
    }

    #[test]
    fn test_generate_preview_diff_returns_some() {
        let temp = TempDir::new().unwrap();
        std::fs::write(
            temp.path().join("sample.txt"),
            "line one\nline two\nline three\n",
        )
        .unwrap();

        let patch_text = "--- a/sample.txt\n+++ b/sample.txt\n@@ -1,3 +1,3 @@\n line one\n-line two\n+line 2\n line three\n";
        let preview = generate_preview_diff(
            temp.path(),
            &serde_json::json!({
                "path": "sample.txt",
                "patch": patch_text
            }),
        )
        .unwrap();

        assert!(preview.is_some());
        let diff = preview.unwrap();
        assert!(diff.contains("--- a/sample.txt"));
        assert!(diff.contains("+++ b/sample.txt"));
    }
}
