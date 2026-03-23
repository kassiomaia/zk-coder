//! Agent status communication tool.
//!
//! Allows the LLM to emit status updates during execution for user feedback.

use std::path::PathBuf;

use tauri::AppHandle;

use crate::tools::{
	context::ToolExecutionContext,
	error::ToolError,
	traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde_json::Value as JsonValue;

/// Tool that allows agents to emit status phrases to the frontend.
pub struct EmitStatusTool;

impl Tool for EmitStatusTool {
	fn meta(&self) -> ToolMeta {
		ToolMeta {
			id: ToolId::new("agent", "emit_status"),
			description: concat!(
				"Emit a status update message to the user about what you're currently working on. ",
				"Use this periodically (every 10-30 seconds) to keep the user informed of your progress. ",
				"Keep messages brief (2-6 words). ",
				"Examples: 'Scanning codebase...', 'Analyzing file structure...', ",
				"'Planning next steps...', 'Running tests...', 'Reading config files...'"
			),
			input_schema: serde_json::json!({
				"type": "object",
				"properties": {
					"message": {
						"type": "string",
						"description": "Brief status message (2-6 words) describing current activity"
					}
				},
				"required": ["message"]
			}),
			requires_workspace: false,
			experimental: false,
			feature_flag: None,
		}
	}

	fn execute(
		&self,
		_context: ToolExecutionContext,
		arguments: JsonValue,
		_app_handle: AppHandle,
		_workspace_root: Option<PathBuf>,
	) -> ToolFuture {
		Box::pin(async move {
			// Extract message from arguments
			let _message = arguments
				.get("message")
				.and_then(|v| v.as_str())
				.ok_or_else(|| ToolError::invalid_args("Missing 'message' parameter"))?;

			// The actual event emission is handled by the agent runtime
			// which intercepts this tool call and emits the StatusPhrase event
			// with the proper session_id
			Ok(serde_json::json!({
				"success": true,
				"message": "Status emitted"
			}))
		})
	}
}
