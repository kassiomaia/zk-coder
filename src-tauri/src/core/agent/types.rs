//! Contains core type definitions for the agent system.
//!
//! This module defines the fundamental data structures used for agent configuration,
//! trust modes, and tool identifiers that are shared across the agent orchestration system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A unique identifier for an agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AgentId(pub String);

impl AgentId {
    /// Creates a new `AgentId` from a string-like type.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Represents the persisted configuration for an agent.
///
/// This struct defines an agent's capabilities, including which tools it has
/// access to and its default behavior regarding permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    /// The unique identifier for the agent.
    pub id: AgentId,
    /// The display name of the agent.
    pub name: String,
    /// A description of the agent's capabilities.
    pub description: String,
    /// The system prompt that guides the agent's behavior.
    pub system_prompt: String,
    /// A list of tool names that this agent is allowed to use.
    pub allowed_tools: Vec<String>,
    /// The default model ID to be used by this agent.
    pub default_model_id: String,
    /// The trust mode that determines the default permission behavior.
    pub trust_mode: TrustMode,
    /// The timestamp of when the agent was created.
    pub created_at: DateTime<Utc>,
    /// The timestamp of when the agent was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Defines the default permission behavior for an agent.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrustMode {
    /// The agent can auto-run all tools without requiring user approval.
    Trusted,
    /// The agent requires approval for destructive operations (default).
    #[default]
    Untrusted,
    /// The agent requires approval for all tool operations.
    Restricted,
}

/// The permission level for a specific tool call.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionLevel {
    /// The tool can be executed immediately without user approval.
    #[default]
    AutoRun,
    /// The tool call must be queued for user approval before execution.
    ApprovalRequired,
    /// The tool is not available for use by the agent.
    Disabled,
}

/// The type of action being performed by a tool.
///
/// This enum is used by the permission system to determine whether an action
/// requires user approval based on its nature.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionType {
    /// An action that reads a file.
    ReadFile {
        /// The path of the file to be read.
        path: String,
    },
    /// An action that writes to a file.
    WriteFile {
        /// The path of the file to be written to.
        path: String,
        /// The size of the content to be written, in bytes.
        size_bytes: u64,
    },
    /// An action that deletes a file.
    DeleteFile {
        /// The path of the file to be deleted.
        path: String,
    },
    /// An action that executes a command.
    ExecuteCommand {
        /// The command to be executed.
        command: String,
    },
    /// An action that makes a web request.
    WebRequest {
        /// The URL to make the request to.
        url: String,
    },
    // Document generation actions
    /// An action to create a new DOCX document.
    CreateDocx {
        /// The filename for the new document.
        filename: String,
    },
    /// An action to mutate a DOCX draft.
    MutateDocxDraft {
        /// The ID of the document to be mutated.
        document_id: String,
    },
    /// An action to inspect a DOCX draft.
    InspectDocxDraft {
        /// The ID of the document to be inspected.
        document_id: String,
    },
    /// An action to create a new PPTX presentation.
    CreatePptx {
        /// The filename for the new presentation.
        filename: String,
    },
    /// An action to mutate a PPTX draft.
    MutatePptxDraft {
        /// The ID of the presentation to be mutated.
        deck_id: String,
    },
    /// An action to inspect a PPTX draft.
    InspectPptxDraft {
        /// The ID of the presentation to be inspected.
        deck_id: String,
    },
    /// An action to create a new PDF document.
    CreatePdf {
        /// The filename for the new document.
        filename: String,
    },
    /// An action to mutate a PDF draft.
    MutatePdfDraft {
        /// The ID of the document to be mutated.
        pdf_id: String,
    },
    /// An action to inspect a PDF draft.
    InspectPdfDraft {
        /// The ID of the document to be inspected.
        pdf_id: String,
    },
    /// An action to finalize a DOCX document.
    FinalizeDocx {
        /// The ID of the document to be finalized.
        document_id: String,
    },
    /// An action to finalize a PPTX presentation.
    FinalizePptx {
        /// The ID of the presentation to be finalized.
        deck_id: String,
    },
    /// An action to finalize a PDF document.
    FinalizePdf {
        /// The ID of the document to be finalized.
        pdf_id: String,
    },
}

/// Represents the preview content for an action that is awaiting approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionPreview {
    /// The content type of the preview, such as "diff", "text", or "json".
    pub content_type: String,
    /// The content of the preview.
    pub content: String,
    /// The programming language for syntax highlighting, if applicable.
    pub language: Option<String>,
}
