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

pub mod client;
pub mod context;
pub mod execution;
pub mod helpers;
pub mod loop_detection;
pub mod permissions;
pub mod types;

use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use crate::core::agent::repositories::{SessionRepository, TaskRepository};
use crate::core::agent::session::AgentSession;
use crate::core::agent::task::{AgentExecutionResult, AgentRequest};
use crate::core::agent::types::TrustMode;
use crate::infrastructure::agent::approval::ApprovalQueue;
use crate::infrastructure::agent::events::AGENT_EVENT_NAME;
use crate::infrastructure::agent::file_snapshot_registry::FileSnapshotRegistry;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::tools::ToolRegistry;

// Re-export submodule types
pub use client::{chat_with_retry, create_llm_client, LlmRetryConfig};
pub use context::{
    estimate_context_tokens, maybe_compact_context, CompactionDetails, COMPRESSION_THRESHOLD,
    CONTEXT_WINDOW, KEEP_RECENT_MESSAGES,
};
pub use execution::execute;
pub use helpers::{derive_system_prompt, generate_diff_from_content, infer_action_type};
pub use loop_detection::{
    add_to_recent_calls, detect_repetitive_loop, hash_tool_arguments, LOOP_SKIP_TOOLS,
    MAX_REPETITIONS,
};
pub use permissions::process_tool_calls_with_permissions;
pub use types::{AgentRuntimeError, SubtaskSummary};

/// The agent runtime orchestrates LLM + tool execution with permissions.
pub struct AgentRuntime {
    session_repo: Arc<dyn SessionRepository>,
    task_repo: Arc<dyn TaskRepository>,
    telemetry_repo: Arc<dyn AgentTelemetryRepository>,
    approval_queue: Arc<ApprovalQueue>,
    tool_registry: Arc<ToolRegistry>,
    file_snapshot_registry: Arc<FileSnapshotRegistry>,
}

impl AgentRuntime {
    /// Create a new agent runtime.
    pub fn new(
        session_repo: Arc<dyn SessionRepository>,
        task_repo: Arc<dyn TaskRepository>,
        telemetry_repo: Arc<dyn AgentTelemetryRepository>,
        approval_queue: Arc<ApprovalQueue>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Self {
        Self {
            session_repo,
            task_repo,
            telemetry_repo,
            approval_queue,
            tool_registry,
            file_snapshot_registry: Arc::new(FileSnapshotRegistry::new()),
        }
    }

    /// Get the file snapshot registry for external access (e.g., Tauri commands).
    pub fn file_snapshot_registry(&self) -> &Arc<FileSnapshotRegistry> {
        &self.file_snapshot_registry
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
            self.task_repo.as_ref(),
            self.telemetry_repo.as_ref(),
            &self.approval_queue,
            &self.tool_registry,
            &self.file_snapshot_registry,
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
        let core_state: crate::core::agent::entities::SessionState = state.clone().into();
        self.session_repo
            .update_state(
                &crate::core::agent::entities::SessionId(session_id.0.clone()),
                &core_state,
            )
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

        // agent_plan_subtasks is NOT in the skip list because it should only be called once
        assert!(!LOOP_SKIP_TOOLS.contains(&"agent_plan_subtasks"));
    }

    #[test]
    fn test_loop_skip_tools_list_is_complete() {
        use loop_detection::LOOP_SKIP_TOOLS;

        // These tools can be called multiple times legitimately:
        // - agent_advance_to_next_subtask: once per subtask advancement
        // - agent_emit_status: for progress updates (can be many)
        // - agent_mark_subtask_status: for manual status updates
        assert_eq!(LOOP_SKIP_TOOLS.len(), 3);

        // agent_plan_subtasks should NOT be skipped (has its own protection)
        assert!(!LOOP_SKIP_TOOLS.contains(&"agent_plan_subtasks"));
    }
}
