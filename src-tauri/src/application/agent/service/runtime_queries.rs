use log::{error, info};
use tauri::{AppHandle, Emitter};

use crate::application::agent::conversions::core_task_to_runtime_task;
use crate::application::agent::read_models::{
    AgentCatalog, AgentExecutionContext, AgentExecutionDebugDetails, AgentSessionRuntimeDetails,
    FileTouchedView, PendingActionsView, TerminalLineView, ToolCallView,
};
use crate::core::agent::entities::{AgentDomainError, SessionState as CoreSessionState};
use crate::core::agent::repositories::RepositoryResult;
use crate::core::agent::session::SessionId;
use crate::core::agent::task::AgentRequest;
use crate::core::agent::types::Agent;
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::agent::file_snapshot_registry::FileSnapshot;

use super::store_error_to_domain;
use super::AgentServiceImpl;

impl AgentServiceImpl {
    /// Spawn a new agent execution for a session.
    pub async fn spawn_execution(
        &self,
        session_id: String,
        content: String,
        app_handle: AppHandle,
    ) -> RepositoryResult<String> {
        let core_session = self
            .session_repo
            .get_session(&crate::core::agent::entities::SessionId(session_id.clone()))?;

        if self.is_execution_active(&session_id).await {
            return Err(AgentDomainError::InvalidInput {
                reason: "An agent run is already active for this session".to_string(),
            });
        }

        match &core_session.state {
            CoreSessionState::Processing { .. } | CoreSessionState::AwaitingApproval { .. } => {
                return Err(AgentDomainError::InvalidInput {
                    reason: "This session is already processing a request".to_string(),
                });
            }
            CoreSessionState::Cancelled { .. } => {
                return Err(AgentDomainError::InvalidInput {
                    reason: "This session was cancelled and cannot accept new requests".to_string(),
                });
            }
            CoreSessionState::WaitForRequest { .. }
            | CoreSessionState::Finished { .. }
            | CoreSessionState::Error { .. }
            | CoreSessionState::Paused { .. } => {}
        }

        let sid = SessionId(session_id.clone());
        self.telemetry_repo
            .clear_session_runtime_data(&sid)
            .map_err(store_error_to_domain)?;
        self.approval_queue.clear_session(&session_id).await;

        let execution = self.prepare_execution(&sid).await?;
        let runtime = self.runtime.clone();
        let active_executions = self.active_executions.clone();
        let session_id_for_task = session_id.clone();
        let session_id_for_cleanup = session_id.clone();

        info!("[AgentCommand] Spawning agent execution for session {session_id}");

        let handle = tokio::spawn(async move {
            let result = runtime
                .execute(
                    &execution.session,
                    AgentRequest {
                        content,
                        attachments: vec![],
                        context: None,
                    },
                    &execution.system_prompt,
                    execution.trust_mode,
                    &app_handle,
                )
                .await;

            match &result {
                Ok(execution_result) => {
                    info!(
                        "[AgentCommand] Agent execution completed for session {}, content length: {}",
                        session_id_for_task,
                        execution_result.content.len()
                    );
                }
                Err(crate::infrastructure::agent::runtime::AgentRuntimeError::Cancelled) => {
                    info!(
                        "[AgentCommand] Agent execution cancelled for session {session_id_for_task}"
                    );
                }
                Err(error) => {
                    error!(
                        "[AgentRuntime] Execution failed for session {session_id_for_task}: {error}"
                    );
                    let _ = app_handle.emit(
                        AGENT_EVENT_NAME,
                        AgentEvent::ExecutionError {
                            session_id: session_id_for_task,
                            error: error.to_string(),
                        },
                    );
                }
            }

            let mut active = active_executions.lock().await;
            active.remove(&session_id_for_cleanup);
        });

        if let Some(stale) = self.register_execution(session_id.clone(), handle).await {
            stale.abort();
        }

        Ok(sid.0)
    }

    /// Get pending approval actions for a session.
    pub async fn pending_actions_view(&self, session_id: &str) -> PendingActionsView {
        PendingActionsView {
            actions: self
                .approval_queue
                .get_pending_for_session(session_id)
                .await,
        }
    }

    /// Get the catalog of available agents.
    pub async fn agent_catalog(&self) -> AgentCatalog {
        AgentCatalog {
            agents: self.registry.list().await,
        }
    }

    /// Find a runtime agent by its ID.
    pub async fn find_runtime_agent(&self, agent_id: &str) -> Option<Agent> {
        self.registry.get(&self.to_agents_agent_id(agent_id)).await
    }

