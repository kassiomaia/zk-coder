//! LLM Tauri IPC commands.
//!
//! This module provides Tauri commands for LLM chat completions and streaming.
//! It acts as a thin adapter that delegates to the application/llm LlmService.

pub mod orchestrator;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use log::{error, info};
use tauri::{AppHandle, Emitter, Manager};

use crate::application::llm::LlmServiceImpl;
use crate::core::llm::services::LlmService;
use crate::commands::config::get_code_operation_model;
use crate::core::chat::entities::{ChatMessage, MessageFeedback, MessageRole};
use crate::core::llm::entities::{
    ChatRequest as CoreChatRequest, ChatResponse as CoreChatResponse, ContentBlock,
    MessageContent, MessageRole as CoreMessageRole,
};
use crate::llm::{ChatRequest, ChatResponse, LlmEvent, Message};
use crate::storage::ConfigStore;

// Re-export ChatState for use in this module
use super::ChatState;

// Re-export orchestrator types
pub use orchestrator::{OrchestratedConversationRunner, ORCHESTRATOR_LLM_MAX_RETRIES};

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
    pub fn new(db_path: &str, llm_service: Arc<dyn LlmService>) -> Result<Self, String> {
        let config_store = ConfigStore::new(db_path).map_err(|e| e.to_string())?;
        Ok(Self {
            config_store: Arc::new(Mutex::new(config_store)),
            active_requests: Arc::new(Mutex::new(HashMap::new())),
            runner: Arc::new(OrchestratedConversationRunner::new(llm_service)),
        })
    }

    /// Creates a new LLM state with an in-memory database (for testing).
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be initialized.
    pub fn in_memory(llm_service: Arc<dyn LlmService>) -> Result<Self, String> {
        let config_store = ConfigStore::in_memory().map_err(|e| e.to_string())?;
        Ok(Self {
            config_store: Arc::new(Mutex::new(config_store)),
            active_requests: Arc::new(Mutex::new(HashMap::new())),
            runner: Arc::new(OrchestratedConversationRunner::new(llm_service)),
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

// =============================================================================
// Request/Response Types
// =============================================================================

/// Request to send a chat message to an LLM.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

/// Converts storage ChatMessage to core ChatMessage.
fn convert_to_core_message(msg: ChatMessage) -> crate::core::llm::entities::ChatMessage {
    crate::core::llm::entities::ChatMessage {
        role: match msg.role {
            MessageRole::User => CoreMessageRole::User,
            MessageRole::Assistant => CoreMessageRole::Assistant,
            MessageRole::System => CoreMessageRole::System,
        },
        content: MessageContent::Text(msg.content),
        tool_calls: Vec::new(),
        tool_result: None,
    }
}

// =============================================================================
// Feedback Context Injection
// =============================================================================

/// System prompt explaining the feedback system.
const FEEDBACK_SYSTEM_PROMPT: &str = r#"The user may provide feedback (like/dislike) on earlier assistant responses. A feedback summary section will be provided, including stable IDs for reference. Use that feedback to improve your response quality and avoid repeating disliked patterns."#;

/// Maximum number of feedback items to include in context.
const FEEDBACK_LIMIT: usize = 5;

/// Builds a feedback summary string referencing message IDs.
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
fn build_system_prompt_with_feedback(
    original_prompt: &str,
    feedback_summary: Option<&str>,
) -> String {
    match feedback_summary {
        Some(summary) => {
            if original_prompt.is_empty() {
                format!(
                    "{}\n\n{}\n\n{}",
                    FEEDBACK_SYSTEM_PROMPT,
                    summary,
                    "Now continue the conversation below:"
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
// Commands
// =============================================================================

/// Sends a chat message and returns the complete response (non-streaming).
#[tauri::command]
pub async fn send_chat_message(
    request: SendChatRequest,
    app_handle: AppHandle,
) -> Result<SendChatResponse, String> {
    let runner = app_handle.state::<LlmState>().runner.clone();
    runner.send_chat_message(request, app_handle).await
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
    use crate::core::chat::entities::MessageFeedback;

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
