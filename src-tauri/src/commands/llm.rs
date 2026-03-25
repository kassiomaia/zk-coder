//! LLM Tauri IPC commands.
//!
//! This module provides Tauri commands for LLM chat completions and streaming.

pub mod orchestrator;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use log::{error, info};
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::config::get_code_operation_model;
use crate::llm::{
    create_client, ChatRequest, ChatResponse, LlmClient, LlmEvent, Message,
};
use crate::nlp::chat::{ChatMessage, MessageFeedback, MessageRole};
use crate::nlp::config::ApiType;
use crate::storage::ConfigStore;

// Re-export ChatState for use in this module
use super::ChatState;

// Re-export orchestrator types
pub use orchestrator::{OrchestratedConversationRunner, ORCHESTRATOR_LLM_MAX_RETRIES};

/// Environment flag for enabling the orchestrated runner path.
const ORCHESTRATOR_FLAG_ENV: &str = "CODER_LLM_ORCHESTRATOR";

/// Application state for LLM commands.
pub struct LlmState {
    /// Config store for provider/model lookup
    pub config_store: Arc<Mutex<ConfigStore>>,
    /// Active streaming tasks keyed by request ID for cancellation.
    pub active_requests: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    /// Conversation runner strategy (orchestrated by default).
    pub runner: Arc<dyn ConversationRunner>,
}

impl LlmState {
    /// Creates a new LLM state.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be initialized.
    pub fn new(db_path: &str) -> Result<Self, String> {
        let config_store = ConfigStore::new(db_path).map_err(|e| e.to_string())?;
        Ok(Self {
            config_store: Arc::new(Mutex::new(config_store)),
            active_requests: Arc::new(Mutex::new(HashMap::new())),
            runner: select_runner_from_env(),
        })
    }

    /// Creates a new LLM state with an in-memory database (for testing).
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be initialized.
    pub fn in_memory() -> Result<Self, String> {
        let config_store = ConfigStore::in_memory().map_err(|e| e.to_string())?;
        Ok(Self {
            config_store: Arc::new(Mutex::new(config_store)),
            active_requests: Arc::new(Mutex::new(HashMap::new())),
            runner: select_runner_from_env(),
        })
    }
}

/// Abstraction for conversation execution strategy.
///
/// This enables branch-by-abstraction migration from the legacy single-turn
/// implementation to future orchestrated/tool-capable runners without changing
/// command signatures.
pub trait ConversationRunner: Send + Sync {
    /// Handles a non-streaming chat request.
    fn send_chat_message<'a>(
        &'a self,
        request: SendChatRequest,
        app_handle: AppHandle,
    ) -> futures::future::BoxFuture<'a, Result<SendChatResponse, String>>;

    /// Handles a streaming chat request.
    fn send_chat_message_streaming<'a>(
        &'a self,
        request: SendChatRequest,
        app_handle: AppHandle,
    ) -> futures::future::BoxFuture<'a, Result<String, String>>;
}

/// Legacy runner that preserves the current request/response behavior.
pub struct LegacyConversationRunner;

impl ConversationRunner for LegacyConversationRunner {
    fn send_chat_message<'a>(
        &'a self,
        request: SendChatRequest,
        app_handle: AppHandle,
    ) -> futures::future::BoxFuture<'a, Result<SendChatResponse, String>> {
        Box::pin(async move { send_chat_message_legacy(request, app_handle).await })
    }

    fn send_chat_message_streaming<'a>(
        &'a self,
        request: SendChatRequest,
        app_handle: AppHandle,
    ) -> futures::future::BoxFuture<'a, Result<String, String>> {
        Box::pin(async move { send_chat_message_streaming_legacy(request, app_handle).await })
    }
}

/// Parses common truthy values from an environment variable.
/// Returns false if the variable is not set or is falsy.
fn parse_env_flag(value: Option<&str>) -> bool {
    match value.map(str::trim).filter(|v| !v.is_empty()).map(|v| v.to_ascii_lowercase()) {
        Some(v) => matches!(v.as_str(), "1" | "true" | "yes" | "on"),
        None => false,
    }
}

