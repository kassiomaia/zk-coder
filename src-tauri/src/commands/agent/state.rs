use std::sync::Arc;

use crate::application::agent::AgentServiceImpl;
use crate::core::agent::repositories::{
    ExecutionRepository, RuntimeTraceRepository, SessionRepository, TaskRepository,
};
use crate::infrastructure::agent::approval::ApprovalQueue;
use crate::infrastructure::agent::registry::AgentRegistry;
use crate::infrastructure::agent::runtime::AgentRuntime;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::sqlite::repositories::agent::SqliteSessionRepository;

/// Application state for agent commands.
pub struct AgentState {
    pub service: Arc<AgentServiceImpl>,
}

impl AgentState {
    /// Creates a new agent state.
    pub fn new(db_path: &str) -> Result<Self, String> {
        let sqlite_repository = Arc::new(
            SqliteSessionRepository::new(db_path)
                .map_err(|e| format!("Failed to init session repository: {e}"))?,
        );
        let session_repository: Arc<dyn SessionRepository> = sqlite_repository.clone();
        let task_repository: Arc<dyn TaskRepository> = sqlite_repository.clone();
        let telemetry_repository: Arc<dyn AgentTelemetryRepository> = sqlite_repository.clone();
        let execution_repository: Arc<dyn ExecutionRepository> = sqlite_repository.clone();
        let trace_repository: Arc<dyn RuntimeTraceRepository> = sqlite_repository.clone();
        let approval_queue = Arc::new(ApprovalQueue::new());
        let tool_registry = Arc::new(
            crate::infrastructure::tools::ToolRegistry::new()
                .map_err(|e| format!("Failed to init tool registry: {e}"))?,
        );
        let runtime = Arc::new(AgentRuntime::new(
            session_repository.clone(),
            task_repository.clone(),
            telemetry_repository.clone(),
            execution_repository.clone(),
            trace_repository.clone(),
            approval_queue.clone(),
            tool_registry,
        ));
        let registry = Arc::new(AgentRegistry::new());

        let service = Arc::new(
            AgentServiceImpl::new(
                registry,
                session_repository,
                task_repository,
                telemetry_repository,
                execution_repository,
                trace_repository,
                approval_queue,
                runtime,
            )
            .map_err(|e| format!("Failed to create agent service: {e}"))?,
        );

        service
            .recover_unterminated_executions()
            .map_err(|e| format!("Failed to recover unterminated agent executions: {e}"))?;

        Ok(Self { service })
    }
}
