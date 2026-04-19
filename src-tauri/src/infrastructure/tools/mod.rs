//! Tool registry and execution framework.
//!
//! Provides a centralized system for managing LLM tools organized by category.

pub mod constants;
pub mod context;
pub mod error;
pub mod traits;

pub mod agent;
pub mod conversation;
pub mod documents;
pub mod fs;
pub mod search;
pub mod system;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tauri::AppHandle;

use self::constants::{is_env_enabled, ENV_WORKSPACE_ROOT};
use self::context::ToolExecutionContext;
use crate::core::llm::entities::{ToolCall, ToolDefinition, ToolResult};

pub use self::error::ToolError;
pub use self::traits::{Tool, ToolCategory, ToolFuture};

/// Orchestrator tool result (internal type).
#[derive(Debug, Clone)]
pub struct OrchestratorToolResult {
    /// The unique identifier for this tool call.
    pub call_id: String,
    /// The name of the tool that was called.
    pub name: String,
    /// The JSON output from the tool execution.
    pub output: serde_json::Value,
    /// Whether the tool execution resulted in an error.
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
        registry.register_category(agent::AgentCategory);
        registry.register_category(documents::docx::DocxCategory);
        registry.register_category(documents::pptx::PptxCategory);
        registry.register_category(documents::pdf::PdfCategory);

        Ok(registry)
    }

    /// Set workspace root for tools that require it.
    #[must_use]
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
    ///
    /// Definitions are returned sorted by name so adjacent LLM requests see a
    /// stable tool catalog (M1.1). Call sites that filter or copy this list
    /// must preserve the canonical order — the plain `Vec::iter().filter(...)`
    /// pattern is sufficient and is the only downstream shape used today.
    #[must_use]
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        let mut defs: Vec<ToolDefinition> = self
            .tools
            .values()
            .filter(|tool| self.is_tool_enabled(tool.as_ref()))
            .map(|tool| {
                let meta = tool.meta();
                ToolDefinition {
                    name: meta.id.full_name(),
                    description: meta.description.to_string(),
                    input_schema: meta.input_schema,
                }
            })
            .collect();
        defs.sort_by(|a, b| a.name.cmp(&b.name));
        defs
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
        let effective_workspace = context
            .workspace_root
            .as_ref()
            .or(self.workspace_root.as_ref());
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
        let workspace_for_tool = context
            .workspace_root
            .clone()
            .or(self.workspace_root.clone());
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
                output: e.into_tool_output(),
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
            ToolError::failed(format!("Failed to resolve workspace root '{path}': {e}"))
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
        assert!(tool_names.contains(&"fs_glob_files".to_string()));
        assert!(tool_names.contains(&"fs_stat_path".to_string()));
        assert!(tool_names.contains(&"fs_replace_in_file".to_string()));
        assert!(tool_names.contains(&"fs_fuzzy_find_files".to_string()));
        assert!(tool_names.contains(&"fs_apply_patch".to_string()));
        assert!(tool_names.contains(&"conversation_search_messages".to_string()));
        assert!(tool_names.contains(&"docx_start".to_string()));
        assert!(tool_names.contains(&"docx_add_section".to_string()));
        assert!(tool_names.contains(&"pptx_start".to_string()));
        assert!(tool_names.contains(&"pptx_add_slide".to_string()));
        assert!(tool_names.contains(&"pdf_start".to_string()));
        assert!(tool_names.contains(&"pdf_add_page".to_string()));
    }

    #[test]
    fn test_tool_names_include_category_prefix() {
        let registry = ToolRegistry::new().unwrap();
        let defs = registry.definitions();

        // All tools should be prefixed with category
        for def in defs {
            assert!(
                def.name.contains('_'),
                "Tool name should have category prefix: {}",
                def.name
            );
        }
    }

    #[test]
    fn test_tool_definitions_are_sorted_by_name() {
        // M1.1: adjacent LLM requests must see a deterministic tool catalog so
        // the prompt-prefix cache stays warm. This test pins that contract at
        // the registry level — if a future refactor swaps the canonical order
        // (e.g., for category-first grouping), this assertion must be updated
        // in the same change that adjusts downstream hashing.
        let registry = ToolRegistry::new().unwrap();
        let names: Vec<_> = registry
            .definitions()
            .iter()
            .map(|d| d.name.clone())
            .collect();

        let mut expected = names.clone();
        expected.sort();

        assert_eq!(
            names, expected,
            "definitions() must return tool definitions sorted by name"
        );
    }

    #[test]
    fn test_repeated_registry_construction_yields_identical_ordering() {
        // HashMap-backed storage historically produced a different iteration
        // order on each construction. After M1.1 the observed order must be
        // stable across constructions in the same process — this is the
        // characterization test behind the "tool-catalog hash churn" signal
        // in docs/planning/agent-context-engineering/measurement-harness.md.
        let first = ToolRegistry::new().unwrap();
        let second = ToolRegistry::new().unwrap();

        let first_names: Vec<_> = first.definitions().iter().map(|d| d.name.clone()).collect();
        let second_names: Vec<_> = second
            .definitions()
            .iter()
            .map(|d| d.name.clone())
            .collect();

        assert_eq!(first_names, second_names);
    }

    #[test]
    fn test_filter_over_definitions_preserves_canonical_order() {
        // Downstream sites (context_phase::resolve_context) filter the catalog
        // by a subset of tool names. `Vec::iter().filter(...)` preserves source
        // order, so the filtered output must also be sorted.
        let registry = ToolRegistry::new().unwrap();
        let all = registry.definitions();

        // Pick an arbitrary subset in non-sorted order.
        let subset_names = ["fs_write_file", "agent_emit_status", "fs_read_file"];
        let filtered: Vec<_> = all
            .iter()
            .filter(|def| subset_names.iter().any(|name| *name == def.name))
            .map(|def| def.name.clone())
            .collect();

        let mut expected = filtered.clone();
        expected.sort();

        assert_eq!(
            filtered, expected,
            "filter over definitions() must yield a sorted subset"
        );
    }

    #[test]
    fn test_search_web_search_is_registered_by_default() {
        // The tool is on-by-default: it appears in the LLM catalog without
        // any env var having to be set. `not_configured` (the "no credentials
        // yet" branch) is the effective gate; covered by the tool's own
        // tests.
        use crate::infrastructure::tools::constants::{
            web_search_test_env_lock, ENV_WEB_SEARCH_ENABLED,
        };

        let _guard = web_search_test_env_lock()
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        std::env::remove_var(ENV_WEB_SEARCH_ENABLED);
        let registry = ToolRegistry::new().unwrap();
        let names: Vec<_> = registry
            .definitions()
            .iter()
            .map(|d| d.name.clone())
            .collect();
        assert!(
            names.contains(&"search_web_search".to_string()),
            "tool must appear even when the legacy flag is unset"
        );
    }
}
