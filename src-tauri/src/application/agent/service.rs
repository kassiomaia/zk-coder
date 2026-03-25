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
use crate::agents::types::{Agent, AgentId, TrustMode};

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
    ) -> Self {
        let runtime_handle = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime");

        let rollback_store = runtime.rollback_store().clone();

        Self {
            registry,
            session_store,
            approval_queue,
            runtime,
            rollback_store,
            runtime_handle,
        }
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

    /// Convert agents::types::Agent to core::agent::entities::Agent.
    fn to_core_agent(&self, agent: Agent) -> CoreAgent {
        use crate::core::agent::entities::AgentTrustMode;

        CoreAgent {
            id: crate::core::agent::entities::AgentId(agent.id.0),
            name: agent.name,
            description: agent.description,
            system_prompt: agent.system_prompt,
            trust_mode: match agent.trust_mode {
                TrustMode::Trusted => AgentTrustMode::Never,
                TrustMode::Untrusted => AgentTrustMode::RiskyOnly,
                TrustMode::Restricted => AgentTrustMode::Always,
            },
        }
    }

    /// Convert agents::session::AgentSession to core::agent::entities::AgentSession.
    fn to_core_session(&self, session: AgentSession) -> CoreAgentSession {
        use crate::core::agent::entities::SessionState as CoreSessionState;

        CoreAgentSession {
            id: crate::core::agent::entities::SessionId(session.id.0),
            agent_id: crate::core::agent::entities::AgentId(session.agent_id.0),
            agent_name: session.agent_name,
            conversation_id: session.conversation_id,
            thread_id: session.thread_id,
            workspace_root: session.workspace_root,
            state: match session.state {
                SessionState::Launching => CoreSessionState::WaitForRequest {
                    last_action_at: Utc::now(),
                },
                SessionState::Running { .. } => CoreSessionState::Processing {
                    started_at: Utc::now(),
                },
                SessionState::WaitForRequest { last_action_at } => {
                    CoreSessionState::WaitForRequest { last_action_at }
                }
                SessionState::UserInputRequired { .. } => CoreSessionState::AwaitingApproval {
                    action_id: String::new(),
                    requested_at: Utc::now(),
                },
                SessionState::Finished { completed_at } => CoreSessionState::Finished {
                    completed_at,
                    result: String::new(),
                },
                SessionState::Cancelled { reason } => CoreSessionState::Cancelled {
                    cancelled_at: Utc::now(),
                    reason,
                },
                SessionState::Error { message, .. } => CoreSessionState::Error {
                    error_at: Utc::now(),
                    error: message,
                },
            },
            created_at: session.created_at,
        }
    }

    /// Convert agents::session::PendingAction to core::agent::entities::PendingAction.
    fn to_core_pending_action(&self, action: PendingAction) -> CorePendingAction {
        use crate::agents::types::ActionType;
        use crate::core::agent::entities::ApprovalStatus;

        CorePendingAction {
            id: action.id,
            session_id: action.session_id.0,
            action_type: match &action.action_type {
                ActionType::ReadFile { path } => format!("read_file:{}", path),
                ActionType::WriteFile { path, .. } => format!("write_file:{}", path),
                ActionType::DeleteFile { path } => format!("delete_file:{}", path),
                ActionType::ExecuteCommand { command } => format!("execute:{}", command),
                ActionType::WebRequest { url } => format!("web_request:{}", url),
            },
            description: action.description,
            parameters: action.arguments,
            requested_at: action.created_at,
            status: ApprovalStatus::Pending,
        }
    }

    /// Convert agents::task::SubTask to core::agent::entities::AgentTask.
    fn subtask_to_core_task(&self, subtask: SubTask) -> CoreAgentTask {
        CoreAgentTask {
            id: subtask.id,
            session_id: String::new(), // Subtasks don't have session_id directly
            title: subtask.title,
            description: subtask.description,
            status: self.to_core_task_status(subtask.status),
            order: subtask.order,
        }
    }

    /// Convert task status from agents to core.
    fn to_core_task_status(&self, status: TaskStatus) -> CoreTaskStatus {
        match status {
            TaskStatus::Pending => CoreTaskStatus::Pending,
            TaskStatus::InProgress => CoreTaskStatus::InProgress,
            TaskStatus::WaitingApproval => CoreTaskStatus::Pending,
            TaskStatus::Completed => CoreTaskStatus::Completed,
            TaskStatus::Failed => CoreTaskStatus::Failed,
            TaskStatus::Cancelled => CoreTaskStatus::Cancelled,
        }
    }

    /// Convert task status from core to agents.
    fn from_core_task_status(status: CoreTaskStatus) -> TaskStatus {
        match status {
            CoreTaskStatus::Pending => TaskStatus::Pending,
            CoreTaskStatus::InProgress => TaskStatus::InProgress,
            CoreTaskStatus::Completed => TaskStatus::Completed,
            CoreTaskStatus::Failed => TaskStatus::Failed,
            CoreTaskStatus::Cancelled => TaskStatus::Cancelled,
        }
    }

    /// Convert SessionId from core to agents.
    fn to_agents_session_id(&self, session_id: &str) -> SessionId {
        SessionId(session_id.to_string())
    }

    /// Convert AgentId from core to agents.
    fn to_agents_agent_id(&self, agent_id: &str) -> AgentId {
        AgentId(agent_id.to_string())
    }

    /// Convert store error to domain error.
    fn store_error_to_domain(&self, err: impl std::fmt::Display) -> AgentDomainError {
        AgentDomainError::InvalidInput {
            reason: err.to_string(),
        }
    }

    /// Block on async future using the runtime handle.
    fn block_on<F, T>(&self, future: F) -> T
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
            .map_err(|e| self.store_error_to_domain(e))?;

        // Transition to WaitForRequest state
        let new_state = SessionState::WaitForRequest {
            last_action_at: Utc::now(),
        };
        self.session_store
            .update_state(&session.id, &new_state)
            .map_err(|e| self.store_error_to_domain(e))?;

        // Fetch and return the updated session
        let updated_session = self
            .session_store
            .get_session(&session.id)
            .map_err(|e| self.store_error_to_domain(e))?
            .ok_or_else(|| AgentDomainError::SessionNotFound {
                id: session.id.0.clone(),
            })?;

        Ok(self.to_core_session(updated_session))
    }

    fn get_session(&self, session_id: &str) -> RepositoryResult<CoreAgentSession> {
        let sid = self.to_agents_session_id(session_id);

        let session = self
            .session_store
            .get_session(&sid)
            .map_err(|e| self.store_error_to_domain(e))?
            .ok_or_else(|| AgentDomainError::SessionNotFound {
                id: session_id.to_string(),
            })?;

        Ok(self.to_core_session(session))
    }

    fn get_conversation_sessions(
        &self,
        conversation_id: &str,
    ) -> RepositoryResult<Vec<CoreAgentSession>> {
        let sessions = self
            .session_store
            .get_sessions_for_conversation(conversation_id)
            .map_err(|e| self.store_error_to_domain(e))?;

        Ok(sessions
            .into_iter()
            .map(|s| self.to_core_session(s))
            .collect())
    }

    fn cancel_session(&self, session_id: &str) -> RepositoryResult<()> {
        let sid = self.to_agents_session_id(session_id);

        let new_state = SessionState::Cancelled {
            reason: "User cancelled".to_string(),
        };

        self.session_store
            .update_state(&sid, &new_state)
            .map_err(|e| self.store_error_to_domain(e))?;

        // Note: Clearing pending approvals is handled by the command layer
        // since it requires async which we can't do here directly
        Ok(())
    }

    fn delete_session(&self, session_id: &str) -> RepositoryResult<()> {
        let sid = self.to_agents_session_id(session_id);

        self.session_store
            .delete_session(&sid)
            .map_err(|e| self.store_error_to_domain(e))?;

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
            .map_err(|e| self.store_error_to_domain(e))?;

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
            .map_err(|e| self.store_error_to_domain(e))?
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
            .map(|a| self.to_core_pending_action(a))
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
            .map(|a| self.to_core_agent(a))
            .collect())
    }

    fn get_agent(&self, agent_id: &str) -> RepositoryResult<CoreAgent> {
        let aid = self.to_agents_agent_id(agent_id);

        let agent = self
            .block_on(self.registry.get(&aid))
            .ok_or_else(|| AgentDomainError::AgentNotFound {
                id: agent_id.to_string(),
            })?;

        Ok(self.to_core_agent(agent))
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
            .map_err(|e| self.store_error_to_domain(e))?;

        Ok(subtask_id)
    }

    fn update_subtask_status(
        &self,
        subtask_id: String,
        status: CoreTaskStatus,
    ) -> RepositoryResult<()> {
        let task_status = Self::from_core_task_status(status);

        self.session_store
            .update_subtask_status(&subtask_id, &task_status)
            .map_err(|e| self.store_error_to_domain(e))?;

        Ok(())
    }

    fn get_subtasks(&self, task_id: &str) -> RepositoryResult<Vec<CoreAgentTask>> {
        let subtasks = self
            .session_store
            .get_subtasks(task_id)
            .map_err(|e| self.store_error_to_domain(e))?;

        Ok(subtasks
            .into_iter()
            .map(|st| self.subtask_to_core_task(st))
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
            .map_err(|e| self.store_error_to_domain(e))?;

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
            .map(|s| self.to_core_session(s))
            .collect())
    }

    fn get_session_details(&self, session_id: &str) -> RepositoryResult<SessionDetails> {
        let sid = self.to_agents_session_id(session_id);

        let session = self
            .session_store
            .get_session(&sid)
            .map_err(|e| self.store_error_to_domain(e))?
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
            session: self.to_core_session(session),
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
    use crate::tools::ToolRegistry;

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

        let core_agent = service.to_core_agent(agent);

        use crate::core::agent::entities::AgentTrustMode;
        assert!(matches!(core_agent.trust_mode, AgentTrustMode::Never));
    }

    #[test]
    fn test_create_and_get_session() {
        let service = create_test_service();

        let request = CreateSessionRequest {
            agent_id: "code-agent".to_string(),
            conversation_id: "conv-1".to_string(),
            thread_id: "thread-1".to_string(),
            workspace_root: Some("/workspace".to_string()),
        };

        let session = service
            .create_session(request)
            .expect("Failed to create session");

        assert_eq!(session.agent_id.0, "code-agent");
        assert_eq!(session.conversation_id, "conv-1");
        assert_eq!(session.thread_id, "thread-1");

        // Get the session
        let fetched = service
            .get_session(&session.id.0)
            .expect("Failed to get session");

        assert_eq!(fetched.id, session.id);
    }

    #[test]
    fn test_list_agents() {
        let service = create_test_service();

        let agents = service.list_agents().expect("Failed to list agents");

        assert!(!agents.is_empty());
        assert!(agents.iter().any(|a| a.id.0 == "code-agent"));
    }
}
