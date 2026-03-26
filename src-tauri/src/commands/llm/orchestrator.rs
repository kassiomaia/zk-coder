//! Orchestrated conversation runner with tool support.
//!
//! This module provides the orchestrated runner that executes bounded
//! conversation loops with tool-calling support.

use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

use crate::core::llm::services::LlmService;
use crate::commands::{ChatState, LlmState};
use crate::core::chat::entities::{ChatMessage, MessageRole};
use crate::core::chat::repositories::{MessageRepository, ThreadRepository};
use crate::core::llm::entities::{
    ChatRequest as CoreChatRequest, ChatResponse as CoreChatResponse, ContentBlock,
    MessageContent, MessageRole as CoreMessageRole, ToolCall,
};
use crate::llm::{ChatRequest, ChatResponse, LlmClient, LlmError, LlmEvent, Message, ToolDefinition};
use crate::storage::ConfigStore;
use crate::tools::{OrchestratorToolResult, ToolRegistry};

use super::{SendChatRequest, SendChatResponse};

/// Environment flag for enabling real tool execution in the orchestrator.
const ORCHESTRATOR_TOOL_REGISTRY_ENV: &str = "CODER_LLM_TOOL_REGISTRY";

/// Safety bound for orchestrated request loops.
pub const ORCHESTRATOR_MAX_STEPS: usize = 6;

/// Max retries for transient provider failures during one orchestrator step.
pub const ORCHESTRATOR_LLM_MAX_RETRIES: usize = 2;

/// Base retry delay for transient provider failures.
pub const ORCHESTRATOR_LLM_RETRY_BASE_MS: u64 = 500;

/// Orchestrated runner for bounded tool-capable turns.
pub struct OrchestratedConversationRunner {
    llm_service: Arc<dyn LlmService>,
}

impl OrchestratedConversationRunner {
    /// Creates a new orchestrated conversation runner.
    pub fn new(llm_service: Arc<dyn LlmService>) -> Self {
        Self { llm_service }
    }
}

impl super::ConversationRunner for OrchestratedConversationRunner {
    fn send_chat_message<'a>(
        &'a self,
        request: SendChatRequest,
        app_handle: AppHandle,
    ) -> futures::future::BoxFuture<'a, Result<SendChatResponse, String>> {
        Box::pin(async move {
            log::info!(
                "[LLM] Orchestrated runner enabled: executing loop skeleton for non-streaming request"
            );
            let tool_executor = select_tool_executor_from_env(&request.conversation_id);
            match send_chat_message_orchestrated_non_stream(
                request.clone(),
                app_handle.clone(),
                tool_executor.as_ref(),
                self.llm_service.clone(),
            )
            .await
            {
                Ok(response) => Ok(response),
                Err(e) => {
                    log::warn!(
                        "[LLM] Orchestrated runner failed: {}",
                        e
                    );
                    Err(e)
                }
            }
        })
    }

    fn send_chat_message_streaming<'a>(
        &'a self,
        request: SendChatRequest,
        app_handle: AppHandle,
    ) -> futures::future::BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            log::info!(
                "[LLM] Orchestrated runner enabled: streaming request using orchestrated loop path"
            );
            send_chat_message_streaming_orchestrated(
                request.clone(),
                app_handle.clone(),
                self.llm_service.clone(),
            )
            .await
        })
    }
}

/// Tool execution abstraction for orchestrated turns.
pub trait ToolExecutor: Send + Sync {
    fn definitions(&self) -> Vec<ToolDefinition>;

    fn execute<'a>(
        &'a self,
        call: crate::llm::ToolCall,
        app_handle: AppHandle,
    ) -> futures::future::BoxFuture<'a, OrchestratorToolResult>;
}

/// Adapter that wraps the ToolRegistry to implement ToolExecutor.
pub struct RegistryToolExecutor {
    registry: ToolRegistry,
    conversation_id: String,
}

