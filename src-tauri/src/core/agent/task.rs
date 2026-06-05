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

/// Structured domain priors carried alongside `TaskFraming`
/// (Workstream H.2).
///
/// The initial schema is intentionally narrow: a free-form `hints`
/// list. Future iterations will enrich this with structured fields
/// (stack defaults for code work, style references for writing work,
/// artifact conventions). Keeping the v1 shape permissive lets the
/// planner declare hints today without blocking on the richer schema.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DomainPriors {
    /// Free-form hints about the task's domain context — stack
    /// defaults, style references, artifact conventions. Rendered
    /// into the task-framing section of the system prompt as a
    /// bulleted list.
    #[serde(default)]
    pub hints: Vec<String>,
}

/// Task-declared framing (Workstream H.2).
///
/// Persona arrives with the task, not the agent. The record carries
/// role, voice constraints, optional structured domain priors, and
/// optional forbidden-pattern regexes. See
/// `docs/planning/agent-fidelity/system-prompt-architecture.md` §Layer 2
/// for the design rationale and the validation rules applied at
/// planning time.
///
/// Validation is the caller's responsibility — the `agent_plan_subtasks`
/// handler rejects framings with fewer than
/// [`TaskFraming::MIN_VOICE_CONSTRAINTS`] voice constraints or with any
/// `forbidden_patterns` entry that fails to compile as a regex.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskFraming {
    /// Short role descriptor: `"senior full-stack engineer"`,
    /// `"senior communication professional"`, `"technical writer"`.
    pub role: String,
    /// Opinionated, specific constraints about the voice the task
    /// calls for. At least [`TaskFraming::MIN_VOICE_CONSTRAINTS`]
    /// entries are required at planning time; bland one-liners such
    /// as "professional and clear" are rejected by the validator.
    #[serde(default)]
    pub voice_constraints: Vec<String>,
    /// Optional structured hints for the task's domain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_priors: Option<DomainPriors>,
    /// Optional regex strings that must not appear in task outputs.
    /// Each entry must compile as a regex at planning time; the
    /// planner runs the validation before persisting the framing.
    #[serde(default)]
    pub forbidden_patterns: Vec<String>,
}

impl TaskFraming {
    /// Minimum number of `voice_constraints` accepted by
    /// [`Self::validate`]. Kept at `2` so a lazy single-line
    /// declaration fails and forces the planner to be opinionated.
    pub const MIN_VOICE_CONSTRAINTS: usize = 2;

    /// Validates the framing at planning time.
    ///
    /// Returns the first validation error encountered. Callers should
    /// reject the plan instead of persisting the framing on `Err`.
    ///
    /// Rules:
    /// - `role` must be non-empty after trimming.
    /// - `voice_constraints.len()` must be at least
    ///   [`Self::MIN_VOICE_CONSTRAINTS`], and every entry must be
    ///   non-empty after trimming.
    /// - Every `forbidden_patterns` entry must compile as a regex.
    pub fn validate(&self) -> Result<(), String> {
        if self.role.trim().is_empty() {
            return Err("framing.role must not be empty".to_string());
        }
        if self.voice_constraints.len() < Self::MIN_VOICE_CONSTRAINTS {
            return Err(format!(
                "framing.voice_constraints requires at least {} entries; got {}",
                Self::MIN_VOICE_CONSTRAINTS,
                self.voice_constraints.len(),
            ));
        }
        for (idx, constraint) in self.voice_constraints.iter().enumerate() {
            if constraint.trim().is_empty() {
                return Err(format!(
                    "framing.voice_constraints[{idx}] must not be empty"
                ));
            }
        }
        for (idx, pattern) in self.forbidden_patterns.iter().enumerate() {
            if let Err(e) = regex::Regex::new(pattern) {
                return Err(format!(
                    "framing.forbidden_patterns[{idx}] is not a valid regex (`{pattern}`): {e}"
                ));
            }
        }
        Ok(())
    }
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
    /// LLM-generated description. A short paragraph explaining what the task will do.
    /// Hydrated from the `agent_tasks.description` column on reload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The current status of the task.
    pub status: TaskStatus,
    /// A list of execution steps that have been performed for this task.
    pub steps: Vec<ExecutionStep>,
    /// A list of sub-tasks that have been created for this task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtasks: Option<Vec<SubTask>>,
    /// Task-declared framing (Workstream H.2). Persona, voice, and
    /// domain priors declared by the planner and stable for the
    /// duration of the task. `None` on legacy tasks and on code-work
    /// plans that rely on the genre-default `default_code_framing()`
    /// section of the system prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub framing: Option<TaskFraming>,
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
            description: None,
            status: TaskStatus::Pending,
            steps: Vec::new(),
            subtasks: None,
            framing: None,
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

/// Classifies the kind of artifact a subtask produces.
///
/// Used by scope-drift detection (Workstream B.3) and the contract-aware
/// active-subtask appendix (Workstream H.3) so writes can be matched
/// against an expectation declared at planning time.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    /// A filesystem file written through `fs_write_file`,
    /// `fs_replace_in_file`, or `fs_apply_patch`.
    File,
    /// A structured document produced through the
    /// `pdf_*` / `docx_*` / `pptx_*` tool family.
    Document,
    /// A shell command execution (kept for completeness; commands do
    /// not typically have a written "path" but contracts may still
    /// declare an output file a command is expected to produce).
    Command,
    /// Any other artifact kind the contract wants to declare.
    Other,
}

