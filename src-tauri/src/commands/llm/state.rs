use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::core::config::ConfigRepository;
use crate::core::llm::services::LlmService;
use crate::infrastructure::sqlite::repositories::config::SqliteConfigRepository;

use super::orchestrator::OrchestratedConversationRunner;
use super::types::{SendChatRequest, SendChatResponse};

/// Application state for LLM commands.
pub struct LlmState {
    /// Configuration repository for provider/model lookup
    pub config_repository: Arc<dyn ConfigRepository>,
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
        let config_repository =
            Arc::new(SqliteConfigRepository::new(db_path).map_err(|e| e.to_string())?)
                as Arc<dyn ConfigRepository>;
        Ok(Self {
            config_repository,
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
        let config_repository =
            Arc::new(SqliteConfigRepository::in_memory().map_err(|e| e.to_string())?)
                as Arc<dyn ConfigRepository>;
        Ok(Self {
            config_repository,
            active_requests: Arc::new(Mutex::new(HashMap::new())),
            runner: Arc::new(OrchestratedConversationRunner::new(llm_service)),
        })
    }
}

/// Abstraction for conversation execution strategy.
pub trait ConversationRunner: Send + Sync {
    /// Handles a non-streaming chat request.
    fn send_chat_message(
        &self,
        request: SendChatRequest,
        app_handle: tauri::AppHandle,
    ) -> futures::future::BoxFuture<'_, Result<SendChatResponse, String>>;

    /// Handles a streaming chat request.
    fn send_chat_message_streaming(
        &self,
        request: SendChatRequest,
        app_handle: tauri::AppHandle,
    ) -> futures::future::BoxFuture<'_, Result<String, String>>;
}
