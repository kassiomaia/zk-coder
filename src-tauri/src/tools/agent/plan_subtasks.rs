//! Agent subtask planning tool.
//!
//! Allows the LLM to decompose its current task into named subtasks
//! that appear in the Activity section of the agent panel.

use std::path::PathBuf;

use tauri::AppHandle;

use crate::tools::{
	context::ToolExecutionContext,
	error::ToolError,
	traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde_json::Value as JsonValue;

/// Tool that allows agents to plan subtasks for the current execution.
pub struct PlanSubtasksTool;

impl Tool for PlanSubtasksTool {
	fn meta(&self) -> ToolMeta {
		ToolMeta {
			id: ToolId::new("agent", "plan_subtasks"),
			description: concat!(
				"Break down your current task into a list of named subtasks. ",
				"These appear in the Activity section so the user can track your progress. ",
				"Call this once early in execution after understanding what needs to be done. ",
				"Each subtask should be a short description of a logical step. ",
				"Example: ['Analyze current code structure', 'Implement new function', 'Write tests']"
			),
			input_schema: serde_json::json!({
				"type": "object",
				"properties": {
					"subtasks": {
						"type": "array",
						"items": {
							"type": "object",
							"properties": {
								"title": {
									"type": "string",
									"description": "Short title for this subtask (2-8 words)"
								},
								"description": {
									"type": "string",
									"description": "Optional longer description of what this subtask involves"
								}
							},
							"required": ["title"]
						},
						"description": "Ordered list of subtasks to plan",
						"minItems": 1,
						"maxItems": 10
					}
				},
				"required": ["subtasks"]
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
			// Validate subtasks array exists
			let subtasks = arguments
				.get("subtasks")
				.and_then(|v| v.as_array())
				.ok_or_else(|| ToolError::invalid_args("Missing 'subtasks' array parameter"))?;

			if subtasks.is_empty() {
				return Err(ToolError::invalid_args("'subtasks' array must not be empty"));
			}

			let count = subtasks.len();

			// The actual subtask creation is handled by the agent runtime
			// which intercepts this tool call and creates SubTask objects
			Ok(serde_json::json!({
				"success": true,
				"message": format!("Planned {count} subtasks"),
				"count": count
			}))
		})
	}
}
