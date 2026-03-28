//! Provides the implementation of the `AgentService`.
//!
//! The service is split into focused submodules so that session lifecycle,
//! runtime execution, approvals, task tracking, and rollback concerns can
//! evolve independently while preserving a single application-layer entry point.

mod approvals;
mod history;
pub mod planner;
mod recovery;
mod registry;
mod retry;
mod rollback;
mod runtime_queries;
mod sessions;
mod tasks;

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::core::agent::entities::{
    Agent as CoreAgent, AgentDomainError, AgentSession as CoreAgentSession,
    AgentTask as CoreAgentTask, PendingAction as CorePendingAction, SessionId as CoreSessionId,
    TaskStatus as CoreTaskStatus,
};
use crate::core::agent::repositories::{
    ExecutionRepository, RepositoryResult, RuntimeTraceRepository, SessionRepository, TaskRepository,
};
use crate::core::agent::services::{
    AgentService, CreateSessionRequest, FileChange, SendAgentRequestRequest, SessionDetails,
};
use crate::core::agent::session::SessionId;
use crate::core::agent::task::TaskStatus;
use crate::core::agent::types::{Agent, AgentId};
use crate::infrastructure::agent::approval::ApprovalQueue;
use crate::infrastructure::agent::file_snapshot_registry::FileSnapshotRegistry;
use crate::infrastructure::agent::registry::AgentRegistry;
use crate::infrastructure::agent::runtime::AgentRuntime;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;

use super::conversions::store_error_to_domain;

/// The concrete implementation of the `AgentService` trait.
pub struct AgentServiceImpl {
    registry: Arc<AgentRegistry>,
    session_repo: Arc<dyn SessionRepository>,
    task_repo: Arc<dyn TaskRepository>,
    telemetry_repo: Arc<dyn AgentTelemetryRepository>,
    execution_repo: Arc<dyn ExecutionRepository>,
    trace_repo: Arc<dyn RuntimeTraceRepository>,
    approval_queue: Arc<ApprovalQueue>,
    runtime: Arc<AgentRuntime>,
    file_snapshot_registry: Arc<FileSnapshotRegistry>,
    active_executions: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    runtime_handle: tokio::runtime::Runtime,
}

impl AgentServiceImpl {
    /// Creates a new `AgentServiceImpl`.
    ///
    /// # Errors
    ///
    /// Returns an error if the Tokio runtime fails to build.
    pub fn new(
        registry: Arc<AgentRegistry>,
        session_repo: Arc<dyn SessionRepository>,
        task_repo: Arc<dyn TaskRepository>,
        telemetry_repo: Arc<dyn AgentTelemetryRepository>,
        execution_repo: Arc<dyn ExecutionRepository>,
        trace_repo: Arc<dyn RuntimeTraceRepository>,
        approval_queue: Arc<ApprovalQueue>,
        runtime: Arc<AgentRuntime>,
    ) -> Result<Self, String> {
        let runtime_handle = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Failed to create runtime: {e}"))?;

        let file_snapshot_registry = runtime.file_snapshot_registry().clone();

        Ok(Self {
            registry,
            session_repo,
            task_repo,
            telemetry_repo,
            execution_repo,
            trace_repo,
            approval_queue,
            runtime,
            file_snapshot_registry,
            active_executions: Arc::new(Mutex::new(HashMap::new())),
            runtime_handle,
        })
    }

    #[cfg(test)]
    pub(crate) fn approval_queue(&self) -> &Arc<ApprovalQueue> {
        &self.approval_queue
    }

    pub(crate) fn runtime(&self) -> &Arc<AgentRuntime> {
        &self.runtime
    }

    pub(crate) async fn is_execution_active(&self, session_id: &str) -> bool {
        let mut active = self.active_executions.lock().await;
        match active.get(session_id) {
            Some(handle) if handle.is_finished() => {
                active.remove(session_id);
                false
            }
            Some(_) => true,
            None => false,
        }
    }

    pub(crate) async fn register_execution(
        &self,
        session_id: String,
        handle: JoinHandle<()>,
    ) -> Option<JoinHandle<()>> {
        let mut active = self.active_executions.lock().await;
        active.insert(session_id, handle)
    }

    pub(crate) async fn abort_execution(&self, session_id: &str) -> bool {
        let mut active = self.active_executions.lock().await;
        if let Some(handle) = active.remove(session_id) {
            handle.abort();
            true
        } else {
            false
        }
    }

    pub(crate) async fn clear_execution(&self, session_id: &str) {
        let mut active = self.active_executions.lock().await;
        active.remove(session_id);
    }

    #[cfg(test)]
    pub(crate) fn telemetry_repo(&self) -> &Arc<dyn AgentTelemetryRepository> {
        &self.telemetry_repo
    }

    pub(crate) fn execution_repo(&self) -> &Arc<dyn ExecutionRepository> {
        &self.execution_repo
    }

