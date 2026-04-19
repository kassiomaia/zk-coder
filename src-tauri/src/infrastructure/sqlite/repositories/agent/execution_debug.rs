//! Persistence for execution snapshots and runtime traces.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};

use super::SessionRepositoryError;
use crate::core::agent::execution::{
    ExecutionPhase, ExecutionSnapshot, RuntimeTrace, RuntimeTraceType,
};

/// Converts an [`ExecutionPhase`] to a string representation for storage.
const fn phase_to_string(phase: ExecutionPhase) -> &'static str {
    match phase {
        ExecutionPhase::Bootstrapping => "bootstrapping",
        ExecutionPhase::PreparingStep => "preparing_step",
        ExecutionPhase::ResolvingContext => "resolving_context",
        ExecutionPhase::CallingLlm => "calling_llm",
        ExecutionPhase::HandlingLlmResponse => "handling_llm_response",
        ExecutionPhase::ExecutingTools => "executing_tools",
        ExecutionPhase::Completed => "completed",
        ExecutionPhase::Failed => "failed",
        ExecutionPhase::Cancelled => "cancelled",
    }
}

/// Converts a stored string representation back to an [`ExecutionPhase`].
fn string_to_phase(value: &str) -> ExecutionPhase {
    match value {
        "bootstrapping" => ExecutionPhase::Bootstrapping,
        "preparing_step" => ExecutionPhase::PreparingStep,
        "resolving_context" => ExecutionPhase::ResolvingContext,
        "calling_llm" => ExecutionPhase::CallingLlm,
        "handling_llm_response" => ExecutionPhase::HandlingLlmResponse,
        "executing_tools" => ExecutionPhase::ExecutingTools,
        "completed" => ExecutionPhase::Completed,
        "failed" => ExecutionPhase::Failed,
        "cancelled" => ExecutionPhase::Cancelled,
        _ => ExecutionPhase::Bootstrapping,
    }
}

/// Converts a [`RuntimeTraceType`] to a string representation for storage.
const fn trace_type_to_string(trace_type: &RuntimeTraceType) -> &'static str {
    match trace_type {
        RuntimeTraceType::ExecutionStarted => "execution_started",
        RuntimeTraceType::StepStarted => "step_started",
        RuntimeTraceType::StepFinished => "step_finished",
        RuntimeTraceType::PhaseTransition => "phase_transition",
        RuntimeTraceType::SubtaskContextResolved => "subtask_context_resolved",
        RuntimeTraceType::RetrievalStarted => "retrieval_started",
        RuntimeTraceType::RetrievalCompleted => "retrieval_completed",
        RuntimeTraceType::ContextPolicyEvaluated => "context_policy_evaluated",
        RuntimeTraceType::ToolsetFiltered => "toolset_filtered",
        RuntimeTraceType::LlmRequestPrepared => "llm_request_prepared",
        RuntimeTraceType::LlmRequestStarted => "llm_request_started",
        RuntimeTraceType::LlmRetryScheduled => "llm_retry_scheduled",
        RuntimeTraceType::LlmResponseReceived => "llm_response_received",
        RuntimeTraceType::LlmResponseClassified => "llm_response_classified",
        RuntimeTraceType::ToolBatchStarted => "tool_batch_started",
        RuntimeTraceType::ToolBatchCompleted => "tool_batch_completed",
        RuntimeTraceType::SpawnStarted => "spawn_started",
        RuntimeTraceType::SpawnFinished => "spawn_finished",
        RuntimeTraceType::SpawnDenied => "spawn_denied",
        RuntimeTraceType::SpawnFailed => "spawn_failed",
        RuntimeTraceType::SpawnTimedOut => "spawn_timed_out",
        RuntimeTraceType::SpawnCancelled => "spawn_cancelled",
        RuntimeTraceType::SpawnKilled => "spawn_killed",
        RuntimeTraceType::SpawnBusy => "spawn_busy",
        RuntimeTraceType::ExecutionCancelled => "execution_cancelled",
        RuntimeTraceType::ExecutionFailed => "execution_failed",
        RuntimeTraceType::ExecutionCompleted => "execution_completed",
        RuntimeTraceType::MemoryRefreshed => "memory_refreshed",
        RuntimeTraceType::SubtaskScopeDrift => "subtask_scope_drift",
        RuntimeTraceType::AwaitingClarification => "awaiting_clarification",
        RuntimeTraceType::SubtaskFailedByRuntime => "subtask_failed_by_runtime",
    }
}

