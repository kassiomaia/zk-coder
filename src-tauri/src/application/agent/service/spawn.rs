//! Spawn orchestration and telemetry wiring for agent-driven process execution.

use tauri::AppHandle;

use crate::application::agent::execution::tracing::RuntimeTraceService;
use crate::core::agent::entities::{SessionId as CoreSessionId, TaskStatus};
use crate::core::agent::execution::RuntimeTraceType;
use crate::core::agent::session::{SessionId, SessionState as CoreSessionState};
use crate::infrastructure::agent::runtime::spawn::{
    SpawnEngine, SpawnError, SpawnObserver, SpawnOutcome, SpawnRequest,
};
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;

use super::AgentServiceImpl;

struct SpawnTraceObserver<'a> {
    tracer: RuntimeTraceService<'a>,
    telemetry_repo: &'a dyn AgentTelemetryRepository,
    session_id: String,
    task_id: String,
    subtask_id: Option<String>,
    step_number: u32,
}

impl SpawnObserver for SpawnTraceObserver<'_> {
    fn record_trace(&self, trace_type: RuntimeTraceType, payload: serde_json::Value) {
        let _ = self.tracer.append_trace(
            &self.session_id,
            &self.task_id,
            self.subtask_id.as_deref(),
            self.step_number,
            trace_type,
            payload,
        );
    }

    fn record_terminal_line(&self, line_type: &str, content: &str) {
        let _ = self.telemetry_repo.add_terminal_line(
            &SessionId(self.session_id.clone()),
            line_type,
            content,
        );
    }

    fn is_cancelled(&self) -> bool {
        self.telemetry_repo
            .get_session(&SessionId(self.session_id.clone()))
            .ok()
            .flatten()
            .map(|session| matches!(session.state, CoreSessionState::Cancelled { .. }))
            .unwrap_or(false)
    }
}

fn resolve_trace_context(
    service: &AgentServiceImpl,
    session_id: &str,
) -> (String, Option<String>, u32) {
    let tasks = service
        .task_repo
        .get_session_tasks(session_id)
        .unwrap_or_default();
    let task_id = tasks
        .iter()
        .find(|task| task.status == TaskStatus::InProgress)
        .or_else(|| tasks.first())
        .map(|task| task.id.clone())
        .unwrap_or_else(|| session_id.to_string());

    let subtask_id = if task_id == session_id {
        None
    } else {
        service
            .task_repo
            .get_subtasks(&task_id)
            .unwrap_or_default()
            .into_iter()
            .find(|task| task.status == TaskStatus::InProgress)
            .map(|task| task.id)
    };

    let step_number = service
        .execution_repo
        .get_execution_snapshot(session_id)
        .ok()
        .flatten()
        .map(|snapshot| snapshot.step_number)
        .unwrap_or(0);

    (task_id, subtask_id, step_number)
}

impl AgentServiceImpl {
    /// Execute a process spawn with telemetry when the session context is available.
    pub async fn spawn_command(
        &self,
        request: SpawnRequest,
        app_handle: &AppHandle,
    ) -> Result<SpawnOutcome, SpawnError> {
        let observer = self
            .session_repo
            .get_session(&CoreSessionId(request.session_id.clone()))
            .ok()
            .map(|_| {
                let (task_id, subtask_id, step_number) =
                    resolve_trace_context(self, &request.session_id);
                SpawnTraceObserver {
                    tracer: RuntimeTraceService::new(
                        self.execution_repo.as_ref(),
                        self.trace_repo.as_ref(),
                        app_handle,
                    ),
                    telemetry_repo: self.telemetry_repo.as_ref(),
                    session_id: request.session_id.clone(),
                    task_id,
                    subtask_id,
                    step_number,
                }
            });

        SpawnEngine::shared()
            .execute(
                request,
                app_handle,
                observer
                    .as_ref()
                    .map(|observer| observer as &dyn SpawnObserver),
            )
            .await
    }
}
