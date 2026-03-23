//! Tool registry and execution framework.
//!
//! Provides a centralized system for managing LLM tools organized by category.

pub mod constants;
pub mod context;
pub mod error;
pub mod traits;

pub mod conversation;
pub mod fs;
pub mod search;
pub mod system;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tauri::AppHandle;

use self::constants::*;
use self::context::ToolExecutionContext;
use crate::llm::{ToolCall, ToolResult};

pub use self::error::ToolError;
pub use self::traits::{Tool, ToolCategory, ToolFuture};

/// Orchestrator tool result (internal type).
#[derive(Debug, Clone)]
pub struct OrchestratorToolResult {
    pub call_id: String,
    pub name: String,
    pub output: serde_json::Value,
    pub is_error: bool,
}

/// Central registry for all LLM tools.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    workspace_root: Option<PathBuf>,
}

impl ToolRegistry {
    /// Create registry with all built-in categories.
    pub fn new() -> Result<Self, ToolError> {
        let mut registry = Self {
            tools: HashMap::new(),
            workspace_root: resolve_workspace_root()?,
        };

        // Register all tool categories
        registry.register_category(system::SystemCategory);
        registry.register_category(fs::FsCategory);
        registry.register_category(conversation::ConversationCategory);
        registry.register_category(search::SearchCategory);

        Ok(registry)
    }

    /// Set workspace root for tools that require it.
    pub fn with_workspace(mut self, root: PathBuf) -> Self {
        self.workspace_root = Some(root);
        self
    }

    /// Register all tools from a category.
    fn register_category(&mut self, category: impl ToolCategory) {
        for tool in category.tools() {
            let meta = tool.meta();
            let name = meta.id.full_name();
            self.tools.insert(name, Arc::from(tool));
        }
    }

    /// Get all tool definitions for the LLM.
    pub fn definitions(&self) -> Vec<crate::llm::ToolDefinition> {
        self.tools
            .values()
            .filter(|tool| self.is_tool_enabled(tool.as_ref()))
            .map(|tool| {
                let meta = tool.meta();
                crate::llm::ToolDefinition {
                    name: meta.id.full_name(),
                    description: meta.description.to_string(),
                    input_schema: meta.input_schema.clone(),
                }
            })
            .collect()
    }

    /// Execute a tool by name.
    pub async fn execute(
        &self,
        call: ToolCall,
        context: ToolExecutionContext,
        app_handle: AppHandle,
    ) -> ToolResult {
        let Some(tool) = self.tools.get(&call.name) else {
            let mut available = self.tools.keys().cloned().collect::<Vec<_>>();
            available.sort();
            return ToolResult {
                call_id: call.id,
                output: serde_json::json!({
                    "error": "Unknown tool name",
                    "available_tools": available,
                }),
                is_error: true,
            };
        };

        let meta = tool.meta();

        // Check feature flag
        if let Some(flag) = meta.feature_flag {
            if !is_env_enabled(flag) {
                return ToolResult {
                    call_id: call.id,
                    output: serde_json::json!({
                        "error": format!("Tool disabled. Set {}=1 to enable.", flag)
                    }),
                    is_error: true,
                };
            }
        }

        // Check workspace requirement - use context workspace first, fall back to registry
        let effective_workspace = context.workspace_root.as_ref().or(self.workspace_root.as_ref());
        if meta.requires_workspace && effective_workspace.is_none() {
            return ToolResult {
                call_id: call.id,
                output: serde_json::json!({
                    "error": "Tool requires workspace but none configured"
                }),
                is_error: true,
            };
        }

        // Execute tool - use context workspace first, fall back to registry workspace
        let workspace_for_tool = context.workspace_root.clone().or(self.workspace_root.clone());
        match tool
            .execute(context, call.arguments, app_handle, workspace_for_tool)
            .await
        {
            Ok(output) => ToolResult {
                call_id: call.id,
                output,
                is_error: false,
            },
            Err(e) => ToolResult {
                call_id: call.id,
                output: serde_json::json!({ "error": e.to_string() }),
                is_error: true,
            },
        }
    }

    /// Check if a tool is enabled (passes feature flag checks).
    fn is_tool_enabled(&self, tool: &dyn Tool) -> bool {
        let meta = tool.meta();
        match meta.feature_flag {
            Some(flag) => is_env_enabled(flag),
            None => true,
        }
    }
}

/// Resolve workspace root from environment or current directory.
fn resolve_workspace_root() -> Result<Option<PathBuf>, ToolError> {
    // Check for override environment variable
    if let Ok(path) = std::env::var(ENV_WORKSPACE_ROOT) {
        let absolute = PathBuf::from(&path).canonicalize().map_err(|e| {
            ToolError::failed(format!("Failed to resolve workspace root '{}': {}", path, e))
        })?;

        if !absolute.is_dir() {
            return Err(ToolError::failed(format!(
                "Tool workspace root is not a directory: {}",
                absolute.display()
            )));
        }

        return Ok(Some(absolute));
    }

    // Default to current directory
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_registry_builds_expected_tools() {
        let registry = ToolRegistry::new().unwrap();

        let defs = registry.definitions();
        let tool_names: Vec<_> = defs.iter().map(|d| d.name.clone()).collect();

        // Check for expected tools
        assert!(tool_names.contains(&"system_get_app_info".to_string()));
        assert!(tool_names.contains(&"fs_read_file".to_string()));
        assert!(tool_names.contains(&"fs_list_files".to_string()));
        assert!(tool_names.contains(&"conversation_search_messages".to_string()));
    }

    #[test]
    fn test_tool_names_include_category_prefix() {
        let registry = ToolRegistry::new().unwrap();
        let defs = registry.definitions();

        // All tools should be prefixed with category
        for def in defs {
            assert!(def.name.contains('_'), "Tool name should have category prefix: {}", def.name);
        }
    }
}
