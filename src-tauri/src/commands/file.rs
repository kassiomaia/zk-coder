//! File viewing commands for the UI.
//!
//! These commands provide file content access for the frontend UI,
//! separate from agent tool execution. They don't consume LLM tokens.

use crate::commands::agent::AgentState;
use std::path::PathBuf;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

const FILE_DIFF_WINDOW_LABEL: &str = "file-diff";

/// Response containing file content and diff information.
#[derive(Debug, serde::Serialize)]
pub struct FileContentResponse {
    /// The file path.
    pub path: String,
    /// Whether the file exists.
    pub exists: bool,
    /// The current content of the file (if it exists).
    pub current_content: Option<String>,
    /// The original content before agent modifications.
    pub original_content: Option<String>,
    /// The diff between original and current content.
    pub diff: Option<String>,
    /// Whether this is a new file (no original content).
    pub is_new: bool,
}

/// Get file content for UI viewing.
///
/// This command is separate from agent tools and doesn't consume LLM tokens.
/// It returns:
/// - Current file content (if file exists)
/// - Original content from rollback store (before agent modifications)
/// - Generated diff (if both contents exist)
/// - New file formatted as addition (if no original content)
#[tauri::command]
pub async fn get_file_content(
    workspace_root: String,
    file_path: String,
    session_id: String,
    agent_state: tauri::State<'_, AgentState>,
) -> Result<FileContentResponse, String> {
    let full_path = PathBuf::from(workspace_root).join(&file_path);
    let exists = full_path.exists();

    // Read current file content
    let current_content = std::fs::read_to_string(&full_path).ok();

    // Get original content from rollback store (before agent modifications)
    let original_content = agent_state
        .service
        .snapshot_content(&session_id, &file_path);

    // Generate diff based on available content
    let diff = match (&original_content, &current_content) {
        (Some(original), Some(current)) => {
            // Both exist - generate diff
            Some(generate_diff_from_content(&file_path, original, current))
        }
        (None, Some(current)) => {
            // New file - show as addition
            Some(format_as_new_file_diff(&file_path, current))
        }
        (Some(original), None) => {
            // File was deleted - show as deletion
            Some(format_as_deleted_file_diff(&file_path, original))
        }
        _ => None,
    };

    let is_new = original_content.is_none();

    Ok(FileContentResponse {
        path: file_path,
        exists,
        current_content,
        original_content,
        diff,
        is_new,
    })
}

/// Generate a unified diff from two content strings.
fn generate_diff_from_content(path: &str, original: &str, new: &str) -> String {
    let diff = similar::TextDiff::from_lines(original, new);

    let hunks: Vec<String> = diff
        .unified_diff()
        .context_radius(3)
        .iter_hunks()
        .map(|h| h.to_string())
        .collect();

    if hunks.is_empty() {
        // No changes - return minimal diff header
        return format!("--- a/{}\n+++ b/{}\n", path, path);
    }

    let mut output = format!("--- a/{}\n+++ b/{}\n", path, path);
    for hunk in hunks {
        output.push_str(&hunk);
    }
    output
}

/// Format a new file's content as a unified diff (all additions).
fn format_as_new_file_diff(path: &str, content: &str) -> String {
    let line_count = content.lines().count();
    let lines: String = content
        .lines()
        .map(|line| format!("+{}", line))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "--- /dev/null\n+++ b/{}\n@@ -0,0 +1,{} @@\n{}",
        path, line_count, lines
    )
}

/// Format a deleted file's content as a unified diff (all deletions).
fn format_as_deleted_file_diff(path: &str, content: &str) -> String {
    let line_count = content.lines().count();
    let lines: String = content
        .lines()
        .map(|line| format!("-{}", line))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "--- a/{}\n+++ /dev/null\n@@ -1,{} +0,0 @@\n{}",
        path, line_count, lines
    )
}

/// Simple URL encoding function that avoids additional dependencies.
fn encode_url(s: &str) -> String {
    let mut encoded = String::new();
    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => encoded.push(c),
            _ => {
                let mut buf = [0; 4];
                for byte in c.encode_utf8(&mut buf).as_bytes() {
                    encoded.push('%');
                    encoded.push_str(&format!("{:02X}", byte));
                }
            }
        }
    }
    encoded
}

/// Open a file diff in a dedicated chromeless window.
#[tauri::command]
pub async fn open_file_diff_window(
    file_path: String,
    workspace_root: String,
    session_id: String,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let encoded_path = encode_url(&file_path);
    let path = format!(
        "/file-diff?filePath={}&workspaceRoot={}&sessionId={}",
        encoded_path, workspace_root, session_id
    );

    if let Some(window) = app_handle.get_webview_window(FILE_DIFF_WINDOW_LABEL) {
        window
            .eval(&format!(
                "window.location.href = '{}'",
                path
            ))
            .map_err(|e| e.to_string())?;
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(
        &app_handle,
        FILE_DIFF_WINDOW_LABEL,
        WebviewUrl::App(path.into()),
    )
    .title("Code Viewer")
    .inner_size(1200.0, 800.0)
    .min_inner_size(600.0, 400.0)
    .decorations(false)
    .transparent(true)
    .maximized(true)
    .devtools(false)
    .build()
    .map_err(|e| e.to_string())?;

    let _ = window.set_focus();
    Ok(())
}