/// A single artifact expectation declared on a subtask contract.
///
/// `path_pattern` is glob-style (`docs/**/*.md`, `reports/*.pdf`, etc.).
/// Matching rules follow the `globset` crate conventions. When absent,
/// the scope-drift detector does not have a target and is a no-op.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactSpec {
    /// Glob-style pattern that a produced artifact's path must match.
    #[serde(alias = "path_pattern")]
    pub path_pattern: String,
    /// The kind of artifact this spec describes.
    pub kind: ArtifactKind,
}

/// The contract a subtask declares at planning time.
///
/// When present, write-class tool calls whose output path does not
/// match any `expected_artifacts` entry are flagged as scope drift:
/// the runtime emits a `subtask_scope_drift` trace, injects a
/// correction system message into the next step's context (soft
/// enforcement), and optionally rejects the tool call with a
/// `subtask_scope_drift` error (hard enforcement, behind the
/// `CODER_SUBTASK_CONTRACT_HARD_ENFORCEMENT` environment flag — ships
/// disabled by default).
///
/// The field is optional on `SubTask` so legacy plans (pre-B) continue
/// to work without behavior change: absence of a contract means the
/// scope-drift detector short-circuits.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SubTaskContract {
    /// The list of expected outputs this subtask must produce. At
    /// least one entry is required for scope-drift detection to
    /// evaluate; an empty list is permitted in the DB round-trip but
    /// is treated as "no expectations" by the runtime (same as
    /// absence).
    #[serde(alias = "expected_artifacts")]
    pub expected_artifacts: Vec<ArtifactSpec>,
}

impl SubTaskContract {
    /// Returns `true` when the contract has no expected artifacts,
    /// which the runtime treats identically to an absent contract
    /// (scope-drift detection is a no-op).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.expected_artifacts.is_empty()
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
    /// The contract describing the artifacts this subtask is expected
    /// to produce (Workstream B.1). When `None`, the legacy behavior
    /// is preserved — scope-drift detection is disabled for this
    /// subtask.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract: Option<SubTaskContract>,
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
            contract: None,
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

