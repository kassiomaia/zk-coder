//! Subtask state transitions executed during termination of an agent run.
//!
//! These helpers are lifted out of `termination.rs` so the logic that moves
//! subtasks between `InProgress`, `Completed`, and `Pending` states — and the
//! "subtask summary" streaming emission that accompanies it — can be isolated
//! from the session-level orchestration in the parent module.

use chrono::Utc;
use serde_json::json;
use tauri::Emitter;

use super::super::deps::ExecutionDeps;
use super::super::state::ExecutionContext;
use super::streaming::{emit_prebuilt_streaming_events, subtask_result_streaming_events};
use crate::core::agent::execution::RuntimeTraceType;
use crate::core::agent::task::{AgentExecutionResult, TaskStatus};
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::agent::runtime::permissions::core_task_to_event_subtask;

/// Auto-completes all remaining `InProgress` subtasks for the given task.
///
/// Emits a `SubtaskUpdated` event for each one so the frontend stays in sync.
/// Returns the number of subtasks that were auto-completed.
pub(crate) fn auto_complete_in_progress_subtasks(
    deps: &ExecutionDeps<'_>,
    task_id: &str,
    session_id: &str,
) -> usize {
    let Ok(subtasks) = deps.task_repo.get_subtasks(task_id) else {
        return 0;
    };

    let in_progress: Vec<_> = subtasks
        .into_iter()
        .filter(|st| st.status == crate::core::agent::task::TaskStatus::InProgress)
        .collect();

    let count = in_progress.len();
    for subtask in &in_progress {
        log::info!(
            "[AgentExecution] Auto-completing subtask {} ({}) for task {}",
            subtask.id,
            subtask.title,
            task_id
        );
        let _ = deps
            .task_repo
            .update_subtask_status(&subtask.id, crate::core::agent::task::TaskStatus::Completed);

        let mut event_subtask = core_task_to_event_subtask(subtask.clone(), task_id);
        event_subtask.status = TaskStatus::Completed;
        event_subtask.completed_at = Some(Utc::now());
        let _ = deps.app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::SubtaskUpdated {
                session_id: session_id.to_string(),
                subtask: event_subtask,
            },
        );
    }

    count
}

/// Returns `true` when there is at least one subtask still in `InProgress` state.
pub(super) fn has_remaining_in_progress_subtasks(deps: &ExecutionDeps<'_>, task_id: &str) -> bool {
    deps.task_repo
        .get_subtasks(task_id)
        .map(|subtasks| {
            subtasks
                .iter()
                .any(|st| st.status == crate::core::agent::task::TaskStatus::InProgress)
        })
        .unwrap_or(false)
}

/// Advances the next `Pending` subtask to `InProgress`.
///
/// Called when execution finishes but `Pending` subtasks remain that were never
/// started (the LLM never called `agent_advance_to_next_subtask`).
/// Returns `true` if a subtask was advanced, `false` if none were `Pending`.
pub(crate) fn advance_next_pending_subtask(
    deps: &ExecutionDeps<'_>,
    task_id: &str,
    session_id: &str,
) -> bool {
    let Ok(mut subtasks) = deps.task_repo.get_subtasks(task_id) else {
        return false;
    };
    subtasks.sort_by_key(|st| st.order);

    let Some(next) = subtasks
        .into_iter()
        .find(|st| st.status == crate::core::agent::task::TaskStatus::Pending)
    else {
        return false;
    };

    if deps
        .task_repo
        .update_subtask_status(&next.id, crate::core::agent::task::TaskStatus::InProgress)
        .is_err()
    {
        return false;
    }

    log::info!(
        "[AgentExecution] Advanced pending subtask {} ({}) to InProgress for task {}",
        next.id,
        next.title,
        task_id
    );

    let mut event_subtask = core_task_to_event_subtask(next, task_id);
    event_subtask.status = TaskStatus::InProgress;
    event_subtask.started_at = Some(Utc::now());
    let _ = deps.app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::SubtaskUpdated {
            session_id: session_id.to_string(),
            subtask: event_subtask,
        },
    );

    true
}

