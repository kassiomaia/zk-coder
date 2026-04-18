//! Defines the types related to agent tasks and execution steps.
//!
//! A task represents a unit of work initiated by a user's request to an agent.
//! It tracks the execution steps, status, and any sub-tasks that are created.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::session::SessionId;

/// Represents a request sent by the user to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRequest {
    /// The main content of the user's request.
    pub content: String,
    /// A list of file or data attachments included with the request.
    pub attachments: Vec<Attachment>,
    /// Additional context for the request, such as the current file or selection.
    pub context: Option<RequestContext>,
}

/// A file or data attachment included in an `AgentRequest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    /// The name of the attachment.
    pub name: String,
    /// The content of the attachment, typically as a base64-encoded string.
    pub content: String,
    /// The MIME type of the attachment's content.
    pub content_type: String,
}

/// Provides additional context for an `AgentRequest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestContext {
    /// The path of the file that was active when the request was made.
    pub file_path: Option<String>,
    /// The text selection that was active when the request was made.
    pub selection: Option<String>,
}

/// Represents a task that tracks the execution of an agent's work.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTask {
    /// The unique identifier for the task.
    pub id: String,
    /// The ID of the session this task belongs to.
    pub session_id: SessionId,
    /// The user's request that initiated this task.
    pub request: AgentRequest,
    /// LLM-generated title. When present, the frontend uses this instead of request.content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The current status of the task.
    pub status: TaskStatus,
    /// A list of execution steps that have been performed for this task.
    pub steps: Vec<ExecutionStep>,
    /// A list of sub-tasks that have been created for this task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtasks: Option<Vec<SubTask>>,
    /// The timestamp of when the task was created.
    pub created_at: DateTime<Utc>,
    /// The timestamp of when the task was completed, if applicable.
    pub completed_at: Option<DateTime<Utc>>,
}

impl AgentTask {
    /// Creates a new `AgentTask` for a given session and request.
    #[must_use]
    pub fn new(session_id: SessionId, request: AgentRequest) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id,
            request,
            title: None,
            status: TaskStatus::Pending,
            steps: Vec::new(),
            subtasks: None,
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    /// Sets the status of the task and updates the completion timestamp if terminal.
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

    /// Adds a new execution step to the task.
    pub fn add_step(&mut self, step: ExecutionStep) {
        self.steps.push(step);
    }
}

/// The execution status of a task or sub-task.
///
/// Deserialization accepts the legacy `"aborted"` discriminator as an alias
/// for [`TaskStatus::Cancelled`], matching the pre-consolidation
/// `entities::TaskStatus::Aborted` variant so historical DB rows still load.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// The task is waiting to be executed.
    Pending,
    /// The task is currently in progress.
    InProgress,
    /// The task is waiting for user approval.
    WaitingApproval,
    /// The task has been completed successfully.
    Completed,
    /// The task has failed.
    Failed,
    /// The task was cancelled by the user.
    #[serde(alias = "aborted")]
    Cancelled,
}

/// A single step within the execution of an agent's task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionStep {
    /// The sequential number of the step.
    pub step_number: u32,
    /// The action that was taken in this step.
    pub action: StepAction,
    /// The result of the action, if available.
    pub result: Option<StepResult>,
    /// The timestamp of when the step was recorded.
    pub timestamp: DateTime<Utc>,
}

/// The type of action that was taken in an `ExecutionStep`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StepAction {
    /// A request was made to the Large Language Model.
    LlmRequest {
        /// The model that was used for the request.
        model: String,
    },
    /// A tool was called as part of the execution.
    ToolCall {
        /// The name of the tool that was called.
        tool: String,
        /// The arguments that were passed to the tool.
        arguments: serde_json::Value,
    },
}

/// The result of an `ExecutionStep`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepResult {
    /// A flag indicating whether the step was successful.
    pub success: bool,
    /// The output of the step, in JSON format.
    pub output: serde_json::Value,
    /// The total number of tokens used in this step, if applicable.
    pub tokens_used: Option<u32>,
    /// The number of input tokens used, if applicable.
    pub input_tokens: Option<u32>,
    /// The number of output tokens generated, if applicable.
    pub output_tokens: Option<u32>,
    /// The duration of the step in milliseconds.
    pub duration_ms: u64,
}

/// The result of a full agent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentExecutionResult {
    /// The final content produced by the agent.
    pub content: String,
    /// The sequence of steps that were executed.
    pub steps: Vec<ExecutionStep>,
    /// The token usage statistics for the execution.
    pub usage: Option<UsageStats>,
}

/// Token usage statistics for an agent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageStats {
    /// The number of input tokens used.
    pub input_tokens: u32,
    /// The number of output tokens generated.
    pub output_tokens: u32,
    /// The total number of tokens used.
    pub total_tokens: u32,
}

