//! LLM client utilities for agent runtime.
//!
//! Provides functions for creating and using LLM clients
//! within the agent execution context.

use tauri::{AppHandle, Manager};

use crate::commands::{ChatState, LlmState};
use crate::llm::{
    create_client, ChatRequest, ChatResponse, LlmClient, Message,
};
use crate::storage::ConfigStore;

use super::AgentRuntimeError;

/// Retry configuration for LLM calls.
pub struct LlmRetryConfig {
    /// Maximum number of retry attempts.
    pub max_retries: usize,
    /// Base delay in milliseconds for exponential backoff.
    pub base_delay_ms: u64,
}

impl Default for LlmRetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 2,
            base_delay_ms: 1000,
        }
    }
}

/// Creates an LLM client from app config for agent execution.
///
/// # Errors
///
/// Returns an error if the model or provider cannot be resolved.
pub async fn create_llm_client(
    app_handle: &AppHandle,
) -> Result<(Box<dyn LlmClient>, String), AgentRuntimeError> {
    log::info!("[AgentRuntime] Creating LLM client for agent execution");
    let llm_state = app_handle.state::<LlmState>();
    let config_store = llm_state.config_store.lock().await;

    // Use the "code" operation default model
    let model = crate::commands::config::get_code_operation_model(&config_store)
        .map_err(|e| {
            log::error!("[AgentRuntime] Failed to get code operation model: {}", e);
            AgentRuntimeError::Config(e.to_string())
        })?;

    log::info!("[AgentRuntime] Using model: {} ({})", model.name, model.provider_id);

    let provider = config_store
        .get_provider(&model.provider_id)
        .map_err(|e| AgentRuntimeError::Config(e.to_string()))?
        .ok_or_else(|| {
            log::error!("[AgentRuntime] Provider '{}' not found", model.provider_id);
            AgentRuntimeError::Config(format!("Provider '{}' not found", model.provider_id))
        })?;

    log::info!("[AgentRuntime] Provider found: {}", provider.id);

    let api_key = get_api_key_for_provider(&provider.id, &config_store)?;
    let model_name = model.name.clone();

    drop(config_store);

    log::info!("[AgentRuntime] Creating client with api_type: {:?}", provider.api_type);
    let client = create_client(provider.api_type, api_key, provider.base_url);
    log::info!("[AgentRuntime] LLM client created successfully");
    Ok((client, model_name))
}

/// Gets the API key for a provider from the database.
fn get_api_key_for_provider(
    provider_id: &str,
    config_store: &ConfigStore,
) -> Result<String, AgentRuntimeError> {
    config_store.retrieve_api_key(provider_id).map_err(|e| {
        AgentRuntimeError::Config(format!(
            "API key not found for provider '{}': {}",
            provider_id, e
        ))
    })
}

/// Executes a chat request with retry on transient errors.
///
/// # Errors
///
/// Returns an error if all retry attempts fail.
pub async fn chat_with_retry(
    client: &dyn LlmClient,
    request: &ChatRequest,
    config: &LlmRetryConfig,
) -> Result<ChatResponse, AgentRuntimeError> {
    let mut attempt = 0usize;
    loop {
        match client.chat(request.clone()).await {
            Ok(response) => return Ok(response),
            Err(err) => {
                let can_retry = err.is_retryable() && attempt < config.max_retries;
                if !can_retry {
                    return Err(AgentRuntimeError::Llm(err.to_string()));
                }

                let delay_ms = config.base_delay_ms * (1u64 << attempt);
                log::warn!(
                    "[AgentRuntime] Transient LLM error (attempt {}/{}), retrying in {}ms: {}",
                    attempt + 1,
                    config.max_retries + 1,
                    delay_ms,
                    err
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                attempt += 1;
            }
        }
    }
}

/// Builds LLM message context from conversation history.
///
/// # Errors
///
/// Returns an error if the chat store cannot be accessed.
pub async fn build_llm_context(
    session: &crate::agents::session::AgentSession,
    request: &crate::agents::task::AgentRequest,
    app_handle: &AppHandle,
) -> Result<Vec<Message>, AgentRuntimeError> {
    let chat_state = app_handle.state::<ChatState>();
    let chat_store = chat_state.store.lock().await;

    let history = chat_store
        .load_thread_messages(&session.thread_id)
        .unwrap_or_default();

    drop(chat_store);

    let mut context: Vec<Message> = history
        .iter()
        .map(|msg| match msg.role {
            crate::nlp::chat::MessageRole::User => Message::user(msg.content.clone()),
            crate::nlp::chat::MessageRole::Assistant => Message::assistant(msg.content.clone()),
            crate::nlp::chat::MessageRole::System => Message::system(msg.content.clone()),
        })
        .collect();

    // Add the current request
    context.push(Message::user(request.content.clone()));

    Ok(context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_retry_config_default() {
        let config = LlmRetryConfig::default();
        assert_eq!(config.max_retries, 2);
        assert_eq!(config.base_delay_ms, 1000);
    }
}