    pub(crate) fn trace_repo(&self) -> &Arc<dyn RuntimeTraceRepository> {
        &self.trace_repo
    }

    #[cfg(test)]
    pub(crate) fn task_repo(&self) -> &Arc<dyn TaskRepository> {
        &self.task_repo
    }

    pub(crate) fn to_agents_session_id(&self, session_id: &str) -> SessionId {
        SessionId::from(session_id)
    }

    pub(crate) fn to_agents_agent_id(&self, agent_id: &str) -> AgentId {
        AgentId::from(agent_id)
    }

    pub(crate) fn block_on<F, T>(&self, future: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        // This service exposes a sync API but depends on async internals. If a caller
        // invokes this from a Tokio worker thread, calling `Handle::block_on` would
        // panic ("Cannot start a runtime from within a runtime"). `block_in_place`
        // shifts the blocking section off the runtime worker thread.
        if tokio::runtime::Handle::try_current().is_ok() {
            tokio::task::block_in_place(|| self.runtime_handle.block_on(future))
        } else {
            self.runtime_handle.block_on(future)
        }
    }

    #[cfg(test)]
    pub(crate) fn to_core_agent(&self, agent: Agent) -> CoreAgent {
        agent.into()
    }

    pub fn to_core_task_status(&self, status: CoreTaskStatus) -> TaskStatus {
        status.into()
    }

    pub fn from_core_task_status(status: TaskStatus) -> CoreTaskStatus {
        status.into()
    }

    #[cfg(test)]
    pub(crate) fn repository_error_to_domain<E: std::fmt::Display>(
        &self,
        err: E,
    ) -> AgentDomainError {
        store_error_to_domain(err)
    }
}

impl AgentService for AgentServiceImpl {
    fn create_session(&self, request: CreateSessionRequest) -> RepositoryResult<CoreAgentSession> {
        self.create_session_impl(request)
    }

    fn get_session(&self, session_id: &str) -> RepositoryResult<CoreAgentSession> {
        self.get_session_impl(session_id)
    }

    fn get_conversation_sessions(
        &self,
        conversation_id: &str,
    ) -> RepositoryResult<Vec<CoreAgentSession>> {
        self.get_conversation_sessions_impl(conversation_id)
    }

    fn cancel_session(&self, session_id: &str) -> RepositoryResult<()> {
        self.cancel_session_impl(session_id)
    }

    fn delete_session(&self, session_id: &str) -> RepositoryResult<()> {
        self.delete_session_impl(session_id)
    }

    fn update_session_workspace(
        &self,
        session_id: &str,
        workspace_root: &str,
    ) -> RepositoryResult<()> {
        self.update_session_workspace_impl(session_id, workspace_root)
    }

    fn send_request(&self, request: SendAgentRequestRequest) -> RepositoryResult<CoreSessionId> {
        self.send_request_impl(request)
    }

    fn get_pending_actions(&self, session_id: &str) -> RepositoryResult<Vec<CorePendingAction>> {
        self.get_pending_actions_impl(session_id)
    }

    fn approve_action(&self, action_id: &str) -> RepositoryResult<()> {
        self.approve_action_impl(action_id)
    }

    fn deny_action(&self, action_id: &str) -> RepositoryResult<()> {
        self.deny_action_impl(action_id)
    }

    fn list_agents(&self) -> RepositoryResult<Vec<CoreAgent>> {
        self.list_agents_impl()
    }

    fn get_agent(&self, agent_id: &str) -> RepositoryResult<CoreAgent> {
        self.get_agent_impl(agent_id)
    }

    fn create_subtask(
        &self,
        task_id: String,
        title: String,
        description: Option<String>,
        order: i32,
    ) -> RepositoryResult<String> {
        self.create_subtask_impl(task_id, title, description, order)
    }

    fn update_subtask_status(
        &self,
        subtask_id: String,
        status: CoreTaskStatus,
    ) -> RepositoryResult<()> {
        self.update_subtask_status_impl(subtask_id, status)
    }

    fn get_subtasks(&self, task_id: &str) -> RepositoryResult<Vec<CoreAgentTask>> {
        self.get_subtasks_impl(task_id)
    }

    fn get_conversation_history(
        &self,
        conversation_id: &str,
    ) -> RepositoryResult<Vec<CoreAgentSession>> {
        self.get_conversation_history_impl(conversation_id)
    }

    fn get_session_details(&self, session_id: &str) -> RepositoryResult<SessionDetails> {
        self.get_session_details_impl(session_id)
    }

    fn get_file_snapshots(&self, session_id: &str) -> RepositoryResult<Vec<FileChange>> {
        self.get_file_snapshots_impl(session_id)
    }

    fn rollback_file(&self, session_id: &str, path: &str) -> RepositoryResult<bool> {
        self.rollback_file_impl(session_id, path)
    }

    fn rollback_session(&self, session_id: &str) -> RepositoryResult<Vec<String>> {
        self.rollback_session_impl(session_id)
    }
}