/// The execution mode for a sub-task, which determines the set of available tools.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubTaskMode {
    /// Read-only mode: Can read files and search, but cannot write or execute.
    Read,
    /// Write mode: Can read and write files, but cannot execute commands.
    Write,
    /// Execute mode: Can read files and execute commands, but cannot write.
    Execute,
    /// Full access mode: Can read, write, and execute.
    FullAccess,
}

impl SubTaskMode {
    /// Returns the list of tools that are allowed for this mode, or `None`
    /// when the mode is unrestricted.
    ///
    /// `Read`, `Write`, and `Execute` return `Some(vec![...])` — the
    /// enumerated list is the intentional gate, and any tool outside it is
    /// rejected at runtime with `subtask_mode_disallowed`. A new tool joins
    /// a restricted mode only by conscious code change; that is the safety
    /// property and the reason the enumeration is hard-coded.
    ///
    /// `FullAccess` returns `None` — *every* registered tool is allowed.
    /// Previously this variant enumerated a stale subset (fs_* + search_grep
    /// + system_execute_command + a few agent_*), which silently rejected
    /// document-generation tools (`pdf_*`, `docx_*`, `pptx_*`) and agent
    /// orchestration tools (`agent_plan_subtasks`, `agent_narrate`,
    /// `agent_mark_subtask_status`, etc.) registered after the list was
    /// authored. The enumeration is the source of truth for "what to
    /// forbid", not "what exists"; FullAccess has nothing to forbid, so it
    /// carries no list at all.
    #[must_use]
    pub fn allowed_tools(&self) -> Option<Vec<&'static str>> {
        match self {
            Self::Read => Some(vec![
                "fs_read_file",
                "fs_stat_path",
                "fs_glob_files",
                "fs_fuzzy_find_files",
                "search_grep",
                "fs_list_files",
                "agent_emit_status",
                "agent_advance_to_next_subtask",
            ]),
            Self::Write => Some(vec![
                "fs_read_file",
                "fs_stat_path",
                "fs_glob_files",
                "fs_fuzzy_find_files",
                "fs_write_file",
                "fs_replace_in_file",
                "fs_apply_patch",
                "search_grep",
                "fs_list_files",
                "agent_emit_status",
                "agent_advance_to_next_subtask",
            ]),
            Self::Execute => Some(vec![
                "fs_read_file",
                "fs_stat_path",
                "fs_glob_files",
                "fs_fuzzy_find_files",
                "search_grep",
                "fs_list_files",
                "system_execute_command",
                "agent_emit_status",
                "agent_advance_to_next_subtask",
            ]),
            Self::FullAccess => None,
        }
    }

    /// Returns the instruction text that corresponds to this mode.
    #[must_use]
    pub const fn instruction_text(&self) -> &'static str {
        match self {
            Self::Read => {
                "READ-ONLY: You may only read files and search code. \
                DO NOT create new plans, write files, delete files, or execute commands."
            }
            Self::Write => {
                "WRITE: You may read and write files. \
                DO NOT create new plans or execute commands."
            }
            Self::Execute => {
                "EXECUTE: You may read files and execute commands. \
                DO NOT create new plans or write files."
            }
            Self::FullAccess => {
                "FULL ACCESS: You may read files, write files, and execute commands."
            }
        }
    }

    /// Returns the icon that represents this mode in the UI.
    #[must_use]
    pub const fn icon(&self) -> &'static str {
        match self {
            Self::Read => "👁️",
            Self::Write => "✏️",
            Self::Execute => "⚡",
            Self::FullAccess => "🔓",
        }
    }
}

/// Represents a sub-task within a larger agent task decomposition.
///
/// Agents can break down complex requests into a series of smaller, more
/// manageable sub-tasks for improved tracking and user visibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubTask {
    /// The unique identifier for the sub-task.
    pub id: String,
    /// The ID of the parent task this sub-task belongs to.
    pub task_id: String,
    /// The title of the sub-task.
    pub title: String,
    /// An optional, more detailed description of the sub-task.
    pub description: Option<String>,
    /// The execution mode for this sub-task, which determines its permissions.
    pub mode: SubTaskMode,
    /// The current status of the sub-task.
    pub status: TaskStatus,
    /// The display order of the sub-task.
    pub order: i32,
    /// The timestamp of when the sub-task was created.
    pub created_at: DateTime<Utc>,
    /// The timestamp of when the sub-task was started.
    pub started_at: Option<DateTime<Utc>>,
    /// The timestamp of when the sub-task was completed.
    pub completed_at: Option<DateTime<Utc>>,
}

impl SubTask {
    /// Creates a new `SubTask` with the given parameters.
    #[must_use]
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

    /// Sets the description for the sub-task.
    #[must_use]
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Marks the sub-task as completed.
    pub fn complete(&mut self) {
        self.status = TaskStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    /// Marks the sub-task as failed.
    pub fn fail(&mut self) {
        self.status = TaskStatus::Failed;
        self.completed_at = Some(Utc::now());
    }
}

/// An estimate of the complexity of a plan step.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Complexity {
    /// A simple step with low complexity.
    Simple,
    /// A step with medium complexity.
    Medium,
    /// A complex step that may require multiple actions.
    Complex,
}

