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
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    pub fn set_status(&mut self, status: TaskStatus) {
        let is_terminal = matches!(status, TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled);
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
    LlmRequest { model: String },
    ToolCall { tool: String, arguments: serde_json::Value },
}

/// The result of a step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepResult {
    pub success: bool,
    pub output: serde_json::Value,
    pub tokens_used: Option<u32>,
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
                let mut subtask = SubTask::new(task_id.to_string(), step.title.clone(), i as i32);
                if let Some(ref desc) = step.description {
                    subtask = subtask.with_description(desc.clone());
                }
                subtask
            })
            .collect()
    }
}