    /// Load complete runtime details for a session including tasks, tool calls, and execution steps.
    pub fn load_session_runtime_details(
        &self,
        session_id: &str,
    ) -> RepositoryResult<AgentSessionRuntimeDetails> {
        let sid = SessionId(session_id.to_string());
        let session = self
            .telemetry_repo
            .get_session(&sid)
            .map_err(store_error_to_domain)?
            .ok_or_else(|| AgentDomainError::SessionNotFound {
                id: session_id.to_string(),
            })?;

        let execution_steps = self
            .telemetry_repo
            .get_session_execution_steps(&sid)
            .map_err(store_error_to_domain)?
            .into_iter()
            .map(|step| serde_json::to_value(step).map_err(store_error_to_domain))
            .collect::<Result<Vec<_>, _>>()?;

        // Load tasks and their subtasks
        let core_tasks = self.task_repo.get_session_tasks(&sid.0)?;
        let tasks = core_tasks
            .into_iter()
            .map(|core_task| {
                // Load subtasks for this task
                let subtasks = self
                    .task_repo
                    .get_subtasks(&core_task.id)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|core_subtask| {
                        // Convert core subtask (AgentTask entity) to runtime SubTask
                        crate::infrastructure::agent::runtime::permissions::core_task_to_event_subtask(
                            core_subtask,
                            &core_task.id,
                        )
                    })
                    .collect();

                // Convert core task to runtime task and attach subtasks
                let mut runtime_task = core_task_to_runtime_task(core_task);
                runtime_task.subtasks = Some(subtasks);
                runtime_task
            })
            .collect();