/// Converts a stored string representation back to a [`RuntimeTraceType`].
fn string_to_trace_type(value: &str) -> RuntimeTraceType {
    match value {
        "execution_started" => RuntimeTraceType::ExecutionStarted,
        "step_started" => RuntimeTraceType::StepStarted,
        "step_finished" => RuntimeTraceType::StepFinished,
        "phase_transition" => RuntimeTraceType::PhaseTransition,
        "subtask_context_resolved" => RuntimeTraceType::SubtaskContextResolved,
        "retrieval_started" => RuntimeTraceType::RetrievalStarted,
        "retrieval_completed" => RuntimeTraceType::RetrievalCompleted,
        "context_policy_evaluated" => RuntimeTraceType::ContextPolicyEvaluated,
        "toolset_filtered" => RuntimeTraceType::ToolsetFiltered,
        "llm_request_prepared" => RuntimeTraceType::LlmRequestPrepared,
        "llm_request_started" => RuntimeTraceType::LlmRequestStarted,
        "llm_retry_scheduled" => RuntimeTraceType::LlmRetryScheduled,
        "llm_response_received" => RuntimeTraceType::LlmResponseReceived,
        "llm_response_classified" => RuntimeTraceType::LlmResponseClassified,
        "tool_batch_started" => RuntimeTraceType::ToolBatchStarted,
        "tool_batch_completed" => RuntimeTraceType::ToolBatchCompleted,
        "spawn_started" => RuntimeTraceType::SpawnStarted,
        "spawn_finished" => RuntimeTraceType::SpawnFinished,
        "spawn_denied" => RuntimeTraceType::SpawnDenied,
        "spawn_failed" => RuntimeTraceType::SpawnFailed,
        "spawn_timed_out" => RuntimeTraceType::SpawnTimedOut,
        "spawn_cancelled" => RuntimeTraceType::SpawnCancelled,
        "spawn_killed" => RuntimeTraceType::SpawnKilled,
        "spawn_busy" => RuntimeTraceType::SpawnBusy,
        "execution_cancelled" => RuntimeTraceType::ExecutionCancelled,
        "execution_failed" => RuntimeTraceType::ExecutionFailed,
        "execution_completed" => RuntimeTraceType::ExecutionCompleted,
        "memory_refreshed" => RuntimeTraceType::MemoryRefreshed,
        "subtask_scope_drift" => RuntimeTraceType::SubtaskScopeDrift,
        "awaiting_clarification" => RuntimeTraceType::AwaitingClarification,
        "subtask_failed_by_runtime" => RuntimeTraceType::SubtaskFailedByRuntime,
        _ => RuntimeTraceType::ExecutionStarted,
    }
}

