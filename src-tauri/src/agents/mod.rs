//! Agent orchestration system.
//!
//! This module contains the agent runtime, permission system, session management,
//! and related types for the agent platform.

pub mod types;
pub mod session;
pub mod task;
pub mod permissions;
pub mod approval;
pub mod registry;
pub mod events;
pub mod store;
pub mod tool_call_store;
pub mod runtime;
pub mod rollback;

#[cfg(test)]
mod tests;

pub use types::*;
pub use session::*;
pub use task::{
    AgentExecutionResult, AgentRequest, AgentTask, ExecutionStep, StepAction, StepResult,
    SubTask, SubTaskMode, TaskStatus, UsageStats,
};

// Re-export for tests
#[cfg(test)]
pub use types::AgentId;
#[cfg(test)]
pub use session::AgentSession;
