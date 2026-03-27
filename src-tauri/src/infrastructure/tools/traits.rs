//! Core tool types and traits.
//!
//! This module defines the standard interface for all LLM tools in the system.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use serde_json::Value as JsonValue;
use tauri::AppHandle;

use super::context::ToolExecutionContext;
use super::error::ToolError;
use crate::core::llm::entities::ToolDefinition;

/// Unique identifier for a tool (category/name format).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ToolId {
    /// Category namespace (e.g., "fs", "search", "conversation").
    pub category: &'static str,
    /// Tool name within the category (e.g., "read_file").
    pub name: &'static str,
}

impl ToolId {
    /// Create a new tool identifier.
    #[must_use]
    pub const fn new(category: &'static str, name: &'static str) -> Self {
        Self { category, name }
    }

    /// Full tool name as exposed to LLM (e.g., "fs_read_file").
    #[must_use]
    pub fn full_name(&self) -> String {
        format!("{}_{}", self.category, self.name)
    }
}

impl std::fmt::Display for ToolId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}", self.category, self.name)
    }
}

/// Tool metadata exposed to the LLM and registry.
#[derive(Debug, Clone)]
pub struct ToolMeta {
    /// Unique tool identifier.
    pub id: ToolId,
    /// Human-readable description for the LLM.
    pub description: &'static str,
    /// JSON Schema for tool arguments.
    pub input_schema: JsonValue,
    /// Whether this tool requires workspace access.
    pub requires_workspace: bool,
    /// Whether this tool is experimental/unstable.
    pub experimental: bool,
    /// Feature flag required to enable this tool (optional).
    pub feature_flag: Option<&'static str>,
}

impl ToolMeta {
    /// Convert to LLM-compatible tool definition.
    #[must_use]
    pub fn to_definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.id.full_name(),
            description: self.description.to_string(),
            input_schema: self.input_schema.clone(),
        }
    }
}

/// Type alias for boxed async tool result.
pub type ToolFuture = Pin<Box<dyn Future<Output = Result<JsonValue, ToolError>> + Send>>;

/// Core trait for all LLM tools.
///
/// Tools are self-contained units that:
/// - Declare their metadata (name, description, schema)
/// - Execute asynchronously with typed context
/// - Return JSON results or structured errors
///
/// # Example
///
/// ```ignore
/// pub struct ReadFileTool;
///
/// impl Tool for ReadFileTool {
///     fn meta(&self) -> ToolMeta {
///         ToolMeta {
///             id: ToolId::new("fs", "read_file"),
///             description: "Read file content from workspace.",
///             input_schema: json!({ "type": "object", ... }),
///             requires_workspace: true,
///             experimental: false,
///             feature_flag: None,
///         }
///     }
///
///     fn execute(&self, ctx: ToolExecutionContext, args: JsonValue) -> ToolFuture {
///         Box::pin(async move { ... })
///     }
/// }
/// ```
pub trait Tool: Send + Sync {
    /// Returns tool metadata (name, description, schema).
    fn meta(&self) -> ToolMeta;

    /// Execute the tool with given arguments.
    ///
    /// # Arguments
    /// * `ctx` - Execution context (conversation ID, workspace root, etc.)
    /// * `arguments` - JSON arguments from the LLM
    /// * `app_handle` - Tauri app handle for state access
    /// * `workspace_root` - Workspace root path (if requires_workspace is true)
    ///
    /// # Returns
    /// * `Ok(JsonValue)` - Tool output to return to the LLM
    /// * `Err(ToolError)` - Structured error message
    fn execute(
        &self,
        ctx: ToolExecutionContext,
        arguments: JsonValue,
        app_handle: AppHandle,
        workspace_root: Option<PathBuf>,
    ) -> ToolFuture;
}

/// Category of related tools.
///
/// A category groups tools by domain (filesystem, search, conversation, etc.)
/// and provides a namespace for tool names.
pub trait ToolCategory: Send + Sync {
    /// Category identifier (e.g., "fs", "search").
    fn name(&self) -> &'static str;

    /// Human-readable category description.
    fn description(&self) -> &'static str;

    /// All tools in this category.
    fn tools(&self) -> Vec<Box<dyn Tool>>;
}
