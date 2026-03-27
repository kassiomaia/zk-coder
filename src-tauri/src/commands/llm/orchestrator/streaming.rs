use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};

use crate::commands::llm::types::{SendChatRequest, SendChatResponse};
use crate::commands::{ChatState, LlmState};
use crate::core::llm::services::{LlmEvent, LlmService};

use super::executors::{select_tool_executor_from_env, ToolExecutor};
use super::loop_runner::{run_orchestrated_loop, OrchestratedLoopOutput};

/// Orchestrated non-streaming execution skeleton with bounded turn loop.
pub async fn send_chat_message_orchestrated_non_stream(
    request: SendChatRequest,
    app_handle: AppHandle,
    tool_executor: &dyn ToolExecutor,
    llm_service: Arc<dyn LlmService>,
) -> Result<SendChatResponse, String> {
    log::info!(
        "send_chat_message_orchestrated_non_stream: conversation_id={}, model_id={:?}",
        request.conversation_id,
        request.model_id
    );

    let final_output = run_orchestrated_loop(
        &request,
        &app_handle,
        tool_executor,
        llm_service,
        false,
        true,
    )
    .await?;

    let assistant_message = crate::core::chat::entities::ChatMessage::assistant(
        &request.conversation_id,
        &final_output.content,
    );
    let chat_state = app_handle.state::<ChatState>();
    chat_state
        .service
        .add_message(&assistant_message)
        .map_err(|e| e.to_string())?;

    Ok(SendChatResponse {
        content: final_output.content,
        model: final_output.model,
        usage: final_output.usage,
        message: assistant_message,
    })
}

/// Orchestrated streaming execution skeleton with bounded turn loop.
pub async fn send_chat_message_streaming_orchestrated(
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
            Ok(final_output) => emit_success(
                &app_handle_clone,
                &request_id_clone,
                &request_for_task,
                final_output,
            ),
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

fn emit_success(
    app_handle: &AppHandle,
    request_id: &str,
    request: &SendChatRequest,
    final_output: OrchestratedLoopOutput,
) {
    if !final_output.content.is_empty() {
        let chunk_event = LlmEvent::Chunk {
            request_id: request_id.to_string(),
            delta: final_output.content.clone(),
            reasoning_delta: None,
        };
        if let Err(e) = app_handle.emit("llm:event", &chunk_event) {
            log::error!("[LLM] Failed to emit orchestrated chunk event: {}", e);
        }
    }

    let complete_event = LlmEvent::Complete {
        request_id: request_id.to_string(),
        content: final_output.content.clone(),
        usage: final_output.usage.clone(),
        finish_reason: Some("stop".to_string()),
    };
    if let Err(e) = app_handle.emit("llm:event", &complete_event) {
        log::error!("[LLM] Failed to emit orchestrated complete event: {}", e);
    }

    if !final_output.content.is_empty() {
        let mut assistant_message = crate::core::chat::entities::ChatMessage::assistant(
            &request.conversation_id,
            &final_output.content,
        );
        if let Some(thread_id) = &request.thread_id {
            assistant_message = assistant_message.with_thread(thread_id);
        }
        assistant_message.metadata = Some(crate::core::chat::entities::MessageMetadata {
            model: Some(final_output.model),
            tokens: final_output.usage.as_ref().map(|u| u.total_tokens as usize),
            ..Default::default()
        });

        let chat_state = app_handle.state::<ChatState>();
        if let Err(e) = chat_state.service.add_message(&assistant_message) {
            log::error!("[LLM] Failed to save orchestrated assistant message: {}", e);
        }
    }
}