/// Saves an execution snapshot to the database.
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn save_execution_snapshot(
    conn: &Connection,
    snapshot: &ExecutionSnapshot,
) -> Result<(), SessionRepositoryError> {
    conn.execute(
        "INSERT INTO agent_execution_snapshots (
            session_id, task_id, step_number, phase, current_subtask_id,
            context_usage_percent, total_input_tokens, total_output_tokens,
            last_model, last_error, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(session_id) DO UPDATE SET
            task_id=excluded.task_id,
            step_number=excluded.step_number,
            phase=excluded.phase,
            current_subtask_id=excluded.current_subtask_id,
            context_usage_percent=excluded.context_usage_percent,
            total_input_tokens=excluded.total_input_tokens,
            total_output_tokens=excluded.total_output_tokens,
            last_model=excluded.last_model,
            last_error=excluded.last_error,
            updated_at=excluded.updated_at",
        params![
            snapshot.session_id,
            snapshot.task_id,
            snapshot.step_number,
            phase_to_string(snapshot.phase),
            snapshot.current_subtask_id,
            snapshot.context_usage_percent,
            i64::from(snapshot.total_input_tokens),
            i64::from(snapshot.total_output_tokens),
            snapshot.last_model,
            snapshot.last_error,
            snapshot.updated_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// Retrieves an execution snapshot by session ID.
///
/// # Errors
///
/// Returns an error if the database query fails.
pub fn get_execution_snapshot(
    conn: &Connection,
    session_id: &str,
) -> Result<Option<ExecutionSnapshot>, SessionRepositoryError> {
    let mut stmt = conn.prepare(
        "SELECT session_id, task_id, step_number, phase, current_subtask_id,
                context_usage_percent, total_input_tokens, total_output_tokens,
                last_model, last_error, updated_at
         FROM agent_execution_snapshots WHERE session_id = ?1",
    )?;

    let mut rows = stmt.query(params![session_id])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };

    let updated_at: String = row.get(10)?;
    Ok(Some(ExecutionSnapshot {
        session_id: row.get(0)?,
        task_id: row.get(1)?,
        step_number: row.get::<_, i64>(2)? as u32,
        phase: string_to_phase(&row.get::<_, String>(3)?),
        current_subtask_id: row.get(4)?,
        context_usage_percent: row.get(5)?,
        total_input_tokens: row.get::<_, i64>(6)? as u32,
        total_output_tokens: row.get::<_, i64>(7)? as u32,
        last_model: row.get(8)?,
        last_error: row.get(9)?,
        updated_at: DateTime::parse_from_rfc3339(&updated_at)
            .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc)),
    }))
}

/// Clears an execution snapshot from the database.
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn clear_execution_snapshot(
    conn: &Connection,
    session_id: &str,
) -> Result<(), SessionRepositoryError> {
    conn.execute(
        "DELETE FROM agent_execution_snapshots WHERE session_id = ?1",
        params![session_id],
    )?;
    Ok(())
}

/// Appends a runtime trace entry to the database.
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn append_runtime_trace(
    conn: &Connection,
    trace: &RuntimeTrace,
) -> Result<(), SessionRepositoryError> {
    conn.execute(
        "INSERT INTO agent_runtime_traces (
            id, session_id, task_id, subtask_id, step_number, trace_type, payload_json, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            trace.id,
            trace.session_id,
            trace.task_id,
            trace.subtask_id,
            i64::from(trace.step_number),
            trace_type_to_string(&trace.trace_type),
            serde_json::to_string(&trace.payload_json)?,
            trace.created_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// Retrieves runtime traces for a session, optionally limited to a specific count.
///
/// # Errors
///
/// Returns an error if the database query fails.
pub fn get_runtime_traces(
    conn: &Connection,
    session_id: &str,
    limit: Option<usize>,
) -> Result<Vec<RuntimeTrace>, SessionRepositoryError> {
    let sql = if limit.is_some() {
        "SELECT id, session_id, task_id, subtask_id, step_number, trace_type, payload_json, created_at
         FROM agent_runtime_traces WHERE session_id = ?1 ORDER BY created_at ASC LIMIT ?2"
    } else {
        "SELECT id, session_id, task_id, subtask_id, step_number, trace_type, payload_json, created_at
         FROM agent_runtime_traces WHERE session_id = ?1 ORDER BY created_at ASC"
    };
    let mut stmt = conn.prepare(sql)?;
    let mapper = |row: &rusqlite::Row<'_>| -> Result<RuntimeTrace, rusqlite::Error> {
        let payload_json: String = row.get(6)?;
        let created_at: String = row.get(7)?;
        Ok(RuntimeTrace {
            id: row.get(0)?,
            session_id: row.get(1)?,
            task_id: row.get(2)?,
            subtask_id: row.get(3)?,
            step_number: row.get::<_, i64>(4)? as u32,
            trace_type: string_to_trace_type(&row.get::<_, String>(5)?),
            payload_json: serde_json::from_str(&payload_json)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
            created_at: DateTime::parse_from_rfc3339(&created_at)
                .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc)),
        })
    };

    let traces = if let Some(limit) = limit {
        stmt.query_map(params![session_id, limit as i64], mapper)?
            .collect::<Result<Vec<_>, _>>()?
    } else {
        stmt.query_map(params![session_id], mapper)?
            .collect::<Result<Vec<_>, _>>()?
    };
    Ok(traces)
}
