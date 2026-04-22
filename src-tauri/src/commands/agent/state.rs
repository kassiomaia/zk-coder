use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::application::agent::AgentServiceImpl;
use crate::core::agent::memory::MemoryRepository;
use crate::core::agent::repositories::{
    ClarificationRepository, ExecutionCheckpointRepository, ExecutionRepository,
    RuntimeTraceRepository, SessionRepository, TaskRepository,
};
use crate::core::chat::services::ChatService;
use crate::core::config::ConfigRepository;
use crate::infrastructure::agent::approval::ApprovalQueue;
use crate::infrastructure::agent::registry::AgentRegistry;
use crate::infrastructure::agent::runtime::AgentRuntime;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::sqlite::repositories::agent::{
    SqliteAgentTelemetryRepository, SqliteClarificationRepository, SqliteExecutionRepository,
    SqliteMemoryRepository, SqliteSessionRepository, SqliteTaskRepository,
};

/// Application state for agent commands.
pub struct AgentState {
    /// The agent service implementation.
    pub service: Arc<AgentServiceImpl>,
}

impl AgentState {
    /// Creates a new agent state.
    ///
    /// Note: Recovery of unterminated executions is handled separately
    /// via the `initialize_agent_recovery` command to allow event emission.
    pub fn new(
        db_path: &str,
        chat_service: Arc<dyn ChatService>,
        config_repository: Arc<dyn ConfigRepository>,
    ) -> Result<Self, String> {
        let session_repo = SqliteSessionRepository::new(db_path)
            .map_err(|e| format!("Failed to init session repository: {e}"))?;
        let conn = session_repo.conn.clone();
        let session_repository: Arc<dyn SessionRepository> = Arc::new(session_repo);
        let task_repository: Arc<dyn TaskRepository> =
            Arc::new(SqliteTaskRepository::from_shared(conn.clone()));
        let telemetry_repository: Arc<dyn AgentTelemetryRepository> = Arc::new(
            SqliteAgentTelemetryRepository::from_shared(conn.clone()),
        );
        let execution_shared = Arc::new(SqliteExecutionRepository::from_shared(conn.clone()));
        let execution_repository: Arc<dyn ExecutionRepository> = execution_shared.clone();
        let trace_repository: Arc<dyn RuntimeTraceRepository> = execution_shared;
        let clarification_shared =
            Arc::new(SqliteClarificationRepository::from_shared(conn.clone()));
        let clarification_repository: Arc<dyn ClarificationRepository> =
            clarification_shared.clone();
        let execution_checkpoint_repository: Arc<dyn ExecutionCheckpointRepository> =
            clarification_shared;
        let memory_repository: Arc<dyn MemoryRepository> =
            Arc::new(SqliteMemoryRepository::from_shared(conn));
        crate::application::agent::clarification::init_clarification_store(
            clarification_repository,
        );
        let approval_queue = Arc::new(ApprovalQueue::new());
        let tool_registry = Arc::new(
            crate::infrastructure::tools::ToolRegistry::new()
                .map_err(|e| format!("Failed to init tool registry: {e}"))?,
        );
        let runtime = Arc::new(AgentRuntime::new(
            session_repository.clone(),
            chat_service,
            task_repository.clone(),
            telemetry_repository.clone(),
            execution_repository.clone(),
            execution_checkpoint_repository,
            trace_repository.clone(),
            memory_repository,
            approval_queue.clone(),
            tool_registry,
            config_repository,
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

        Ok(Self { service })
    }

    /// Creates agent state from a shared database connection.
    ///
    /// The schema must already have been applied via `migrations::init_all`.
    ///
    /// # Errors
    ///
    /// Returns an error if any sub-component fails to initialize.
    pub fn from_shared(
        conn: Arc<Mutex<Connection>>,
        chat_service: Arc<dyn ChatService>,
        config_repository: Arc<dyn ConfigRepository>,
    ) -> Result<Self, String> {
        let session_repository: Arc<dyn SessionRepository> =
            Arc::new(SqliteSessionRepository::from_shared(conn.clone()));
        let task_repository: Arc<dyn TaskRepository> =
            Arc::new(SqliteTaskRepository::from_shared(conn.clone()));
        let telemetry_repository: Arc<dyn AgentTelemetryRepository> = Arc::new(
            SqliteAgentTelemetryRepository::from_shared(conn.clone()),
        );
        let execution_shared = Arc::new(SqliteExecutionRepository::from_shared(conn.clone()));
        let execution_repository: Arc<dyn ExecutionRepository> = execution_shared.clone();
        let trace_repository: Arc<dyn RuntimeTraceRepository> = execution_shared;
        let clarification_shared =
            Arc::new(SqliteClarificationRepository::from_shared(conn.clone()));
        let clarification_repository: Arc<dyn ClarificationRepository> =
            clarification_shared.clone();
        let execution_checkpoint_repository: Arc<dyn ExecutionCheckpointRepository> =
            clarification_shared;
        let memory_repository: Arc<dyn MemoryRepository> =
            Arc::new(SqliteMemoryRepository::from_shared(conn));
        crate::application::agent::clarification::init_clarification_store(
            clarification_repository,
        );
        let approval_queue = Arc::new(ApprovalQueue::new());
        let tool_registry = Arc::new(
            crate::infrastructure::tools::ToolRegistry::new()
                .map_err(|e| format!("Failed to init tool registry: {e}"))?,
        );
        let runtime = Arc::new(AgentRuntime::new(
            session_repository.clone(),
            chat_service,
            task_repository.clone(),
            telemetry_repository.clone(),
            execution_repository.clone(),
            execution_checkpoint_repository,
            trace_repository.clone(),
            memory_repository,
            approval_queue.clone(),
            tool_registry,
            config_repository,
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

        Ok(Self { service })
    }
}
