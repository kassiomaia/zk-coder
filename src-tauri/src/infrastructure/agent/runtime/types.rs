//! Types and error definitions for agent runtime.

use crate::core::agent::task::{SubTask, SubTaskMode, TaskStatus};

/// Agent runtime errors.
#[derive(Debug, thiserror::Error)]
pub enum AgentRuntimeError {
    /// Agent was not found.
    #[error("Agent not found: {0}")]
    AgentNotFound(String),
    /// Session was not found.
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    /// LLM error occurred.
    #[error("LLM error: {0}")]
    Llm(String),
    /// Detected repetitive action loop.
    #[error("Detected repetitive action loop")]
    LoopDetected,
    /// Detected a narration loop: the LLM produced no tool calls across multiple
    /// consecutive iterations while in-progress subtasks remained, which would
    /// otherwise cause the execution loop to spin indefinitely.
    #[error("Detected narration loop: LLM produced no tool calls across multiple iterations")]
    NarrationLoop,
    /// Action was denied by user.
    #[error("Action denied by user")]
    ActionDenied,
    /// Approval timeout - action timed out waiting for approval.
    #[error("Approval timeout - action timed out waiting for approval")]
    ApprovalTimeout,
    /// Execution was cancelled.
    #[error("Execution cancelled")]
    Cancelled,
    /// Repository error occurred.
    #[error("Repository error: {0}")]
    Repository(String),
    /// Configuration error.
    #[error("Config error: {0}")]
    Config(String),
}

/// Summary of subtask data needed for prompt derivation and tracking.
#[derive(Debug, Clone)]
pub struct SubtaskSummary {
    /// Title of the subtask.
    pub title: String,
    /// Execution mode for the subtask.
    pub mode: SubTaskMode,
    /// Current status of the subtask.
    pub status: TaskStatus,
    /// Order of the subtask in the task list.
    pub order: i32,
}

impl SubtaskSummary {
    /// Creates a subtask summary from a full subtask.
    ///
    /// # Arguments
    ///
    /// * `subtask` - The subtask to create a summary from.
    pub fn from_subtask(subtask: &SubTask) -> Self {
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

    fn make_subtask(title: &str, mode: SubTaskMode, status: TaskStatus, order: i32) -> SubTask {
        SubTask {
            id: "test-id".to_string(),
            task_id: "parent-task".to_string(),
            title: title.to_string(),
            description: None,
            mode,
            status,
            order,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        }
    }

    #[test]
    fn test_subtask_summary_from_subtask() {
        let subtask = make_subtask("Test Subtask", SubTaskMode::Read, TaskStatus::Pending, 1);

        let summary = SubtaskSummary::from_subtask(&subtask);
        assert_eq!(summary.title, "Test Subtask");
        assert_eq!(summary.order, 1);
        assert_eq!(summary.status, TaskStatus::Pending);
        assert!(matches!(summary.mode, SubTaskMode::Read));
    }

    #[test]
    fn test_subtask_summary_with_all_statuses() {
        for status in &[
            TaskStatus::Pending,
            TaskStatus::InProgress,
            TaskStatus::WaitingApproval,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Cancelled,
        ] {
            let subtask = make_subtask("Test", SubTaskMode::Read, status.clone(), 0);
            let summary = SubtaskSummary::from_subtask(&subtask);
            assert_eq!(summary.status, *status);
        }
    }

    #[test]
    fn test_subtask_summary_with_all_modes() {
        for mode in &[
            SubTaskMode::Read,
            SubTaskMode::Write,
            SubTaskMode::Execute,
            SubTaskMode::FullAccess,
        ] {
            let subtask = make_subtask("Test", mode.clone(), TaskStatus::Pending, 0);
            let summary = SubtaskSummary::from_subtask(&subtask);
            assert_eq!(summary.mode, *mode);
        }
    }

    #[test]
    fn test_subtask_summary_order_preserved() {
        for order in [0, 1, 5, 10, 100] {
            let subtask = make_subtask("Test", SubTaskMode::Read, TaskStatus::Pending, order);
            let summary = SubtaskSummary::from_subtask(&subtask);
            assert_eq!(summary.order, order);
        }
    }

    #[test]
    fn test_agent_runtime_error_display_messages() {
        use super::super::AgentRuntimeError;

        let errors = vec![
            AgentRuntimeError::AgentNotFound("agent-1".to_string()),
            AgentRuntimeError::SessionNotFound("session-1".to_string()),
            AgentRuntimeError::Llm("API error".to_string()),
            AgentRuntimeError::LoopDetected,
            AgentRuntimeError::NarrationLoop,
            AgentRuntimeError::ActionDenied,
            AgentRuntimeError::ApprovalTimeout,
            AgentRuntimeError::Cancelled,
            AgentRuntimeError::Repository("Database error".to_string()),
            AgentRuntimeError::Config("Missing config".to_string()),
        ];

        for error in errors {
            let msg = error.to_string();
            assert!(!msg.is_empty(), "Error message should not be empty");
        }
    }

    #[test]
    fn test_subtask_summary_clone() {
        let subtask = make_subtask("Test Subtask", SubTaskMode::Read, TaskStatus::Pending, 1);

        let summary1 = SubtaskSummary::from_subtask(&subtask);
        let summary2 = summary1.clone();

        assert_eq!(summary1.title, summary2.title);
        assert_eq!(summary1.order, summary2.order);
        assert_eq!(summary1.status, summary2.status);
        assert_eq!(summary1.mode, summary2.mode);
    }

    #[test]
    fn test_subtask_summary_debug_format() {
        let summary = SubtaskSummary {
            title: "Test".to_string(),
            mode: SubTaskMode::Read,
            status: TaskStatus::InProgress,
            order: 0,
        };

        let debug_str = format!("{:?}", summary);
        assert!(debug_str.contains("Test"));
        assert!(debug_str.contains("Read"));
    }

    #[test]
    fn test_task_status_equality() {
        assert_eq!(TaskStatus::Pending, TaskStatus::Pending);
        assert_eq!(TaskStatus::InProgress, TaskStatus::InProgress);
        assert_ne!(TaskStatus::Pending, TaskStatus::InProgress);
    }

    #[test]
    fn test_subtask_mode_equality() {
        assert_eq!(SubTaskMode::Read, SubTaskMode::Read);
        assert_eq!(SubTaskMode::Write, SubTaskMode::Write);
        assert_ne!(SubTaskMode::Read, SubTaskMode::Write);
    }
}
