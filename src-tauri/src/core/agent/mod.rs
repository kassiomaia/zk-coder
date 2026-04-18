//! Agent domain module.
//!
//! Contains agent entities, repository interfaces, and service interfaces.

pub mod errors;
pub mod execution;
pub mod memory;
pub mod permissions;
pub mod repositories;
pub mod services;
pub mod session;
pub mod task;
#[cfg(test)]
mod tests;
pub mod types;

// Re-export core types
pub use errors::AgentDomainError;
pub use session::{AgentSession, PendingAction, SessionId, SessionState};
pub use task::{
    AgentExecutionResult, AgentRequest, AgentTask, Attachment, Complexity, ExecutionStep, Plan,
    PlanStep, RequestContext, StepAction, StepResult, SubTask, SubTaskMode, TaskStatus,
    UsageStats,
};
pub use types::{ActionPreview, ActionType, Agent, AgentId, PermissionLevel, TrustMode};
pub use execution::{
    ExecutionPhase, ExecutionSnapshot, ExecutionTransition, LlmTracePayload, RuntimeTrace,
    RuntimeTraceType,
};
pub use memory::{MemoryRepository, SessionMemory};
pub use repositories::{
    AgentRepository, ApprovalRepository, ExecutionRepository, RepositoryResult,
    RuntimeTraceRepository, SessionRepository, TaskRepository,
};
pub use services::AgentService;
