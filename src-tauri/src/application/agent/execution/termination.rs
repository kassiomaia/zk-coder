use chrono::Utc;
use serde_json::json;
use tauri::Emitter;

use super::deps::ExecutionDeps;
use super::state::ExecutionContext;
use crate::core::agent::execution::{ExecutionPhase, RuntimeTraceType};
use crate::core::agent::session::{SessionId, SessionState};
use crate::core::agent::task::{AgentExecutionResult, TaskStatus, UsageStats};
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::agent::runtime::AgentRuntimeError;

const STREAMING_CHUNK_BYTES: usize = 96;
const STREAMING_CHUNK_DELAY_MS: u64 = 24;

pub fn update_session_state(
    deps: &ExecutionDeps<'_>,
    session_id: &SessionId,
    state: SessionState,
) -> Result<(), AgentRuntimeError> {
    let core_state: crate::core::agent::entities::SessionState = state.clone().into();
    deps.session_repo
        .update_state(
            &crate::core::agent::entities::SessionId(session_id.0.clone()),
            &core_state,
        )
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;

    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::SessionStateChanged {
            session_id: session_id.0.clone(),
            state,
        },
    );

    Ok(())
}

pub fn cancel_execution(
    deps: &ExecutionDeps<'_>,
    ctx: &ExecutionContext,
    reason: &str,
) -> Result<(), AgentRuntimeError> {
    let tracer = deps.tracer();
    let mut snapshot = ctx.snapshot(
        ExecutionPhase::Cancelled,
        None,
        Some("Execution cancelled".to_string()),
    );
    snapshot.current_subtask_id = ctx.current_subtask_id.clone();
    let _ = tracer.save_snapshot(&snapshot);
    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        ctx.current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::ExecutionCancelled,
        json!({ "reason": reason }),
    );
    let _ = deps.task_repo.update_task_status(
        &ctx.task_id,
        crate::core::agent::entities::TaskStatus::Cancelled,
    );
    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::TaskUpdated {
            session_id: ctx.session_id.clone(),
            task_id: ctx.task_id.clone(),
            status: TaskStatus::Cancelled,
        },
    );
    Ok(())
}

pub fn fail_execution(
    deps: &ExecutionDeps<'_>,
    ctx: &ExecutionContext,
    error: &str,
    kind: &str,
    last_model: Option<String>,
) -> Result<(), AgentRuntimeError> {
    update_session_state(
        deps,
        &SessionId(ctx.session_id.clone()),
        SessionState::Error {
            message: error.to_string(),
            recoverable: true,
        },
    )?;

    let _ = deps.task_repo.update_task_status(
        &ctx.task_id,
        crate::core::agent::entities::TaskStatus::Failed,
    );
    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::TaskUpdated {
            session_id: ctx.session_id.clone(),
            task_id: ctx.task_id.clone(),
            status: TaskStatus::Failed,
        },
    );
    let tracer = deps.tracer();
    let _ = tracer.save_snapshot(&ctx.snapshot(
        ExecutionPhase::Failed,
        last_model,
        Some(error.to_string()),
    ));
    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        ctx.current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::ExecutionFailed,
        json!({ "error": error, "kind": kind }),
    );
    Ok(())
}