    /// Sets the contract for the sub-task (Workstream B.1).
    #[must_use]
    pub fn with_contract(mut self, contract: SubTaskContract) -> Self {
        self.contract = Some(contract);
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

    // ------------------------------------------------------------------------
    // SubTaskContract serialization (Workstream B.1)
    // ------------------------------------------------------------------------

    #[test]
    fn subtask_contract_deserializes_from_snake_case_schema_shape() {
        // The tool JSON schema exposed to the LLM uses snake_case
        // (`expected_artifacts` / `path_pattern`); serde's default
        // camelCase rename on the struct would otherwise reject the
        // exact shape the schema instructs the model to produce.
        // The `#[serde(alias = ...)]` attributes on these fields are
        // what bridges the two spellings, so keep this test as the
        // canonical proof that the bridge works.
        let payload = serde_json::json!({
            "expected_artifacts": [
                { "path_pattern": "docs/*.md", "kind": "file" }
            ]
        });

        let parsed: SubTaskContract =
            serde_json::from_value(payload).expect("snake_case payload must deserialize");

        assert_eq!(parsed.expected_artifacts.len(), 1);
        assert_eq!(parsed.expected_artifacts[0].path_pattern, "docs/*.md");
        assert_eq!(parsed.expected_artifacts[0].kind, ArtifactKind::File);
    }

    #[test]
    fn subtask_contract_still_deserializes_from_camel_case_shape() {
        // Round-trip from serialized form (which remains camelCase for
        // DB + frontend consistency) must keep working.
        let payload = serde_json::json!({
            "expectedArtifacts": [
                { "pathPattern": "docs/*.md", "kind": "file" }
            ]
        });

        let parsed: SubTaskContract =
            serde_json::from_value(payload).expect("camelCase payload must deserialize");

        assert_eq!(parsed.expected_artifacts[0].path_pattern, "docs/*.md");
    }

    #[test]
    fn subtask_contract_serializes_as_camel_case() {
        // The on-wire/on-disk format stays camelCase — the frontend
        // type (`SubTaskContract.expectedArtifacts` in TS) depends on
        // it.
        let contract = SubTaskContract {
            expected_artifacts: vec![ArtifactSpec {
                path_pattern: "docs/*.md".to_string(),
                kind: ArtifactKind::File,
            }],
        };

        let json = serde_json::to_value(&contract).expect("serialize");
        assert!(json.get("expectedArtifacts").is_some());
        assert!(json.get("expected_artifacts").is_none());
        assert!(json["expectedArtifacts"][0].get("pathPattern").is_some());
    }

    #[test]
    fn subtask_contract_serde_round_trip_preserves_kinds() {
        let contract = SubTaskContract {
            expected_artifacts: vec![
                ArtifactSpec {
                    path_pattern: "docs/communications/social/*.md".to_string(),
                    kind: ArtifactKind::File,
                },
                ArtifactSpec {
                    path_pattern: "reports/launch/*.pdf".to_string(),
                    kind: ArtifactKind::Document,
                },
                ArtifactSpec {
                    path_pattern: "scripts/deploy.sh".to_string(),
                    kind: ArtifactKind::Command,
                },
                ArtifactSpec {
                    path_pattern: "misc/*".to_string(),
                    kind: ArtifactKind::Other,
                },
            ],
        };

        let json = serde_json::to_string(&contract).expect("serialize");
        let parsed: SubTaskContract = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, contract);
    }

    #[test]
    fn subtask_without_contract_serializes_without_field() {
        let subtask = SubTask::new(
            "task-1".to_string(),
            "no contract".to_string(),
            SubTaskMode::Read,
            0,
        );
        let json = serde_json::to_value(&subtask).expect("serialize");
        assert!(
            json.get("contract").is_none(),
            "absent contract must be skipped in serialized form, got {json}",
        );
    }

