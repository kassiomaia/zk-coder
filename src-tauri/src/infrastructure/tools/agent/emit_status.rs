//! Agent status communication tool.
//!
//! Allows the LLM to emit status updates during execution for user feedback.

use std::path::PathBuf;

use tauri::AppHandle;

use crate::infrastructure::tools::{
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
				"Emit a brief status phrase (2-6 words) for the spinner UI. ",
				"Use every 10-30 seconds during active work. ",
				"For longer, user-facing updates that persist in chat history, use agent_narrate instead. ",
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

#[cfg(all(test, feature = "integration-test"))]
mod tests {
    use super::*;

    // === Tool metadata tests ===

    #[test]
    fn test_tool_metadata() {
        let tool = EmitStatusTool;
        let meta = tool.meta();

        assert_eq!(meta.id.to_string(), "agent_emit_status");
        assert!(meta.description.contains("status"));
        assert!(meta.description.contains("progress"));
    }

    #[test]
    fn test_tool_id_format() {
        let tool = EmitStatusTool;
        let meta = tool.meta();

        assert_eq!(meta.id.category, "agent");
        assert_eq!(meta.id.name, "emit_status");
        assert_eq!(meta.id.full_name(), "agent_emit_status");
    }

    #[test]
    fn test_input_schema() {
        let tool = EmitStatusTool;
        let meta = tool.meta();
        let schema = &meta.input_schema;

        assert_eq!(schema["type"], "object");

        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("message")));

        // Check message property
        let message_prop = &schema["properties"]["message"];
        assert_eq!(message_prop["type"], "string");
        assert!(message_prop["description"]
            .as_str()
            .unwrap()
            .contains("status"));
    }

    #[test]
    fn test_description_mentions_examples() {
        let tool = EmitStatusTool;
        let meta = tool.meta();

        assert!(meta.description.contains("Scanning codebase"));
        assert!(meta.description.contains("Planning"));
        assert!(meta.description.contains("Running tests"));
    }

    #[test]
    fn test_no_workspace_required() {
        let tool = EmitStatusTool;
        let meta = tool.meta();
        assert!(!meta.requires_workspace);
    }

    #[test]
    fn test_not_experimental() {
        let tool = EmitStatusTool;
        let meta = tool.meta();
        assert!(!meta.experimental);
    }

    #[test]
    fn test_no_feature_flag() {
        let tool = EmitStatusTool;
        let meta = tool.meta();
        assert!(meta.feature_flag.is_none());
    }

    // === Execution tests ===

    #[tokio::test]
    async fn test_execute_with_valid_message() {
        let tool = EmitStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "message": "Scanning codebase..."
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["success"], true);
        assert_eq!(output["message"], "Status emitted");
    }

    #[tokio::test]
    async fn test_execute_with_brief_message() {
        let tool = EmitStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "message": "Working..."
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_long_message() {
        let tool = EmitStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "message": "Analyzing the codebase structure and planning the next steps for implementation..."
        });

        let result = tool.execute(context, args, app_handle, None).await;

        // Tool doesn't validate message length, that's up to the LLM
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_unicode_message() {
        let tool = EmitStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "message": "Analyzing 分析代码..."
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
    }

    // === Error handling tests ===

    #[tokio::test]
    async fn test_execute_missing_message() {
        let tool = EmitStatusTool;
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
        let tool = EmitStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "message": null
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_empty_message() {
        let tool = EmitStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "message": ""
        });

        let result = tool.execute(context, args, app_handle, None).await;

        // Empty string is still a valid string
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_wrong_type_for_message() {
        let tool = EmitStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "message": 123
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_array_for_message() {
        let tool = EmitStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "message": ["Not", "a", "string"]
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
    }

    // === Output validation tests ===

    #[tokio::test]
    async fn test_output_structure() {
        let tool = EmitStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "message": "Test status"
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
        let output = result.unwrap();

        assert_eq!(output["success"], true);
        assert!(output["message"].is_string());
    }

    #[tokio::test]
    async fn test_output_message_value() {
        let tool = EmitStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "message": "Working..."
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["message"], "Status emitted");
    }

    #[tokio::test]
    async fn test_output_is_json_serializable() {
        let tool = EmitStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "message": "Test"
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
        let output = result.unwrap();

        // Verify we can serialize back to JSON string
        let json_string = serde_json::to_string(&output);
        assert!(json_string.is_ok());
    }

    // === Context tests ===

    #[tokio::test]
    async fn test_execute_with_thread_context() {
        let tool = EmitStatusTool;
        let context = ToolExecutionContext::new("test-conversation").with_thread("thread-123");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "message": "Working..."
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_session_context() {
        let tool = EmitStatusTool;
        let context = ToolExecutionContext::new("test-conversation").with_session("session-456");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "message": "Working..."
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_without_workspace() {
        let tool = EmitStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "message": "Test"
        });

        // Should work without workspace
        let result = tool.execute(context, args, app_handle, None).await;
        assert!(result.is_ok());
    }

    // === Edge case tests ===

    #[tokio::test]
    async fn test_execute_with_extra_fields_ignored() {
        let tool = EmitStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "message": "Test",
            "unexpected": "field",
            "another": 123
        });

        // Extra fields should be ignored
        let result = tool.execute(context, args, app_handle, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_newlines_in_message() {
        let tool = EmitStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "message": "Working\non\nmultiple\nlines"
        });

        let result = tool.execute(context, args, app_handle, None).await;

        // Should accept newlines
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_special_characters() {
        let tool = EmitStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "message": "Working @#$%^&*()..."
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_emoji() {
        let tool = EmitStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "message": "Working 🚀..."
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_whitespace_only_message() {
        let tool = EmitStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = crate::test_support::mock_app_handle();

        let args = serde_json::json!({
            "message": "   "
        });

        // Whitespace-only is still a valid string
        let result = tool.execute(context, args, app_handle, None).await;
        assert!(result.is_ok());
    }
}
