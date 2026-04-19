//! Agent runtime for executing requests with permission-checked tool calls.
//!
//! This module is organized into submodules for better maintainability:
//!
//! - [`types`] - Error types and helper structs
//! - [`client`] - LLM client creation and utilities
//! - [`context`] - Context compaction and token estimation
//! - [`loop_detection`] - Repetitive loop detection
//! - [`execution`] - Main execution orchestration
//! - [`permissions`] - Permission handling and tool execution
//! - [`helpers`] - Utility functions
//! - [`spawn`] - Process execution with policy and telemetry controls

pub(crate) mod artifact_probe;
pub mod client;
pub(crate) mod session_scope;
pub(crate) mod tool_handlers;
pub mod context;
pub mod execution;
pub mod helpers;
pub mod loop_detection;
pub mod permissions;
pub mod request_shape;
pub mod spawn;
pub mod summarizer;
mod tool_feedback;
pub mod types;

use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use crate::application::agent::harness::{HarnessStateStore, InMemoryHarnessStateStore};
use crate::core::agent::memory::MemoryRepository;
use crate::core::agent::repositories::{
    ExecutionCheckpointRepository, ExecutionRepository, RuntimeTraceRepository, SessionRepository,
    TaskRepository,
};
use crate::core::agent::session::AgentSession;
use crate::core::agent::task::{AgentExecutionResult, AgentRequest};
use crate::core::agent::types::TrustMode;
use crate::core::chat::services::ChatService;
use crate::infrastructure::agent::approval::ApprovalQueue;
use crate::infrastructure::agent::events::AGENT_EVENT_NAME;
use crate::infrastructure::agent::file_snapshot_registry::FileSnapshotRegistry;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::tools::ToolRegistry;

// Re-export submodule types
pub use client::{chat_with_retry, create_llm_client, LlmRetryConfig};
pub use context::{
    apply_context_policy_v2, estimate_context_tokens, maybe_compact_context, CompactionDetails,
    ContextPolicyInput, COMPRESSION_THRESHOLD, CONTEXT_WINDOW, KEEP_RECENT_MESSAGES,
    KEEP_RECENT_TAIL_MESSAGES, adaptive_compression_threshold, model_context_window,
};
pub use execution::execute;
pub use helpers::{derive_system_prompt, generate_diff_from_content, infer_action_type};
pub use loop_detection::{
    add_to_recent_calls, detect_repetitive_loop, detect_repetitive_loop_in_state,
    hash_tool_arguments, LOOP_SKIP_TOOLS, MAX_REPETITIONS,
};
pub use permissions::process_tool_calls_with_permissions;
pub use spawn::{
    SpawnEngine, SpawnError, SpawnInvocation, SpawnMode, SpawnObserver, SpawnOutcome, SpawnPolicy,
    SpawnRequest,
};
pub use summarizer::summarize_dropped_messages;
pub use types::{AgentRuntimeError, SubtaskSummary};

/// The agent runtime orchestrates LLM + tool execution with permissions.
pub struct AgentRuntime {
    session_repo: Arc<dyn SessionRepository>,
    chat_service: Arc<dyn ChatService>,
    task_repo: Arc<dyn TaskRepository>,
    telemetry_repo: Arc<dyn AgentTelemetryRepository>,
    execution_repo: Arc<dyn ExecutionRepository>,
    execution_checkpoint_repo: Arc<dyn ExecutionCheckpointRepository>,
    trace_repo: Arc<dyn RuntimeTraceRepository>,
    memory_repo: Arc<dyn MemoryRepository>,
    approval_queue: Arc<ApprovalQueue>,
    tool_registry: Arc<ToolRegistry>,
    file_snapshot_registry: Arc<FileSnapshotRegistry>,
    harness_state_store: Arc<dyn HarnessStateStore>,
}

impl AgentRuntime {
    /// Create a new agent runtime.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_repo: Arc<dyn SessionRepository>,
        chat_service: Arc<dyn ChatService>,
        task_repo: Arc<dyn TaskRepository>,
        telemetry_repo: Arc<dyn AgentTelemetryRepository>,
        execution_repo: Arc<dyn ExecutionRepository>,
        execution_checkpoint_repo: Arc<dyn ExecutionCheckpointRepository>,
        trace_repo: Arc<dyn RuntimeTraceRepository>,
        memory_repo: Arc<dyn MemoryRepository>,
        approval_queue: Arc<ApprovalQueue>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Self {
        Self {
            session_repo,
            chat_service,
            task_repo,
            telemetry_repo,
            execution_repo,
            execution_checkpoint_repo,
            trace_repo,
            memory_repo,
            approval_queue,
            tool_registry,
            file_snapshot_registry: Arc::new(FileSnapshotRegistry::new()),
            harness_state_store: Arc::new(InMemoryHarnessStateStore::new()),
        }
    }