    #[test]
    fn subtask_with_contract_round_trips_through_serde() {
        let contract = SubTaskContract {
            expected_artifacts: vec![ArtifactSpec {
                path_pattern: "docs/*.md".to_string(),
                kind: ArtifactKind::File,
            }],
        };
        let subtask = SubTask::new(
            "task-1".to_string(),
            "with contract".to_string(),
            SubTaskMode::Write,
            0,
        )
        .with_contract(contract.clone());

        let json = serde_json::to_string(&subtask).expect("serialize");
        let parsed: SubTask = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.contract, Some(contract));
    }

    #[test]
    fn subtask_contract_is_empty_when_no_artifacts() {
        let empty = SubTaskContract {
            expected_artifacts: vec![],
        };
        assert!(empty.is_empty());
        let non_empty = SubTaskContract {
            expected_artifacts: vec![ArtifactSpec {
                path_pattern: "x".to_string(),
                kind: ArtifactKind::File,
            }],
        };
        assert!(!non_empty.is_empty());
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

    // ------------------------------------------------------------------------
    // TaskFraming (Workstream H.2.a)
    // ------------------------------------------------------------------------

    fn sample_framing() -> TaskFraming {
        TaskFraming {
            role: "senior communication professional".to_string(),
            voice_constraints: vec![
                "one claim per paragraph".to_string(),
                "no filler openers".to_string(),
            ],
            domain_priors: Some(DomainPriors {
                hints: vec!["press release voice".to_string()],
            }),
            forbidden_patterns: vec![r"(?i)game[- ]changing".to_string()],
        }
    }

    #[test]
    fn task_framing_serde_round_trips() {
        let framing = sample_framing();
        let json = serde_json::to_string(&framing).expect("serialize");
        let parsed: TaskFraming = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, framing);
    }

    #[test]
    fn task_framing_validates_minimum_voice_constraints() {
        let mut framing = sample_framing();
        framing.voice_constraints = vec!["professional and clear".to_string()];
        let err = framing
            .validate()
            .expect_err("single voice constraint must fail");
        assert!(err.contains("voice_constraints"));
        assert!(err.contains(&TaskFraming::MIN_VOICE_CONSTRAINTS.to_string()));
    }

    #[test]
    fn task_framing_validates_empty_role() {
        let mut framing = sample_framing();
        framing.role = "   ".to_string();
        let err = framing.validate().expect_err("blank role must fail");
        assert!(err.contains("role"));
    }

    #[test]
    fn task_framing_validates_empty_voice_entry() {
        let mut framing = sample_framing();
        framing.voice_constraints[1] = "  ".to_string();
        let err = framing.validate().expect_err("empty voice entry must fail");
        assert!(err.contains("voice_constraints"));
    }

    #[test]
    fn task_framing_rejects_uncompilable_forbidden_pattern() {
        let mut framing = sample_framing();
        framing.forbidden_patterns = vec!["[unclosed".to_string()];
        let err = framing
            .validate()
            .expect_err("invalid regex must fail validation");
        assert!(err.contains("forbidden_patterns"));
    }

    #[test]
    fn task_framing_with_compilable_patterns_passes() {
        let framing = sample_framing();
        framing.validate().expect("sample framing is valid");
    }

    #[test]
    fn agent_task_framing_defaults_to_none() {
        let session_id = SessionId("session-framing".to_string());
        let request = AgentRequest {
            content: "Test".to_string(),
            attachments: vec![],
            context: None,
        };
        let task = AgentTask::new(session_id, request);
        assert!(task.framing.is_none());
    }

    #[test]
    fn agent_task_with_framing_round_trips_through_serde() {
        let session_id = SessionId("session-framing".to_string());
        let request = AgentRequest {
            content: "Test".to_string(),
            attachments: vec![],
            context: None,
        };
        let mut task = AgentTask::new(session_id, request);
        task.framing = Some(sample_framing());

        let json = serde_json::to_string(&task).expect("serialize");
        let parsed: AgentTask = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.framing, Some(sample_framing()));
    }

    #[test]
    fn agent_task_without_framing_omits_field_in_json() {
        let session_id = SessionId("session-framing".to_string());
        let request = AgentRequest {
            content: "Test".to_string(),
            attachments: vec![],
            context: None,
        };
        let task = AgentTask::new(session_id, request);
        let json = serde_json::to_value(&task).expect("serialize");
        assert!(
            json.get("framing").is_none(),
            "absent framing must be skipped in serialized form"
        );
    }

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
