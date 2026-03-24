//! Agent subtask advancement tool.
//!
//! Allows the LLM to mark the current subtask as completed
//! and advance to the next subtask in the plan.

use std::path::PathBuf;

use tauri::AppHandle;

use crate::tools::{
    context::ToolExecutionContext,
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde_json::Value as JsonValue;

/// Tool that allows agents to advance to the next subtask.
pub struct AdvanceToNextSubtaskTool;

impl Tool for AdvanceToNextSubtaskTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("agent", "advance_to_next_subtask"),
            description: concat!(
                "Mark the current subtask as completed and advance to the next subtask. ",
                "Call this when you have finished working on the current subtask. ",
                "This will mark the current subtask as 'completed' and the next subtask as 'in_progress'. ",
                "If there are no more subtasks, the task will be marked as complete."
            ),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
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
        _arguments: JsonValue,
        _app_handle: AppHandle,
        _workspace_root: Option<PathBuf>,
    ) -> ToolFuture {
        Box::pin(async move {
            // The actual subtask advancement is handled by the agent runtime
            // which intercepts this tool call and updates SubTask statuses
            Ok(serde_json::json!({
                "success": true,
                "message": "Advanced to next subtask"
            }))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_metadata() {
        let tool = AdvanceToNextSubtaskTool;
        let meta = tool.meta();

        assert_eq!(meta.id.to_string(), "agent_advance_to_next_subtask");
        assert!(meta.description.contains("completed"));
        assert!(meta.description.contains("next subtask"));
    }

    #[test]
    fn test_input_schema() {
        let tool = AdvanceToNextSubtaskTool;
        let meta = tool.meta();
        let schema = &meta.input_schema;

        // Should accept empty object
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].as_object().unwrap().is_empty());
    }

    #[test]
    fn test_no_workspace_required() {
        let tool = AdvanceToNextSubtaskTool;
        let meta = tool.meta();
        assert!(!meta.requires_workspace);
    }

    #[test]
    fn test_not_experimental() {
        let tool = AdvanceToNextSubtaskTool;
        let meta = tool.meta();
        assert!(!meta.experimental);
    }
}