    /// Get the file snapshot registry for external access (e.g., Tauri commands).
    pub fn file_snapshot_registry(&self) -> &Arc<FileSnapshotRegistry> {
        &self.file_snapshot_registry
    }

    /// Get the harness state store for external access.
    ///
    /// Shared with the application service so query and execution paths see
    /// the same authoritative overrides.
    pub fn harness_state_store(&self) -> &Arc<dyn HarnessStateStore> {
        &self.harness_state_store
    }

    /// Exposes the chat service to application-level callers that need to
    /// persist messages adjacent to a runtime flow (e.g. the clarification
    /// reply IPC in Phase 5 of the agent-clarification-resume plan).
    pub fn chat_service(&self) -> &Arc<dyn ChatService> {
        &self.chat_service
    }

    /// Exposes the checkpoint repository used by the coordinator's
    /// park stage. Application services reuse this for the restart
    /// rehydrate path (agent-clarification-resume plan, Phase 6).
    pub fn execution_checkpoint_repo(&self) -> &Arc<dyn ExecutionCheckpointRepository> {
        &self.execution_checkpoint_repo
    }

    /// Exposes the task repository. Mirrors [`Self::chat_service`]:
    /// application services need it for the Phase-6 rehydrate path
    /// where subtask metadata informs the restored clarification event.
    pub fn task_repo(&self) -> &Arc<dyn TaskRepository> {
        &self.task_repo
    }

    /// Exposes the session repository for the Phase-6 rehydrate path.
    pub fn session_repo(&self) -> &Arc<dyn SessionRepository> {
        &self.session_repo
    }

    /// Execute an agent request within a session.
    ///
    /// This is the main entry point. It:
    /// 1. Resolves the LLM client from config
    /// 2. Builds context from conversation history
    /// 3. Runs the tool-calling loop with permission checks
    /// 4. Emits events for each step
    /// 5. Returns the final result
    pub async fn execute(
        &self,
        session: &AgentSession,
        request: AgentRequest,
        agent_system_prompt: &str,
        trust_mode: TrustMode,
        app_handle: &AppHandle,
    ) -> Result<AgentExecutionResult, AgentRuntimeError> {
        let all_tool_definitions = self.tool_registry.definitions();

        execute(
            session,
            request,
            agent_system_prompt,
            trust_mode,
            app_handle,
            self.session_repo.as_ref(),
            self.chat_service.as_ref(),
            self.task_repo.as_ref(),
            self.telemetry_repo.as_ref(),
            self.execution_repo.as_ref(),
            self.execution_checkpoint_repo.as_ref(),
            self.trace_repo.as_ref(),
            self.memory_repo.as_ref(),
            &self.approval_queue,
            &self.tool_registry,
            &self.file_snapshot_registry,
            &self.harness_state_store,
            &all_tool_definitions,
        )
        .await
    }

    /// Update session state and emit corresponding event.
    pub fn update_session_state(
        &self,
        session_id: &crate::core::agent::session::SessionId,
        state: crate::core::agent::session::SessionState,
        app_handle: &AppHandle,
    ) -> Result<(), AgentRuntimeError> {
        self.session_repo
            .update_state(session_id, &state)
            .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;

        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            crate::infrastructure::agent::events::AgentEvent::SessionStateChanged {
                session_id: session_id.0.clone(),
                state,
            },
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::repositories::TaskRepository;

    #[test]
    fn test_loop_detection_skips_agent_tools() {
        use loop_detection::LOOP_SKIP_TOOLS;

        assert!(LOOP_SKIP_TOOLS.contains(&"agent_advance_to_next_subtask"));
        assert!(LOOP_SKIP_TOOLS.contains(&"agent_emit_status"));
        assert!(LOOP_SKIP_TOOLS.contains(&"agent_mark_subtask_status"));
        assert!(LOOP_SKIP_TOOLS.contains(&"agent_plan_subtasks"));
    }

    #[test]
    fn test_loop_skip_tools_list_is_complete() {
        use loop_detection::LOOP_SKIP_TOOLS;

        // These tools can be called multiple times legitimately:
        // - agent_advance_to_next_subtask: once per subtask advancement
        // - agent_emit_status: for progress updates (can be many)
        // - agent_mark_subtask_status: for manual status updates
        // - agent_narrate: for persistent chat updates
        // - agent_plan_subtasks: idempotent once subtasks already exist
        assert_eq!(LOOP_SKIP_TOOLS.len(), 5);
    }
}
