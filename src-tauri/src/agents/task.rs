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
