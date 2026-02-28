//! Agent type definitions and traits.

use serde::{Deserialize, Serialize};

/// Represents an AI agent that can perform tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub description: String,
    pub agent_type: AgentType,
}

/// Types of AI agents available in the application.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    CodeGenerator,
    TaskAutomation,
    Productivity,
}

/// Represents a task that can be executed by an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub status: TaskStatus,
}

/// Status of a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}
