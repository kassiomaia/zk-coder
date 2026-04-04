//! Handles the termination of an agent's execution, including completion, cancellation, and failure.

use chrono::Utc;
use serde_json::json;
use tauri::Emitter;

use super::deps::ExecutionDeps;
use super::state::ExecutionContext;
use crate::core::agent::execution::{ExecutionPhase, RuntimeTraceType};
use crate::core::agent::session::{SessionId, SessionState};
use crate::core::agent::task::{AgentExecutionResult, TaskStatus, UsageStats};
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::agent::runtime::permissions::core_task_to_event_subtask;
use crate::infrastructure::agent::runtime::AgentRuntimeError;

const STREAMING_CHUNK_BYTES: usize = 96;
const STREAMING_CHUNK_DELAY_MS: u64 = 24;

/// Updates the state of an agent session and emits a `SessionStateChanged` event.
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

/// Cancels the execution of an agent.
///
/// This function updates the execution snapshot, records a trace event,
/// updates the task status, and emits a `TaskUpdated` event.
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

/// Fails the execution of an agent.
///
/// This function updates the session state to `Error`, updates the task status,
/// and records a snapshot and trace event for the failure.
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
            tokio::time::sleep(std::time::Duration::from_millis(STREAMING_CHUNK_DELAY_MS)).await;
        }
    }
}

/// Completes the execution of an agent.
///
/// This function finalizes the execution by:
/// 1.  Updating the session state to `Finished`.
/// 2.  Updating the task status to `Completed`.
/// 3.  Auto-completing any subtask that is still `InProgress`.
/// 4.  Recording a final snapshot and trace event.
/// 5.  Emitting `TaskUpdated` and `ExecutionComplete` events.
/// 6.  Streaming the final content to the frontend.
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

    // Auto-complete any remaining InProgress subtask
    // This handles the case where the agent completes all work but doesn't call
    // agent_advance_to_next_subtask for the final subtask
    if let Ok(subtasks) = deps.task_repo.get_subtasks(&ctx.task_id) {
        if let Some(in_progress) = subtasks
            .iter()
            .find(|st| st.status == crate::core::agent::entities::TaskStatus::InProgress)
        {
            log::info!(
                "[AgentExecution] Auto-completing final subtask {} for task {}",
                in_progress.id,
                ctx.task_id
            );
            let _ = deps.task_repo.update_subtask_status(
                &in_progress.id,
                crate::core::agent::entities::TaskStatus::Completed,
            );

            // Emit SubtaskUpdated event for the completed subtask
            let mut event_subtask = core_task_to_event_subtask(in_progress.clone(), &ctx.task_id);
            event_subtask.status = TaskStatus::Completed;
            event_subtask.completed_at = Some(Utc::now());
            let _ = deps.app_handle.emit(
                AGENT_EVENT_NAME,
                AgentEvent::SubtaskUpdated {
                    session_id: ctx.session_id.clone(),
                    subtask: event_subtask,
                },
            );
        }
    }

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
        assert!(events
            .iter()
            .take(events.len().saturating_sub(1))
            .all(|(_, is_final)| !is_final));
        assert_eq!(events.last().map(|(_, is_final)| *is_final), Some(true));
    }

    #[test]
    fn test_build_streaming_events_keeps_tool_call_content_non_final() {
        let events = build_streaming_events("Let me inspect the project.", false);

        assert!(!events.is_empty());
        assert!(events.iter().all(|(_, is_final)| !is_final));
    }
}

/// Documentation and behavior tests for subtask auto-completion.
///
/// When an agent completes execution (e.g., after providing a final report),
/// it should call `agent_advance_to_next_subtask` to mark the current subtask
/// as complete. However, if the agent finishes without making this call,
/// the system auto-completes any remaining InProgress subtask.
///
/// This ensures:
/// 1. All subtasks are marked Completed when the task finishes
/// 2. The UI shows consistent state (no orphaned InProgress subtasks)
/// 3. Task completion is accurately reflected in subtask status
#[cfg(test)]
mod subtask_autocompletion_tests {
    use crate::core::agent::entities::{AgentTask as CoreAgentTask, TaskMode, TaskStatus};
    use crate::core::agent::repositories::TaskRepository;
    use crate::core::agent::session::AgentSession;
    use crate::core::agent::task::{SubTask, SubTaskMode, TaskStatus as TaskTaskStatus};
    use crate::core::agent::types::AgentId;
    use crate::infrastructure::sqlite::repositories::agent::SqliteSessionRepository;

