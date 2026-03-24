//! Agent subtask status marking tool.
//!
//! Allows the LLM to manually update the status of a specific subtask.

use std::path::PathBuf;

use tauri::AppHandle;

use crate::tools::{
    context::ToolExecutionContext,
    error::ToolError,
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde_json::Value as JsonValue;

/// Tool that allows agents to mark a specific subtask with a status.
pub struct MarkSubtaskStatusTool;

impl Tool for MarkSubtaskStatusTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("agent", "mark_subtask_status"),
            description: concat!(
                "Manually update the status of a specific subtask by index. ",
                "Use this to mark a subtask as 'in_progress', 'completed', or 'failed'. ",
                "The index is zero-based (0 = first subtask, 1 = second, etc.). ",
                "Normally you should use 'advance_to_next_subtask' instead, ",
                "but this tool gives you more control if needed."
            ),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "index": {
                        "type": "integer",
                        "description": "Zero-based index of the subtask (0 = first, 1 = second, etc.)",
                        "minimum": 0
                    },
                    "status": {
                        "type": "string",
                        "enum": ["pending", "in_progress", "completed", "failed"],
                        "description": "The new status for the subtask"
                    }
                },
                "required": ["index", "status"],
                "additionalProperties": false
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
            // Validate inputs
            let index = arguments
                .get("index")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| ToolError::invalid_args("Missing 'index' parameter"))?;

            let status = arguments
                .get("status")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::invalid_args("Missing 'status' parameter"))?;

            // Validate status enum
            if !matches!(status, "pending" | "in_progress" | "completed" | "failed") {
                return Err(ToolError::invalid_args(&format!(
                    "Invalid status: '{}'. Must be one of: pending, in_progress, completed, failed",
                    status
                )));
            }

            // The actual status update is handled by the agent runtime
            // which intercepts this tool call and updates the SubTask
            Ok(serde_json::json!({
                "success": true,
                "message": format!("Marked subtask {} as {}", index, status)
            }))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_tool_metadata() {
        let tool = MarkSubtaskStatusTool;
        let meta = tool.meta();

        assert_eq!(meta.id.to_string(), "agent_mark_subtask_status");
        assert!(meta.description.contains("status"));
        assert!(meta.description.contains("index"));
    }

    #[test]
    fn test_input_schema_requires_index() {
        let tool = MarkSubtaskStatusTool;
        let meta = tool.meta();
        let schema = &meta.input_schema;

        assert_eq!(schema["type"], "object");
        let required = schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v.as_str() == Some("index")));
        assert!(required.iter().any(|v| v.as_str() == Some("status")));
    }

    #[test]
    fn test_status_enum_values() {
        let tool = MarkSubtaskStatusTool;
        let meta = tool.meta();
        let schema = &meta.input_schema;
        let status_enum = schema["properties"]["status"]["enum"]
            .as_array()
            .expect("Status should have enum values");

        let valid_statuses: Vec<&str> = status_enum
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        assert!(valid_statuses.contains(&"pending"));
        assert!(valid_statuses.contains(&"in_progress"));
        assert!(valid_statuses.contains(&"completed"));
        assert!(valid_statuses.contains(&"failed"));
    }

    #[test]
    fn test_index_minimum_value() {
        let tool = MarkSubtaskStatusTool;
        let meta = tool.meta();
        let schema = &meta.input_schema;

        let index_prop = &schema["properties"]["index"];
        assert_eq!(index_prop["type"], "integer");
        assert_eq!(index_prop["minimum"], 0);
    }

    #[test]
    fn test_tool_does_not_require_workspace() {
        let tool = MarkSubtaskStatusTool;
        let meta = tool.meta();
        assert!(!meta.requires_workspace);
    }

    #[test]
    fn test_tool_is_not_experimental() {
        let tool = MarkSubtaskStatusTool;
        let meta = tool.meta();
        assert!(!meta.experimental);
    }
}
