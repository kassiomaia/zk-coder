//! Agent subtask status marking tool.
//!
//! Allows the LLM to manually update the status of a specific subtask.

use std::path::PathBuf;

use tauri::AppHandle;

use crate::infrastructure::tools::{
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
                    },
                    "rationale": {
                        "type": "string",
                        "description": "Optional explicit rationale used as progress evidence when no file/command evidence exists"
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

#[cfg(all(test, feature = "integration-test"))]
mod tests {
    use super::*;

    // === Tool metadata tests ===

    #[test]
    fn test_tool_metadata() {
        let tool = MarkSubtaskStatusTool;
        let meta = tool.meta();

        assert_eq!(meta.id.to_string(), "agent_mark_subtask_status");
        assert!(meta.description.contains("status"));
        assert!(meta.description.contains("index"));
    }

    #[test]
    fn test_tool_id_format() {
        let tool = MarkSubtaskStatusTool;
        let meta = tool.meta();

        assert_eq!(meta.id.category, "agent");
        assert_eq!(meta.id.name, "mark_subtask_status");
        assert_eq!(meta.id.full_name(), "agent_mark_subtask_status");
    }

    #[test]
    fn test_input_schema_requires_index() {
        let tool = MarkSubtaskStatusTool;
        let meta = tool.meta();
        let schema = &meta.input_schema;

        assert_eq!(schema["type"], "object");
        assert_eq!(schema["additionalProperties"], false);
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

        let valid_statuses: Vec<&str> = status_enum.iter().filter_map(|v| v.as_str()).collect();

        assert!(valid_statuses.contains(&"pending"));
        assert!(valid_statuses.contains(&"in_progress"));
        assert!(valid_statuses.contains(&"completed"));
        assert!(valid_statuses.contains(&"failed"));
        assert_eq!(valid_statuses.len(), 4);
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

    #[test]
    fn test_no_feature_flag() {
        let tool = MarkSubtaskStatusTool;
        let meta = tool.meta();
        assert!(meta.feature_flag.is_none());
    }

    // === Execution tests ===

    #[tokio::test]
    async fn test_execute_valid_status() {
        let tool = MarkSubtaskStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        for status in ["pending", "in_progress", "completed", "failed"] {
            let args = serde_json::json!({
                "index": 0,
                "status": status
            });

            let result = tool
                .execute(context.clone(), args, app_handle.clone(), None)
                .await;

            assert!(result.is_ok(), "Status '{}' should be valid", status);
            let output = result.unwrap();
            assert_eq!(output["success"], true);
        }
    }

    #[tokio::test]
    async fn test_execute_with_zero_index() {
        let tool = MarkSubtaskStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "index": 0,
            "status": "completed"
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_large_index() {
        let tool = MarkSubtaskStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "index": 999999,
            "status": "pending"
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
    }

    // === Error handling tests ===

    #[tokio::test]
    async fn test_execute_missing_index() {
        let tool = MarkSubtaskStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "status": "completed"
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("index"));
    }

    #[tokio::test]
    async fn test_execute_missing_status() {
        let tool = MarkSubtaskStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "index": 0
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("status"));
    }

    #[tokio::test]
    async fn test_execute_invalid_status() {
        let tool = MarkSubtaskStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "index": 0,
            "status": "invalid_status"
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid status"));
        assert!(err.to_string().contains("invalid_status"));
    }

    #[tokio::test]
    async fn test_execute_empty_status() {
        let tool = MarkSubtaskStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "index": 0,
            "status": ""
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_mixed_case_status() {
        let tool = MarkSubtaskStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        // Status is case-sensitive
        let args = serde_json::json!({
            "index": 0,
            "status": "COMPLETED"
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_wrong_type_for_index() {
        let tool = MarkSubtaskStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "index": "not_a_number",
            "status": "completed"
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_wrong_type_for_status() {
        let tool = MarkSubtaskStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "index": 0,
            "status": 123
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
    }

    // === Output validation tests ===

    #[tokio::test]
    async fn test_output_structure() {
        let tool = MarkSubtaskStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "index": 2,
            "status": "in_progress"
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
        let output = result.unwrap();

        assert_eq!(output["success"], true);
        assert!(output["message"].is_string());
        assert!(output["message"].as_str().unwrap().contains("2"));
        assert!(output["message"].as_str().unwrap().contains("in_progress"));
    }

    #[tokio::test]
    async fn test_output_is_json_serializable() {
        let tool = MarkSubtaskStatusTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "index": 0,
            "status": "completed"
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
        let tool = MarkSubtaskStatusTool;
        let context = ToolExecutionContext::new("test-conversation").with_thread("thread-123");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "index": 0,
            "status": "completed"
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_session_context() {
        let tool = MarkSubtaskStatusTool;
        let context = ToolExecutionContext::new("test-conversation").with_session("session-456");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "index": 0,
            "status": "pending"
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
    }
}
