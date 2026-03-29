use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use super::AgentState;

const TRACE_WINDOW_LABEL: &str = "agent-trace-dev";

/// Response containing execution debug data for the trace window.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionDebugResponse {
    /// Execution snapshot at the current point in time.
    pub snapshot: Option<crate::core::agent::execution::ExecutionSnapshot>,
    /// Runtime trace events for the session.
    pub traces: Vec<crate::core::agent::execution::RuntimeTrace>,
}

/// Open or focus the development-only agent trace window.
#[tauri::command]
pub async fn open_agent_trace_window(
    session_id: Option<String>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    if !cfg!(debug_assertions) {
        return Err("Agent trace window is only available in development builds".to_string());
    }

    let path = if let Some(ref session_id) = session_id {
        format!("/dev/agent-trace?sessionId={session_id}")
    } else {
        "/dev/agent-trace".to_string()
    };

    if let Some(window) = app_handle.get_webview_window(TRACE_WINDOW_LABEL) {
        window
            .emit("agent:trace-focus", session_id)
            .map_err(|e| e.to_string())?;
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(
        &app_handle,
        TRACE_WINDOW_LABEL,
        WebviewUrl::App(path.into()),
    )
    .title("Agent Trace")
    .inner_size(1400.0, 920.0)
    .min_inner_size(900.0, 600.0)
    .decorations(false)
    .transparent(true)
    .build()
    .map_err(|e| e.to_string())?;

    let _ = window.set_focus();
    Ok(())
}

/// Load persisted execution snapshot and recent traces for the development trace window.
#[tauri::command]
pub async fn get_agent_execution_debug(
    session_id: String,
    state: tauri::State<'_, AgentState>,
) -> Result<ExecutionDebugResponse, String> {
    if !cfg!(debug_assertions) {
        return Err("Agent trace debug data is only available in development builds".to_string());
    }

    let details = state
        .service
        .load_execution_debug_details(&session_id)
        .map_err(|e| e.to_string())?;

    Ok(ExecutionDebugResponse {
        snapshot: details.snapshot,
        traces: details.traces,
    })
}
