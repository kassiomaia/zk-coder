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
pub mod loop_detection;
pub mod types;
pub mod helpers;
pub mod execution;
pub mod permissions;

use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use crate::agents::approval::ApprovalQueue;
use crate::agents::events::AGENT_EVENT_NAME;
use crate::agents::rollback::RollbackStore;
use crate::agents::session::AgentSession;
use crate::agents::store::SessionStore;
use crate::agents::task::{AgentExecutionResult, AgentRequest};
use crate::agents::types::TrustMode;
use crate::tools::ToolRegistry;

// Re-export submodule types
pub use client::{chat_with_retry, create_llm_client, LlmRetryConfig};
pub use context::{
    estimate_context_tokens, maybe_compact_context, CompactionDetails, COMPRESSION_THRESHOLD,
    CONTEXT_WINDOW, KEEP_RECENT_MESSAGES,
};
pub use execution::execute;
pub use helpers::{derive_system_prompt, generate_diff_from_content, infer_action_type};
pub use loop_detection::{add_to_recent_calls, detect_repetitive_loop, hash_tool_arguments,
                        LOOP_SKIP_TOOLS, MAX_REPETITIONS};
pub use permissions::process_tool_calls_with_permissions;
pub use types::{AgentRuntimeError, SubtaskSummary};

/// The agent runtime orchestrates LLM + tool execution with permissions.
pub struct AgentRuntime {
    session_store: Arc<SessionStore>,
    approval_queue: Arc<ApprovalQueue>,
    tool_registry: Arc<ToolRegistry>,
    rollback_store: Arc<RollbackStore>,
}

impl AgentRuntime {
    /// Create a new agent runtime.
    pub fn new(
        session_store: Arc<SessionStore>,
        approval_queue: Arc<ApprovalQueue>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Self {
        Self {
            session_store,
            approval_queue,
            tool_registry,
            rollback_store: Arc::new(RollbackStore::new()),
        }
    }

    /// Get the rollback store for external access (e.g., Tauri commands).
    pub fn rollback_store(&self) -> &Arc<RollbackStore> {
        &self.rollback_store
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
            &self.session_store,
            &self.approval_queue,
            &self.tool_registry,
            &self.rollback_store,
            &all_tool_definitions,
        )
        .await
    }

    /// Update session state and emit corresponding event.
    pub fn update_session_state(
        &self,
        session_id: &crate::agents::session::SessionId,
        state: crate::agents::session::SessionState,
        app_handle: &AppHandle,
    ) -> Result<(), AgentRuntimeError> {
        self.session_store
            .update_state(session_id, &state)
            .map_err(|e| AgentRuntimeError::Store(e.to_string()))?;

        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            crate::agents::events::AgentEvent::SessionStateChanged {
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
