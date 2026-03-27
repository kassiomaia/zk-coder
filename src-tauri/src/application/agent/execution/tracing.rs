//! Runtime tracing helpers for agent execution.

use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager};

use crate::core::agent::execution::{
    ExecutionPhase, ExecutionSnapshot, ExecutionTransition, RuntimeTrace, RuntimeTraceType,
};
use crate::core::agent::repositories::{ExecutionRepository, RepositoryResult, RuntimeTraceRepository};
use crate::infrastructure::agent::events::{AgentTraceEvent, AGENT_TRACE_EVENT_NAME};

const TRACE_WINDOW_LABEL: &str = "agent-trace-dev";

/// Coordinates trace persistence and live event emission for execution internals.
pub struct RuntimeTraceService<'a> {
    execution_repo: &'a dyn ExecutionRepository,
    trace_repo: &'a dyn RuntimeTraceRepository,
    app_handle: &'a AppHandle,
}

impl<'a> RuntimeTraceService<'a> {
    pub fn new(
        execution_repo: &'a dyn ExecutionRepository,
        trace_repo: &'a dyn RuntimeTraceRepository,
        app_handle: &'a AppHandle,
    ) -> Self {
        Self {
            execution_repo,
            trace_repo,
            app_handle,
        }
    }

    pub fn save_snapshot(&self, snapshot: &ExecutionSnapshot) -> RepositoryResult<()> {
        self.execution_repo.save_execution_snapshot(snapshot)?;
        self.emit_trace_event(AgentTraceEvent::SnapshotUpdated {
            snapshot: snapshot.clone(),
        });
        Ok(())
    }

    pub fn clear_snapshot(&self, session_id: &str) -> RepositoryResult<()> {
        self.execution_repo.clear_execution_snapshot(session_id)
    }

    pub fn append_trace(
        &self,
        session_id: &str,
        task_id: &str,
        subtask_id: Option<&str>,
        step_number: u32,
        trace_type: RuntimeTraceType,
        payload_json: serde_json::Value,
    ) -> RepositoryResult<RuntimeTrace> {
        let trace = RuntimeTrace {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            task_id: task_id.to_string(),
            subtask_id: subtask_id.map(ToOwned::to_owned),
            step_number,
            trace_type,
            payload_json,
            created_at: Utc::now(),
        };
        self.trace_repo.append_runtime_trace(&trace)?;
        self.emit_trace_event(AgentTraceEvent::RuntimeTrace { trace: trace.clone() });
        Ok(trace)
    }

    fn emit_trace_event(&self, event: AgentTraceEvent) {
        if let Some(window) = self.app_handle.get_webview_window(TRACE_WINDOW_LABEL) {
            let _ = window.emit(AGENT_TRACE_EVENT_NAME, event);
        }
    }

    pub fn emit_transition(
        &self,
        session_id: &str,
        task_id: &str,
        step_number: u32,
        from: Option<ExecutionPhase>,
        to: ExecutionPhase,
        reason: impl Into<String>,
    ) -> RepositoryResult<RuntimeTrace> {
        let transition = ExecutionTransition {
            from,
            to,
            reason: reason.into(),
        };
        self.append_trace(
            session_id,
            task_id,
            None,
            step_number,
            RuntimeTraceType::PhaseTransition,
            serde_json::to_value(transition).unwrap_or(serde_json::Value::Null),
        )
    }
}
