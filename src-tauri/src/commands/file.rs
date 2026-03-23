//! File viewing commands for the UI.
//!
//! These commands provide file content access for the frontend UI,
//! separate from agent tool execution. They don't consume LLM tokens.

use crate::commands::agent::AgentState;
use std::path::PathBuf;

/// Response containing file content and diff information.
#[derive(Debug, serde::Serialize)]
pub struct FileContentResponse {
	pub path: String,
	pub exists: bool,
	pub current_content: Option<String>,
	pub original_content: Option<String>,
	pub diff: Option<String>,
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
	let rollback_store = agent_state.runtime.rollback_store();
	let full_path = PathBuf::from(workspace_root).join(&file_path);
	let exists = full_path.exists();

	// Read current file content
	let current_content = std::fs::read_to_string(&full_path).ok();

	// Get original content from rollback store (before agent modifications)
	let original_content = rollback_store.get_snapshot_content(&session_id, &file_path);

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