        Ok(AgentSessionRuntimeDetails {
            session,
            files_touched: self
                .telemetry_repo
                .get_session_files(&sid)
                .map_err(store_error_to_domain)?
                .into_iter()
                .map(|file| FileTouchedView {
                    path: file.file_path,
                    modified: file.action != "read",
                    action: match file.action.as_str() {
                        "read" => "read".to_string(),
                        "write" => "write".to_string(),
                        "delete" => "delete".to_string(),
                        _ => "read".to_string(),
                    },
                    diff: file.diff,
                })
                .collect(),
            terminal_output: self
                .telemetry_repo
                .get_session_terminal_output(&sid)
                .map_err(store_error_to_domain)?
                .into_iter()
                .map(|line| TerminalLineView {
                    id: line.id,
                    line_type: line.line_type,
                    content: line.content,
                    timestamp: line.timestamp,
                })
                .collect(),
            execution_steps,
            status_phrases: self
                .telemetry_repo
                .get_session_status_phrases(&sid)
                .map_err(store_error_to_domain)?,
            tasks,
            tool_calls: self
                .telemetry_repo
                .get_tool_calls_for_session(&sid)
                .map_err(store_error_to_domain)?
                .into_iter()
                .map(|tool_call| ToolCallView {
                    id: tool_call.id,
                    session_id: tool_call.session_id,
                    tool_name: tool_call.tool_name,
                    arguments: tool_call.arguments,
                    result: tool_call.result,
                    is_error: tool_call.is_error,
                    duration_ms: tool_call.duration_ms,
                    subtask_id: tool_call.subtask_id,
                    timestamp: tool_call.timestamp,
                })
                .collect(),
        })
    }

    /// Get file snapshots for a session.
    pub fn runtime_session_snapshots(&self, session_id: &str) -> Vec<FileSnapshot> {
        self.file_snapshot_registry
            .get_session_snapshots(session_id)
    }

    /// Get the content of a file snapshot for a session.
    pub fn snapshot_content(&self, session_id: &str, path: &str) -> Option<String> {
        self.file_snapshot_registry
            .get_snapshot_content(session_id, path)
    }

    /// Append a terminal output line for a session.
    pub fn append_terminal_line(
        &self,
        session_id: &str,
        line_type: &str,
        content: &str,
    ) -> RepositoryResult<()> {
        self.telemetry_repo
            .add_terminal_line(&SessionId(session_id.to_string()), line_type, content)
            .map_err(store_error_to_domain)
    }

    /// Load execution debug details including snapshots and runtime traces.
    pub fn load_execution_debug_details(
        &self,
        session_id: &str,
    ) -> RepositoryResult<AgentExecutionDebugDetails> {
        Ok(AgentExecutionDebugDetails {
            snapshot: self.execution_repo.get_execution_snapshot(session_id)?,
            traces: self.trace_repo.get_runtime_traces(session_id, None)?,
        })
    }

    async fn prepare_execution(
        &self,
        session_id: &SessionId,
    ) -> RepositoryResult<AgentExecutionContext> {
        let session = self
            .telemetry_repo
            .get_session(session_id)
            .map_err(store_error_to_domain)?
            .ok_or_else(|| AgentDomainError::SessionNotFound {
                id: session_id.0.clone(),
            })?;

        let agent = self.registry.get(&session.agent_id).await.ok_or_else(|| {
            AgentDomainError::AgentNotFound {
                id: session.agent_id.0.clone(),
            }
        })?;

        Ok(AgentExecutionContext {
            session,
            system_prompt: agent.system_prompt,
            trust_mode: agent.trust_mode,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::application::ChatServiceImpl;
    use crate::core::agent::services::{AgentService, CreateSessionRequest};
    use crate::core::chat::services::ChatService;
    use crate::infrastructure::agent::approval::ApprovalQueue;
    use crate::infrastructure::agent::registry::AgentRegistry;
    use crate::infrastructure::agent::runtime::AgentRuntime;
    use crate::infrastructure::sqlite::repositories::agent::SqliteSessionRepository;
    use crate::infrastructure::sqlite::repositories::chat::SqliteChatRepository;
    use crate::infrastructure::tools::ToolRegistry;

    fn create_test_service() -> AgentServiceImpl {
        let registry = Arc::new(AgentRegistry::new());
        let session_store =
            Arc::new(SqliteSessionRepository::new(":memory:").expect("Failed to create store"));
        let chat_repository =
            Arc::new(SqliteChatRepository::in_memory().expect("Failed to create chat repository"));
        let chat_service = Arc::new(ChatServiceImpl::new(chat_repository)) as Arc<dyn ChatService>;
        let approval_queue = Arc::new(ApprovalQueue::new());
        let tool_registry = Arc::new(ToolRegistry::new().expect("Failed to create tool registry"));
        let runtime = Arc::new(AgentRuntime::new(
            session_store.clone(),
            chat_service,
            session_store.clone(),
            session_store.clone(),
            session_store.clone(),
            session_store.clone(),
            approval_queue.clone(),
            tool_registry,
        ));

        AgentServiceImpl::new(
            registry,
            session_store.clone(),
            session_store.clone(),
            session_store.clone(),
            session_store.clone(),
            session_store,
            approval_queue,
            runtime,
        )
        .expect("Failed to create test service")
    }

    #[test]
    fn load_session_runtime_details_includes_persisted_files_and_tool_calls() {
        let service = create_test_service();
        let session = service
            .create_session(CreateSessionRequest {
                agent_id: "code-agent".to_string(),
                conversation_id: "conv-runtime".to_string(),
                thread_id: "thread-runtime".to_string(),
                workspace_root: Some("/workspace".to_string()),
                model_id: None,
            })
            .expect("failed to create session");
        let session_id = SessionId(session.id.0.clone());

        service
            .telemetry_repo()
            .add_file_touched(
                &session_id,
                "/workspace/src/main.rs",
                "write",
                Some("@@ -1 +1 @@"),
            )
            .expect("failed to store touched file");

        service
            .telemetry_repo()
            .insert_tool_call(
                &session_id,
                "tool-call-1",
                "fs_write_file",
                &serde_json::json!({ "path": "/workspace/src/main.rs" }),
                None,
            )
            .expect("failed to store tool call");

        service
            .telemetry_repo()
            .update_tool_call_result(
                "tool-call-1",
                &serde_json::json!({ "ok": true }),
                false,
                Some(17),
            )
            .expect("failed to store tool call result");

        let details = service
            .load_session_runtime_details(&session.id.0)
            .expect("failed to load session details");

        assert_eq!(details.files_touched.len(), 1);
        assert_eq!(details.files_touched[0].path, "/workspace/src/main.rs");
        assert_eq!(details.files_touched[0].action, "write");

        assert_eq!(details.tool_calls.len(), 1);
        assert_eq!(details.tool_calls[0].tool_name, "fs_write_file");
        assert_eq!(
            details.tool_calls[0].arguments,
            serde_json::json!({ "path": "/workspace/src/main.rs" })
        );
        assert_eq!(
            details.tool_calls[0].result,
            Some(serde_json::json!({ "ok": true }))
        );
        assert_eq!(details.tool_calls[0].duration_ms, Some(17));
        assert!(!details.tool_calls[0].timestamp.is_empty());
    }
}
