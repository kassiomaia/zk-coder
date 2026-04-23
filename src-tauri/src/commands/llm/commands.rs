use log::{debug, info};
use tauri::{AppHandle, Manager};

use super::slash_resolver;
use super::state::LlmState;
use super::types::{SendChatRequest, SendChatResponse};

/// Expands any leading slash command in `request.content` against the user's
/// prompt-template library exactly once, before the orchestrator sees it.
fn expand_slash_commands(request: &mut SendChatRequest, llm_state: &LlmState) {
    if !request.content.starts_with('/') {
        return;
    }
    let expanded = slash_resolver::resolve(&request.content, &llm_state.config_repository);
    if expanded != request.content {
        debug!(
            "[LLM] Expanded slash command (pre-resolve len={}, post-resolve len={})",
            request.content.len(),
            expanded.len()
        );
        request.content = expanded;
    }
}

/// Sends a chat message and returns the complete response (non-streaming).
#[tauri::command]
pub async fn send_chat_message(
    mut request: SendChatRequest,
    app_handle: AppHandle,
) -> Result<SendChatResponse, String> {
    let llm_state = app_handle.state::<LlmState>();
    expand_slash_commands(&mut request, &llm_state);
    let runner = llm_state.runner.clone();
    runner.send_chat_message(request, app_handle).await
}

/// Sends a chat message with streaming response.
/// Events are emitted: llm:started, llm:chunk, llm:complete, llm:error
#[tauri::command]
pub async fn send_chat_message_streaming(
    mut request: SendChatRequest,
    app_handle: AppHandle,
) -> Result<String, String> {
    let llm_state = app_handle.state::<LlmState>();
    expand_slash_commands(&mut request, &llm_state);
    let runner = llm_state.runner.clone();
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
        info!("[LLM] Cancelling active stream request_id={request_id}");
        handle.abort();
    } else {
        info!("[LLM] cancel_chat_request: no active stream found for request_id={request_id}");
    }

    Ok(())
}
