//! Agent service implementation.
//!
//! Provides the concrete implementation of the core AgentService trait.
//! This service coordinates session management, approvals, and agent execution.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use log::info;

use crate::agents::approval::ApprovalQueue;
use crate::agents::registry::AgentRegistry;
use crate::agents::rollback::RollbackStore;
use crate::agents::runtime::AgentRuntime;
use crate::agents::session::{AgentSession, PendingAction, SessionId, SessionState};
use crate::agents::store::SessionStore;
use crate::agents::task::{SubTask, SubTaskMode, TaskStatus};
use crate::agents::types::{Agent, AgentId};

// Core imports
use crate::core::agent::entities::{
    Agent as CoreAgent, AgentDomainError, AgentSession as CoreAgentSession,
    AgentTask as CoreAgentTask, PendingAction as CorePendingAction,
    SessionId as CoreSessionId, TaskStatus as CoreTaskStatus,
};
use crate::core::agent::repositories::RepositoryResult;
use crate::core::agent::services::{
    AgentService, CreateSessionRequest, ExecutionStep, FileAction, FileChange,
    SendAgentRequestRequest, SessionDetails, TerminalLine,
};

// Conversion utilities
use super::conversions::store_error_to_domain;

/// Implementation of the AgentService trait.
///
/// This service coordinates:
/// - Session lifecycle (create, get, cancel, delete)
/// - Agent execution (send requests to sessions)
/// - Approval management (approve/deny actions)
/// - Agent registry (list/get available agents)
/// - Task management (create subtasks, update status)
/// - History and rollback operations
pub struct AgentServiceImpl {
    /// Agent registry for available agent configurations
    registry: Arc<AgentRegistry>,
    /// Session store for persistence
    session_store: Arc<SessionStore>,
    /// Approval queue for pending actions
    approval_queue: Arc<ApprovalQueue>,
    /// Agent runtime for execution
    runtime: Arc<AgentRuntime>,
    /// Rollback store for file snapshots
    rollback_store: Arc<RollbackStore>,
    /// Tokio runtime for async operations
    runtime_handle: tokio::runtime::Runtime,
}

impl AgentServiceImpl {
    /// Creates a new agent service.
    pub fn new(
        registry: Arc<AgentRegistry>,
        session_store: Arc<SessionStore>,
        approval_queue: Arc<ApprovalQueue>,
        runtime: Arc<AgentRuntime>,
    ) -> Result<Self, String> {
        let runtime_handle = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Failed to create runtime: {e}"))?;

        let rollback_store = runtime.rollback_store().clone();

        Ok(Self {
            registry,
            session_store,
            approval_queue,
            runtime,
            rollback_store,
            runtime_handle,
        })
    }

    /// Get the agent registry.
    pub fn registry(&self) -> &Arc<AgentRegistry> {
        &self.registry
    }

    /// Get the approval queue.
    pub fn approval_queue(&self) -> &Arc<ApprovalQueue> {
        &self.approval_queue
    }

    /// Get the agent runtime.
    pub fn runtime(&self) -> &Arc<AgentRuntime> {
        &self.runtime
    }

    /// Get the session store.
    pub fn session_store(&self) -> &Arc<SessionStore> {
        &self.session_store
    }

    /// Convert SessionId from core to agents.
    pub(crate) fn to_agents_session_id(&self, session_id: &str) -> SessionId {
        SessionId::from(session_id)
    }

    /// Convert AgentId from core to agents.
    pub(crate) fn to_agents_agent_id(&self, agent_id: &str) -> AgentId {
        AgentId::from(agent_id)
    }

    /// Block on async future using the runtime handle.
    pub(crate) fn block_on<F, T>(&self, future: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        self.runtime_handle.block_on(future)
    }
}

/// Wrapper to allow async trait methods.
///
/// The AgentService trait from core doesn't require async, but some
/// of our dependencies (registry, approval_queue) use async.
/// This implementation blocks on async operations internally.
impl AgentService for AgentServiceImpl {
    // ========================================================================
    // Session Operations
    // ========================================================================

    fn create_session(&self, request: CreateSessionRequest) -> RepositoryResult<CoreAgentSession> {
        info!("[AgentService] Creating session for agent: {}", request.agent_id);

        let agent_id = self.to_agents_agent_id(&request.agent_id);
        let workspace_root = request.workspace_root.map(PathBuf::from);

        // Create the session
        let session = AgentSession::new(
            agent_id.clone(),
            request.agent_id.clone(),
            request.conversation_id.clone(),
            request.thread_id.clone(),
            workspace_root,
        );

        // Persist to store
        self.session_store
            .create_session(&session)
            .map_err(|e| store_error_to_domain(e))?;

        // Transition to WaitForRequest state
        let new_state = SessionState::WaitForRequest {
            last_action_at: Utc::now(),
        };
        self.session_store
            .update_state(&session.id, &new_state)
            .map_err(|e| store_error_to_domain(e))?;

        // Fetch and return the updated session
        let updated_session = self
            .session_store
            .get_session(&session.id)
            .map_err(|e| store_error_to_domain(e))?
            .ok_or_else(|| AgentDomainError::SessionNotFound {
                id: session.id.0.clone(),
            })?;

        Ok(updated_session.into())
    }

