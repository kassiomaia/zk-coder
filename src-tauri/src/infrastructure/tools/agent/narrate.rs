//! Agent narration tool.
//!
//! Allows the LLM to send natural-language updates that persist in the chat history.

use std::path::PathBuf;

use tauri::AppHandle;

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    error::ToolError,
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde_json::Value as JsonValue;

/// Tool that allows agents to send persistent narration updates to the chat.
pub struct NarrateTool;

impl Tool for NarrateTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("agent", "narrate"),
            description: concat!(
                "Send a natural-language update to the user about your progress, findings, or decisions. ",
                "Use 1-2 clear sentences. Focus on what you discovered, decided, or completed — not every small action. ",
                "These updates persist in the chat history. ",
                "Examples: 'I found 3 test files that need updating. Starting with auth.test.ts.', ",
                "'The refactoring is complete. I updated 5 files and all existing tests pass.'"
            ),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "Natural-language update (1-2 sentences) about progress, findings, or decisions"
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
            // which intercepts this tool call and emits the AgentNarration event
            // with the proper session_id, conversation_id, and thread_id
            Ok(serde_json::json!({
                "success": true,
                "message": "Update sent"
            }))
        })
    }
}

#[cfg(all(test, feature = "integration-test"))]
mod tests {
    use super::*;

    #[test]
    fn test_tool_metadata() {
        let tool = NarrateTool;
        let meta = tool.meta();

        assert_eq!(meta.id.to_string(), "agent_narrate");
        assert!(meta.description.contains("natural-language"));
        assert!(meta.description.contains("persist"));
    }

    #[test]
    fn test_tool_id_format() {
        let tool = NarrateTool;
        let meta = tool.meta();

        assert_eq!(meta.id.category, "agent");
        assert_eq!(meta.id.name, "narrate");
        assert_eq!(meta.id.full_name(), "agent_narrate");
    }

    #[test]
    fn test_input_schema() {
        let tool = NarrateTool;
        let meta = tool.meta();
        let schema = &meta.input_schema;

        assert_eq!(schema["type"], "object");

        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("message")));

        let message_prop = &schema["properties"]["message"];
        assert_eq!(message_prop["type"], "string");
    }

    #[test]
    fn test_no_workspace_required() {
        let tool = NarrateTool;
        let meta = tool.meta();
        assert!(!meta.requires_workspace);
    }

    #[test]
    fn test_not_experimental() {
        let tool = NarrateTool;
        let meta = tool.meta();
        assert!(!meta.experimental);
    }

    #[test]
    fn test_no_feature_flag() {
        let tool = NarrateTool;
        let meta = tool.meta();
        assert!(meta.feature_flag.is_none());
    }

    #[tokio::test]
    async fn test_execute_with_valid_message() {
        let tool = NarrateTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "message": "I found the main entry point and now tracing the data flow."
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["success"], true);
        assert_eq!(output["message"], "Update sent");
    }

    #[tokio::test]
    async fn test_execute_missing_message() {
        let tool = NarrateTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({});

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("message"));
    }

    #[tokio::test]
    async fn test_execute_null_message() {
        let tool = NarrateTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "message": null
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
    }
}
