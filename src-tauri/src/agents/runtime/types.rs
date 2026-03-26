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
    #[error("Approval timeout - action timed out waiting for approval")]
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

    #[test]
    fn test_subtask_summary_with_all_statuses() {
        let subtask_base = crate::agents::task::SubTask {
            id: "test-id".to_string(),
            task_id: "task-id".to_string(),
            title: "Test".to_string(),
            description: None,
            mode: SubTaskMode::Read,
            status: TaskStatus::Pending,
            order: 0,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        };

        for status in &[
            TaskStatus::Pending,
            TaskStatus::InProgress,
            TaskStatus::WaitingApproval,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Cancelled,
        ] {
            let mut subtask = subtask_base.clone();
            subtask.status = status.clone();
            let summary = SubtaskSummary::from_subtask(&subtask);
            assert_eq!(summary.status, *status);
        }
    }

    #[test]
    fn test_subtask_summary_with_all_modes() {
        let subtask_base = crate::agents::task::SubTask {
            id: "test-id".to_string(),
            task_id: "task-id".to_string(),
            title: "Test".to_string(),
            description: None,
            mode: SubTaskMode::Read,
            status: TaskStatus::Pending,
            order: 0,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        };

        for mode in &[
            SubTaskMode::Read,
            SubTaskMode::Write,
            SubTaskMode::Execute,
            SubTaskMode::FullAccess,
        ] {
            let mut subtask = subtask_base.clone();
            subtask.mode = mode.clone();
            let summary = SubtaskSummary::from_subtask(&subtask);
            assert_eq!(summary.mode, *mode);
        }
    }

    #[test]
    fn test_subtask_summary_order_preserved() {
        for order in [0, 1, 5, 10, 100] {
            let subtask = crate::agents::task::SubTask {
                id: "test-id".to_string(),
                task_id: "task-id".to_string(),
                title: "Test".to_string(),
                description: None,
                order,
                status: TaskStatus::Pending,
                mode: SubTaskMode::Read,
                created_at: Utc::now(),
                started_at: None,
                completed_at: None,
            };

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
            AgentRuntimeError::ActionDenied,
            AgentRuntimeError::ApprovalTimeout,
            AgentRuntimeError::Store("Database error".to_string()),
            AgentRuntimeError::Config("Missing config".to_string()),
        ];

        for error in errors {
            let msg = error.to_string();
            assert!(!msg.is_empty(), "Error message should not be empty");
        }
    }

    #[test]
    fn test_subtask_summary_clone() {
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
