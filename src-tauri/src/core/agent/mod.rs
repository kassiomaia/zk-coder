//! Agent domain module.
//!
//! Contains agent entities, repository interfaces, and service interfaces.

pub mod entities;
pub mod permissions;
pub mod repositories;
pub mod services;
pub mod session;
pub mod task;
#[cfg(test)]
mod tests;
pub mod types;

// Re-export core types
pub use entities::{
    Agent, AgentExecutionResult, AgentId, AgentRequest, AgentSession, AgentTask, ApprovalAction,
    ApprovalStatus, PendingAction, SessionId, SessionState,
};
pub use repositories::{AgentRepository, ApprovalRepository, SessionRepository};
pub use services::AgentService;
