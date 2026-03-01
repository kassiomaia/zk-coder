//! LLM Tauri IPC commands.
//!
//! This module provides Tauri commands for LLM chat completions and streaming.

use std::sync::Arc;
use tokio::sync::Mutex;

use log::{error, info};
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::config::get_code_operation_model;
use crate::llm::{ChatRequest, ChatResponse, LlmClient, LlmEvent, Message, create_client};
use crate::nlp::chat::{ChatMessage, MessageRole};
use crate::nlp::config::ApiType;
use crate::storage::ConfigStore;

// Re-export ChatState for use in this module
use super::ChatState;

/// Application state for LLM commands.
pub struct LlmState {
    /// Config store for provider/model lookup
    pub config_store: Arc<Mutex<ConfigStore>>,
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
        })
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
fn get_api_key_for_provider(provider_id: &str, config_store: &ConfigStore) -> Result<String, String> {
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
            })
        })
        .collect()
}

// =============================================================================
// Chat Commands
// =============================================================================

/// Sends a chat message and returns the complete response (non-streaming).
#[tauri::command]
pub async fn send_chat_message(
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

    // Get model info
    let model = config_store
        .get_model(&model_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Model '{}' not found", model_id))?;

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

/// Sends a chat message with streaming response.
/// Events are emitted: llm:started, llm:chunk, llm:complete, llm:error
#[tauri::command]
pub async fn send_chat_message_streaming(
    request: SendChatRequest,
    app_handle: AppHandle,
) -> Result<String, String> {
    info!(
        "send_chat_message_streaming: conversation_id={}, model_id={:?}",
        request.conversation_id, request.model_id
    );

    // Generate request ID
    let request_id = uuid::Uuid::new_v4().to_string();
    info!("send_chat_message_streaming: generated request_id={}", request_id);

    // Get config store
    let llm_state = app_handle.state::<LlmState>();
    let config_store = llm_state.config_store.lock().await;

    // Resolve model ID (use code operation default if not provided)
    info!("send_chat_message_streaming: resolving model_id...");
    let model_id = match resolve_model_id(request.model_id, &config_store).await {
        Ok(id) => {
            info!("send_chat_message_streaming: resolved model_id={}", id);
            id
        }
        Err(e) => {
            error!("send_chat_message_streaming: failed to resolve model_id: {}", e);
            return Err(e);
        }
    };

    // Get model info
    info!("send_chat_message_streaming: fetching model info for {}", model_id);
    let model = config_store
        .get_model(&model_id)
        .map_err(|e| {
            error!("send_chat_message_streaming: failed to get model: {}", e);
            e.to_string()
        })?
        .ok_or_else(|| {
            error!("send_chat_message_streaming: model '{}' not found", model_id);
            format!("Model '{}' not found", model_id)
        })?;
    info!("send_chat_message_streaming: found model name={}, provider_id={}", model.name, model.provider_id);

    // Get provider info
    info!("send_chat_message_streaming: fetching provider info for {}", model.provider_id);
    let provider = config_store
        .get_provider(&model.provider_id)
        .map_err(|e| {
            error!("send_chat_message_streaming: failed to get provider: {}", e);
            e.to_string()
        })?
        .ok_or_else(|| {
            error!("send_chat_message_streaming: provider '{}' not found", model.provider_id);
            format!("Provider '{}' not found", model.provider_id)
        })?;
    info!("send_chat_message_streaming: found provider name={}, api_type={:?}", provider.name, provider.api_type);

    // Get API key
    info!("send_chat_message_streaming: retrieving API key for provider {}", provider.id);
    let api_key = match get_api_key_for_provider(&provider.id, &config_store) {
        Ok(key) => {
            info!("send_chat_message_streaming: API key retrieved (length={})", key.len());
            key
        }
        Err(e) => {
            error!("send_chat_message_streaming: failed to get API key: {}", e);
            return Err(e);
        }
    };

    drop(config_store); // Release lock before async operations

    // Create client
    info!("send_chat_message_streaming: creating LLM client for {:?}", provider.api_type);
    let client = create_client_for_provider(provider.api_type, api_key, provider.base_url);

    // Get chat state and load conversation history
    let chat_state = app_handle.state::<ChatState>();
    let chat_store = chat_state.store.lock().await;

    let history = chat_store
        .load_messages(&request.conversation_id)
        .map_err(|e| {
            error!("send_chat_message_streaming: failed to load messages: {}", e);
            e.to_string()
        })?;
    info!("send_chat_message_streaming: loaded {} history messages", history.len());

    // Add the new user message to history for context
    let user_message = ChatMessage::user(&request.conversation_id, &request.content);
    let mut context_messages = history;
    context_messages.push(user_message.clone());

    // Save user message
    chat_store
        .add_message(&user_message)
        .map_err(|e| {
            error!("send_chat_message_streaming: failed to save user message: {}", e);
            e.to_string()
        })?;
    info!("send_chat_message_streaming: saved user message");

    drop(chat_store); // Release lock before streaming

    // Convert messages for LLM
    let llm_messages = convert_messages(&context_messages);
    info!("send_chat_message_streaming: converted {} messages for LLM", llm_messages.len());

    // Create chat request with streaming enabled
    let chat_request = ChatRequest::new(&model.name, llm_messages)
        .with_system(request.system_prompt.unwrap_or_default())
        .with_temperature(request.temperature.unwrap_or(0.7))
        .with_max_tokens(request.max_tokens.unwrap_or(4096))
        .with_stream();

    info!("send_chat_message_streaming: created chat request for model {}", model.name);

    // Get app handle for async task
    let app_handle_clone = app_handle.clone();
    let request_id_clone = request_id.clone();
    let conversation_id = request.conversation_id.clone();
    let model_name_for_log = model.name.clone();

    info!("send_chat_message_streaming: spawning streaming task for request_id={}", request_id);

    // Spawn streaming task
    tokio::spawn(async move {
        info!("streaming_task: starting stream for request_id={}, model={}", request_id_clone, model_name_for_log);
        let stream = client.chat_stream(chat_request);
        let events = crate::llm::stream_to_events(request_id_clone.clone(), model_name_for_log.clone(), stream);

        #[allow(clippy::expect_used)]
        let mut events = std::pin::pin!(events);
        let mut final_content = String::new();
        let mut final_usage = None;

        use futures::StreamExt;

        while let Some(event) = events.next().await {
            match &event {
                LlmEvent::Started { model, .. } => {
                    info!("streaming_task: received Started event for model={}", model);
                }
                LlmEvent::Chunk { delta, .. } => {
                    final_content.push_str(delta);
                }
                LlmEvent::Complete { content, usage, .. } => {
                    info!("streaming_task: received Complete event, content_length={}", content.len());
                    final_content = content.clone();
                    final_usage = usage.clone();
                }
                LlmEvent::Error { error, .. } => {
                    error!("streaming_task: received Error event: {}", error);
                }
            }

            // Emit event to frontend
            if let Err(e) = app_handle_clone.emit("llm:event", &event) {
                error!("Failed to emit LLM event: {}", e);
            }
        }

        // Save assistant message after streaming completes
        if !final_content.is_empty() {
            let mut assistant_message = ChatMessage::assistant(&conversation_id, &final_content);

            // Add metadata about model and tokens
            assistant_message.metadata = Some(crate::nlp::chat::MessageMetadata {
                model: Some(model.name.clone()),
                tokens: final_usage.as_ref().map(|u| u.total_tokens as usize),
                ..Default::default()
            });

            let chat_state = app_handle_clone.state::<ChatState>();
            let chat_store = chat_state.store.lock().await;
            if let Err(e) = chat_store.add_message(&assistant_message) {
                error!("Failed to save assistant message: {}", e);
            }
        }
    });

    Ok(request_id)
}

/// Cancels an in-progress streaming request.
#[tauri::command]
pub async fn cancel_chat_request(
    request_id: String,
) -> Result<(), String> {
    // For now, just log the cancellation
    // In the future, we could implement proper cancellation
    info!("cancel_chat_request: request_id={}", request_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_state_creation() {
        let state = LlmState::in_memory();
        assert!(state.is_ok());
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
}
