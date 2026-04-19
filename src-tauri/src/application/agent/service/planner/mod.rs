//! Task planning service for single-shot LLM-based task planning.
//!
//! This service handles:
//! - Single-shot planning at task creation (title, description, subtasks)
//! - Replanning for interrupted tasks (review progress, update plan)
//! - Unified `plan_or_continue` entry point: given optional prior
//!   execution state, returns a decision (Fresh / Continue / Replan)
//!   so bootstrap can reuse an existing plan instead of regenerating
//!   one on resume.

mod decision;
mod parsing;
mod replan;
mod single_shot;

#[cfg(test)]
mod test_support;

use crate::core::agent::task::{SubTask, SubTaskMode, TaskStatus};
use serde::{Deserialize, Serialize};

pub use decision::{plan_or_continue, ExecutionState, PlanAction, PlanDecision};
pub use replan::{review_and_replan, build_replan_system_prompt, ReplanAction, ReplanOutput};
pub use single_shot::{build_planning_system_prompt, plan_new_task};

/// Output token budget for single-shot planning calls.
///
/// The planner emits a JSON object whose size is dominated by the
/// subtask list. 4000 tokens comfortably fits ~15 subtasks with
/// descriptions; previous 2000-token cap truncated mid-JSON on plans
/// of 5+ subtasks with descriptions and produced the cryptic
/// "Could not locate a JSON object in planner response" error.
pub(super) const PLANNER_MAX_OUTPUT_TOKENS: u32 = 4000;

/// Result of a single-shot planning operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    /// Generated task title (concise, clear)
    pub title: String,
    /// Generated task description (one-sentence summary)
    pub description: String,
    /// Whether this task requires subtask planning
    pub is_non_trivial: bool,
    /// Planned subtasks (if non-trivial)
    pub subtasks: Vec<SubtaskPlan>,
}

/// A single subtask in the plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskPlan {
    /// Subtask title
    pub title: String,
    /// Optional subtask description
    pub description: Option<String>,
    /// Execution order
    pub order: i32,
}

/// Convert a `SubtaskPlan` to an `AgentTask` entity.
#[must_use]
pub fn subtask_plan_to_agent_task(
    subtask_plan: &SubtaskPlan,
    _session_id: &str,
    parent_task_id: &str,
) -> SubTask {
    SubTask {
        id: uuid::Uuid::new_v4().to_string(),
        task_id: parent_task_id.to_string(),
        title: subtask_plan.title.clone(),
        description: subtask_plan.description.clone(),
        mode: SubTaskMode::FullAccess,
        status: TaskStatus::Pending,
        order: subtask_plan.order,
        contract: None,
        created_at: chrono::Utc::now(),
        started_at: None,
        completed_at: None,
    }
}

/// Hint from the user's explicit action — Continue vs Replan vs Fresh —
/// carried alongside the request content so the planner can bias its
/// decision without inventing that signal from text cues.
///
/// `Fresh` is the default for first-run execution (no prior state).
/// `Resume` is set by the Continue button. `Replan` is set by the Replan
/// button. The planner is still free to override — for example, if a
/// `Resume` hint is paired with a user message that clearly changes the
/// task scope, returning `PlanAction::Replan` is valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserIntentHint {
    /// No prior execution state — treat as a brand-new task.
    Fresh,
    /// User wants to resume the existing plan from the current cursor.
    Resume,
    /// User wants the planner to revise the existing plan.
    Replan,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_plan_serialization() {
        let plan = TaskPlan {
            title: "Test Task".to_string(),
            description: "A test task".to_string(),
            is_non_trivial: true,
            subtasks: vec![SubtaskPlan {
                title: "Step 1".to_string(),
                description: Some("First step".to_string()),
                order: 0,
            }],
        };

        let json = serde_json::to_string(&plan).unwrap();
        assert!(json.contains("Test Task"));
        assert!(json.contains("Step 1"));
    }

    #[test]
    fn test_subtask_plan_to_agent_task() {
        let plan = SubtaskPlan {
            title: "Test subtask".to_string(),
            description: Some("Test description".to_string()),
            order: 0,
        };

        let task = subtask_plan_to_agent_task(&plan, "session-123", "task-456");

        assert_eq!(task.title, "Test subtask");
        assert_eq!(task.description, Some("Test description".to_string()));
        assert_eq!(task.task_id, "task-456");
        assert_eq!(task.order, 0);
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(matches!(task.mode, SubTaskMode::FullAccess));
    }
}
