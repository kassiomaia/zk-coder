//! Agent type definitions.
//!
//! Core types for agent configuration, trust modes, and tool identifiers
//! used across the agent orchestration system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unique identifier for an agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Agent configuration (persisted).
///
/// Defines what an agent can do, which tools it has access to,
/// and its default behavior regarding permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    pub id: AgentId,
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub allowed_tools: Vec<String>,
    pub default_model_id: String,
    pub trust_mode: TrustMode,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Trust mode determines the default permission behavior for an agent.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrustMode {
    /// Auto-run all tools without approval.
    Trusted,
    /// Require approval for destructive operations (default).
    #[default]
    Untrusted,
    /// Require approval for all tool operations.
    Restricted,
}

/// Permission level for a specific tool.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionLevel {
    /// Execute immediately without user approval.
    #[default]
    AutoRun,
    /// Queue for user approval before executing.
    ApprovalRequired,
    /// Tool is not available for this agent.
    Disabled,
}

/// Type of action being performed by a tool.
///
/// Used by the permission system to determine if approval is needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionType {
    ReadFile { path: String },
    WriteFile { path: String, size_bytes: u64 },
    DeleteFile { path: String },
    ExecuteCommand { command: String },
    WebRequest { url: String },
}

/// Preview content for an action awaiting approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionPreview {
    /// Content type: "diff", "text", or "json".
    pub content_type: String,
    /// Preview content.
    pub content: String,
    /// Programming language for syntax highlighting.
    pub language: Option<String>,
}