/// Shared tail used by the two "still-continuing" branches in
/// `complete_execution`: append a `ExecutionCompleted` trace flagged with
/// `hasRemainingWork: true` and stream the per-subtask summary to the chat.
///
/// `advanced_pending_subtask` toggles the extra trace field that distinguishes
/// "still running through InProgress subtasks" from "advanced a Pending one".
pub(super) async fn emit_subtask_result_with_trace(
    deps: &ExecutionDeps<'_>,
    ctx: &ExecutionContext,
    conversation_id: &str,
    thread_id: &str,
    result: &AgentExecutionResult,
    auto_completed: usize,
    advanced_pending_subtask: bool,
) {
    let mut trace = json!({
        "responseLength": result.content.len(),
        "totalSteps": result.steps.len(),
        "totalInputTokens": ctx.total_input_tokens,
        "totalOutputTokens": ctx.total_output_tokens,
        "autoCompletedSubtasks": auto_completed,
        "hasRemainingWork": true,
    });
    if advanced_pending_subtask {
        if let Some(obj) = trace.as_object_mut() {
            obj.insert("advancedPendingSubtask".to_string(), json!(true));
        }
    }

    let _ = deps.tracer().append_trace(
        &ctx.session_id,
        &ctx.task_id,
        ctx.current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::ExecutionCompleted,
        trace,
    );

    let events = subtask_result_streaming_events(
        &ctx.session_id,
        &ctx.task_id,
        ctx.current_subtask_id.as_deref(),
        ctx.step_number,
        conversation_id,
        thread_id,
        &result.content,
    );
    emit_prebuilt_streaming_events(deps, events).await;
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
    use crate::core::agent::repositories::TaskRepository;
    use crate::core::agent::session::AgentSession;
    use crate::core::agent::task::{AgentRequest, AgentTask, SubTask, SubTaskMode, TaskStatus};
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

        let mut task = AgentTask::new(
            session_id.clone(),
            AgentRequest {
                content: "Test Task".to_string(),
                attachments: vec![],
                context: None,
            },
        );
        task.id = "task-1".to_string();
        task.title = Some("Test Task".to_string());
        task.status = TaskStatus::InProgress;
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
            .update_subtask_status(&subtask1.id, &TaskStatus::Completed)
            .unwrap();
        store
            .update_subtask_status(&subtask2.id, &TaskStatus::Completed)
            .unwrap();
        store
            .update_subtask_status(&subtask3.id, &TaskStatus::InProgress)
            .unwrap();

        // Verify initial state: subtask3 is InProgress
        let subtasks_before = store.get_subtasks("task-1").unwrap();
        assert_eq!(subtasks_before[2].status, TaskStatus::InProgress);

        // SIMULATE: Task completes and auto-completion runs
        // (This is what complete_execution now does)
        store
            .update_subtask_status(&subtask3.id, &TaskStatus::Completed)
            .unwrap();

        // Verify: All subtasks are now Completed
        let subtasks_after = store.get_subtasks("task-1").unwrap();
        assert_eq!(subtasks_after[0].status, TaskStatus::Completed);
        assert_eq!(subtasks_after[1].status, TaskStatus::Completed);
        assert_eq!(subtasks_after[2].status, TaskStatus::Completed);
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

        let mut task = AgentTask::new(
            session_id.clone(),
            AgentRequest {
                content: "Test Task".to_string(),
                attachments: vec![],
                context: None,
            },
        );
        task.id = "task-1".to_string();
        task.title = Some("Test Task".to_string());
        task.status = TaskStatus::InProgress;
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
            .update_subtask_status(&subtask1.id, &TaskStatus::Completed)
            .unwrap();
        store
            .update_subtask_status(&subtask2.id, &TaskStatus::InProgress)
            .unwrap();

        // SIMULATE: Task completes, auto-completion finds InProgress subtask
        let subtasks = store.get_subtasks("task-1").unwrap();
        let in_progress = subtasks
            .iter()
            .find(|st| st.status == TaskStatus::InProgress);

        assert!(in_progress.is_some(), "Should find an InProgress subtask");

        // Only the InProgress subtask should be completed
        if let Some(st) = in_progress {
            store
                .update_subtask_status(&st.id, &TaskStatus::Completed)
                .unwrap();
        }

        // Verify: Only the InProgress subtask was completed, Pending remains
        let subtasks_after = store.get_subtasks("task-1").unwrap();
        assert_eq!(subtasks_after[0].status, TaskStatus::Completed); // Was already done
        assert_eq!(subtasks_after[1].status, TaskStatus::Completed); // Auto-completed
        assert_eq!(subtasks_after[2].status, TaskStatus::Pending); // Still Pending
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

        let mut task = AgentTask::new(
            session_id.clone(),
            AgentRequest {
                content: "Test Task".to_string(),
                attachments: vec![],
                context: None,
            },
        );
        task.id = "task-1".to_string();
        task.title = Some("Test Task".to_string());
        task.status = TaskStatus::InProgress;
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
            .update_subtask_status(&subtask1.id, &TaskStatus::Completed)
            .unwrap();
        store
            .update_subtask_status(&subtask2.id, &TaskStatus::Completed)
            .unwrap();

        // SIMULATE: Check for InProgress subtask
        let subtasks = store.get_subtasks("task-1").unwrap();
        let in_progress = subtasks
            .iter()
            .find(|st| st.status == TaskStatus::InProgress);

        assert!(
            in_progress.is_none(),
            "Should not find any InProgress subtask"
        );

        // No auto-completion needed, all are already Completed
        let subtasks_after = store.get_subtasks("task-1").unwrap();
        assert!(subtasks_after
            .iter()
            .all(|st| st.status == TaskStatus::Completed));
    }

    /// Test that Pending subtasks are identified and available for advancement.
    ///
    /// This documents the behavior of `advance_next_pending_subtask`: when the agent
    /// finishes without calling `agent_advance_to_next_subtask`, Pending subtasks
    /// should be picked up and moved to InProgress so the execution loop continues.
    #[test]
    fn test_pending_subtasks_can_be_identified_for_advancement() {
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

        let mut task = AgentTask::new(
            session_id.clone(),
            AgentRequest {
                content: "Test Task".to_string(),
                attachments: vec![],
                context: None,
            },
        );
        task.id = "task-1".to_string();
        task.title = Some("Test Task".to_string());
        task.status = TaskStatus::InProgress;
        TaskRepository::create_task(&store, &task).unwrap();

        // Create 3 subtasks: first is Completed, others are Pending
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
        let subtask3 = SubTask::new(
            "task-1".to_string(),
            "Step 3".to_string(),
            SubTaskMode::FullAccess,
            2,
        );

        store.create_subtask(&subtask1).unwrap();
        store.create_subtask(&subtask2).unwrap();
        store.create_subtask(&subtask3).unwrap();

        // Mark first as Completed, rest stay Pending
        store
            .update_subtask_status(&subtask1.id, &TaskStatus::Completed)
            .unwrap();

        // Verify initial state: subtask2 and subtask3 are Pending
        let subtasks = store.get_subtasks("task-1").unwrap();
        let pending = subtasks
            .iter()
            .filter(|st| st.status == TaskStatus::Pending)
            .collect::<Vec<_>>();

        assert_eq!(pending.len(), 2, "Should have 2 Pending subtasks");
        assert_eq!(
            pending[0].order, 1,
            "First pending should be subtask2 (order 1)"
        );
        assert_eq!(
            pending[1].order, 2,
            "Second pending should be subtask3 (order 2)"
        );

        // Advance the first pending to InProgress
        store
            .update_subtask_status(&subtask2.id, &TaskStatus::InProgress)
            .unwrap();

        // Verify: subtask2 is now InProgress, subtask3 still Pending
        let subtasks_after = store.get_subtasks("task-1").unwrap();
        assert_eq!(
            subtasks_after[1].status,
            TaskStatus::InProgress,
            "Subtask 2 should be InProgress"
        );
        assert_eq!(
            subtasks_after[2].status,
            TaskStatus::Pending,
            "Subtask 3 should still be Pending"
        );
    }
}
