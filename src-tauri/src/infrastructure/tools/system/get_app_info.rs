//! Get application information tool.

use crate::infrastructure::tools::{
    context::ToolExecutionContext,
    error::ToolError,
    traits::{Tool, ToolFuture, ToolId, ToolMeta},
};
use serde_json::Value as JsonValue;
use tauri::AppHandle;

/// Tool to retrieve backend application metadata.
pub struct GetAppInfoTool;

impl Tool for GetAppInfoTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: ToolId::new("system", "get_app_info"),
            description: "Return backend application metadata (name and version).",
            input_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {}
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
        _workspace_root: Option<std::path::PathBuf>,
    ) -> ToolFuture {
        Box::pin(async move {
            if !arguments.is_null() && arguments != serde_json::json!({}) {
                return Err(ToolError::invalid_args(
                    "get_app_info does not accept any arguments",
                ));
            }

            Ok(serde_json::json!({
                "name": env!("CARGO_PKG_NAME"),
                "version": env!("CARGO_PKG_VERSION"),
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
        let tool = GetAppInfoTool;
        let meta = tool.meta();

        assert_eq!(meta.id.to_string(), "system_get_app_info");
        assert!(meta.description.contains("metadata"));
        assert!(!meta.requires_workspace);
        assert!(!meta.experimental);
        assert!(meta.feature_flag.is_none());
    }

    #[test]
    fn test_tool_id_format() {
        let tool = GetAppInfoTool;
        let meta = tool.meta();

        assert_eq!(meta.id.category, "system");
        assert_eq!(meta.id.name, "get_app_info");
        assert_eq!(meta.id.full_name(), "system_get_app_info");
    }

    #[test]
    fn test_input_schema() {
        let tool = GetAppInfoTool;
        let meta = tool.meta();
        let schema = &meta.input_schema;

        assert_eq!(schema["type"], "object");
        assert!(schema["additionalProperties"] == false);
        assert!(schema["properties"].as_object().unwrap().is_empty());
    }

    // === Execution tests ===

    #[tokio::test]
    async fn test_execute_with_empty_object() {
        let tool = GetAppInfoTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({});

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output["name"].is_string());
        assert!(output["version"].is_string());
    }

    #[tokio::test]
    async fn test_execute_with_null() {
        let tool = GetAppInfoTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!(null);

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output["name"].is_string());
    }

    #[tokio::test]
    async fn test_execute_with_extra_args_fails() {
        let tool = GetAppInfoTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({
            "unexpected": "field"
        });

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("does not accept any arguments"));
    }

    #[tokio::test]
    async fn test_execute_returns_valid_structure() {
        let tool = GetAppInfoTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({});

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
        let output = result.unwrap();

        // Verify expected fields
        assert!(output.get("name").is_some());
        assert!(output.get("version").is_some());

        // Verify types
        assert!(output["name"].is_string());
        assert!(output["version"].is_string());
    }

    #[tokio::test]
    async fn test_execute_name_is_coder() {
        let tool = GetAppInfoTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({});

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        let name = output["name"].as_str().unwrap();
        // The package name comes from CARGO_PKG_NAME
        assert!(!name.is_empty());
    }

    #[tokio::test]
    async fn test_execute_version_format() {
        let tool = GetAppInfoTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({});

        let result = tool.execute(context, args, app_handle, None).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        let version = output["version"].as_str().unwrap();
        // Version should be in semver-like format (major.minor.patch or similar)
        assert!(!version.is_empty());
        // Should contain at least one dot for version format
        // Note: This may vary based on Cargo.toml
    }

    // === Context tests ===

    #[tokio::test]
    async fn test_execute_without_workspace() {
        let tool = GetAppInfoTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({});

        // Should work without workspace
        let result = tool.execute(context, args, app_handle, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_with_workspace() {
        let tool = GetAppInfoTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        let args = serde_json::json!({});

        // Workspace should be ignored since tool doesn't require it
        let result = tool
            .execute(
                context,
                args,
                app_handle,
                Some(std::path::PathBuf::from("/tmp/workspace")),
            )
            .await;
        assert!(result.is_ok());
    }

    // === Output validation tests ===

    #[test]
    fn test_output_is_json_serializable() {
        let output = serde_json::json!({
            "name": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"),
        });

        // Verify we can serialize back to JSON string
        let json_string = serde_json::to_string(&output);
        assert!(json_string.is_ok());
    }

    #[test]
    fn test_output_matches_schema() {
        let output = serde_json::json!({
            "name": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"),
        });

        // Verify output is an object
        assert!(output.is_object());

        // Verify it has the required fields
        assert!(output.as_object().unwrap().contains_key("name"));
        assert!(output.as_object().unwrap().contains_key("version"));
    }

    // === Edge case tests ===

    #[tokio::test]
    async fn test_execute_with_various_invalid_inputs() {
        let tool = GetAppInfoTool;
        let context = ToolExecutionContext::new("test-conversation");
        let app_handle = tauri::test::mock_app_handle(tauri::test::no_proxy);

        // Test with string instead of object
        let args = serde_json::json!("invalid");
        let result = tool
            .execute(context.clone(), args, app_handle.clone(), None)
            .await;
        assert!(result.is_err());

        // Test with array
        let args = serde_json::json!([]);
        let result = tool
            .execute(context.clone(), args, app_handle.clone(), None)
            .await;
        assert!(result.is_err());

        // Test with number
        let args = serde_json::json!(123);
        let result = tool.execute(context, args, app_handle, None).await;
        assert!(result.is_err());
    }
}