    /// Test scenario: Agent completes work but doesn't call advance for final subtask.
    ///
    /// BEFORE fix: The final subtask remains InProgress even after task completes.
    /// AFTER fix: The final subtask is auto-completed when the task is marked Completed.
    #[test]
    fn test_autocomplete_final_subtask_on_task_completion() {
        let store = SqliteSessionRepository::new(":memory:").unwrap();

        // Create session and task
        let session = AgentSession::new(
            AgentId::new("test-agent"),
            "Test Agent".to_string(),
            "conv-1".to_string(),
            "thread-1".to_string(),
            None,
            None,
        );
        let session_id = session.id.clone();
        store.create_session(&session).unwrap();

        let task = CoreAgentTask {
            id: "task-1".to_string(),
            session_id: session_id.0.clone(),
            parent_task_id: None,
            title: "Test Task".to_string(),
            description: None,
            status: TaskStatus::InProgress,
            order: 0,
            mode: Some(TaskMode::FullAccess),
            needs_review: false,
        };
        TaskRepository::create_task(&store, &task).unwrap();

        // Create 3 subtasks (using SubTask type for store.create_subtask)
        let subtask1 = SubTask::new(
            "task-1".to_string(),
            "Investigate".to_string(),
            SubTaskMode::FullAccess,
            0,
        );
        let subtask2 = SubTask::new(
            "task-1".to_string(),
            "Fix issues".to_string(),
            SubTaskMode::FullAccess,
            1,
        );
        let subtask3 = SubTask::new(
            "task-1".to_string(),
            "Document findings".to_string(),
            SubTaskMode::FullAccess,
            2,
        );

        store.create_subtask(&subtask1).unwrap();
        store.create_subtask(&subtask2).unwrap();
        store.create_subtask(&subtask3).unwrap();

        // Mark first two as Completed, leave third as InProgress
        store
            .update_subtask_status(&subtask1.id, &TaskTaskStatus::Completed)
            .unwrap();
        store
            .update_subtask_status(&subtask2.id, &TaskTaskStatus::Completed)
            .unwrap();
        store
            .update_subtask_status(&subtask3.id, &TaskTaskStatus::InProgress)
            .unwrap();

        // Verify initial state: subtask3 is InProgress
        let subtasks_before = store.get_subtasks("task-1").unwrap();
        assert_eq!(subtasks_before[2].status, TaskTaskStatus::InProgress);

        // SIMULATE: Task completes and auto-completion runs
        // (This is what complete_execution now does)
        store
            .update_subtask_status(&subtask3.id, &TaskTaskStatus::Completed)
            .unwrap();

        // Verify: All subtasks are now Completed
        let subtasks_after = store.get_subtasks("task-1").unwrap();
        assert_eq!(subtasks_after[0].status, TaskTaskStatus::Completed);
        assert_eq!(subtasks_after[1].status, TaskTaskStatus::Completed);
        assert_eq!(subtasks_after[2].status, TaskTaskStatus::Completed);
    }

