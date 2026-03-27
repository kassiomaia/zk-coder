//! Agent task types.
//!
//! Tasks represent units of work that an agent performs within a session.
//! Each request to an agent creates a task that tracks execution steps.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::session::SessionId;

/// Agent request (what the user sends to the agent).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRequest {
    pub content: String,
    pub attachments: Vec<Attachment>,
    pub context: Option<RequestContext>,
}

/// File or data attachment to a request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub name: String,
    pub content: String,
    pub content_type: String,
}

/// Additional context for a request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestContext {
    pub file_path: Option<String>,
    pub selection: Option<String>,
}

/// Task execution tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTask {
    pub id: String,
    pub session_id: SessionId,
    pub request: AgentRequest,
    pub status: TaskStatus,
    pub steps: Vec<ExecutionStep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtasks: Option<Vec<SubTask>>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl AgentTask {
    pub fn new(session_id: SessionId, request: AgentRequest) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id,
            request,
            status: TaskStatus::Pending,
            steps: Vec::new(),
            subtasks: None,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    pub fn set_status(&mut self, status: TaskStatus) {
        let is_terminal = matches!(
            status,
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
        );
        self.status = status;
        if is_terminal {
            self.completed_at = Some(Utc::now());
        }
    }

    pub fn add_step(&mut self, step: ExecutionStep) {
        self.steps.push(step);
    }
}

/// Task execution status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    WaitingApproval,
    Completed,
    Failed,
    Cancelled,
}

/// A single step in the agent's execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionStep {
    pub step_number: u32,
    pub action: StepAction,
    pub result: Option<StepResult>,
    pub timestamp: DateTime<Utc>,
}

/// What action was taken in this step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StepAction {
    LlmRequest {
        model: String,
    },
    ToolCall {
        tool: String,
        arguments: serde_json::Value,
    },
}

/// The result of a step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepResult {
    pub success: bool,
    pub output: serde_json::Value,
    pub tokens_used: Option<u32>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub duration_ms: u64,
}

/// Result of a full agent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentExecutionResult {
    pub content: String,
    pub steps: Vec<ExecutionStep>,
    pub usage: Option<UsageStats>,
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageStats {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

/// Execution mode for a subtask. Determines what tools are available.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubTaskMode {
    /// Read-only mode: Can read files and search. Cannot write or execute.
    Read,
    /// Write mode: Can read and write files. Cannot execute commands.
    Write,
    /// Execute mode: Can read files and execute commands. Cannot write.
    Execute,
    /// Full access: Can read, write, and execute.
    FullAccess,
}

impl SubTaskMode {
    /// Get allowed tools for this mode.
    pub fn allowed_tools(&self) -> Vec<&'static str> {
        match self {
            SubTaskMode::Read => vec![
                "fs_read_file",
                "search_grep",
                "fs_list_files",
                "agent_emit_status",
                "agent_advance_to_next_subtask",
            ],
            SubTaskMode::Write => vec![
                "fs_read_file",
                "fs_write_file",
                "search_grep",
                "fs_list_files",
                "agent_emit_status",
                "agent_advance_to_next_subtask",
            ],
            SubTaskMode::Execute => vec![
                "fs_read_file",
                "search_grep",
                "fs_list_files",
                "system_execute",
                "agent_emit_status",
                "agent_advance_to_next_subtask",
            ],
            SubTaskMode::FullAccess => vec![
                "fs_read_file",
                "fs_write_file",
                "fs_list_files",
                "search_grep",
                "system_execute",
                "agent_emit_status",
                "agent_plan_subtasks",
                "agent_advance_to_next_subtask",
            ],
        }
    }

    /// Get mode-specific instruction text.
    pub fn instruction_text(&self) -> &'static str {
        match self {
            SubTaskMode::Read => {
                "READ-ONLY: You may only read files and search code. \
                DO NOT create new plans, write files, delete files, or execute commands."
            }
            SubTaskMode::Write => {
                "WRITE: You may read and write files. \
                DO NOT create new plans or execute commands."
            }
            SubTaskMode::Execute => {
                "EXECUTE: You may read files and execute commands. \
                DO NOT create new plans or write files."
            }
            SubTaskMode::FullAccess => {
                "FULL ACCESS: You may read files, write files, and execute commands."
            }
        }
    }

    /// Get display icon for UI.
    pub fn icon(&self) -> &'static str {
        match self {
            SubTaskMode::Read => "👁️",
            SubTaskMode::Write => "✏️",
            SubTaskMode::Execute => "⚡",
            SubTaskMode::FullAccess => "🔓",
        }
    }
}