/// Selects the runner strategy from environment configuration.
fn select_runner_from_env() -> Arc<dyn ConversationRunner> {
    if parse_env_flag(std::env::var(ORCHESTRATOR_FLAG_ENV).ok().as_deref()) {
        info!(
            "[LLM] {}=true -> using OrchestratedConversationRunner",
            ORCHESTRATOR_FLAG_ENV
        );
        Arc::new(OrchestratedConversationRunner)
    } else {
        info!("[LLM] Defaulting to OrchestratedConversationRunner (tools enabled)");
        Arc::new(OrchestratedConversationRunner)
    }
}

// =============================================================================
// Request/Response Types
// =============================================================================

/// Request to send a chat message to an LLM.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendChatRequest {
    /// Conversation ID
    pub conversation_id: String,
    /// Thread ID (optional)
    pub thread_id: Option<String>,
    /// The user's message content
    pub content: String,
    /// Model ID to use (e.g., "openai-gpt-4"). If not provided, uses the 'code' operation default.
    #[serde(default)]
    pub model_id: Option<String>,
    /// System prompt (optional)
    pub system_prompt: Option<String>,
    /// Temperature (optional)
    pub temperature: Option<f32>,
    /// Max tokens (optional)
    pub max_tokens: Option<u32>,
}

/// Response from a non-streaming chat request.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendChatResponse {
    /// The generated response content
    pub content: String,
    /// The model used
    pub model: String,
    /// Usage statistics
    pub usage: Option<crate::llm::UsageStats>,
    /// The saved assistant message
    pub message: ChatMessage,
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Resolves the model ID to use for a chat request.
/// If model_id is provided, uses it directly. Otherwise, resolves from 'code' operation.
async fn resolve_model_id(
    model_id: Option<String>,
    config_store: &ConfigStore,
) -> Result<String, String> {
    if let Some(id) = model_id {
        return Ok(id);
    }

    // Resolve from 'code' operation default
    let model = get_code_operation_model(config_store)?;
    Ok(model.id)
}

/// Gets the API key for a provider from the database.
fn get_api_key_for_provider(
    provider_id: &str,
    config_store: &ConfigStore,
) -> Result<String, String> {
    config_store.retrieve_api_key(provider_id).map_err(|e| {
        format!(
            "API key not found for provider '{}'. Please configure it first. Error: {}",
            provider_id, e
        )
    })
}

/// Creates an LLM client for the given provider.
fn create_client_for_provider(
    api_type: ApiType,
    api_key: String,
    base_url: Option<String>,
) -> Box<dyn LlmClient> {
    create_client(api_type, api_key, base_url)
}

/// Converts chat messages from storage to LLM format.
fn convert_messages(messages: &[ChatMessage]) -> Vec<Message> {
    messages
        .iter()
        .filter_map(|msg| {
            let role = match msg.role {
                MessageRole::User => crate::llm::MessageRole::User,
                MessageRole::Assistant => crate::llm::MessageRole::Assistant,
                MessageRole::System => crate::llm::MessageRole::System,
            };
            Some(Message {
                role,
                content: crate::llm::MessageContent::Text(msg.content.clone()),
                tool_calls: Vec::new(),
                tool_result: None,
            })
        })
        .collect()
}

// =============================================================================
// Feedback Context Injection
// =============================================================================

/// System prompt explaining the feedback system.
/// This is injected when feedback exists for the conversation.
const FEEDBACK_SYSTEM_PROMPT: &str = r#"The user may provide feedback (like/dislike) on earlier assistant responses. A feedback summary section will be provided, including stable IDs for reference. Use that feedback to improve your response quality and avoid repeating disliked patterns."#;

/// Maximum number of feedback items to include in context.
const FEEDBACK_LIMIT: usize = 5;

/// Builds a feedback summary string referencing message IDs.
/// Returns None if no feedback messages are provided.
fn build_feedback_summary(feedback_messages: &[(ChatMessage, MessageFeedback)]) -> Option<String> {
    if feedback_messages.is_empty() {
        return None;
    }

    let feedback_lines: Vec<String> = feedback_messages
        .iter()
        .map(|(msg, feedback)| {
            let short_id = &msg.id[..8.min(msg.id.len())];
            let action = match feedback {
                MessageFeedback::Like => "LIKED",
                MessageFeedback::Dislike => "DISLIKED",
            };
            format!("- [ID:{}] {} by user", short_id, action)
        })
        .collect();

    Some(format!(
        "Feedback on previous messages:\n{}",
        feedback_lines.join("\n")
    ))
}

