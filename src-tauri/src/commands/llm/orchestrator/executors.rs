use tauri::AppHandle;

use crate::core::llm::entities::{ToolCall, ToolDefinition};
use crate::infrastructure::tools::{OrchestratorToolResult, ToolRegistry};

/// Environment flag for enabling real tool execution in the orchestrator.
pub const ORCHESTRATOR_TOOL_REGISTRY_ENV: &str = "CODER_LLM_TOOL_REGISTRY";

/// Tool execution abstraction for orchestrated turns.
///
/// This trait defines the interface for executing tools during an orchestrated
/// LLM conversation loop.
pub trait ToolExecutor: Send + Sync {
    /// Returns the list of tool definitions available for this executor.
    fn definitions(&self) -> Vec<ToolDefinition>;

    /// Execute a tool call and return the result.
    fn execute(
        &self,
        call: ToolCall,
        app_handle: AppHandle,
    ) -> futures::future::BoxFuture<'_, OrchestratorToolResult>;
}

/// Adapter that wraps the `ToolRegistry` to implement `ToolExecutor`.
pub struct RegistryToolExecutor {
    registry: ToolRegistry,
    conversation_id: String,
}

impl RegistryToolExecutor {
    /// Creates a new `RegistryToolExecutor` with the given registry and conversation ID.
    #[must_use] 
    pub const fn new(registry: ToolRegistry, conversation_id: String) -> Self {
        Self {
            registry,
            conversation_id,
        }
    }
}

impl ToolExecutor for RegistryToolExecutor {
    fn definitions(&self) -> Vec<ToolDefinition> {
        self.registry.definitions()
    }

    fn execute(
        &self,
        call: ToolCall,
        app_handle: AppHandle,
    ) -> futures::future::BoxFuture<'_, OrchestratorToolResult> {
        Box::pin(async move {
            let context = crate::infrastructure::tools::context::ToolExecutionContext::new(
                &self.conversation_id,
            );
            let result = self.registry.execute(call, context, app_handle).await;
            OrchestratorToolResult {
                call_id: result.call_id,
                name: String::new(),
                output: result.output,
                is_error: result.is_error,
            }
        })
    }
}

/// Default executor placeholder that returns an error for all tool calls.
///
/// This is used when tool execution is not enabled via environment configuration.
pub struct NoopToolExecutor;

impl ToolExecutor for NoopToolExecutor {
    fn definitions(&self) -> Vec<ToolDefinition> {
        Vec::new()
    }

    fn execute(
        &self,
        call: ToolCall,
        _app_handle: AppHandle,
    ) -> futures::future::BoxFuture<'_, OrchestratorToolResult> {
        Box::pin(async move {
            OrchestratorToolResult {
                call_id: call.id,
                name: call.name,
                output: serde_json::json!({
                    "error": "Tool execution not implemented yet in this environment",
                    "arguments": call.arguments
                }),
                is_error: true,
            }
        })
    }
}

/// Selects the tool executor strategy from environment configuration.
pub fn select_tool_executor_from_env(conversation_id: &str) -> Box<dyn ToolExecutor> {
    let enabled = parse_env_flag(
        std::env::var(ORCHESTRATOR_TOOL_REGISTRY_ENV)
            .ok()
            .as_deref(),
    );

    if !enabled {
        return Box::new(NoopToolExecutor);
    }

    match ToolRegistry::new() {
        Ok(registry) => {
            log::info!(
                "[LLM] {}=true -> enabling registry-backed tool execution ({} tools)",
                ORCHESTRATOR_TOOL_REGISTRY_ENV,
                registry.definitions().len()
            );
            Box::new(RegistryToolExecutor::new(
                registry,
                conversation_id.to_string(),
            ))
        }
        Err(err) => {
            log::warn!(
                "[LLM] Tool registry initialization failed, using noop executor. error={err}"
            );
            Box::new(NoopToolExecutor)
        }
    }
}

/// Parses common truthy values from an environment variable.
pub fn parse_env_flag(value: Option<&str>) -> bool {
    match value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_ascii_lowercase)
    {
        Some(v) => matches!(v.as_str(), "1" | "true" | "yes" | "on"),
        None => false,
    }
}

/// Appends structured tool results to LLM context for the next model turn.
pub fn append_tool_results_to_llm_context(
    llm_messages: &mut Vec<crate::infrastructure::llm::Message>,
    results: &[OrchestratorToolResult],
) {
    for result in results {
        llm_messages.push(crate::infrastructure::llm::Message::tool_result(
            result.call_id.clone(),
            result.output.clone(),
            result.is_error,
        ));
    }
}

/// Checks if content contains a textual tool plan instead of native tool calls.
pub fn is_textual_tool_plan(content: &str) -> bool {
    fn is_tool_call_object(value: &serde_json::Value) -> bool {
        let serde_json::Value::Object(object) = value else {
            return false;
        };
        let Some(serde_json::Value::String(name)) = object.get("name") else {
            return false;
        };
        if name.trim().is_empty() {
            return false;
        }

        matches!(
            object.get("arguments"),
            Some(serde_json::Value::Object(_) | serde_json::Value::String(_))
        )
    }

    let trimmed = content.trim();
    if trimmed.is_empty() {
        return false;
    }

    let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
        return false;
    };

    match value {
        serde_json::Value::Array(items) => {
            !items.is_empty() && items.iter().all(is_tool_call_object)
        }
        serde_json::Value::Object(object) => object
            .get("tool_calls")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|items| !items.is_empty() && items.iter().all(is_tool_call_object)),
        _ => false,
    }
}
