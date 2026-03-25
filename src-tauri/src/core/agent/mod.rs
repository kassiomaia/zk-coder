//! Agent domain module.
//!
//! Contains agent entities, repository interfaces, and service interfaces.

pub mod entities;
pub mod repositories;
pub mod services;

// Re-export core types
pub use entities::{
    Agent, AgentId, AgentExecutionResult, AgentRequest, AgentSession, AgentTask, ApprovalAction,
    ApprovalStatus, PendingAction, SessionId, SessionState,
};
pub use repositories::{AgentRepository, ApprovalRepository, SessionRepository};
pub use services::AgentService;
