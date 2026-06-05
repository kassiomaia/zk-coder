//! Agent domain module.
//!
//! Contains agent entities, repository interfaces, and service interfaces.

pub mod clarification;
pub mod context;
pub mod errors;
pub mod execution;
pub mod memory;
pub mod permissions;
pub mod planning;
pub mod repositories;
pub mod services;
pub mod session;
pub mod task;
#[cfg(test)]
mod tests;
pub mod types;

// Re-export core types
pub use clarification::PendingClarificationRecord;
pub use context::{CompactionDetails, ContextCompactionStrategy, ContextPackOutcome, ContextState};
pub use errors::AgentDomainError;
pub use execution::{
    ExecutionCheckpoint, ExecutionPhase, ExecutionSnapshot, ExecutionTransition, LlmTracePayload,
    RuntimeTrace, RuntimeTraceType,
};
pub use memory::{MemoryRepository, SessionMemory};
pub use repositories::{
    AgentRepository, ApprovalRepository, ClarificationRepository, ExecutionCheckpointRepository,
    ExecutionRepository, RepositoryResult, RuntimeTraceRepository, SessionRepository,
    TaskRepository,
};
pub use services::AgentService;
pub use session::{AgentSession, PendingAction, SessionId, SessionState};
pub use task::{
    AgentExecutionResult, AgentRequest, AgentTask, Attachment, Complexity, ExecutionStep, Plan,
    PlanStep, RequestContext, StepAction, StepResult, SubTask, SubTaskMode, TaskStatus, UsageStats,
};
pub use types::{ActionPreview, ActionType, Agent, AgentId, PermissionLevel, TrustMode};