/// A sub-task within an agent's task decomposition.
///
/// Agents can break down complex requests into smaller sub-tasks
/// for better tracking and user visibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubTask {
    pub id: String,
    pub task_id: String,
    pub title: String,
    pub description: Option<String>,
    pub mode: SubTaskMode,
    pub status: TaskStatus,
    pub order: i32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl SubTask {
    pub fn new(task_id: String, title: String, mode: SubTaskMode, order: i32) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            task_id,
            title,
            description: None,
            mode,
            status: TaskStatus::Pending,
            order,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn complete(&mut self) {
        self.status = TaskStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    pub fn fail(&mut self) {
        self.status = TaskStatus::Failed;
        self.completed_at = Some(Utc::now());
    }
}

/// Complexity estimate for a plan step.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Complexity {
    Simple,
    Medium,
    Complex,
}

/// A single step in an execution plan.
///
/// Generated during the planning phase before execution begins.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanStep {
    pub title: String,
    pub description: Option<String>,
    pub complexity: Complexity,
}

impl PlanStep {
    pub fn new(title: String) -> Self {
        Self {
            title,
            description: None,
            complexity: Complexity::Medium,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_complexity(mut self, complexity: Complexity) -> Self {
        self.complexity = complexity;
        self
    }
}

/// An execution plan generated during the planning phase.
///
/// The planner analyzes the user's request and generates a structured
/// plan before execution begins.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub summary: String,
    pub steps: Vec<PlanStep>,
    pub estimated_steps: usize,
    pub created_at: DateTime<Utc>,
}

impl Plan {
    pub fn new(summary: String, steps: Vec<PlanStep>) -> Self {
        let estimated_steps = steps.len();
        Self {
            summary,
            steps,
            estimated_steps,
            created_at: Utc::now(),
        }
    }

    /// Convert plan steps to subtasks for a given task.
    pub fn to_subtasks(&self, task_id: &str) -> Vec<SubTask> {
        self.steps
            .iter()
            .enumerate()
            .map(|(i, step)| {
                // Default to FullAccess for plan-generated subtasks
                // Agent will specify mode directly via agent_plan_subtasks tool
                let mut subtask = SubTask::new(
                    task_id.to_string(),
                    step.title.clone(),
                    SubTaskMode::FullAccess,
                    i as i32,
                );
                if let Some(ref desc) = step.description {
                    subtask = subtask.with_description(desc.clone());
                }
                subtask
            })
            .collect()
    }
}

// =========================================================================
// Unit Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------------
    // SubTaskMode Tests
    // ------------------------------------------------------------------------

    #[test]
    fn read_mode_excludes_write_tools() {
        let tools = SubTaskMode::Read.allowed_tools();
        assert!(
            !tools.contains(&"fs_write_file"),
            "Read mode should not allow fs_write_file"
        );
        assert!(
            !tools.contains(&"system_execute"),
            "Read mode should not allow system_execute"
        );
        assert!(
            tools.contains(&"fs_read_file"),
            "Read mode should allow fs_read_file"
        );
        assert!(
            tools.contains(&"search_grep"),
            "Read mode should allow search_grep"
        );
    }

    #[test]
    fn write_mode_includes_read_tools() {
        let tools = SubTaskMode::Write.allowed_tools();
        assert!(
            tools.contains(&"fs_read_file"),
            "Write mode should allow fs_read_file"
        );
        assert!(
            tools.contains(&"fs_write_file"),
            "Write mode should allow fs_write_file"
        );
        assert!(
            !tools.contains(&"system_execute"),
            "Write mode should not allow system_execute"
        );
    }

    #[test]
    fn execute_mode_includes_read_and_execute() {
        let tools = SubTaskMode::Execute.allowed_tools();
        assert!(
            tools.contains(&"fs_read_file"),
            "Execute mode should allow fs_read_file"
        );
        assert!(
            tools.contains(&"system_execute"),
            "Execute mode should allow system_execute"
        );
        assert!(
            !tools.contains(&"fs_write_file"),
            "Execute mode should not allow fs_write_file"
        );
    }

    #[test]
    fn full_access_has_all_tools() {
        let tools = SubTaskMode::FullAccess.allowed_tools();
        assert!(
            tools.contains(&"fs_read_file"),
            "FullAccess should allow fs_read_file"
        );
        assert!(
            tools.contains(&"fs_write_file"),
            "FullAccess should allow fs_write_file"
        );
        assert!(
            tools.contains(&"system_execute"),
            "FullAccess should allow system_execute"
        );
        assert!(
            tools.contains(&"agent_plan_subtasks"),
            "FullAccess should allow agent_plan_subtasks"
        );
    }

    #[test]
    fn instruction_text_mode_specific() {
        assert!(SubTaskMode::Read.instruction_text().contains("READ-ONLY"));
        assert!(SubTaskMode::Write.instruction_text().contains("WRITE"));
        assert!(SubTaskMode::Execute.instruction_text().contains("EXECUTE"));
        assert!(SubTaskMode::FullAccess
            .instruction_text()
            .contains("FULL ACCESS"));
    }