    fn get_session(&self, session_id: &str) -> RepositoryResult<CoreAgentSession> {
        let sid = self.to_agents_session_id(session_id);

        let session = self
            .session_store
            .get_session(&sid)
            .map_err(|e| store_error_to_domain(e))?
            .ok_or_else(|| AgentDomainError::SessionNotFound {
                id: session_id.to_string(),
            })?;

        Ok(session.into())
    }

    fn get_conversation_sessions(
        &self,
        conversation_id: &str,
    ) -> RepositoryResult<Vec<CoreAgentSession>> {
        let sessions = self
            .session_store
            .get_sessions_for_conversation(conversation_id)
            .map_err(|e| store_error_to_domain(e))?;

        Ok(sessions
            .into_iter()
            .map(|s| s.into())
            .collect())
    }

    fn cancel_session(&self, session_id: &str) -> RepositoryResult<()> {
        let sid = self.to_agents_session_id(session_id);

        let new_state = SessionState::Cancelled {
            reason: "User cancelled".to_string(),
        };

        self.session_store
            .update_state(&sid, &new_state)
            .map_err(|e| store_error_to_domain(e))?;

        // Note: Clearing pending approvals is handled by the command layer
        // since it requires async which we can't do here directly
        Ok(())
    }

    fn delete_session(&self, session_id: &str) -> RepositoryResult<()> {
        let sid = self.to_agents_session_id(session_id);

        self.session_store
            .delete_session(&sid)
            .map_err(|e| store_error_to_domain(e))?;

        Ok(())
    }

    fn update_session_workspace(
        &self,
        session_id: &str,
        workspace_root: &str,
    ) -> RepositoryResult<()> {
        let sid = self.to_agents_session_id(session_id);

        self.session_store
            .update_workspace(&sid, workspace_root)
            .map_err(|e| store_error_to_domain(e))?;

        Ok(())
    }

    // ========================================================================
    // Execution Operations
    // ========================================================================

    fn send_request(&self, request: SendAgentRequestRequest) -> RepositoryResult<CoreSessionId> {
        let sid = self.to_agents_session_id(&request.session_id);

        // Verify session exists
        let _session = self
            .session_store
            .get_session(&sid)
            .map_err(|e| store_error_to_domain(e))?
            .ok_or_else(|| AgentDomainError::SessionNotFound {
                id: request.session_id.clone(),
            })?;

        // Note: Actual execution is handled by the runtime in an async context
        // This method validates the session and returns immediately
        // The runtime will handle execution via tokio::spawn in the command layer
        info!(
            "[AgentService] Request validated for session {}",
            request.session_id
        );

        Ok(CoreSessionId(request.session_id))
    }

    // ========================================================================
    // Approval Operations
    // ========================================================================

    fn get_pending_actions(&self, session_id: &str) -> RepositoryResult<Vec<CorePendingAction>> {
        let actions = self.block_on(self.approval_queue.get_pending_for_session(session_id));

        Ok(actions
            .into_iter()
            .map(|a| a.into())
            .collect())
    }

    fn approve_action(&self, action_id: &str) -> RepositoryResult<()> {
        self.block_on(self.approval_queue.approve(action_id))
            .ok_or_else(|| AgentDomainError::ActionNotFound {
                id: action_id.to_string(),
            })?;

        Ok(())
    }

    fn deny_action(&self, action_id: &str) -> RepositoryResult<()> {
        let denied = self.block_on(self.approval_queue.deny(action_id));

        if !denied {
            return Err(AgentDomainError::ActionNotFound {
                id: action_id.to_string(),
            });
        }

        Ok(())
    }

    // ========================================================================
    // Registry Operations
    // ========================================================================

    fn list_agents(&self) -> RepositoryResult<Vec<CoreAgent>> {
        let agents = self.block_on(self.registry.list());

        Ok(agents
            .into_iter()
            .map(|a| a.into())
            .collect())
    }

    fn get_agent(&self, agent_id: &str) -> RepositoryResult<CoreAgent> {
        let aid = self.to_agents_agent_id(agent_id);

        let agent = self
            .block_on(self.registry.get(&aid))
            .ok_or_else(|| AgentDomainError::AgentNotFound {
                id: agent_id.to_string(),
            })?;

        Ok(agent.into())
    }

    // ========================================================================
    // Task Operations
    // ========================================================================

    fn create_subtask(
        &self,
        task_id: String,
        title: String,
        description: Option<String>,
        order: i32,
    ) -> RepositoryResult<String> {
        let subtask = SubTask::new(task_id, title, SubTaskMode::FullAccess, order);

        let subtask = if let Some(desc) = description {
            subtask.with_description(desc)
        } else {
            subtask
        };

        let subtask_id = subtask.id.clone();

        self.session_store
            .create_subtask(&subtask)
            .map_err(|e| store_error_to_domain(e))?;

        Ok(subtask_id)
    }

    fn update_subtask_status(
        &self,
        subtask_id: String,
        status: CoreTaskStatus,
    ) -> RepositoryResult<()> {
        let task_status: TaskStatus = status.into();

        self.session_store
            .update_subtask_status(&subtask_id, &task_status)
            .map_err(|e| store_error_to_domain(e))?;

        Ok(())
    }

    fn get_subtasks(&self, task_id: &str) -> RepositoryResult<Vec<CoreAgentTask>> {
        let subtasks = self
            .session_store
            .get_subtasks(task_id)
            .map_err(|e| store_error_to_domain(e))?;

        Ok(subtasks
            .into_iter()
            .map(|st| st.into())
            .collect())
    }