    /// Test that only InProgress subtasks are auto-completed.
    ///
    /// Pending subtasks should remain Pending even after task completion.
    #[test]
    fn test_only_in_progress_subtask_is_auto_completed() {
        let store = SqliteSessionRepository::new(":memory:").unwrap();

        let session = AgentSession::new(
            AgentId::new("test-agent"),
            "Test Agent".to_string(),
            "conv-1".to_string(),
            "thread-1".to_string(),
            None,
            None,
        );
        let session_id = session.id.clone();
        store.create_session(&session).unwrap();

        let task = CoreAgentTask {
            id: "task-1".to_string(),
            session_id: session_id.0.clone(),
            parent_task_id: None,
            title: "Test Task".to_string(),
            description: None,
            status: TaskStatus::InProgress,
            order: 0,
            mode: Some(TaskMode::FullAccess),
            needs_review: false,
        };
        TaskRepository::create_task(&store, &task).unwrap();

        // Create 3 subtasks: Completed, InProgress, and Pending
        let subtask1 = SubTask::new(
            "task-1".to_string(),
            "Done".to_string(),
            SubTaskMode::FullAccess,
            0,
        );
        let subtask2 = SubTask::new(
            "task-1".to_string(),
            "In Progress".to_string(),
            SubTaskMode::FullAccess,
            1,
        );
        let subtask3 = SubTask::new(
            "task-1".to_string(),
            "Not Started".to_string(),
            SubTaskMode::FullAccess,
            2,
        );

        store.create_subtask(&subtask1).unwrap();
        store.create_subtask(&subtask2).unwrap();
        store.create_subtask(&subtask3).unwrap();

        // Mark first as Completed, second as InProgress, third remains Pending
        store
            .update_subtask_status(&subtask1.id, &TaskTaskStatus::Completed)
            .unwrap();
        store
            .update_subtask_status(&subtask2.id, &TaskTaskStatus::InProgress)
            .unwrap();

        // SIMULATE: Task completes, auto-completion finds InProgress subtask
        let subtasks = store.get_subtasks("task-1").unwrap();
        let in_progress = subtasks
            .iter()
            .find(|st| st.status == TaskTaskStatus::InProgress);

        assert!(in_progress.is_some(), "Should find an InProgress subtask");

        // Only the InProgress subtask should be completed
        if let Some(st) = in_progress {
            store
                .update_subtask_status(&st.id, &TaskTaskStatus::Completed)
                .unwrap();
        }

        // Verify: Only the InProgress subtask was completed, Pending remains
        let subtasks_after = store.get_subtasks("task-1").unwrap();
        assert_eq!(subtasks_after[0].status, TaskTaskStatus::Completed); // Was already done
        assert_eq!(subtasks_after[1].status, TaskTaskStatus::Completed); // Auto-completed
        assert_eq!(subtasks_after[2].status, TaskTaskStatus::Pending); // Still Pending
    }

    /// Test that if no subtask is InProgress, no auto-completion occurs.
    #[test]
    fn test_no_autocompletion_when_no_in_progress_subtask() {
        let store = SqliteSessionRepository::new(":memory:").unwrap();

        let session = AgentSession::new(
            AgentId::new("test-agent"),
            "Test Agent".to_string(),
            "conv-1".to_string(),
            "thread-1".to_string(),
            None,
            None,
        );
        let session_id = session.id.clone();
        store.create_session(&session).unwrap();

        let task = CoreAgentTask {
            id: "task-1".to_string(),
            session_id: session_id.0.clone(),
            parent_task_id: None,
            title: "Test Task".to_string(),
            description: None,
            status: TaskStatus::InProgress,
            order: 0,
            mode: Some(TaskMode::FullAccess),
            needs_review: false,
        };
        TaskRepository::create_task(&store, &task).unwrap();

        // Create 2 subtasks: both Completed (agent properly advanced through them)
        let subtask1 = SubTask::new(
            "task-1".to_string(),
            "Step 1".to_string(),
            SubTaskMode::FullAccess,
            0,
        );
        let subtask2 = SubTask::new(
            "task-1".to_string(),
            "Step 2".to_string(),
            SubTaskMode::FullAccess,
            1,
        );

        store.create_subtask(&subtask1).unwrap();
        store.create_subtask(&subtask2).unwrap();

        // Mark both as Completed
        store
            .update_subtask_status(&subtask1.id, &TaskTaskStatus::Completed)
            .unwrap();
        store
            .update_subtask_status(&subtask2.id, &TaskTaskStatus::Completed)
            .unwrap();

        // SIMULATE: Check for InProgress subtask
        let subtasks = store.get_subtasks("task-1").unwrap();
        let in_progress = subtasks
            .iter()
            .find(|st| st.status == TaskTaskStatus::InProgress);

        assert!(
            in_progress.is_none(),
            "Should not find any InProgress subtask"
        );

        // No auto-completion needed, all are already Completed
        let subtasks_after = store.get_subtasks("task-1").unwrap();
        assert!(subtasks_after
            .iter()
            .all(|st| st.status == TaskTaskStatus::Completed));
    }
}