    #[test]
    fn icon_returns_emoji() {
        assert_eq!(SubTaskMode::Read.icon(), "👁️");
        assert_eq!(SubTaskMode::Write.icon(), "✏️");
        assert_eq!(SubTaskMode::Execute.icon(), "⚡");
        assert_eq!(SubTaskMode::FullAccess.icon(), "🔓");
    }

    // ------------------------------------------------------------------------
    // SubTask Tests
    // ------------------------------------------------------------------------

    #[test]
    fn subtask_new_creates_pending() {
        let subtask = SubTask::new(
            "task-1".to_string(),
            "Test".to_string(),
            SubTaskMode::Read,
            0,
        );
        assert_eq!(subtask.status, TaskStatus::Pending);
        assert_eq!(subtask.task_id, "task-1");
        assert_eq!(subtask.title, "Test");
        assert_eq!(subtask.mode, SubTaskMode::Read);
        assert_eq!(subtask.order, 0);
        assert!(subtask.started_at.is_none());
        assert!(subtask.completed_at.is_none());
    }

    #[test]
    fn subtask_with_description_adds_description() {
        let subtask = SubTask::new(
            "task-1".to_string(),
            "Test".to_string(),
            SubTaskMode::Read,
            0,
        )
        .with_description("A description".to_string());
        assert_eq!(subtask.description, Some("A description".to_string()));
    }

    #[test]
    fn subtask_complete_sets_status_and_timestamp() {
        let mut subtask = SubTask::new(
            "task-1".to_string(),
            "Test".to_string(),
            SubTaskMode::Read,
            0,
        );
        assert!(subtask.completed_at.is_none());
        subtask.complete();
        assert_eq!(subtask.status, TaskStatus::Completed);
        assert!(subtask.completed_at.is_some());
    }

    #[test]
    fn subtask_fail_sets_status_and_timestamp() {
        let mut subtask = SubTask::new(
            "task-1".to_string(),
            "Test".to_string(),
            SubTaskMode::Read,
            0,
        );
        assert!(subtask.completed_at.is_none());
        subtask.fail();
        assert_eq!(subtask.status, TaskStatus::Failed);
        assert!(subtask.completed_at.is_some());
    }

    // ------------------------------------------------------------------------
    // Plan Tests
    // ------------------------------------------------------------------------

    #[test]
    fn plan_new_sets_estimated_steps() {
        let steps = vec![
            PlanStep::new("Step 1".to_string()),
            PlanStep::new("Step 2".to_string()),
        ];
        let plan = Plan::new("Summary".to_string(), steps);
        assert_eq!(plan.estimated_steps, 2);
        assert_eq!(plan.summary, "Summary");
    }

    #[test]
    fn plan_to_subtasks_creates_correct_subtasks() {
        let steps = vec![
            PlanStep::new("Step 1".to_string()),
            PlanStep::new("Step 2".to_string()).with_description("Description".to_string()),
        ];
        let plan = Plan::new("Summary".to_string(), steps);
        let subtasks = plan.to_subtasks("task-1");

        assert_eq!(subtasks.len(), 2);
        assert_eq!(subtasks[0].title, "Step 1");
        assert_eq!(subtasks[0].order, 0);
        assert_eq!(subtasks[0].mode, SubTaskMode::FullAccess);
        assert_eq!(subtasks[1].title, "Step 2");
        assert_eq!(subtasks[1].order, 1);
        assert_eq!(subtasks[1].description, Some("Description".to_string()));
    }

    // ------------------------------------------------------------------------
    // AgentTask Tests
    // ------------------------------------------------------------------------

    #[test]
    fn agent_task_new_creates_pending() {
        let session_id = SessionId("session-1".to_string());
        let request = AgentRequest {
            content: "Test request".to_string(),
            attachments: vec![],
            context: None,
        };
        let task = AgentTask::new(session_id, request);

        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.steps.is_empty());
        assert!(task.completed_at.is_none());
    }

    #[test]
    fn agent_task_set_status_terminal_sets_completed_at() {
        let session_id = SessionId("session-1".to_string());
        let request = AgentRequest {
            content: "Test request".to_string(),
            attachments: vec![],
            context: None,
        };
        let mut task = AgentTask::new(session_id, request);
        assert!(task.completed_at.is_none());

        task.set_status(TaskStatus::Completed);
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn agent_task_add_step_increases_steps() {
        let session_id = SessionId("session-1".to_string());
        let request = AgentRequest {
            content: "Test request".to_string(),
            attachments: vec![],
            context: None,
        };
        let mut task = AgentTask::new(session_id, request);

        let step = ExecutionStep {
            step_number: 1,
            action: StepAction::LlmRequest {
                model: "test-model".to_string(),
            },
            result: None,
            timestamp: Utc::now(),
        };

        task.add_step(step);
        assert_eq!(task.steps.len(), 1);
    }
}
