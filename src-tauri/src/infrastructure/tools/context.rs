//! Tool execution context.
//!
//! Provides shared context passed to all tool executions.

use std::path::PathBuf;

/// Shared context passed to all tool executions.
///
/// This struct carries request-scoped information that tools may need
/// to perform their operations, such as the conversation ID and workspace path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolExecutionContext {
    /// The conversation ID this tool invocation belongs to.
    pub conversation_id: String,
    /// Optional thread ID for threaded conversations.
    pub thread_id: Option<String>,
    /// Optional workspace root path for filesystem tools.
    pub workspace_root: Option<PathBuf>,
    /// Optional agent session ID (set when running inside an agent).
    pub session_id: Option<String>,
}

impl ToolExecutionContext {
    /// Create a new context with the minimum required information.
    #[must_use]
    pub fn new(conversation_id: impl Into<String>) -> Self {
        Self {
            conversation_id: conversation_id.into(),
            thread_id: None,
            workspace_root: None,
            session_id: None,
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

    /// Add agent session ID to the context.
    #[must_use]
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Constructor tests ===

    #[test]
    fn test_new_with_string() {
        let ctx = ToolExecutionContext::new("conv-123");
        assert_eq!(ctx.conversation_id, "conv-123");
        assert!(ctx.thread_id.is_none());
        assert!(ctx.workspace_root.is_none());
        assert!(ctx.session_id.is_none());
    }

    #[test]
    fn test_new_with_str() {
        let ctx = ToolExecutionContext::new("conv-456");
        assert_eq!(ctx.conversation_id, "conv-456");
    }

    #[test]
    fn test_new_with_empty_string() {
        let ctx = ToolExecutionContext::new("");
        assert_eq!(ctx.conversation_id, "");
    }

    #[test]
    fn test_new_with_owned_string() {
        let id = String::from("owned-id");
        let ctx = ToolExecutionContext::new(id);
        assert_eq!(ctx.conversation_id, "owned-id");
    }

    // === Builder method tests ===

    #[test]
    fn test_with_thread() {
        let ctx = ToolExecutionContext::new("conv-123").with_thread("thread-456");
        assert_eq!(ctx.thread_id, Some("thread-456".to_string()));
        assert!(ctx.session_id.is_none());
        assert!(ctx.workspace_root.is_none());
    }

    #[test]
    fn test_with_thread_str() {
        let ctx = ToolExecutionContext::new("conv-123").with_thread("thread-str");
        assert_eq!(ctx.thread_id, Some("thread-str".to_string()));
    }

    #[test]
    fn test_with_thread_owned() {
        let thread = String::from("owned-thread");
        let ctx = ToolExecutionContext::new("conv-123").with_thread(thread);
        assert_eq!(ctx.thread_id, Some("owned-thread".to_string()));
    }

    #[test]
    fn test_with_workspace() {
        let workspace = PathBuf::from("/home/user/project");
        let ctx = ToolExecutionContext::new("conv-123").with_workspace(workspace.clone());
        assert_eq!(ctx.workspace_root, Some(workspace));
        assert!(ctx.thread_id.is_none());
        assert!(ctx.session_id.is_none());
    }

    #[test]
    fn test_with_workspace_empty_path() {
        let workspace = PathBuf::new();
        let ctx = ToolExecutionContext::new("conv-123").with_workspace(workspace);
        assert_eq!(ctx.workspace_root, Some(PathBuf::new()));
    }

    #[test]
    fn test_with_session() {
        let ctx = ToolExecutionContext::new("conv-123").with_session("session-789");
        assert_eq!(ctx.session_id, Some("session-789".to_string()));
        assert!(ctx.thread_id.is_none());
        assert!(ctx.workspace_root.is_none());
    }

    #[test]
    fn test_with_session_str() {
        let ctx = ToolExecutionContext::new("conv-123").with_session("session-str");
        assert_eq!(ctx.session_id, Some("session-str".to_string()));
    }

    #[test]
    fn test_with_session_owned() {
        let session = String::from("owned-session");
        let ctx = ToolExecutionContext::new("conv-123").with_session(session);
        assert_eq!(ctx.session_id, Some("owned-session".to_string()));
    }

    // === Chaining builder tests ===

    #[test]
    fn test_builder_chain_all_fields() {
        let workspace = PathBuf::from("/workspace");
        let ctx = ToolExecutionContext::new("conv-123")
            .with_thread("thread-456")
            .with_workspace(workspace.clone())
            .with_session("session-789");

        assert_eq!(ctx.conversation_id, "conv-123");
        assert_eq!(ctx.thread_id, Some("thread-456".to_string()));
        assert_eq!(ctx.workspace_root, Some(workspace));
        assert_eq!(ctx.session_id, Some("session-789".to_string()));
    }

    #[test]
    fn test_builder_chain_thread_then_session() {
        let ctx = ToolExecutionContext::new("conv-123")
            .with_thread("thread-1")
            .with_session("session-1");

        assert_eq!(ctx.thread_id, Some("thread-1".to_string()));
        assert_eq!(ctx.session_id, Some("session-1".to_string()));
    }

    #[test]
    fn test_builder_chain_session_then_thread() {
        let ctx = ToolExecutionContext::new("conv-123")
            .with_session("session-1")
            .with_thread("thread-1");

        assert_eq!(ctx.session_id, Some("session-1".to_string()));
        assert_eq!(ctx.thread_id, Some("thread-1".to_string()));
    }

    #[test]
    fn test_builder_chain_workspace_then_thread() {
        let workspace = PathBuf::from("/workspace");
        let ctx = ToolExecutionContext::new("conv-123")
            .with_workspace(workspace)
            .with_thread("thread-1");

        assert!(ctx.workspace_root.is_some());
        assert!(ctx.thread_id.is_some());
    }

    // === Clone tests ===

    #[test]
    fn test_clone_independence() {
        let workspace = PathBuf::from("/workspace");
        let ctx1 = ToolExecutionContext::new("conv-1")
            .with_thread("thread-1")
            .with_workspace(workspace)
            .with_session("session-1");

        let ctx2 = ctx1.clone();

        assert_eq!(ctx1, ctx2);

        // Modify ctx2
        let ctx2 = ctx2.with_thread("thread-2");

        // ctx1 should be unchanged
        assert_eq!(ctx1.thread_id, Some("thread-1".to_string()));
        assert_eq!(ctx2.thread_id, Some("thread-2".to_string()));
    }

    #[test]
    fn test_clone_with_workspace_pathbuf() {
        let workspace = PathBuf::from("/workspace");
        let ctx1 = ToolExecutionContext::new("conv-1").with_workspace(workspace);
        let ctx2 = ctx1.clone();

        assert_eq!(ctx1, ctx2);
    }

    // === PartialEq tests ===

    #[test]
    fn test_partial_eq_equal() {
        let workspace = PathBuf::from("/workspace");
        let ctx1 = ToolExecutionContext::new("conv-1")
            .with_thread("thread-1")
            .with_workspace(workspace.clone())
            .with_session("session-1");

        let ctx2 = ToolExecutionContext::new("conv-1")
            .with_thread("thread-1")
            .with_workspace(workspace)
            .with_session("session-1");

        assert_eq!(ctx1, ctx2);
    }

    #[test]
    fn test_partial_eq_not_equal_conversation() {
        let ctx1 = ToolExecutionContext::new("conv-1");
        let ctx2 = ToolExecutionContext::new("conv-2");
        assert_ne!(ctx1, ctx2);
    }

    #[test]
    fn test_partial_eq_not_equal_thread() {
        let ctx1 = ToolExecutionContext::new("conv-1").with_thread("thread-1");
        let ctx2 = ToolExecutionContext::new("conv-1").with_thread("thread-2");
        assert_ne!(ctx1, ctx2);
    }

    #[test]
    fn test_partial_eq_not_equal_workspace() {
        let ctx1 = ToolExecutionContext::new("conv-1").with_workspace(PathBuf::from("/path1"));
        let ctx2 = ToolExecutionContext::new("conv-1").with_workspace(PathBuf::from("/path2"));
        assert_ne!(ctx1, ctx2);
    }

    #[test]
    fn test_partial_eq_not_equal_session() {
        let ctx1 = ToolExecutionContext::new("conv-1").with_session("session-1");
        let ctx2 = ToolExecutionContext::new("conv-1").with_session("session-2");
        assert_ne!(ctx1, ctx2);
    }

    #[test]
    fn test_partial_eq_some_vs_none_thread() {
        let ctx1 = ToolExecutionContext::new("conv-1").with_thread("thread-1");
        let ctx2 = ToolExecutionContext::new("conv-1");
        assert_ne!(ctx1, ctx2);
    }

    #[test]
    fn test_partial_eq_some_vs_none_workspace() {
        let ctx1 = ToolExecutionContext::new("conv-1").with_workspace(PathBuf::from("/workspace"));
        let ctx2 = ToolExecutionContext::new("conv-1");
        assert_ne!(ctx1, ctx2);
    }

    // === Debug output tests ===

    #[test]
    fn test_debug_format() {
        let ctx = ToolExecutionContext::new("conv-123")
            .with_thread("thread-456")
            .with_session("session-789");

        let debug_str = format!("{:?}", ctx);
        assert!(debug_str.contains("conv-123"));
        assert!(debug_str.contains("thread-456"));
        assert!(debug_str.contains("session-789"));
    }

    #[test]
    fn test_debug_with_workspace() {
        let ctx =
            ToolExecutionContext::new("conv-123").with_workspace(PathBuf::from("/workspace/path"));

        let debug_str = format!("{:?}", ctx);
        assert!(debug_str.contains("/workspace/path"));
    }

    // === Use case tests ===

    #[test]
    fn test_filesystem_tool_context() {
        // Context for a filesystem tool that needs workspace
        let workspace = PathBuf::from("/home/user/project");
        let ctx = ToolExecutionContext::new("conv-123").with_workspace(workspace.clone());

        assert!(ctx.workspace_root.is_some());
        assert_eq!(ctx.workspace_root.as_ref().unwrap(), &workspace);
    }

    #[test]
    fn test_agent_tool_context() {
        // Context for an agent tool that needs session
        let ctx = ToolExecutionContext::new("conv-123").with_session("agent-session-456");

        assert!(ctx.session_id.is_some());
        assert_eq!(ctx.session_id.as_ref().unwrap(), "agent-session-456");
    }

    #[test]
    fn test_threaded_conversation_context() {
        // Context for a threaded conversation
        let ctx = ToolExecutionContext::new("conv-123").with_thread("thread-456");

        assert!(ctx.thread_id.is_some());
        assert_eq!(ctx.thread_id.as_ref().unwrap(), "thread-456");
    }

    #[test]
    fn test_full_context() {
        // Full context with all fields populated
        let workspace = PathBuf::from("/home/user/project");
        let ctx = ToolExecutionContext::new("conv-123")
            .with_thread("thread-456")
            .with_workspace(workspace)
            .with_session("agent-session-789");

        assert_eq!(ctx.conversation_id, "conv-123");
        assert_eq!(ctx.thread_id, Some("thread-456".to_string()));
        assert!(ctx.workspace_root.is_some());
        assert_eq!(ctx.session_id, Some("agent-session-789".to_string()));
    }

    #[test]
    fn test_minimal_context() {
        // Minimal context with just conversation ID
        let ctx = ToolExecutionContext::new("conv-123");

        assert_eq!(ctx.conversation_id, "conv-123");
        assert!(ctx.thread_id.is_none());
        assert!(ctx.workspace_root.is_none());
        assert!(ctx.session_id.is_none());
    }

    // === Edge case tests ===

    #[test]
    fn test_empty_conversation_id() {
        let ctx = ToolExecutionContext::new("");
        assert_eq!(ctx.conversation_id, "");
    }

    #[test]
    fn test_special_characters_in_ids() {
        let ctx = ToolExecutionContext::new("conv-with-dashes_and_underscores")
            .with_thread("thread/with/slashes")
            .with_session("session.with.dots");

        assert_eq!(ctx.conversation_id, "conv-with-dashes_and_underscores");
        assert_eq!(ctx.thread_id, Some("thread/with/slashes".to_string()));
        assert_eq!(ctx.session_id, Some("session.with.dots".to_string()));
    }

    #[test]
    fn test_unicode_in_ids() {
        let ctx = ToolExecutionContext::new("conv-世界")
            .with_thread("thread-🌍")
            .with_session("session-😀");

        assert_eq!(ctx.conversation_id, "conv-世界");
        assert_eq!(ctx.thread_id, Some("thread-🌍".to_string()));
        assert_eq!(ctx.session_id, Some("session-😀".to_string()));
    }

    #[test]
    fn test_long_ids() {
        let long_id = "a".repeat(10_000);
        let ctx = ToolExecutionContext::new(&long_id)
            .with_thread(&long_id)
            .with_session(&long_id);

        assert_eq!(ctx.conversation_id.len(), 10_000);
        assert_eq!(ctx.thread_id.as_ref().unwrap().len(), 10_000);
        assert_eq!(ctx.session_id.as_ref().unwrap().len(), 10_000);
    }

    #[test]
    fn test_builder_mutability_original_unchanged() {
        let ctx1 = ToolExecutionContext::new("conv-1");
        let original_id = ctx1.conversation_id.clone();

        // Create a new context by modifying
        let ctx2 = ctx1.clone().with_thread("thread-1");

        // Original should be unchanged
        assert_eq!(ctx1.conversation_id, original_id);
        assert!(ctx1.thread_id.is_none());

        // New context should have the thread
        assert!(ctx2.thread_id.is_some());
    }

    #[test]
    fn test_workspace_pathbuf_preserved() {
        let mut workspace = PathBuf::from("/home/user");
        workspace.push("project");

        let ctx = ToolExecutionContext::new("conv-1").with_workspace(workspace.clone());

        assert_eq!(ctx.workspace_root, Some(workspace));
        assert_eq!(
            ctx.workspace_root.as_ref().unwrap().to_str().unwrap(),
            "/home/user/project"
        );
    }

    #[test]
    fn test_relative_workspace_path() {
        let workspace = PathBuf::from("./relative/path");
        let ctx = ToolExecutionContext::new("conv-1").with_workspace(workspace);

        assert!(ctx.workspace_root.is_some());
        assert_eq!(
            ctx.workspace_root.as_ref().unwrap().to_str().unwrap(),
            "./relative/path"
        );
    }

    #[test]
    fn test_context_reusability() {
        // Context can be reused as a base for different tool invocations
        let base =
            ToolExecutionContext::new("conv-123").with_workspace(PathBuf::from("/workspace"));

        let for_agent = base.clone().with_session("agent-session");
        let for_thread = base.clone().with_thread("thread-456");

        assert!(for_agent.session_id.is_some());
        assert!(for_agent.thread_id.is_none());

        assert!(for_thread.thread_id.is_some());
        assert!(for_thread.session_id.is_none());

        // Base should have neither
        assert!(base.thread_id.is_none());
        assert!(base.session_id.is_none());
    }

    #[test]
    fn test_multiple_with_same_type_overwrites() {
        let ctx = ToolExecutionContext::new("conv-1")
            .with_thread("thread-1")
            .with_thread("thread-2");

        assert_eq!(ctx.thread_id, Some("thread-2".to_string()));
    }

    #[test]
    fn test_multiple_with_session_overwrites() {
        let ctx = ToolExecutionContext::new("conv-1")
            .with_session("session-1")
            .with_session("session-2");

        assert_eq!(ctx.session_id, Some("session-2".to_string()));
    }

    #[test]
    fn test_multiple_with_workspace_overwrites() {
        let ctx = ToolExecutionContext::new("conv-1")
            .with_workspace(PathBuf::from("/path1"))
            .with_workspace(PathBuf::from("/path2"));

        assert_eq!(ctx.workspace_root, Some(PathBuf::from("/path2")));
    }
}
