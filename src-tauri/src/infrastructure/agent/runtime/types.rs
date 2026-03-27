//! Types and error definitions for agent runtime.

use crate::core::agent::entities::TaskMode as CoreTaskMode;
use crate::core::agent::entities::TaskStatus;

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
    #[error("Repository error: {0}")]
    Repository(String),
    #[error("Config error: {0}")]
    Config(String),
}

/// Summary of subtask data needed for prompt derivation and tracking.
#[derive(Debug, Clone)]
pub struct SubtaskSummary {
    pub title: String,
    pub mode: CoreTaskMode,
    pub status: TaskStatus,
    pub order: i32,
}

impl SubtaskSummary {
    pub fn from_subtask(subtask: &crate::core::agent::entities::AgentTask) -> Self {
        Self {
            title: subtask.title.clone(),
            mode: subtask.mode.unwrap_or(CoreTaskMode::FullAccess),
            status: subtask.status.clone(),
            order: subtask.order,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::entities::{AgentTask, TaskMode};
    use chrono::Utc;

    #[test]
    fn test_subtask_summary_from_subtask() {
        let subtask = AgentTask {
            id: "test-id".to_string(),
            session_id: "task-id".to_string(),
            parent_task_id: Some("parent-task".to_string()),
            title: "Test Subtask".to_string(),
            description: None,
            order: 1,
            status: TaskStatus::Pending,
            mode: Some(TaskMode::Read),
        };

        let summary = SubtaskSummary::from_subtask(&subtask);
        assert_eq!(summary.title, "Test Subtask");
        assert_eq!(summary.order, 1);
        assert_eq!(summary.status, TaskStatus::Pending);
        assert!(matches!(summary.mode, TaskMode::Read));
    }

    #[test]
    fn test_subtask_summary_with_all_statuses() {
        let subtask_base = AgentTask {
            id: "test-id".to_string(),
            session_id: "task-id".to_string(),
            parent_task_id: Some("parent-task".to_string()),
            title: "Test".to_string(),
            description: None,
            mode: Some(TaskMode::Read),
            status: TaskStatus::Pending,
            order: 0,
        };

        for status in &[
            TaskStatus::Pending,
            TaskStatus::InProgress,
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
        let subtask_base = AgentTask {
            id: "test-id".to_string(),
            session_id: "task-id".to_string(),
            parent_task_id: Some("parent-task".to_string()),
            title: "Test".to_string(),
            description: None,
            mode: Some(TaskMode::Read),
            status: TaskStatus::Pending,
            order: 0,
        };

        for mode in &[
            TaskMode::Read,
            TaskMode::Write,
            TaskMode::Execute,
            TaskMode::FullAccess,
        ] {
            let mut subtask = subtask_base.clone();
            subtask.mode = Some(*mode);
            let summary = SubtaskSummary::from_subtask(&subtask);
            assert_eq!(summary.mode, *mode);
        }
    }

    #[test]
    fn test_subtask_summary_order_preserved() {
        for order in [0, 1, 5, 10, 100] {
            let subtask = AgentTask {
                id: "test-id".to_string(),
                session_id: "task-id".to_string(),
                parent_task_id: Some("parent-task".to_string()),
                title: "Test".to_string(),
                description: None,
                order,
                status: TaskStatus::Pending,
                mode: Some(TaskMode::Read),
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
        let subtask = AgentTask {
            id: "test-id".to_string(),
            session_id: "task-id".to_string(),
            parent_task_id: Some("parent-task".to_string()),
            title: "Test Subtask".to_string(),
            description: None,
            order: 1,
            status: TaskStatus::Pending,
            mode: Some(TaskMode::Read),
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
            mode: CoreTaskMode::Read,
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
        assert_eq!(CoreTaskMode::Read, CoreTaskMode::Read);
        assert_eq!(CoreTaskMode::Write, CoreTaskMode::Write);
        assert_ne!(CoreTaskMode::Read, CoreTaskMode::Write);
    }
}
