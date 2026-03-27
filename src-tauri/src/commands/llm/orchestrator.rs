//! Orchestrated conversation runner with tool support.
//!
//! This module keeps the command-layer runner thin while delegating loop,
//! tool-execution strategy, and streaming mechanics to focused submodules.

mod executors;
mod loop_runner;
mod streaming;

use std::sync::Arc;

use tauri::AppHandle;

use crate::core::llm::services::LlmService;

use super::{SendChatRequest, SendChatResponse};

use executors::select_tool_executor_from_env;
#[cfg(test)]
use executors::NoopToolExecutor;
pub use executors::{
    append_tool_results_to_llm_context, is_textual_tool_plan, parse_env_flag, RegistryToolExecutor,
    ToolExecutor, ORCHESTRATOR_TOOL_REGISTRY_ENV,
};
pub use loop_runner::{
    resolve_model_id, run_chat_with_retry, run_orchestrated_loop, OrchestratedLoopOutput,
    ORCHESTRATOR_LLM_MAX_RETRIES, ORCHESTRATOR_LLM_RETRY_BASE_MS, ORCHESTRATOR_MAX_STEPS,
};
use streaming::{
    send_chat_message_orchestrated_non_stream, send_chat_message_streaming_orchestrated,
};

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
                    log::warn!("[LLM] Orchestrated runner failed: {}", e);
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

#[cfg(test)]
mod tests;