/// A single step within an execution plan.
///
/// These steps are generated during the planning phase before execution begins.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanStep {
    /// The title of the plan step.
    pub title: String,
    /// An optional, more detailed description of the step.
    pub description: Option<String>,
    /// The estimated complexity of the step.
    pub complexity: Complexity,
}

impl PlanStep {
    /// Creates a new `PlanStep` with the given title.
    #[must_use]
    pub const fn new(title: String) -> Self {
        Self {
            title,
            description: None,
            complexity: Complexity::Medium,
        }
    }

    /// Sets the description for the plan step.
    #[must_use]
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Sets the complexity for the plan step.
    #[must_use]
    pub const fn with_complexity(mut self, complexity: Complexity) -> Self {
        self.complexity = complexity;
        self
    }
}

/// Represents an execution plan generated during the planning phase.
///
/// The planner analyzes the user's request and generates a structured plan
/// of steps to be taken before execution begins.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    /// A summary of the overall plan.
    pub summary: String,
    /// The list of steps in the plan.
    pub steps: Vec<PlanStep>,
    /// The estimated number of steps in the plan.
    pub estimated_steps: usize,
    /// The timestamp of when the plan was created.
    pub created_at: DateTime<Utc>,
}

impl Plan {
    /// Creates a new `Plan` with the given summary and steps.
    #[must_use]
    pub fn new(summary: String, steps: Vec<PlanStep>) -> Self {
        let estimated_steps = steps.len();
        Self {
            summary,
            steps,
            estimated_steps,
            created_at: Utc::now(),
        }
    }

    /// Converts the plan's steps into a list of `SubTask` objects for a given task.
    #[must_use]
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
        let tools = SubTaskMode::Read
            .allowed_tools()
            .expect("Read mode must return an enumerated allowlist");
        assert!(
            !tools.contains(&"fs_write_file"),
            "Read mode should not allow fs_write_file"
        );
        assert!(
            !tools.contains(&"system_execute_command"),
            "Read mode should not allow system_execute_command"
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
        let tools = SubTaskMode::Write
            .allowed_tools()
            .expect("Write mode must return an enumerated allowlist");
        assert!(
            tools.contains(&"fs_read_file"),
            "Write mode should allow fs_read_file"
        );
        assert!(
            tools.contains(&"fs_write_file"),
            "Write mode should allow fs_write_file"
        );
        assert!(
            tools.contains(&"fs_replace_in_file"),
            "Write mode should allow fs_replace_in_file"
        );
        assert!(
            !tools.contains(&"system_execute_command"),
            "Write mode should not allow system_execute_command"
        );
    }

    #[test]
    fn execute_mode_includes_read_and_execute() {
        let tools = SubTaskMode::Execute
            .allowed_tools()
            .expect("Execute mode must return an enumerated allowlist");
        assert!(
            tools.contains(&"fs_read_file"),
            "Execute mode should allow fs_read_file"
        );
        assert!(
            tools.contains(&"fs_stat_path"),
            "Execute mode should allow fs_stat_path"
        );
        assert!(
            tools.contains(&"fs_glob_files"),
            "Execute mode should allow fs_glob_files"
        );
        assert!(
            tools.contains(&"system_execute_command"),
            "Execute mode should allow system_execute_command"
        );
        assert!(
            !tools.contains(&"fs_write_file"),
            "Execute mode should not allow fs_write_file"
        );
        assert!(
            !tools.contains(&"fs_replace_in_file"),
            "Execute mode should not allow fs_replace_in_file"
        );
    }

    /// FullAccess is the *unrestricted* mode — there is no allowlist.
    ///
    /// The previous version of this test enumerated fs/search/execute tool
    /// names and asserted each one was present in `allowed_tools()`. That
    /// encoded the bug: the list was a stale subset that silently rejected
    /// document-generation tools (`pdf_*`, `docx_*`, `pptx_*`) and agent
    /// orchestration tools registered later. The contract now is:
    /// FullAccess carries no allowlist, so the runtime enforcement layer
    /// passes every registered tool through unchanged.
    #[test]
    fn full_access_returns_none_allowlist() {
        assert_eq!(SubTaskMode::FullAccess.allowed_tools(), None);
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
        assert!(task.title.is_none());
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

    // ---- Legacy serde compatibility ----

    #[test]
    fn legacy_aborted_deserializes_to_cancelled() {
        let json = serde_json::json!("aborted");
        let status: TaskStatus = serde_json::from_value(json).expect("aborted alias");
        assert_eq!(status, TaskStatus::Cancelled);
    }

    #[test]
    fn canonical_cancelled_roundtrips() {
        let json = serde_json::to_value(TaskStatus::Cancelled).expect("serialize");
        assert_eq!(json, serde_json::json!("cancelled"));
        let status: TaskStatus = serde_json::from_value(json).expect("deserialize");
        assert_eq!(status, TaskStatus::Cancelled);
    }
}
