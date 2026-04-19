//! Defines the dependency container for the agent execution process.
//!
//! This module provides the `ExecutionDeps` struct, which aggregates all the
//! necessary dependencies for running an agent, making them easily accessible
//! throughout the execution lifecycle.

use std::sync::Arc;

use tauri::AppHandle;

use super::tracing::RuntimeTraceService;
use crate::application::agent::harness::HarnessStateStore;
use crate::core::agent::memory::MemoryRepository;
use crate::core::agent::repositories::{
    ExecutionCheckpointRepository, ExecutionRepository, RuntimeTraceRepository, SessionRepository,
    TaskRepository,
};
use crate::core::chat::services::ChatService;
use crate::core::config::ConfigRepository;
use crate::core::llm::entities::ToolDefinition;
use crate::infrastructure::agent::approval::ApprovalQueue;
use crate::infrastructure::agent::file_snapshot_registry::FileSnapshotRegistry;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::tools::ToolRegistry;

/// A container for all the dependencies required for an agent's execution.
pub struct ExecutionDeps<'a> {
    /// A handle to the Tauri application.
    pub app_handle: &'a AppHandle,
    /// The repository for managing agent sessions.
    pub session_repo: &'a dyn SessionRepository,
    /// The chat service for thread metadata lookups.
    pub chat_service: &'a dyn ChatService,
    /// The configuration repository, used to resolve LLM provider/model
    /// metadata when the runtime needs to construct an LLM client.
    pub config_repository: &'a dyn ConfigRepository,
    /// The repository for managing agent tasks.
    pub task_repo: &'a dyn TaskRepository,
    /// The repository for agent telemetry.
    pub telemetry_repo: &'a dyn AgentTelemetryRepository,
    /// The repository for execution checkpoints.
    pub execution_repo: &'a dyn ExecutionRepository,
    /// The repository for resume-capable execution checkpoints
    /// (agent-clarification-resume plan). Used by the park stage to
    /// persist a full `ExecutionCheckpoint` on pause entry and by the
    /// rehydrate path on startup.
    pub execution_checkpoint_repo: &'a dyn ExecutionCheckpointRepository,
    /// The repository for runtime traces.
    pub trace_repo: &'a dyn RuntimeTraceRepository,
    /// The repository for durable session working-memory artifacts
    /// (Milestone 3.1). Populated for every session; callers fetch or
    /// upsert `SessionMemory` rows keyed by session id.
    pub memory_repo: &'a dyn MemoryRepository,
    /// The queue for handling pending approvals.
    pub approval_queue: &'a Arc<ApprovalQueue>,
    /// The registry of available tools.
    pub tool_registry: &'a Arc<ToolRegistry>,
    /// The registry for file snapshots.
    pub file_snapshot_registry: &'a Arc<FileSnapshotRegistry>,
    /// The durable harness-state store shared with the application service.
    pub harness_state_store: &'a Arc<dyn HarnessStateStore>,
    /// A slice of all available tool definitions.
    pub all_tool_definitions: &'a [ToolDefinition],
}

impl ExecutionDeps<'_> {
    /// Returns a `RuntimeTraceService` for recording execution traces.
    #[must_use]
    pub fn tracer(&self) -> RuntimeTraceService<'_> {
        RuntimeTraceService::new(self.execution_repo, self.trace_repo, self.app_handle)
    }
}