fn chunk_response(content: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();

    for ch in content.chars() {
        current.push(ch);
        if current.len() >= STREAMING_CHUNK_BYTES && ch.is_whitespace() {
            chunks.push(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    if chunks.is_empty() {
        chunks.push(String::new());
    }

    chunks
}

fn build_streaming_events(content: &str, finalize_stream: bool) -> Vec<(String, bool)> {
    let chunks = chunk_response(content);
    let last_index = chunks.len().saturating_sub(1);

    chunks
        .into_iter()
        .enumerate()
        .map(|(index, chunk)| {
            let is_final = finalize_stream && index == last_index;
            (chunk, is_final)
        })
        .collect()
}

pub(super) async fn emit_streaming_content(
	deps: &ExecutionDeps<'_>,
	session_id: &str,
	conversation_id: &str,
	thread_id: &str,
	content: &str,
	finalize_stream: bool,
) {
	emit_streaming_message(
		deps,
		session_id,
		conversation_id,
		thread_id,
		content,
		finalize_stream,
		None,
		false,
	)
	.await;
}

pub(super) async fn emit_streaming_message(
	deps: &ExecutionDeps<'_>,
	session_id: &str,
	conversation_id: &str,
	thread_id: &str,
	content: &str,
	finalize_stream: bool,
	message_id: Option<String>,
	persist_message: bool,
) {
	let events = build_streaming_events(content, finalize_stream);
	let last_index = events.len().saturating_sub(1);

	for (index, (chunk, is_final)) in events.into_iter().enumerate() {
		let _ = deps.app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::StreamingContent {
                session_id: session_id.to_string(),
				conversation_id: conversation_id.to_string(),
				thread_id: thread_id.to_string(),
				chunk,
				is_final,
				message_id: message_id.clone(),
				persist_message: persist_message.then_some(true),
			},
		);

        if index != last_index {
            tokio::time::sleep(std::time::Duration::from_millis(
                STREAMING_CHUNK_DELAY_MS,
            ))
            .await;
        }
    }
}

pub async fn complete_execution(
    deps: &ExecutionDeps<'_>,
    ctx: &ExecutionContext,
    conversation_id: &str,
    thread_id: &str,
    content: String,
) -> Result<AgentExecutionResult, AgentRuntimeError> {
    let result = AgentExecutionResult {
        content,
        steps: ctx.steps.clone(),
        usage: Some(UsageStats {
            input_tokens: ctx.total_input_tokens,
            output_tokens: ctx.total_output_tokens,
            total_tokens: ctx.total_input_tokens + ctx.total_output_tokens,
        }),
    };

    update_session_state(
        deps,
        &SessionId(ctx.session_id.clone()),
        SessionState::Finished {
            completed_at: Utc::now(),
        },
    )?;
    let _ = deps.task_repo.update_task_status(
        &ctx.task_id,
        crate::core::agent::entities::TaskStatus::Completed,
    );

    let tracer = deps.tracer();
    let _ = tracer.save_snapshot(&ctx.snapshot(ExecutionPhase::Completed, None, None));
    let _ = tracer.append_trace(
        &ctx.session_id,
        &ctx.task_id,
        None,
        ctx.step_number,
        RuntimeTraceType::ExecutionCompleted,
        json!({
            "responseLength": result.content.len(),
            "totalSteps": result.steps.len(),
            "totalInputTokens": ctx.total_input_tokens,
            "totalOutputTokens": ctx.total_output_tokens,
        }),
    );

    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::TaskUpdated {
            session_id: ctx.session_id.clone(),
            task_id: ctx.task_id.clone(),
            status: TaskStatus::Completed,
        },
    );
    emit_streaming_content(
        deps,
        &ctx.session_id,
        conversation_id,
        thread_id,
        &result.content,
        true,
    )
    .await;
    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::ExecutionComplete {
            session_id: ctx.session_id.clone(),
            conversation_id: conversation_id.to_string(),
            thread_id: thread_id.to_string(),
            result: result.clone(),
        },
    );

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::build_streaming_events;

    #[test]
    fn test_build_streaming_events_marks_last_chunk_final_when_requested() {
        let events = build_streaming_events("hello world", true);

        assert!(!events.is_empty());
        assert!(events.iter().take(events.len().saturating_sub(1)).all(|(_, is_final)| !is_final));
        assert_eq!(events.last().map(|(_, is_final)| *is_final), Some(true));
    }

    #[test]
    fn test_build_streaming_events_keeps_tool_call_content_non_final() {
        let events = build_streaming_events("Let me inspect the project.", false);

        assert!(!events.is_empty());
        assert!(events.iter().all(|(_, is_final)| !is_final));
    }
}
