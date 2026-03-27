//! Agent Tauri IPC commands.
//!
//! The agent command surface is split by concern so IPC handlers stay thin and
//! depend on application-level queries instead of runtime internals.

pub mod approvals;
pub mod history;
pub mod registry;
pub mod rollback;
pub mod sessions;
mod state;
pub mod subtasks;

pub use approvals::{approve_agent_action, deny_agent_action, get_pending_agent_actions};
pub use history::{get_conversation_session_history, get_session_details, SessionDetailsResponse};
pub use registry::{get_agent, list_agents};
pub use rollback::{get_session_file_snapshots, rollback_file, rollback_session};
pub use sessions::{
    cancel_agent_session, create_agent_session, delete_agent_session, get_agent_session,
    get_agent_sessions_for_conversation, retry_last_agent_request, send_agent_request,
    update_agent_session_workspace,
};
pub use state::AgentState;
pub use subtasks::{create_subtask, get_subtasks, update_subtask_status};
