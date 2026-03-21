//! Get application information tool.

use crate::tools::{
    context::ToolExecutionContext, error::ToolError, traits::{Tool, ToolFuture, ToolId, ToolMeta},
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
