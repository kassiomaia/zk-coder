//! Types and error definitions for agent runtime.

use crate::agents::task::{SubTaskMode, TaskStatus};

/// Agent runtime errors.
#[derive(Debug, thiserror::Error)]
pub enum AgentRuntimeError {
    #[error("Agent not found: {0}")]
    AgentNotFound(String),
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("LLM error: {0}")]
    Llm(String),
    #[error("Detected repetitive action loop")]
    LoopDetected,
    #[error("Action denied by user")]
    ActionDenied,
    #[error("Action timed out waiting for approval")]
    ApprovalTimeout,
    #[error("Store error: {0}")]
    Store(String),
    #[error("Config error: {0}")]
    Config(String),
}

/// Summary of subtask data needed for prompt derivation and tracking.
#[derive(Debug, Clone)]
pub struct SubtaskSummary {
    pub title: String,
    pub mode: SubTaskMode,
    pub status: TaskStatus,
    pub order: i32,
}

impl SubtaskSummary {
    pub fn from_subtask(subtask: &crate::agents::task::SubTask) -> Self {
        Self {
            title: subtask.title.clone(),
            mode: subtask.mode.clone(),
            status: subtask.status.clone(),
            order: subtask.order,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_subtask_summary_from_subtask() {
        let subtask = crate::agents::task::SubTask {
            id: "test-id".to_string(),
            task_id: "task-id".to_string(),
            title: "Test Subtask".to_string(),
            description: None,
            order: 1,
            status: TaskStatus::Pending,
            mode: SubTaskMode::Read,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        };

        let summary = SubtaskSummary::from_subtask(&subtask);
        assert_eq!(summary.title, "Test Subtask");
        assert_eq!(summary.order, 1);
        assert_eq!(summary.status, TaskStatus::Pending);
        assert!(matches!(summary.mode, SubTaskMode::Read));
    }
}