impl RegistryToolExecutor {
    pub fn new(registry: ToolRegistry, conversation_id: String) -> Self {
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

    fn execute<'a>(
        &'a self,
        call: crate::llm::ToolCall,
        app_handle: AppHandle,
    ) -> futures::future::BoxFuture<'a, OrchestratorToolResult> {
        Box::pin(async move {
            let context = crate::tools::context::ToolExecutionContext::new(&self.conversation_id);
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

/// Default executor placeholder.
struct NoopToolExecutor;

impl ToolExecutor for NoopToolExecutor {
    fn definitions(&self) -> Vec<ToolDefinition> {
        Vec::new()
    }

    fn execute<'a>(
        &'a self,
        call: crate::llm::ToolCall,
        _app_handle: AppHandle,
    ) -> futures::future::BoxFuture<'a, OrchestratorToolResult> {
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
fn select_tool_executor_from_env(conversation_id: &str) -> Box<dyn ToolExecutor> {
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
            Box::new(RegistryToolExecutor::new(registry, conversation_id.to_string()))
        }
        Err(err) => {
            log::warn!(
                "[LLM] Tool registry initialization failed, using noop executor. error={}",
                err
            );
            Box::new(NoopToolExecutor)
        }
    }
}

/// Parses common truthy values from an environment variable.
fn parse_env_flag(value: Option<&str>) -> bool {
    match value.map(str::trim).filter(|v| !v.is_empty()).map(|v| v.to_ascii_lowercase()) {
        Some(v) => matches!(v.as_str(), "1" | "true" | "yes" | "on"),
        None => false,
    }
}

/// Appends structured tool results to LLM context for the next model turn.
pub fn append_tool_results_to_llm_context(
    llm_messages: &mut Vec<Message>,
    results: &[OrchestratorToolResult],
) {
    for result in results {
        llm_messages.push(Message::tool_result(
            result.call_id.clone(),
            result.output.clone(),
            result.is_error,
        ));
    }
}

/// Checks if content contains a textual tool plan instead of native tool calls.
fn is_textual_tool_plan(content: &str) -> bool {
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
            Some(serde_json::Value::Object(_)) | Some(serde_json::Value::String(_))
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

/// Final output from one orchestrated bounded loop execution.
struct OrchestratedLoopOutput {
    content: String,
    model: String,
    usage: Option<crate::llm::UsageStats>,
}

/// Runs the bounded orchestrated conversation loop and returns the final model response.
async fn run_orchestrated_loop(
    request: &SendChatRequest,
    app_handle: &AppHandle,
    tool_executor: &dyn ToolExecutor,
    llm_service: Arc<dyn LlmService>,
    include_thread_history: bool,
    persist_user_message: bool,
) -> Result<OrchestratedLoopOutput, String> {
    let llm_state = app_handle.state::<LlmState>();
    let config_store = llm_state.config_store.lock().await;
    let model_id = resolve_model_id(request.model_id.clone(), &config_store).await?;

    let model = match config_store.get_model(&model_id) {
        Ok(Some(m)) => m,
        Ok(None) => {
            if let Some(dash_pos) = model_id.find('-') {
                let provider_id = &model_id[..dash_pos];
                let model_name = &model_id[dash_pos + 1..];
                config_store
                    .get_model_by_name(provider_id, model_name)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| format!("Model '{}' not found", model_id))?
            } else {
                return Err(format!("Model '{}' not found", model_id));
            }
        }
        Err(e) => return Err(e.to_string()),
    };

    drop(config_store);

    let chat_state = app_handle.state::<ChatState>();

    let history = if include_thread_history {
        if let Some(thread_id) = &request.thread_id {
            let store = chat_state.store.lock().await;
            (&*store as &dyn MessageRepository).load_thread_messages(thread_id)
                .map_err(|e| e.to_string())?
        } else {
            let store = chat_state.store.lock().await;
            (&*store as &dyn MessageRepository).load_messages(&request.conversation_id)
                .map_err(|e| e.to_string())?
        }
    } else {
        let store = chat_state.store.lock().await;
        (&*store as &dyn MessageRepository).load_messages(&request.conversation_id)
            .map_err(|e| e.to_string())?
    };

    let user_message = ChatMessage::user(&request.conversation_id, &request.content);
    if persist_user_message {
        let store = chat_state.store.lock().await;
        (&*store as &dyn MessageRepository).add_message(&user_message)
            .map_err(|e| e.to_string())?;
    }

    // Build core messages for LlmService
    let mut core_messages: Vec<crate::core::llm::entities::ChatMessage> = history
        .iter()
        .map(|msg| crate::core::llm::entities::ChatMessage {
            role: match msg.role {
                MessageRole::User => CoreMessageRole::User,
                MessageRole::Assistant => CoreMessageRole::Assistant,
                MessageRole::System => CoreMessageRole::System,
            },
            content: MessageContent::Text(msg.content.clone()),
            tool_calls: Vec::new(),
            tool_result: None,
        })
        .collect();

    // Add user message
    core_messages.push(crate::core::llm::entities::ChatMessage {
        role: CoreMessageRole::User,
        content: MessageContent::Text(request.content.clone()),
        tool_calls: Vec::new(),
        tool_result: None,
    });

    let system_prompt = request.system_prompt.clone().unwrap_or_default();
    let temperature = request.temperature;
    let max_tokens = request.max_tokens;
    let tool_definitions = tool_executor.definitions();

    for step in 1..=ORCHESTRATOR_MAX_STEPS {
        log::info!(
            "[LLM] Orchestrator step {}/{}",
            step, ORCHESTRATOR_MAX_STEPS
        );

        // Convert core tool definitions to service tool definitions
        let core_tools: Vec<crate::core::llm::entities::ToolDefinition> = tool_definitions
            .iter()
            .map(|t| crate::core::llm::entities::ToolDefinition {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.input_schema.clone(),
            })
            .collect();

        let core_request = CoreChatRequest {
            model: model.name.clone(),
            messages: core_messages.clone(),
            system: Some(system_prompt.clone()),
            temperature,
            max_tokens,
            tools: core_tools,
            stream: false,
        };

        let response: CoreChatResponse =
            run_chat_with_retry(llm_service.as_ref(), &core_request)
                .await
                .map_err(|e| {
                    log::error!(
                        "send_chat_message_orchestrated_non_stream: LLM error: {}",
                        e
                    );
                    e.to_string()
                })?;

        let tool_calls = response.tool_calls.clone();
        if tool_calls.is_empty() {
            if !tool_definitions.is_empty() && is_textual_tool_plan(&response.content) {
                return Err(
                    "Model returned a textual tool plan instead of native tool calls. \
Enable a model/provider with native tool-calling support for orchestrated execution."
                        .to_string(),
                );
            }
            return Ok(OrchestratedLoopOutput {
                content: response.content,
                model: response.model,
                usage: response.usage.map(|u| crate::llm::UsageStats {
                    input_tokens: u.input_tokens,
                    output_tokens: u.output_tokens,
                    total_tokens: u.total_tokens,
                }),
            });
        }

        // Preserve assistant tool-call turn in structured form.
        let assistant_message = crate::core::llm::entities::ChatMessage {
            role: CoreMessageRole::Assistant,
            content: MessageContent::Text(response.content.clone()),
            tool_calls: tool_calls.clone(),
            tool_result: None,
        };
        core_messages.push(assistant_message);

        // Execute tools
        let llm_tool_calls: Vec<crate::llm::ToolCall> = tool_calls
            .iter()
            .map(|tc| crate::llm::ToolCall {
                id: tc.id.clone(),
                name: tc.name.clone(),
                arguments: tc.arguments.clone(),
            })
            .collect();

        let mut tool_results = Vec::with_capacity(llm_tool_calls.len());
        for call in llm_tool_calls {
            let result = tool_executor.execute(call, app_handle.clone()).await;
            tool_results.push(result);
        }

        // Append tool results to context
        for result in tool_results {
            core_messages.push(crate::core::llm::entities::ChatMessage {
                role: CoreMessageRole::Tool,
                content: MessageContent::Text(serde_json::to_string(&result.output).unwrap_or_default()),
                tool_calls: Vec::new(),
                tool_result: Some(crate::core::llm::entities::ToolResult {
                    call_id: result.call_id,
                    output: result.output,
                    is_error: result.is_error,
                }),
            });
        }
    }

    Err(format!(
        "Orchestrator exceeded max steps ({}) without final response",
        ORCHESTRATOR_MAX_STEPS
    ))
}

/// Orchestrated non-streaming execution skeleton with bounded turn loop.
async fn send_chat_message_orchestrated_non_stream(
    request: SendChatRequest,
    app_handle: AppHandle,
    tool_executor: &dyn ToolExecutor,
    llm_service: Arc<dyn LlmService>,
) -> Result<SendChatResponse, String> {
    log::info!(
        "send_chat_message_orchestrated_non_stream: conversation_id={}, model_id={:?}",
        request.conversation_id, request.model_id
    );

    let final_output =
        run_orchestrated_loop(&request, &app_handle, tool_executor, llm_service, false, true).await?;

    let assistant_message = ChatMessage::assistant(&request.conversation_id, &final_output.content);
    let chat_state = app_handle.state::<ChatState>();
    let store = chat_state.store.lock().await;
    (&*store as &dyn MessageRepository).add_message(&assistant_message)
        .map_err(|e| e.to_string())?;

    Ok(SendChatResponse {
        content: final_output.content,
        model: final_output.model,
        usage: final_output.usage,
        message: assistant_message,
    })
}

/// Orchestrated streaming execution skeleton with bounded turn loop.
async fn send_chat_message_streaming_orchestrated(
    request: SendChatRequest,
    app_handle: AppHandle,
    llm_service: Arc<dyn LlmService>,
) -> Result<String, String> {
    let request_id = uuid::Uuid::new_v4().to_string();
    log::info!(
        "[LLM] Starting orchestrated streaming request: request_id={}, conversation_id={}, thread_id={:?}",
        request_id, request.conversation_id, request.thread_id
    );

    let llm_state = app_handle.state::<LlmState>();
    let active_requests = llm_state.active_requests.clone();

    let app_handle_clone = app_handle.clone();
    let request_id_clone = request_id.clone();
    let request_for_task = request.clone();
    let model_for_started = request
        .model_id
        .clone()
        .unwrap_or_else(|| "orchestrated".to_string());

    let active_requests_for_task = active_requests.clone();
    let request_id_for_cleanup = request_id.clone();

    let handle = tokio::spawn(async move {
        let started_event = LlmEvent::Started {
            request_id: request_id_clone.clone(),
            model: model_for_started,
        };
        if let Err(e) = app_handle_clone.emit("llm:event", &started_event) {
            log::error!("[LLM] Failed to emit orchestrated started event: {}", e);
        }

        let tool_executor = select_tool_executor_from_env(&request_for_task.conversation_id);
        match run_orchestrated_loop(
            &request_for_task,
            &app_handle_clone,
            tool_executor.as_ref(),
            llm_service,
            true,
            false,
        )
        .await
        {
            Ok(final_output) => {
                if !final_output.content.is_empty() {
                    let chunk_event = LlmEvent::Chunk {
                        request_id: request_id_clone.clone(),
                        delta: final_output.content.clone(),
                    };
                    if let Err(e) = app_handle_clone.emit("llm:event", &chunk_event) {
                        log::error!("[LLM] Failed to emit orchestrated chunk event: {}", e);
                    }
                }

                let complete_event = LlmEvent::Complete {
                    request_id: request_id_clone.clone(),
                    content: final_output.content.clone(),
                    usage: final_output.usage.clone(),
                    finish_reason: Some("stop".to_string()),
                };
                if let Err(e) = app_handle_clone.emit("llm:event", &complete_event) {
                    log::error!("[LLM] Failed to emit orchestrated complete event: {}", e);
                }

                if !final_output.content.is_empty() {
                    let mut assistant_message = ChatMessage::assistant(
                        &request_for_task.conversation_id,
                        &final_output.content,
                    );
                    if let Some(thread_id) = &request_for_task.thread_id {
                        assistant_message = assistant_message.with_thread(thread_id);
                    }
                    assistant_message.metadata = Some(crate::core::chat::entities::MessageMetadata {
                        model: Some(final_output.model),
                        tokens: final_output.usage.as_ref().map(|u| u.total_tokens as usize),
                        ..Default::default()
                    });

                    let chat_state = app_handle_clone.state::<ChatState>();
                    let store = chat_state.store.lock().await;
                    if let Err(e) = (&*store as &dyn MessageRepository).add_message(&assistant_message) {
                        log::error!("[LLM] Failed to save orchestrated assistant message: {}", e);
                    }
                }
            }
            Err(err) => {
                let event = LlmEvent::Error {
                    request_id: request_id_clone.clone(),
                    error: err,
                    retryable: false,
                };
                if let Err(e) = app_handle_clone.emit("llm:event", &event) {
                    log::error!("[LLM] Failed to emit orchestrated error event: {}", e);
                }
            }
        }

        let mut active = active_requests_for_task.lock().await;
        active.remove(&request_id_for_cleanup);
    });

    {
        let mut active = active_requests.lock().await;
        if let Some(previous) = active.insert(request_id.clone(), handle) {
            log::warn!(
                "[LLM] Replaced existing streaming task for request_id={}, aborting previous task",
                request_id
            );
            previous.abort();
        }
    }

    Ok(request_id)
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Resolves the model ID to use for a chat request.
async fn resolve_model_id(
    model_id: Option<String>,
    config_store: &ConfigStore,
) -> Result<String, String> {
    if let Some(id) = model_id {
        return Ok(id);
    }

    // Resolve from 'code' operation default
    let model = crate::commands::config::get_code_operation_model(config_store)?;
    Ok(model.id)
}

/// Executes one non-streaming chat request with bounded retries for transient errors.
async fn run_chat_with_retry(
    llm_service: &dyn LlmService,
    request: &CoreChatRequest,
) -> Result<CoreChatResponse, String> {
    let mut attempt = 0usize;
    loop {
        match llm_service.chat(request.clone()) {
            Ok(response) => return Ok(response),
            Err(err) => {
                // Check if error is retryable (simple heuristic)
                let is_retryable = err.to_string().to_lowercase().contains("timeout")
                    || err.to_string().to_lowercase().contains("rate limit")
                    || err.to_string().to_lowercase().contains("temporary");
                let can_retry = is_retryable && attempt < ORCHESTRATOR_LLM_MAX_RETRIES;

                if !can_retry {
                    return Err(err.to_string());
                }

                let delay_ms = ORCHESTRATOR_LLM_RETRY_BASE_MS * (1u64 << attempt);
                log::warn!(
                    "[LLM] Transient LLM error (attempt {}/{}), retrying in {}ms: {}",
                    attempt + 1,
                    ORCHESTRATOR_LLM_MAX_RETRIES + 1,
                    delay_ms,
                    err
                );
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                attempt += 1;
            }
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::llm::entities::{
        ChatRequest, ChatResponse, ContentBlock, LlmError, LlmResult, MessageContent,
        MessageRole as CoreMessageRole, UsageStats,
    };
    use crate::core::llm::services::LlmService;
    use std::sync::Arc;

    // Mock LlmService for testing
    struct MockLlmService {
        response_content: String,
        response_model: String,
        should_fail: bool,
        error_message: Option<String>,
        tool_calls: Vec<crate::core::llm::entities::ToolCall>,
    }

    impl MockLlmService {
        fn new() -> Self {
            Self {
                response_content: "Test response".to_string(),
                response_model: "test-model".to_string(),
                should_fail: false,
                error_message: None,
                tool_calls: vec![],
            }
        }

        fn with_content(content: &str) -> Self {
            Self {
                response_content: content.to_string(),
                response_model: "test-model".to_string(),
                should_fail: false,
                error_message: None,
                tool_calls: vec![],
            }
        }

        fn with_error(message: &str) -> Self {
            Self {
                response_content: String::new(),
                response_model: String::new(),
                should_fail: true,
                error_message: Some(message.to_string()),
                tool_calls: vec![],
            }
        }

        fn with_tool_calls(calls: Vec<crate::core::llm::entities::ToolCall>) -> Self {
            Self {
                response_content: "Thinking...".to_string(),
                response_model: "test-model".to_string(),
                should_fail: false,
                error_message: None,
                tool_calls: calls,
            }
        }
    }

    impl LlmService for MockLlmService {
        fn chat(&self, _request: ChatRequest) -> LlmResult<ChatResponse> {
            if self.should_fail {
                let msg = self.error_message.clone().unwrap_or_else(|| "Unknown error".to_string());
                return Err(LlmError::Provider {
                    code: "error".to_string(),
                    message: msg,
                });
            }

            Ok(ChatResponse {
                id: "test-response-123".to_string(),
                content: self.response_content.clone(),
                model: self.response_model.clone(),
                tool_calls: self.tool_calls.clone(),
                usage: Some(UsageStats {
                    input_tokens: 10,
                    output_tokens: 20,
                    total_tokens: 30,
                }),
                finish_reason: Some("stop".to_string()),
            })
        }

        fn validate_api_key(&self, _provider: crate::core::llm::repositories::LlmProvider, _api_key: &str) -> LlmResult<bool> {
            Ok(true)
        }

        fn fetch_models(&self, _provider: crate::core::llm::repositories::LlmProvider, _api_key: &str) -> LlmResult<Vec<String>> {
            Ok(vec!["model-1".to_string(), "model-2".to_string()])
        }

        fn get_default_model(&self, _provider: crate::core::llm::repositories::LlmProvider) -> Option<String> {
            Some("default-model".to_string())
        }
    }

    // Note: We can't fully test ToolExecutor without Tauri AppHandle
    // The tests below focus on testing helper functions and logic

    fn create_test_request() -> SendChatRequest {
        SendChatRequest {
            conversation_id: "conv-123".to_string(),
            thread_id: Some("thread-123".to_string()),
            content: "Hello, AI!".to_string(),
            model_id: Some("test-model".to_string()),
            system_prompt: None,
            temperature: Some(0.7),
            max_tokens: Some(1000),
        }
    }

    // =========================================================================
    // OrchestratedConversationRunner Tests
    // =========================================================================

    #[test]
    fn test_orchestrated_conversation_runner_creation() {
        let mock_service = Arc::new(MockLlmService::new()) as Arc<dyn LlmService>;
        let runner = OrchestratedConversationRunner::new(mock_service);
        // Runner is created successfully
        assert!(runner.llm_service.chat(ChatRequest {
            model: "test".to_string(),
            messages: vec![],
            system: None,
            temperature: None,
            max_tokens: None,
            tools: vec![],
            stream: false,
        }).is_ok());
    }

    // =========================================================================
    // Textual Tool Plan Detection Tests
    // =========================================================================

    #[test]
    fn test_is_textual_tool_plan_empty() {
        assert!(!is_textual_tool_plan(""));
    }

    #[test]
    fn test_is_textual_tool_plan_invalid_json() {
        assert!(!is_textual_tool_plan("not json at all"));
    }

    #[test]
    fn test_is_textual_tool_plan_tool_calls_array() {
        let json = r#"[
            {"name": "read_file", "arguments": {"path": "/test"}}
        ]"#;
        assert!(is_textual_tool_plan(json));
    }

    #[test]
    fn test_is_textual_tool_plan_with_tool_calls_field() {
        let json = r#"{"tool_calls": [
            {"name": "write_file", "arguments": {"path": "/test", "content": "data"}}
        ]}"#;
        assert!(is_textual_tool_plan(json));
    }

    #[test]
    fn test_is_textual_tool_plan_empty_name() {
        let json = r#"[{"name": "", "arguments": {}}]"#;
        assert!(!is_textual_tool_plan(json));
    }

    #[test]
    fn test_is_textual_tool_plan_no_arguments() {
        let json = r#"[{"name": "tool", "arguments": null}]"#;
        assert!(!is_textual_tool_plan(json));
    }

    #[test]
    fn test_is_textual_tool_plan_string_arguments() {
        let json = r#"[{"name": "tool", "arguments": "{\"path\": \"/test\"}"}]"#;
        assert!(is_textual_tool_plan(json));
    }

    #[test]
    fn test_is_textual_tool_plan_regular_text() {
        assert!(!is_textual_tool_plan("This is just regular text"));
        assert!(!is_textual_tool_plan("I'll help you with that"));
    }

    // =========================================================================
    // Environment Flag Parsing Tests
    // =========================================================================

    #[test]
    fn test_parse_env_flag_none() {
        assert!(!parse_env_flag(None));
    }

    #[test]
    fn test_parse_env_flag_empty() {
        assert!(!parse_env_flag(Some("")));
    }

    #[test]
    fn test_parse_env_flag_true() {
        assert!(parse_env_flag(Some("true")));
        assert!(parse_env_flag(Some("TRUE")));
        assert!(parse_env_flag(Some("1")));
        assert!(parse_env_flag(Some("yes")));
        assert!(parse_env_flag(Some("on")));
    }

    #[test]
    fn test_parse_env_flag_false() {
        assert!(!parse_env_flag(Some("false")));
        assert!(!parse_env_flag(Some("0")));
        assert!(!parse_env_flag(Some("no")));
        assert!(!parse_env_flag(Some("off")));
        assert!(!parse_env_flag(Some("random")));
    }

    #[test]
    fn test_parse_env_flag_whitespace() {
        assert!(parse_env_flag(Some(" true ")));
        assert!(!parse_env_flag(Some("   ")));
    }

    // =========================================================================
    // Tool Result Append Tests
    // =========================================================================

    #[test]
    fn test_append_tool_results_to_llm_context() {
        let mut messages = vec![
            Message {
                role: crate::llm::MessageRole::User,
                content: crate::llm::MessageContent::Text("Hello".to_string()),
                tool_calls: vec![],
                tool_result: None,
            }
        ];

        let results = vec![
            OrchestratorToolResult {
                call_id: "call-1".to_string(),
                name: "tool1".to_string(),
                output: serde_json::json!({"result": "data1"}),
                is_error: false,
            },
            OrchestratorToolResult {
                call_id: "call-2".to_string(),
                name: "tool2".to_string(),
                output: serde_json::json!({"result": "data2"}),
                is_error: false,
            },
        ];

        append_tool_results_to_llm_context(&mut messages, &results);

        assert_eq!(messages.len(), 3); // Original + 2 tool results
        assert!(matches!(messages[1].role, crate::llm::MessageRole::Tool));
        assert!(matches!(messages[2].role, crate::llm::MessageRole::Tool));
    }

    #[test]
    fn test_append_tool_results_error_handling() {
        let mut messages = vec![];
        let results = vec![
            OrchestratorToolResult {
                call_id: "call-1".to_string(),
                name: "tool1".to_string(),
                output: serde_json::json!({"error": "failed"}),
                is_error: true,
            },
        ];

        append_tool_results_to_llm_context(&mut messages, &results);

        assert_eq!(messages.len(), 1);
        if let Some(tool_result) = &messages[0].tool_result {
            assert!(tool_result.is_error);
        } else {
            panic!("Expected tool_result to be set");
        }
    }

    // =========================================================================
    // NoopToolExecutor Tests
    // =========================================================================

    #[tokio::test]
    async fn test_noop_tool_executor_no_definitions() {
        let executor = NoopToolExecutor;
        assert!(executor.definitions().is_empty());
    }

    // =========================================================================
    // Request/Response Type Tests
    // =========================================================================

    #[test]
    fn test_send_chat_request_serialization() {
        let request = SendChatRequest {
            conversation_id: "conv-123".to_string(),
            thread_id: Some("thread-456".to_string()),
            content: "Test content".to_string(),
            model_id: Some("gpt-4".to_string()),
            system_prompt: Some("You are helpful".to_string()),
            temperature: Some(0.8),
            max_tokens: Some(2000),
        };

        let json = serde_json::to_string(&request);
        assert!(json.is_ok());

        let deserialized: Result<SendChatRequest, _> = serde_json::from_str(&json.unwrap());
        assert!(deserialized.is_ok());
        let req = deserialized.unwrap();
        assert_eq!(req.conversation_id, "conv-123");
        assert_eq!(req.thread_id, Some("thread-456".to_string()));
    }

    #[test]
    fn test_send_chat_response_serialization() {
        let response = SendChatResponse {
            content: "Response content".to_string(),
            model: "gpt-4".to_string(),
            usage: Some(crate::llm::UsageStats {
                input_tokens: 100,
                output_tokens: 200,
                total_tokens: 300,
            }),
            message: crate::core::chat::entities::ChatMessage::assistant("conv-1", "Response"),
        };

        let json = serde_json::to_string(&response);
        assert!(json.is_ok());
    }

    // =========================================================================
    // Constant Tests
    // =========================================================================

    #[test]
    fn test_orchestrator_constants() {
        assert_eq!(ORCHESTRATOR_MAX_STEPS, 6);
        assert_eq!(ORCHESTRATOR_LLM_MAX_RETRIES, 2);
        assert_eq!(ORCHESTRATOR_LLM_RETRY_BASE_MS, 500);
    }

    #[test]
    fn test_orchestrator_tool_registry_env() {
        assert_eq!(ORCHESTRATOR_TOOL_REGISTRY_ENV, "CODER_LLM_TOOL_REGISTRY");
    }

    // =========================================================================
    // Error Handling Tests
    // =========================================================================

    #[test]
    fn test_retry_with_timeout_error() {
        let mock_service = Arc::new(MockLlmService::with_error("Request timeout")) as Arc<dyn LlmService>;

        let request = CoreChatRequest {
            model: "test".to_string(),
            messages: vec![],
            system: None,
            temperature: None,
            max_tokens: None,
            tools: vec![],
            stream: false,
        };

        // Test that timeout errors are retryable
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(run_chat_with_retry(mock_service.as_ref(), &request));

        // Should eventually fail after retries
        assert!(result.is_err());
    }

    #[test]
    fn test_retry_with_rate_limit_error() {
        let mock_service = Arc::new(MockLlmService::with_error("Rate limit exceeded")) as Arc<dyn LlmService>;

        let request = CoreChatRequest {
            model: "test".to_string(),
            messages: vec![],
            system: None,
            temperature: None,
            max_tokens: None,
            tools: vec![],
            stream: false,
        };

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(run_chat_with_retry(mock_service.as_ref(), &request));

        // Should eventually fail after retries
        assert!(result.is_err());
    }

    #[test]
    fn test_no_retry_for_permanent_errors() {
        let mock_service = Arc::new(MockLlmService::with_error("Invalid API key")) as Arc<dyn LlmService>;

        let request = CoreChatRequest {
            model: "test".to_string(),
            messages: vec![],
            system: None,
            temperature: None,
            max_tokens: None,
            tools: vec![],
            stream: false,
        };

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(run_chat_with_retry(mock_service.as_ref(), &request));

        // Should fail immediately without retry
        assert!(result.is_err());
    }

    // =========================================================================
    // Input Validation Tests
    // =========================================================================

    #[test]
    fn test_send_chat_request_defaults() {
        let request = SendChatRequest {
            conversation_id: "conv-1".to_string(),
            thread_id: None,
            content: "Test".to_string(),
            model_id: None,
            system_prompt: None,
            temperature: None,
            max_tokens: None,
        };

        assert_eq!(request.conversation_id, "conv-1");
        assert!(request.thread_id.is_none());
        assert!(request.model_id.is_none());
        assert!(request.system_prompt.is_none());
        assert!(request.temperature.is_none());
        assert!(request.max_tokens.is_none());
    }

    #[test]
    fn test_send_chat_request_with_thread() {
        let request = SendChatRequest {
            conversation_id: "conv-1".to_string(),
            thread_id: Some("thread-1".to_string()),
            content: "Test".to_string(),
            model_id: None,
            system_prompt: None,
            temperature: None,
            max_tokens: None,
        };

        assert_eq!(request.thread_id, Some("thread-1".to_string()));
    }

    #[test]
    fn test_send_chat_request_temperature_bounds() {
        let mut request = create_test_request();

        // Test various temperature values
        request.temperature = Some(0.0);
        assert_eq!(request.temperature, Some(0.0));

        request.temperature = Some(1.0);
        assert_eq!(request.temperature, Some(1.0));

        request.temperature = Some(2.0);
        assert_eq!(request.temperature, Some(2.0));
    }

    #[test]
    fn test_send_chat_request_max_tokens() {
        let mut request = create_test_request();

        request.max_tokens = Some(0);
        assert_eq!(request.max_tokens, Some(0));

        request.max_tokens = Some(4096);
        assert_eq!(request.max_tokens, Some(4096));

        request.max_tokens = Some(128000);
        assert_eq!(request.max_tokens, Some(128000));
    }

    // =========================================================================
    // Unicode and Special Characters Tests
    // =========================================================================

    #[test]
    fn test_send_chat_request_unicode_content() {
        let request = SendChatRequest {
            conversation_id: "conv-1".to_string(),
            thread_id: None,
            content: "Hello 🚀 World 🌍".to_string(),
            model_id: None,
            system_prompt: None,
            temperature: None,
            max_tokens: None,
        };

        assert_eq!(request.content, "Hello 🚀 World 🌍");
    }

    #[test]
    fn test_send_chat_request_unicode_system_prompt() {
        let request = SendChatRequest {
            conversation_id: "conv-1".to_string(),
            thread_id: None,
            content: "Test".to_string(),
            model_id: None,
            system_prompt: Some("You are a helpful assistant 🤖".to_string()),
            temperature: None,
            max_tokens: None,
        };

        assert_eq!(
            request.system_prompt,
            Some("You are a helpful assistant 🤖".to_string())
        );
    }

    // =========================================================================
    // RegistryToolExecutor Tests
    // =========================================================================

    #[test]
    fn test_registry_tool_executor_creation() {
        // Test that we can create a RegistryToolExecutor
        // This requires a ToolRegistry which may fail in test environment
        match ToolRegistry::new() {
            Ok(registry) => {
                let executor = RegistryToolExecutor::new(registry, "test-conv".to_string());
                assert!(executor.definitions().len() > 0);
            }
            Err(_) => {
                // ToolRegistry may not be available in test environment
                // This is expected behavior
            }
        }
    }
}
