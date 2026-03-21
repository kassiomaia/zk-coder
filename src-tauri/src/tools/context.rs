//! Tool execution context.
//!
//! Provides shared context passed to all tool executions.

use std::path::PathBuf;

/// Shared context passed to all tool executions.
///
/// This struct carries request-scoped information that tools may need
/// to perform their operations, such as the conversation ID and workspace path.
#[derive(Debug, Clone)]
pub struct ToolExecutionContext {
    /// The conversation ID this tool invocation belongs to.
    pub conversation_id: String,
    /// Optional thread ID for threaded conversations.
    pub thread_id: Option<String>,
    /// Optional workspace root path for filesystem tools.
    pub workspace_root: Option<PathBuf>,
}

impl ToolExecutionContext {
    /// Create a new context with the minimum required information.
    #[must_use]
    pub fn new(conversation_id: impl Into<String>) -> Self {
        Self {
            conversation_id: conversation_id.into(),
            thread_id: None,
            workspace_root: None,
        }
    }

    /// Add thread ID to the context.
    #[must_use]
    pub fn with_thread(mut self, thread_id: impl Into<String>) -> Self {
        self.thread_id = Some(thread_id.into());
        self
    }

    /// Add workspace root to the context.
    #[must_use]
    pub fn with_workspace(mut self, workspace_root: PathBuf) -> Self {
        self.workspace_root = Some(workspace_root);
        self
    }
}