/// Builds the combined system prompt including feedback context.
/// Returns the original system prompt if no feedback exists.
fn build_system_prompt_with_feedback(
    original_prompt: &str,
    feedback_summary: Option<&str>,
) -> String {
    match feedback_summary {
        Some(summary) => {
            if original_prompt.is_empty() {
                format!(
                    "{}\n\n{}\n\n{}",
                    FEEDBACK_SYSTEM_PROMPT, summary, "Now continue the conversation below:"
                )
            } else {
                format!(
                    "{}\n\n{}\n\n{}\n\n{}",
                    original_prompt,
                    FEEDBACK_SYSTEM_PROMPT,
                    summary,
                    "Now continue the conversation below:"
                )
            }
        }
        None => original_prompt.to_string(),
    }
}

// =============================================================================
// Legacy Chat Commands (non-streaming)
// =============================================================================

/// Legacy implementation for non-streaming chat requests.
async fn send_chat_message_legacy(
    request: SendChatRequest,
    app_handle: AppHandle,
) -> Result<SendChatResponse, String> {
    info!(
        "send_chat_message: conversation_id={}, model_id={:?}",
        request.conversation_id, request.model_id
    );

    // Get config store
    let llm_state = app_handle.state::<LlmState>();
    let config_store = llm_state.config_store.lock().await;

    // Resolve model ID (use code operation default if not provided)
    let model_id = resolve_model_id(request.model_id, &config_store).await?;

    // Get model info - try by ID first, then by name if ID lookup fails
    let model = match config_store.get_model(&model_id) {
        Ok(Some(m)) => m,
        Ok(None) => {
            // Try to parse provider_id and model name from the ID
            info!(
                "Model '{}' not found by ID, trying to resolve by name",
                model_id
            );

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

    // Get provider info
    let provider = config_store
        .get_provider(&model.provider_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Provider '{}' not found", model.provider_id))?;

    // Get API key
    let api_key = get_api_key_for_provider(&provider.id, &config_store)?;

    drop(config_store); // Release lock before async operations

    // Create client
    let client = create_client_for_provider(provider.api_type, api_key, provider.base_url);

    // Get chat state and load conversation history
    let chat_state = app_handle.state::<ChatState>();
    let chat_store = chat_state.store.lock().await;

    let history = chat_store
        .load_messages(&request.conversation_id)
        .map_err(|e| e.to_string())?;

    // Add the new user message to history for context
    let user_message = ChatMessage::user(&request.conversation_id, &request.content);
    let mut context_messages = history;
    context_messages.push(user_message.clone());

    // Save user message
    chat_store
        .add_message(&user_message)
        .map_err(|e| e.to_string())?;

    drop(chat_store); // Release lock before LLM call

    // Convert messages for LLM
    let llm_messages = convert_messages(&context_messages);

    // Create chat request
    let chat_request = ChatRequest::new(&model.name, llm_messages)
        .with_system(request.system_prompt.unwrap_or_default())
        .with_temperature(request.temperature.unwrap_or(0.7))
        .with_max_tokens(request.max_tokens.unwrap_or(4096));

    // Make LLM call
    let response: ChatResponse = client.chat(chat_request).await.map_err(|e| {
        error!("send_chat_message: LLM error: {}", e);
        e.to_string()
    })?;

    info!(
        "send_chat_message: Got response, content length={}",
        response.content.len()
    );

    // Save assistant message
    let assistant_message = ChatMessage::assistant(&request.conversation_id, &response.content);
    let chat_store = chat_state.store.lock().await;
    chat_store
        .add_message(&assistant_message)
        .map_err(|e| e.to_string())?;

    Ok(SendChatResponse {
        content: response.content,
        model: response.model,
        usage: response.usage,
        message: assistant_message,
    })
}

/// Sends a chat message and returns the complete response (non-streaming).
#[tauri::command]
pub async fn send_chat_message(
    request: SendChatRequest,
    app_handle: AppHandle,
) -> Result<SendChatResponse, String> {
    let runner = app_handle.state::<LlmState>().runner.clone();
    runner.send_chat_message(request, app_handle).await
}

// =============================================================================
// Legacy Chat Commands (streaming)
// =============================================================================

/// Sends a chat message with streaming response.
/// Events are emitted: llm:started, llm:chunk, llm:complete, llm:error
///
/// Note: The frontend is responsible for saving the user message BEFORE calling this.
/// This function only saves the assistant message after streaming completes.
async fn send_chat_message_streaming_legacy(
    request: SendChatRequest,
    app_handle: AppHandle,
) -> Result<String, String> {
    use futures::StreamExt;

    // Generate request ID
    let request_id = uuid::Uuid::new_v4().to_string();
    info!(
        "[LLM] Starting streaming request: request_id={}, conversation_id={}, thread_id={:?}",
        request_id, request.conversation_id, request.thread_id
    );

    // Get config store
    let llm_state = app_handle.state::<LlmState>();
    let active_requests = llm_state.active_requests.clone();
    let config_store = llm_state.config_store.lock().await;

    // Resolve model ID (use code operation default if not provided)
    let model_id = match resolve_model_id(request.model_id, &config_store).await {
        Ok(id) => id,
        Err(e) => {
            error!("[LLM] Failed to resolve model_id: {}", e);
            return Err(e);
        }
    };

    // Get model info - try by ID first, then by name if ID lookup fails
    let model = match config_store.get_model(&model_id) {
        Ok(Some(m)) => m,
        Ok(None) => {
            // Try to parse provider_id and model name from the ID
            // ID format is typically: "provider_id-model-name"
            info!(
                "[LLM] Model '{}' not found by ID, trying to resolve by name",
                model_id
            );

            // Find the first dash to split provider_id from model name
            if let Some(dash_pos) = model_id.find('-') {
                let provider_id = &model_id[..dash_pos];
                let model_name = &model_id[dash_pos + 1..];

                config_store
                    .get_model_by_name(provider_id, model_name)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| {
                        format!(
                            "Model '{}' not found (also tried name lookup: provider={}, name={})",
                            model_id, provider_id, model_name
                        )
                    })?
            } else {
                return Err(format!("Model '{}' not found", model_id));
            }
        }
        Err(e) => return Err(e.to_string()),
    };

    // Get provider info
    let provider = config_store
        .get_provider(&model.provider_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Provider '{}' not found", model.provider_id))?;

    // Get API key
    let api_key = match get_api_key_for_provider(&provider.id, &config_store) {
        Ok(key) => key,
        Err(e) => {
            error!("[LLM] Failed to get API key: {}", e);
            return Err(e);
        }
    };

    drop(config_store); // Release lock before async operations

    // Create client
    let client = create_client_for_provider(provider.api_type, api_key, provider.base_url);

    // Get chat state and load conversation history
    let chat_state = app_handle.state::<ChatState>();
    let chat_store = chat_state.store.lock().await;

    // Load thread messages if thread_id is provided, otherwise load all conversation messages
    let history = if let Some(ref thread_id) = request.thread_id {
        chat_store
            .load_thread_messages(thread_id)
            .map_err(|e| e.to_string())?
    } else {
        chat_store
            .load_messages(&request.conversation_id)
            .map_err(|e| e.to_string())?
    };

    // Load feedback for the current thread (only if thread_id exists)
    let feedback_messages = if let Some(ref thread_id) = request.thread_id {
        chat_store
            .load_thread_feedback(thread_id, FEEDBACK_LIMIT)
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    // Build feedback summary if feedback exists
    let feedback_summary = build_feedback_summary(&feedback_messages);
    let has_feedback = feedback_summary.is_some();

    if has_feedback {
        info!(
            "[LLM] Injecting feedback context: {} feedback items for thread_id={:?}",
            feedback_messages.len(),
            request.thread_id
        );
    }

    // DO NOT save user message here - frontend already saved it
    // Just add to context for LLM
    let user_context_message = ChatMessage::user(&request.conversation_id, &request.content);
    let mut context_messages = history;
    context_messages.push(user_context_message);

    drop(chat_store); // Release lock before streaming

    // Convert messages for LLM.
    // Feedback context is provided via system prompt only; message bodies are not rewritten.
    let llm_messages = convert_messages(&context_messages);

    // Build system prompt with feedback context if available
    let system_prompt = build_system_prompt_with_feedback(
        &request.system_prompt.unwrap_or_default(),
        feedback_summary.as_deref(),
    );

    // Create chat request with streaming enabled
    let chat_request = ChatRequest::new(&model.name, llm_messages)
        .with_system(&system_prompt)
        .with_temperature(request.temperature.unwrap_or(0.7))
        .with_max_tokens(request.max_tokens.unwrap_or(4096))
        .with_stream();

    // Get app handle for async task
    let app_handle_clone = app_handle.clone();
    let request_id_clone = request_id.clone();
    let conversation_id = request.conversation_id.clone();
    let thread_id = request.thread_id.clone();
    let model_name_for_log = model.name.clone();

    // Spawn streaming task
    let active_requests_for_task = active_requests.clone();
    let request_id_for_cleanup = request_id.clone();

    let handle = tokio::spawn(async move {
        let stream = client.chat_stream(chat_request);
        let events = crate::llm::stream_to_events(
            request_id_clone.clone(),
            model_name_for_log.clone(),
            stream,
        );

        #[allow(clippy::expect_used)]
        let mut events = std::pin::pin!(events);
        let mut final_content = String::new();
        let mut final_usage = None;

        while let Some(event) = events.next().await {
            match &event {
                LlmEvent::Chunk { delta, .. } => {
                    final_content.push_str(delta);
                }
                LlmEvent::Complete { content, usage, .. } => {
                    final_content = content.clone();
                    final_usage = usage.clone();
                }
                LlmEvent::Error { error, .. } => {
                    error!("[LLM] Stream error: {}", error);
                }
                _ => {}
            }

            // Emit event to frontend
            if let Err(e) = app_handle_clone.emit("llm:event", &event) {
                error!("[LLM] Failed to emit event: {}", e);
            }
        }

        info!(
            "[LLM] Stream completed: request_id={}, content_length={}",
            request_id_clone,
            final_content.len()
        );

        // Save assistant message after streaming completes
        if !final_content.is_empty() {
            let mut assistant_message = ChatMessage::assistant(&conversation_id, &final_content);

            // Set thread_id if provided - this is critical for proper message organization
            if let Some(tid) = thread_id {
                assistant_message = assistant_message.with_thread(&tid);
            }

            // Add metadata about model and tokens
            assistant_message.metadata = Some(crate::nlp::chat::MessageMetadata {
                model: Some(model_name_for_log.clone()),
                tokens: final_usage.as_ref().map(|u| u.total_tokens as usize),
                ..Default::default()
            });

            let chat_state = app_handle_clone.state::<ChatState>();
            let chat_store = chat_state.store.lock().await;
            if let Err(e) = chat_store.add_message(&assistant_message) {
                error!("[LLM] Failed to save assistant message: {}", e);
            }
        }

        let mut active = active_requests_for_task.lock().await;
        active.remove(&request_id_for_cleanup);
    });

    {
        let mut active = active_requests.lock().await;
        if let Some(previous) = active.insert(request_id.clone(), handle) {
            info!(
                "[LLM] Replaced existing streaming task for request_id={}, aborting previous task",
                request_id
            );
            previous.abort();
        }
    }

    Ok(request_id)
}

/// Sends a chat message with streaming response.
/// Events are emitted: llm:started, llm:chunk, llm:complete, llm:error
#[tauri::command]
pub async fn send_chat_message_streaming(
    request: SendChatRequest,
    app_handle: AppHandle,
) -> Result<String, String> {
    let runner = app_handle.state::<LlmState>().runner.clone();
    runner
        .send_chat_message_streaming(request, app_handle)
        .await
}

/// Cancels an in-progress streaming request.
#[tauri::command]
pub async fn cancel_chat_request(request_id: String, app_handle: AppHandle) -> Result<(), String> {
    let llm_state = app_handle.state::<LlmState>();
    let mut active_requests = llm_state.active_requests.lock().await;

    if let Some(handle) = active_requests.remove(&request_id) {
        info!("[LLM] Cancelling active stream request_id={}", request_id);
        handle.abort();
    } else {
        info!(
            "[LLM] cancel_chat_request: no active stream found for request_id={}",
            request_id
        );
    }

    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nlp::chat::MessageFeedback;

    #[test]
    fn test_llm_state_creation() {
        let state = LlmState::in_memory();
        assert!(state.is_ok());
    }

    #[test]
    fn test_parse_env_flag_truthy_values() {
        for value in ["1", "true", "TRUE", "yes", "on", " On "] {
            assert!(parse_env_flag(Some(value)), "expected truthy for {}", value);
        }
    }

    #[test]
    fn test_parse_env_flag_falsy_values() {
        for value in ["0", "false", "no", "off", "", "unexpected"] {
            assert!(!parse_env_flag(Some(value)), "expected falsy for {}", value);
        }
        assert!(!parse_env_flag(None));
    }

    #[test]
    fn test_convert_messages() {
        let messages = vec![
            ChatMessage::user("conv-1", "Hello"),
            ChatMessage::assistant("conv-1", "Hi there!"),
        ];

        let llm_messages = convert_messages(&messages);

        assert_eq!(llm_messages.len(), 2);
        assert_eq!(llm_messages[0].role, crate::llm::MessageRole::User);
        assert_eq!(llm_messages[1].role, crate::llm::MessageRole::Assistant);
    }

    #[test]
    fn test_build_feedback_summary_empty() {
        let feedback: Vec<(ChatMessage, MessageFeedback)> = vec![];
        let summary = build_feedback_summary(&feedback);
        assert!(summary.is_none());
    }

    #[test]
    fn test_build_feedback_summary_with_likes() {
        let msg1 = ChatMessage::assistant("conv-1", "Response 1");
        let msg2 = ChatMessage::assistant("conv-1", "Response 2");

        let feedback = vec![
            (msg1, MessageFeedback::Like),
            (msg2, MessageFeedback::Dislike),
        ];

        let summary = build_feedback_summary(&feedback);
        assert!(summary.is_some());

        let summary = summary.unwrap();
        assert!(summary.contains("Feedback on previous messages:"));
        assert!(summary.contains("LIKED"));
        assert!(summary.contains("DISLIKED"));
        assert!(summary.contains("[ID:"));
    }

    #[test]
    fn test_build_system_prompt_with_feedback() {
        let original = "You are a helpful assistant.";
        let feedback_summary =
            Some("Feedback on previous messages:\n- [ID:abc12345] LIKED by user");

        let combined = build_system_prompt_with_feedback(original, feedback_summary);

        assert!(combined.contains("You are a helpful assistant."));
        assert!(combined.contains("including stable IDs for reference"));
        assert!(combined.contains("Feedback on previous messages:"));
    }

    #[test]
    fn test_build_system_prompt_without_feedback() {
        let original = "You are a helpful assistant.";
        let combined = build_system_prompt_with_feedback(original, None);

        // Should return original when no feedback
        assert_eq!(combined, original);
    }

    #[test]
    fn test_build_system_prompt_with_feedback_and_empty_original() {
        let feedback_summary =
            Some("Feedback on previous messages:\n- [ID:abc12345] DISLIKED by user");

        let combined = build_system_prompt_with_feedback("", feedback_summary);

        assert!(combined.starts_with(FEEDBACK_SYSTEM_PROMPT));
        assert!(combined.contains("Feedback on previous messages:"));
        assert!(combined.contains("Now continue the conversation below:"));
    }

    #[test]
    fn test_build_feedback_summary_uses_stable_short_ids() {
        let msg = ChatMessage::assistant("conv-1", "Response");
        let full_id = msg.id.clone();

        let summary = build_feedback_summary(&[(msg, MessageFeedback::Like)])
            .expect("summary should exist for one feedback item");

        let short_id = &full_id[..8];
        assert!(summary.contains(&format!("[ID:{}]", short_id)));
        assert!(!summary.contains(&format!("[ID:{}]", full_id)));
    }
}
