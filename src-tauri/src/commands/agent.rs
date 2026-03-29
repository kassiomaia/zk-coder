//! Agent Tauri IPC commands.
//!
//! The agent command surface is split by concern so IPC handlers stay thin and
//! depend on application-level queries instead of runtime internals.

/// Re-exports for approval-related commands.
pub mod approvals;
/// Re-exports for debug-related commands.
pub mod debug;
/// Re-exports for history-related commands.
pub mod history;
/// Re-exports for agent registry commands.
pub mod registry;
/// Re-exports for rollback-related commands.
pub mod rollback;
/// Re-exports for session-related commands.
pub mod sessions;
mod state;
/// Re-exports for subtask-related commands.
pub mod subtasks;

// Re-exports from approvals module
pub use approvals::{approve_agent_action, deny_agent_action, get_pending_agent_actions};
// Re-exports from debug module
pub use debug::{get_agent_execution_debug, open_agent_trace_window};
// Re-exports from history module
pub use history::{get_conversation_session_history, get_session_details, SessionDetailsResponse};
// Re-exports from registry module
pub use registry::{get_agent, list_agents};
// Re-exports from rollback module
pub use rollback::{get_session_file_snapshots, rollback_file, rollback_session};
// Re-exports from sessions module
pub use sessions::{
    cancel_agent_session, continue_agent_execution, create_agent_session, delete_agent_session,
    get_agent_session, get_agent_sessions_for_conversation, initialize_agent_recovery,
    replan_agent_execution, retry_last_agent_request, send_agent_request, update_agent_session_workspace,
};
// Re-exports from state module
pub use state::AgentState;
// Re-exports from subtasks module
pub use subtasks::{create_subtask, get_subtasks, update_subtask_status};
