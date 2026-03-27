use log::info;
use tauri::{AppHandle, Manager};

use super::state::LlmState;
use super::types::{SendChatRequest, SendChatResponse};

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
