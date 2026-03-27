use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::application::llm::get_code_operation_model;
use crate::commands::{ChatState, LlmState};
use crate::core::chat::entities::{ChatMessage, MessageRole};
use crate::core::llm::entities::{
    ChatRequest as CoreChatRequest, ChatResponse as CoreChatResponse, MessageContent,
    MessageRole as CoreMessageRole, ToolCall, UsageStats,
};
use crate::core::llm::services::LlmService;

use super::executors::{is_textual_tool_plan, ToolExecutor};
use crate::commands::llm::types::SendChatRequest;

/// Safety bound for orchestrated request loops.
pub const ORCHESTRATOR_MAX_STEPS: usize = 6;

/// Max retries for transient provider failures during one orchestrator step.
pub const ORCHESTRATOR_LLM_MAX_RETRIES: usize = 2;

/// Base retry delay for transient provider failures.
pub const ORCHESTRATOR_LLM_RETRY_BASE_MS: u64 = 500;

/// Final output from one orchestrated bounded loop execution.
pub struct OrchestratedLoopOutput {
    pub content: String,
    pub model: String,
    pub usage: Option<UsageStats>,
}

pub(crate) fn build_core_messages(
    history: &[ChatMessage],
    request_content: &str,
    history_includes_request: bool,
) -> Vec<crate::core::llm::entities::ChatMessage> {
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

    if !history_includes_request {
        core_messages.push(crate::core::llm::entities::ChatMessage {
            role: CoreMessageRole::User,
            content: MessageContent::Text(request_content.to_string()),
            tool_calls: Vec::new(),
            tool_result: None,
        });
    }

    core_messages
}

/// Runs the bounded orchestrated conversation loop and returns the final model response.
pub async fn run_orchestrated_loop(
    request: &SendChatRequest,
    app_handle: &AppHandle,
    tool_executor: &dyn ToolExecutor,
    llm_service: Arc<dyn LlmService>,
    include_thread_history: bool,
    persist_user_message: bool,
) -> Result<OrchestratedLoopOutput, String> {
    let llm_state = app_handle.state::<LlmState>();
    let config_repository = llm_state.config_repository.clone();
    let model_id = resolve_model_id(request.model_id.clone(), config_repository.as_ref()).await?;

    let model = match config_repository.get_model(&model_id) {
        Ok(Some(m)) => m,
        Ok(None) => {
            if let Some(dash_pos) = model_id.find('-') {
                let provider_id = &model_id[..dash_pos];
                let model_name = &model_id[dash_pos + 1..];
                config_repository
                    .get_model_by_name(provider_id, model_name)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| format!("Model '{}' not found", model_id))?
            } else {
                return Err(format!("Model '{}' not found", model_id));
            }
        }
        Err(e) => return Err(e.to_string()),
    };

    let chat_state = app_handle.state::<ChatState>();

    let history = if include_thread_history {
        if let Some(thread_id) = &request.thread_id {
            chat_state
                .service
                .load_thread_messages(thread_id)
                .map_err(|e| e.to_string())?
        } else {
            chat_state
                .service
                .load_messages(&request.conversation_id)
                .map_err(|e| e.to_string())?
        }
    } else {
        chat_state
            .service
            .load_messages(&request.conversation_id)
            .map_err(|e| e.to_string())?
    };

    let history_includes_request = include_thread_history;
    let user_message = ChatMessage::user(&request.conversation_id, &request.content);
    if persist_user_message {
        chat_state
            .service
            .add_message(&user_message)
            .map_err(|e| e.to_string())?;
    }

    let mut core_messages = build_core_messages(&history, &request.content, history_includes_request);

    let system_prompt = request.system_prompt.clone().unwrap_or_default();
    let temperature = request.temperature;
    let max_tokens = request.max_tokens;
    let tool_definitions = tool_executor.definitions();

    for step in 1..=ORCHESTRATOR_MAX_STEPS {
        log::info!(
            "[LLM] Orchestrator step {}/{}",
            step,
            ORCHESTRATOR_MAX_STEPS
        );

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
            top_p: None,
            stop: None,
            tools: core_tools,
            stream: false,
        };

        let response: CoreChatResponse =
            run_chat_with_retry(llm_service.clone(), core_request.clone())
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
                usage: response.usage.map(|u| UsageStats {
                    input_tokens: u.input_tokens,
                    output_tokens: u.output_tokens,
                    total_tokens: u.total_tokens,
                }),
            });
        }

        let assistant_message = crate::core::llm::entities::ChatMessage {
            role: CoreMessageRole::Assistant,
            content: MessageContent::Text(response.content.clone()),
            tool_calls: tool_calls.clone(),
            tool_result: None,
        };
        core_messages.push(assistant_message);

        let llm_tool_calls: Vec<ToolCall> = tool_calls
            .iter()
            .map(|tc| ToolCall {
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

        for result in tool_results {
            core_messages.push(crate::core::llm::entities::ChatMessage {
                role: CoreMessageRole::Tool,
                content: MessageContent::Text(
                    serde_json::to_string(&result.output).unwrap_or_default(),
                ),
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

/// Resolves the model ID to use for a chat request.
pub async fn resolve_model_id(
    model_id: Option<String>,
    config_repository: &dyn crate::core::config::ConfigRepository,
) -> Result<String, String> {
    if let Some(id) = model_id {
        return Ok(id);
    }

    let model = get_code_operation_model(config_repository)?;
    Ok(model.id)
}

/// Executes one non-streaming chat request with bounded retries for transient errors.
pub async fn run_chat_with_retry(
    llm_service: Arc<dyn LlmService>,
    request: CoreChatRequest,
) -> Result<CoreChatResponse, String> {
    let mut attempt = 0usize;
    loop {
        // `LlmService` is synchronous. Calling it directly from async code can
        // block a Tokio worker thread and may panic if the implementation uses
        // `block_on` internally. Run it in the dedicated blocking pool.
        let svc = llm_service.clone();
        let req = request.clone();
        let result = tokio::task::spawn_blocking(move || svc.chat(req))
            .await
            .map_err(|e| format!("LLM task failed: {e}"))?;

        match result {
            Ok(response) => return Ok(response),
            Err(err) => {
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