    // ========================================================================
    // History Operations
    // ========================================================================

    fn get_conversation_history(
        &self,
        conversation_id: &str,
    ) -> RepositoryResult<Vec<CoreAgentSession>> {
        let sessions = self
            .session_store
            .get_sessions_for_conversation(conversation_id)
            .map_err(|e| store_error_to_domain(e))?;

        // Filter to only completed/terminal sessions
        let history: Vec<AgentSession> = sessions
            .into_iter()
            .filter(|s| {
                matches!(
                    s.state,
                    SessionState::Finished { .. }
                        | SessionState::Error { .. }
                        | SessionState::Cancelled { .. }
                )
            })
            .collect();

        Ok(history
            .into_iter()
            .map(|s| s.into())
            .collect())
    }

    fn get_session_details(&self, session_id: &str) -> RepositoryResult<SessionDetails> {
        let sid = self.to_agents_session_id(session_id);

        let session = self
            .session_store
            .get_session(&sid)
            .map_err(|e| store_error_to_domain(e))?
            .ok_or_else(|| AgentDomainError::SessionNotFound {
                id: session_id.to_string(),
            })?;

        // Get files touched
        let files_touched = self
            .session_store
            .get_session_files(&sid)
            .map(|files| {
                files
                    .into_iter()
                    .map(|f| FileChange {
                        path: f.file_path,
                        action: match f.action.as_str() {
                            "read" => FileAction::Read,
                            "write" => FileAction::Write,
                            "delete" => FileAction::Delete,
                            _ => FileAction::Read,
                        },
                        diff: f.diff,
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Get terminal output - convert timestamp string to DateTime
        let terminal_output = self
            .session_store
            .get_session_terminal_output(&sid)
            .map(|lines| {
                lines
                    .into_iter()
                    .filter_map(|l| {
                        // Parse timestamp from RFC3339 string
                        let timestamp = chrono::DateTime::parse_from_rfc3339(&l.timestamp)
                            .ok()
                            .map(|dt| dt.with_timezone(&chrono::Utc));

                        timestamp.map(|ts| TerminalLine {
                            id: l.id,
                            line_type: l.line_type,
                            content: l.content,
                            timestamp: ts,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Get execution steps - convert from store's ExecutionStep to service's ExecutionStep
        let execution_steps = self
            .session_store
            .get_session_execution_steps(&sid)
            .map(|steps| {
                use crate::agents::task::StepAction;
                steps
                    .into_iter()
                    .map(|s| {
                        let (step_type, description) = match &s.action {
                            StepAction::LlmRequest { model } => (
                                "llm_request".to_string(),
                                format!("LLM request using model: {}", model),
                            ),
                            StepAction::ToolCall { tool, arguments } => {
                                let arg_count = if arguments.is_object() {
                                    arguments.as_object().map(|o| o.len()).unwrap_or(0)
                                } else if arguments.is_array() {
                                    arguments.as_array().map(|a| a.len()).unwrap_or(0)
                                } else {
                                    1
                                };
                                (
                                    "tool_call".to_string(),
                                    format!("Tool call: {} with {} arguments", tool, arg_count),
                                )
                            }
                        };

                        let status = if s.result.is_some() {
                            "completed".to_string()
                        } else {
                            "pending".to_string()
                        };

                        ExecutionStep {
                            id: format!("step-{}", s.step_number),
                            step_type,
                            description,
                            status,
                            timestamp: s.timestamp,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(SessionDetails {
            session: session.into(),
            files_touched,
            terminal_output,
            execution_steps,
        })
    }

    // ========================================================================
    // Rollback Operations
    // ========================================================================

    fn get_file_snapshots(&self, session_id: &str) -> RepositoryResult<Vec<FileChange>> {
        let snapshots = self.rollback_store.get_session_snapshots(session_id);

        Ok(snapshots
            .into_iter()
            .map(|s| FileChange {
                path: s.path.display().to_string(),
                action: if s.original_content.is_some() {
                    FileAction::Write
                } else {
                    FileAction::Read
                },
                diff: None,
            })
            .collect())
    }

    fn rollback_file(&self, session_id: &str, path: &str) -> RepositoryResult<bool> {
        use std::path::Path;

        self.rollback_store
            .rollback_file(session_id, Path::new(path))
            .map_err(|e| AgentDomainError::InvalidInput {
                reason: format!("Failed to rollback file: {}", e),
            })
    }

    fn rollback_session(&self, session_id: &str) -> RepositoryResult<Vec<String>> {
        let paths = self
            .rollback_store
            .rollback_session(session_id)
            .map_err(|e| AgentDomainError::InvalidInput {
                reason: format!("Failed to rollback session: {}", e),
            })?;

        Ok(paths
            .into_iter()
            .map(|p| p.display().to_string())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::session::PendingAction;
    use crate::agents::types::ActionType;
    use crate::core::agent::entities::AgentTrustMode;
    use crate::tools::ToolRegistry;

    /// Creates a test service with all dependencies initialized.
    fn create_test_service() -> AgentServiceImpl {
        use std::sync::Arc;

        let registry = Arc::new(AgentRegistry::new());
        let session_store = Arc::new(
            SessionStore::new(":memory:")
                .expect("Failed to create in-memory store"),
        );
        let approval_queue = Arc::new(ApprovalQueue::new());
        let tool_registry = Arc::new(
            ToolRegistry::new()
                .expect("Failed to create tool registry"),
        );
        let runtime = Arc::new(AgentRuntime::new(
            session_store.clone(),
            approval_queue.clone(),
            tool_registry,
        ));

        AgentServiceImpl::new(registry, session_store, approval_queue, runtime)
            .expect("Failed to create test service")
    }

    /// Creates a test agent session.
    fn create_test_session(service: &AgentServiceImpl, conversation_id: &str) -> CoreAgentSession {
        let request = CreateSessionRequest {
            agent_id: "code-agent".to_string(),
            conversation_id: conversation_id.to_string(),
            thread_id: format!("thread-{}", conversation_id),
            workspace_root: Some("/test/workspace".to_string()),
        };

        service.create_session(request)
            .expect("Failed to create test session")
    }

    /// Creates a test subtask in the store.
    /// Note: This creates a parent session and task first to satisfy foreign key constraints.
    /// Each call creates a unique session to avoid cross-test contamination.
    fn create_test_subtask(
        service: &AgentServiceImpl,
        task_id: &str,
        title: &str,
        order: i32,
    ) -> String {
        use crate::agents::session::SessionId;
        use crate::agents::task::{AgentRequest, SubTaskMode};

        // Create a unique session per call to avoid cross-test contamination
        let session = create_test_session(service, &format!("test-conv-subtask-{}-{}", task_id, title));
        let agents_session_id = SessionId(session.id.0.clone());

        // Create the parent task
        let request = AgentRequest {
            content: "Test request".to_string(),
            attachments: vec![],
            context: None,
        };
        let mut task = crate::agents::task::AgentTask::new(agents_session_id, request);
        task.id = task_id.to_string();
        service
            .session_store()
            .create_task(&task)
            .expect("Failed to create test task");

        // Now create the subtask
        service
            .create_subtask(
                task_id.to_string(),
                title.to_string(),
                Some(format!("Description for {}", title)),
                order,
            )
            .expect("Failed to create test subtask")
    }

    // ========================================================================
    // Session Operation Tests
    // ========================================================================

    #[test]
    fn test_create_session_success() {
        let service = create_test_service();

        let request = CreateSessionRequest {
            agent_id: "code-agent".to_string(),
            conversation_id: "conv-123".to_string(),
            thread_id: "thread-456".to_string(),
            workspace_root: Some("/workspace".to_string()),
        };

        let result = service.create_session(request);

        assert!(result.is_ok());
        let session = result.unwrap();

        assert_eq!(session.agent_id.0, "code-agent");
        assert_eq!(session.conversation_id, "conv-123");
        assert_eq!(session.thread_id, "thread-456");
        assert_eq!(session.workspace_root, Some(std::path::PathBuf::from("/workspace")));
    }

    #[test]
    fn test_create_session_without_workspace() {
        let service = create_test_service();

        let request = CreateSessionRequest {
            agent_id: "code-agent".to_string(),
            conversation_id: "conv-123".to_string(),
            thread_id: "thread-456".to_string(),
            workspace_root: None,
        };

        let result = service.create_session(request);

        assert!(result.is_ok());
        let session = result.unwrap();
        assert_eq!(session.workspace_root, None);
    }

    #[test]
    fn test_get_session_success() {
        let service = create_test_service();

        let created = create_test_session(&service, "conv-1");
        let session_id = created.id.0.clone();

        let result = service.get_session(&session_id);

        assert!(result.is_ok());
        let session = result.unwrap();

        assert_eq!(session.id.0, session_id);
        assert_eq!(session.agent_id.0, "code-agent");
    }

    #[test]
    fn test_get_session_not_found() {
        let service = create_test_service();

        let result = service.get_session("nonexistent-session-id");

        assert!(result.is_err());
        assert!(matches!(result, Err(AgentDomainError::SessionNotFound { .. })));
    }

    #[test]
    fn test_get_conversation_sessions_empty() {
        let service = create_test_service();

        let result = service.get_conversation_sessions("conv-no-sessions");

        assert!(result.is_ok());
        let sessions = result.unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_get_conversation_sessions_multiple() {
        let service = create_test_service();

        // Create multiple sessions for the same conversation
        let _session1 = create_test_session(&service, "conv-multi");
        let _session2 = create_test_session(&service, "conv-multi");

        let result = service.get_conversation_sessions("conv-multi");

        assert!(result.is_ok());
        let sessions = result.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_cancel_session_success() {
        let service = create_test_service();

        let session = create_test_session(&service, "conv-cancel");
        let session_id = session.id.0.clone();

        let result = service.cancel_session(&session_id);

        assert!(result.is_ok());

        // Verify session is in cancelled state
        let fetched = service.get_session(&session_id).unwrap();
        assert!(matches!(fetched.state, crate::core::agent::entities::SessionState::Cancelled { .. }));
    }

    #[test]
    fn test_delete_session_success() {
        let service = create_test_service();

        let session = create_test_session(&service, "conv-delete");
        let session_id = session.id.0.clone();

        // Delete should succeed
        let result = service.delete_session(&session_id);
        assert!(result.is_ok());

        // Session should no longer exist
        let result = service.get_session(&session_id);
        assert!(matches!(result, Err(AgentDomainError::SessionNotFound { .. })));
    }

    #[test]
    fn test_update_session_workspace_success() {
        let service = create_test_service();

        let session = create_test_session(&service, "conv-update");
        let session_id = session.id.0.clone();

        let result = service.update_session_workspace(&session_id, "/new/workspace");

        assert!(result.is_ok());

        // Verify the workspace was updated
        let store = service.session_store();
        let agents_session_id = SessionId(session_id.to_string());
        let updated = store.get_session(&agents_session_id).unwrap().unwrap();
        assert_eq!(updated.workspace_root, Some(std::path::PathBuf::from("/new/workspace")));
    }

    #[test]
    fn test_send_request_validates_session_exists() {
        let service = create_test_service();

        let request = SendAgentRequestRequest {
            session_id: "nonexistent-session".to_string(),
            content: "Hello agent".to_string(),
        };

        let result = service.send_request(request);

        assert!(matches!(result, Err(AgentDomainError::SessionNotFound { .. })));
    }

    #[test]
    fn test_send_request_success_with_valid_session() {
        let service = create_test_service();

        let session = create_test_session(&service, "conv-request");
        let session_id = session.id.0.clone();

        let request = SendAgentRequestRequest {
            session_id: session_id.clone(),
            content: "Hello agent".to_string(),
        };

        let result = service.send_request(request);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, session_id);
    }

    // ========================================================================
    // Registry Operation Tests
    // ========================================================================

    #[test]
    fn test_list_agents_returns_code_agent() {
        let service = create_test_service();

        let result = service.list_agents();

        assert!(result.is_ok());
        let agents = result.unwrap();

        assert!(!agents.is_empty());
        assert!(agents.iter().any(|a| a.id.0 == "code-agent"));
        assert!(agents.iter().any(|a| a.name == "Code Agent"));
    }

    #[test]
    fn test_get_agent_by_id_success() {
        let service = create_test_service();

        let result = service.get_agent("code-agent");

        assert!(result.is_ok());
        let agent = result.unwrap();

        assert_eq!(agent.id.0, "code-agent");
        assert_eq!(agent.name, "Code Agent");
        assert!(!agent.system_prompt.is_empty());
    }

    #[test]
    fn test_get_agent_not_found() {
        let service = create_test_service();

        let result = service.get_agent("nonexistent-agent");

        assert!(matches!(result, Err(AgentDomainError::AgentNotFound { .. })));
    }

    #[test]
    fn test_to_core_agent_converts_trust_mode() {
        let service = create_test_service();

        let agent = Agent {
            id: AgentId::new("test"),
            name: "Test".to_string(),
            description: "Test agent".to_string(),
            system_prompt: "Test".to_string(),
            allowed_tools: vec![],
            default_model_id: "test".to_string(),
            trust_mode: TrustMode::Trusted,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let core_agent: CoreAgent = agent.into();
        assert!(matches!(core_agent.trust_mode, AgentTrustMode::Never));
    }

    #[test]
    fn test_agent_trust_mode_conversion() {
        let service = create_test_service();

        // Test Restricted -> Always
        let restricted_agent = Agent {
            id: AgentId::new("test-restricted"),
            name: "Restricted".to_string(),
            description: "Test".to_string(),
            system_prompt: "Test".to_string(),
            allowed_tools: vec![],
            default_model_id: "test".to_string(),
            trust_mode: TrustMode::Restricted,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let core_agent = service.to_core_agent(restricted_agent);
        assert!(matches!(core_agent.trust_mode, AgentTrustMode::Always));

        // Test Untrusted -> RiskyOnly
        let untrusted_agent = Agent {
            id: AgentId::new("test-untrusted"),
            name: "Untrusted".to_string(),
            description: "Test".to_string(),
            system_prompt: "Test".to_string(),
            allowed_tools: vec![],
            default_model_id: "test".to_string(),
            trust_mode: TrustMode::Untrusted,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let core_agent = service.to_core_agent(untrusted_agent);
        assert!(matches!(core_agent.trust_mode, AgentTrustMode::RiskyOnly));
    }

    // ========================================================================
    // Task Operation Tests
    // ========================================================================

    #[test]
    fn test_create_subtask_success() {
        let service = create_test_service();

        // First create a session (required for task creation)
        let session = create_test_session(&service, "test-conv-subtask-1");

        // Then create the parent task
        use crate::agents::task::AgentRequest;
        // Convert core SessionId to agents SessionId
        let session_id = crate::agents::session::SessionId(session.id.0.clone());
        let request = AgentRequest {
            content: "Test".to_string(),
            attachments: vec![],
            context: None,
        };
        let mut task = crate::agents::task::AgentTask::new(session_id, request);
        task.id = "task-123".to_string(); // Use specific task_id
        service.session_store().create_task(&task)
            .expect("Failed to create parent task");

        let result = service.create_subtask(
            "task-123".to_string(),
            "Test Subtask".to_string(),
            Some("Description".to_string()),
            1,
        );

        assert!(result.is_ok());
        let subtask_id = result.unwrap();
        assert!(!subtask_id.is_empty());
    }

    #[test]
    fn test_create_subtask_without_description() {
        let service = create_test_service();

        // First create a session (required for task creation)
        let session = create_test_session(&service, "test-conv-subtask-2");

        // Then create the parent task
        use crate::agents::task::AgentRequest;
        // Convert core SessionId to agents SessionId
        let session_id = crate::agents::session::SessionId(session.id.0.clone());
        let request = AgentRequest {
            content: "Test".to_string(),
            attachments: vec![],
            context: None,
        };
        let mut task = crate::agents::task::AgentTask::new(session_id, request);
        task.id = "task-456".to_string(); // Use specific task_id
        service.session_store().create_task(&task)
            .expect("Failed to create parent task");

        let result = service.create_subtask(
            "task-456".to_string(),
            "Minimal Subtask".to_string(),
            None,
            0,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_get_subtasks_returns_created() {
        let service = create_test_service();

        // Create session and task explicitly for this test
        let session = create_test_session(&service, "test-get-subtasks");
        use crate::agents::session::SessionId;
        use crate::agents::task::AgentRequest;
        let agents_session_id = SessionId(session.id.0.clone());

        let task_id = "task-get";
        let request = AgentRequest {
            content: "Test request".to_string(),
            attachments: vec![],
            context: None,
        };
        let mut task = crate::agents::task::AgentTask::new(agents_session_id, request);
        task.id = task_id.to_string();
        service.session_store().create_task(&task).unwrap();

        // Create two subtasks
        service.create_subtask(task_id.to_string(), "First".to_string(), None, 0).unwrap();
        service.create_subtask(task_id.to_string(), "Second".to_string(), None, 1).unwrap();

        let result = service.get_subtasks(task_id);

        assert!(result.is_ok());
        let subtasks = result.unwrap();
        assert_eq!(subtasks.len(), 2);
        assert_eq!(subtasks[0].title, "First");
        assert_eq!(subtasks[1].title, "Second");
    }

    #[test]
    fn test_get_subtasks_empty_for_nonexistent_task() {
        let service = create_test_service();

        let result = service.get_subtasks("nonexistent-task");

        assert!(result.is_ok());
        let subtasks = result.unwrap();
        assert!(subtasks.is_empty());
    }

    #[test]
    fn test_update_subtask_status() {
        let service = create_test_service();

        let task_id = "task-update";
        let subtask_id = create_test_subtask(&service, task_id, "Update Me", 0);

        let result = service.update_subtask_status(
            subtask_id.clone(),
            CoreTaskStatus::InProgress,
        );

        assert!(result.is_ok());

        // Verify the status was updated
        let subtasks = service.get_subtasks(task_id).unwrap();
        assert_eq!(subtasks[0].status, CoreTaskStatus::InProgress);
    }

    #[test]
    fn test_task_status_conversion_roundtrip() {
        use super::conversions::TaskStatus;

        // Test all status variants
        let statuses = vec![
            CoreTaskStatus::Pending,
            CoreTaskStatus::InProgress,
            CoreTaskStatus::Completed,
            CoreTaskStatus::Failed,
            CoreTaskStatus::Cancelled,
        ];

        for status in statuses {
            let converted: TaskStatus = status.into();
            let back: CoreTaskStatus = converted.into();
            assert_eq!(status, back);
        }
    }

    // ========================================================================
    // Approval Operation Tests
    // ========================================================================

    #[test]
    fn test_get_pending_actions_empty_initially() {
        let service = create_test_service();

        let session = create_test_session(&service, "conv-approve");
        let session_id = session.id.0.clone();

        let result = service.get_pending_actions(&session_id);

        assert!(result.is_ok());
        let actions = result.unwrap();
        assert!(actions.is_empty());
    }

    #[test]
    fn test_get_pending_actions_returns_queued() {
        let service = create_test_service();

        let session = create_test_session(&service, "conv-pending");
        let session_id = session.id.0.clone();

        // Queue a pending action directly
        let queue = service.approval_queue();
        service.block_on(async {
            let action = PendingAction::new(
                SessionId(session_id.clone()),
                "fs_write_file".to_string(),
                ActionType::WriteFile {
                    path: "/test/path".to_string(),
                    size_bytes: 100,
                },
                "Test action".to_string(),
                serde_json::json!({}),
            );
            queue.queue(action).await;
        });

        let result = service.get_pending_actions(&session_id);

        assert!(result.is_ok());
        let actions = result.unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, "write_file:/test/path");
    }

    #[test]
    fn test_approve_action_success() {
        let service = create_test_service();

        let session = create_test_session(&service, "conv-approve-ok");
        let session_id = session.id.0.clone();

        // Queue an action
        let queue = service.approval_queue();
        let action_id = service.block_on(async {
            let action = PendingAction::new(
                SessionId(session_id.clone()),
                "fs_read_file".to_string(),
                ActionType::ReadFile {
                    path: "/test/read".to_string(),
                },
                "Read test".to_string(),
                serde_json::json!({}),
            );
            queue.queue(action.clone()).await;
            action.id
        });

        let result = service.approve_action(&action_id);

        assert!(result.is_ok());

        // Action should no longer be pending
        let pending = service.get_pending_actions(&session_id).unwrap();
        assert!(pending.is_empty());
    }

    #[test]
    fn test_approve_nonexistent_action_fails() {
        let service = create_test_service();

        let result = service.approve_action("nonexistent-action");

        assert!(matches!(result, Err(AgentDomainError::ActionNotFound { .. })));
    }

    #[test]
    fn test_deny_action_success() {
        let service = create_test_service();

        let session = create_test_session(&service, "conv-deny-ok");
        let session_id = session.id.0.clone();

        // Queue an action
        let queue = service.approval_queue();
        let action_id = service.block_on(async {
            let action = PendingAction::new(
                SessionId(session_id.clone()),
                "fs_delete_file".to_string(),
                ActionType::DeleteFile {
                    path: "/test/delete".to_string(),
                },
                "Delete test".to_string(),
                serde_json::json!({}),
            );
            queue.queue(action.clone()).await;
            action.id
        });

        let result = service.deny_action(&action_id);

        assert!(result.is_ok());

        // Action should no longer be pending
        let pending = service.get_pending_actions(&session_id).unwrap();
        assert!(pending.is_empty());
    }

    #[test]
    fn test_deny_nonexistent_action_fails() {
        let service = create_test_service();

        let result = service.deny_action("nonexistent-action");

        assert!(matches!(result, Err(AgentDomainError::ActionNotFound { .. })));
    }

    // ========================================================================
    // History Operation Tests
    // ========================================================================

    #[test]
    fn test_get_conversation_history_empty() {
        let service = create_test_service();

        let result = service.get_conversation_history("conv-no-history");

        assert!(result.is_ok());
        let history = result.unwrap();
        assert!(history.is_empty());
    }

    #[test]
    fn test_get_conversation_history_excludes_active_sessions() {
        let service = create_test_service();

        // Create an active session (not in terminal state)
        let _session = create_test_session(&service, "conv-active");

        let result = service.get_conversation_history("conv-active");

        assert!(result.is_ok());
        let history = result.unwrap();
        // Active sessions shouldn't be in history
        assert!(history.is_empty());
    }

    #[test]
    fn test_get_session_details_success() {
        let service = create_test_service();

        let session = create_test_session(&service, "conv-details");
        let session_id = session.id.0.clone();

        let result = service.get_session_details(&session_id);

        assert!(result.is_ok());
        let details = result.unwrap();

        assert_eq!(details.session.id.0, session_id);
        assert_eq!(details.files_touched.len(), 0);
        assert_eq!(details.terminal_output.len(), 0);
        assert_eq!(details.execution_steps.len(), 0);
    }

    #[test]
    fn test_get_session_details_not_found() {
        let service = create_test_service();

        let result = service.get_session_details("nonexistent-session");

        assert!(matches!(result, Err(AgentDomainError::SessionNotFound { .. })));
    }

    // ========================================================================
    // Rollback Operation Tests
    // ========================================================================

    #[test]
    fn test_get_file_snapshots_empty_initially() {
        let service = create_test_service();

        let session = create_test_session(&service, "conv-snapshots");
        let session_id = session.id.0.clone();

        let result = service.get_file_snapshots(&session_id);

        assert!(result.is_ok());
        let snapshots = result.unwrap();
        assert!(snapshots.is_empty());
    }

    #[test]
    fn test_rollback_file_not_tracked() {
        let service = create_test_service();

        let session = create_test_session(&service, "conv-rollback");
        let session_id = session.id.0.clone();

        let result = service.rollback_file(&session_id, "/nonexistent/file.txt");

        // Should return an error since file is not tracked
        assert!(result.is_err());
        assert!(matches!(result, Err(AgentDomainError::InvalidInput { .. })));
    }

    #[test]
    fn test_rollback_session_not_found() {
        let service = create_test_service();

        let result = service.rollback_session("nonexistent-session");

        assert!(result.is_err());
        assert!(matches!(result, Err(AgentDomainError::InvalidInput { .. })));
    }

    // ========================================================================
    // Conversion Tests
    // ========================================================================

    #[test]
    fn test_session_state_conversion() {
        // Test WaitForRequest conversion
        let mut agents_session = AgentSession::new(
            AgentId::new("test"),
            "Test Agent".to_string(),
            "conv-1".to_string(),
            "thread-1".to_string(),
            None,
        );
        agents_session.set_state(SessionState::WaitForRequest {
            last_action_at: chrono::Utc::now(),
        });

        let core_session: CoreAgentSession = agents_session.into();
        assert!(matches!(
            core_session.state,
            crate::core::agent::entities::SessionState::WaitForRequest { .. }
        ));
    }

    #[test]
    fn test_session_id_conversion() {
        let test_id = "test-session-id-123";
        let converted = SessionId::from(test_id);
        assert_eq!(converted.0, test_id);
    }

    #[test]
    fn test_agent_id_conversion() {
        let test_id = "test-agent-id-456";
        let converted = AgentId::from(test_id);
        assert_eq!(converted.0, test_id);
    }

    #[test]
    fn test_subtask_to_core_task_conversion() {
        use crate::agents::task::SubTaskMode;

        let subtask = SubTask::new("task-1".to_string(), "Test Task".to_string(), SubTaskMode::FullAccess, 1)
            .with_description("Test description".to_string());

        let core_task: CoreAgentTask = subtask.into();

        assert_eq!(core_task.title, "Test Task");
        assert_eq!(core_task.description, Some("Test description".to_string()));
        assert_eq!(core_task.order, 1);
        assert_eq!(core_task.status, CoreTaskStatus::Pending);
    }

    // ========================================================================
    // Error Handling Tests
    // ========================================================================

    #[test]
    fn test_service_handles_runtime_creation_failure() {
        // This test verifies the service can be created with valid dependencies
        let registry = Arc::new(AgentRegistry::new());
        let session_store = Arc::new(
            SessionStore::new(":memory:").expect("Failed to create session store"),
        );
        let approval_queue = Arc::new(ApprovalQueue::new());
        let tool_registry = Arc::new(
            ToolRegistry::new().expect("Failed to create tool registry"),
        );
        let runtime = Arc::new(AgentRuntime::new(
            session_store.clone(),
            approval_queue.clone(),
            tool_registry,
        ));

        let service = AgentServiceImpl::new(registry, session_store, approval_queue, runtime)
            .expect("Failed to create test service");

        // Verify the service was created and can list agents
        let agents = service.list_agents();
        assert!(agents.is_ok());
        assert!(!agents.unwrap().is_empty());
    }

    #[test]
    fn test_store_error_to_domain_error_conversion() {
        let service = create_test_service();

        let domain_err = service.store_error_to_domain("test error message");

        assert!(matches!(domain_err, AgentDomainError::InvalidInput { .. }));
        assert_eq!(domain_err.to_string(), "Invalid input: test error message");
    }

    // ========================================================================
    // Integration Tests
    // ========================================================================

    #[test]
    fn test_full_session_lifecycle() {
        let service = create_test_service();

        // Create session
        let session = create_test_session(&service, "conv-lifecycle");
        let session_id = session.id.0.clone();

        // Verify session exists
        let fetched = service.get_session(&session_id).unwrap();
        assert_eq!(fetched.id.0, session_id);

        // Update workspace
        service.update_session_workspace(&session_id, "/updated/workspace")
            .expect("Failed to update workspace");

        // Create parent task first (required for subtasks)
        use crate::agents::session::SessionId;
        use crate::agents::task::AgentRequest;
        let agents_session_id = SessionId(session_id.clone());
        let request = AgentRequest {
            content: "Lifecycle task".to_string(),
            attachments: vec![],
            context: None,
        };
        let mut task = crate::agents::task::AgentTask::new(agents_session_id, request);
        task.id = "lifecycle-task".to_string();
        service.session_store().create_task(&task)
            .expect("Failed to create parent task");

        // Create subtasks
        service.create_subtask("lifecycle-task".to_string(), "Step 1".to_string(), None, 0)
            .expect("Failed to create subtask 1");
        service.create_subtask("lifecycle-task".to_string(), "Step 2".to_string(), None, 1)
            .expect("Failed to create subtask 2");

        // Verify subtasks
        let subtasks = service.get_subtasks("lifecycle-task").unwrap();
        assert_eq!(subtasks.len(), 2);

        // Cancel session
        service.cancel_session(&session_id)
            .expect("Failed to cancel session");

        // Verify cancelled state
        let cancelled = service.get_session(&session_id).unwrap();
        assert!(matches!(
            cancelled.state,
            crate::core::agent::entities::SessionState::Cancelled { .. }
        ));

        // Delete session
        service.delete_session(&session_id)
            .expect("Failed to delete session");

        // Verify deletion
        let result = service.get_session(&session_id);
        assert!(matches!(result, Err(AgentDomainError::SessionNotFound { .. })));
    }

    #[test]
    fn test_multiple_conversations_isolated() {
        let service = create_test_service();

        // Create sessions for different conversations
        let session1 = create_test_session(&service, "conv-isolated-1");
        let session2 = create_test_session(&service, "conv-isolated-2");

        // Create parent tasks for each conversation first
        use crate::agents::session::SessionId;
        use crate::agents::task::AgentRequest;

        let task1_id = "task-conv-1";
        let task2_id = "task-conv-2";

        // Create task for conv 1
        let agents_session_id1 = SessionId(session1.id.0.clone());
        let request1 = AgentRequest {
            content: "Task 1".to_string(),
            attachments: vec![],
            context: None,
        };
        let mut task1 = crate::agents::task::AgentTask::new(agents_session_id1, request1);
        task1.id = task1_id.to_string();
        service.session_store().create_task(&task1)
            .expect("Failed to create task 1");

        // Create task for conv 2
        let agents_session_id2 = SessionId(session2.id.0.clone());
        let request2 = AgentRequest {
            content: "Task 2".to_string(),
            attachments: vec![],
            context: None,
        };
        let mut task2 = crate::agents::task::AgentTask::new(agents_session_id2, request2);
        task2.id = task2_id.to_string();
        service.session_store().create_task(&task2)
            .expect("Failed to create task 2");

        // Now create subtasks for each
        service.create_subtask(task1_id.to_string(), "Conv1 Task".to_string(), None, 0)
            .expect("Failed to create subtask for conv 1");
        service.create_subtask(task2_id.to_string(), "Conv2 Task".to_string(), None, 0)
            .expect("Failed to create subtask for conv 2");

        // Verify isolation
        let conv1_sessions = service.get_conversation_sessions("conv-isolated-1").unwrap();
        let conv2_sessions = service.get_conversation_sessions("conv-isolated-2").unwrap();

        assert_eq!(conv1_sessions.len(), 1);
        assert_eq!(conv2_sessions.len(), 1);
        assert_eq!(conv1_sessions[0].id, session1.id);
        assert_eq!(conv2_sessions[0].id, session2.id);
    }
}
