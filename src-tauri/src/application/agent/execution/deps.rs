use std::sync::Arc;

use tauri::AppHandle;

use super::tracing::RuntimeTraceService;
use crate::core::agent::repositories::{
    ExecutionRepository, RuntimeTraceRepository, SessionRepository, TaskRepository,
};
use crate::core::llm::entities::ToolDefinition;
use crate::infrastructure::agent::approval::ApprovalQueue;
use crate::infrastructure::agent::file_snapshot_registry::FileSnapshotRegistry;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::tools::ToolRegistry;

pub struct ExecutionDeps<'a> {
    pub app_handle: &'a AppHandle,
    pub session_repo: &'a dyn SessionRepository,
    pub task_repo: &'a dyn TaskRepository,
    pub telemetry_repo: &'a dyn AgentTelemetryRepository,
    pub execution_repo: &'a dyn ExecutionRepository,
    pub trace_repo: &'a dyn RuntimeTraceRepository,
    pub approval_queue: &'a Arc<ApprovalQueue>,
    pub tool_registry: &'a Arc<ToolRegistry>,
    pub file_snapshot_registry: &'a Arc<FileSnapshotRegistry>,
    pub all_tool_definitions: &'a [ToolDefinition],
}

impl<'a> ExecutionDeps<'a> {
    pub fn tracer(&self) -> RuntimeTraceService<'_> {
        RuntimeTraceService::new(self.execution_repo, self.trace_repo, self.app_handle)
    }
}
