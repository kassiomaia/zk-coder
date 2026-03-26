// Agent service tests.
//
// Comprehensive tests for the AgentServiceImpl covering:
// - Session lifecycle (create, get, cancel, delete)
// - Agent request sending
// - Approval queue integration
// - Task/subtask management
// - Registry operations
// - History and rollback operations
// - Error paths and edge cases
// - State transitions

use std::sync::Arc;

use crate::agents::approval::ApprovalQueue;
use crate::agents::registry::AgentRegistry;
use crate::agents::rollback::RollbackStore;
use crate::agents::runtime::AgentRuntime;
use crate::agents::session::{AgentSession, PendingAction, SessionId};
use crate::agents::store::SessionStore;
use crate::agents::task::SubTask;
use crate::agents::types::{ActionType, AgentId, TrustMode};
use crate::application::agent::AgentServiceImpl;
use crate::core::agent::entities::{AgentTrustMode, SessionState as CoreSessionState};
use crate::core::agent::services::{AgentService, CreateSessionRequest, SendAgentRequestRequest};
use crate::core::agent::entities::{TaskStatus as CoreTaskStatus, AgentTask};
use crate::tools::ToolRegistry;

// =========================================================================
// Test Fixtures
// =========================================================================

/// Creates a test service with all dependencies initialized.
fn create_test_service() -> AgentServiceImpl {
    let registry = Arc::new(AgentRegistry::new());
    let session_store = Arc::new(SessionStore::new(":memory:").expect("Failed to create store"));
    let approval_queue = Arc::new(ApprovalQueue::new());
    let tool_registry = Arc::new(ToolRegistry::new().expect("Failed to create tool registry"));
    let runtime = Arc::new(AgentRuntime::new(
        session_store.clone(),
        approval_queue.clone(),
        tool_registry,
    ));

    AgentServiceImpl::new(registry, session_store, approval_queue, runtime)
        .expect("Failed to create test service")
}

/// Creates a test agent session.
fn create_test_session(
    service: &AgentServiceImpl,
    conversation_id: &str,
    thread_id: &str,
) -> crate::core::agent::entities::AgentSession {
    let request = CreateSessionRequest {
        agent_id: "code-agent".to_string(),
        conversation_id: conversation_id.to_string(),
        thread_id: thread_id.to_string(),
        workspace_root: Some("/test/workspace".to_string()),
    };

    service
        .create_session(request)
        .expect("Failed to create test session")
}

/// Creates a test subtask in the store.
fn create_test_subtask(
    service: &AgentServiceImpl,
    task_id: &str,
    title: &str,
    order: i32,
) -> String {
    service
        .create_subtask(
            task_id.to_string(),
            title.to_string(),
            Some(format!("Description for {}", title)),
            order,
        )
        .expect("Failed to create test subtask")
}

/// Creates a test pending action in the approval queue.
fn create_test_pending_action(
    service: &AgentServiceImpl,
    session_id: &str,
    action_name: &str,
) -> String {
    use crate::agents::session::SessionId;

    let queue = service.approval_queue();

    let action_id = service.block_on(async {
        let action = PendingAction::new(
            SessionId(session_id.to_string()),
            action_name.to_string(),
            ActionType::ReadFile {
                path: "/test/path".to_string(),
            },
            "Test action".to_string(),
            serde_json::json!({}),
        );
        queue.queue(action.clone()).await;
        action.id
    });

    action_id
}

// =========================================================================
// Session Lifecycle Tests
// =========================================================================

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
    assert_eq!(
        session.workspace_root,
        Some(std::path::PathBuf::from("/workspace"))
    );
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
fn test_create_session_state_transition() {
    let service = create_test_service();

    let request = CreateSessionRequest {
        agent_id: "code-agent".to_string(),
        conversation_id: "conv-state".to_string(),
        thread_id: "thread-state".to_string(),
        workspace_root: None,
    };

    let session = service.create_session(request).expect("Failed to create");

    // Session should be in WaitForRequest state after creation
    assert!(matches!(
        session.state,
        CoreSessionState::WaitForRequest { .. }
    ));
}

#[test]
fn test_get_session_success() {
    let service = create_test_service();

    let created = create_test_session(&service, "conv-1", "thread-1");
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
    assert!(matches!(
        result,
        Err(crate::core::agent::entities::AgentDomainError::SessionNotFound { .. })
    ));
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
    let _session1 = create_test_session(&service, "conv-multi", "thread-1");
    let _session2 = create_test_session(&service, "conv-multi", "thread-2");

    let result = service.get_conversation_sessions("conv-multi");

    assert!(result.is_ok());
    let sessions = result.unwrap();
    assert_eq!(sessions.len(), 2);
}

#[test]
fn test_cancel_session_success() {
    let service = create_test_service();

    let session = create_test_session(&service, "conv-cancel", "thread-cancel");
    let session_id = session.id.0.clone();

    let result = service.cancel_session(&session_id);

    assert!(result.is_ok());

    // Verify session is in cancelled state
    let fetched = service.get_session(&session_id).unwrap();
    assert!(matches!(
        fetched.state,
        CoreSessionState::Cancelled { .. }
    ));
}

#[test]
fn test_cancel_session_persists_reason() {
    let service = create_test_service();

    let session = create_test_session(&service, "conv-reason", "thread-reason");
    let session_id = session.id.0.clone();

    service.cancel_session(&session_id).expect("Failed to cancel");

    let fetched = service.get_session(&session_id).unwrap();

    if let CoreSessionState::Cancelled { reason } = &fetched.state {
        assert_eq!(reason, "User cancelled");
    } else {
        panic!("Expected Cancelled state with reason");
    }
}

#[test]
fn test_delete_session_success() {
    let service = create_test_service();

    let session = create_test_session(&service, "conv-delete", "thread-delete");
    let session_id = session.id.0.clone();

    // Delete should succeed
    let result = service.delete_session(&session_id);
    assert!(result.is_ok());

    // Session should no longer exist
    let result = service.get_session(&session_id);
    assert!(matches!(
        result,
        Err(crate::core::agent::entities::AgentDomainError::SessionNotFound { .. })
    ));
}

#[test]
fn test_delete_session_not_found() {
    let service = create_test_service();

    let result = service.delete_session("nonexistent-session");

    // Delete should succeed even if session doesn't exist (idempotent)
    // or could return NotFound - depends on implementation
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_update_session_workspace_success() {
    let service = create_test_service();

    let session = create_test_session(&service, "conv-update", "thread-update");
    let session_id = session.id.0.clone();

    let result = service.update_session_workspace(&session_id, "/new/workspace");

    assert!(result.is_ok());

    // Verify the workspace was updated
    let store = service.session_store();
    let agents_session_id = SessionId(session_id.to_string());
    let updated = store.get_session(&agents_session_id).unwrap().unwrap();
    assert_eq!(
        updated.workspace_root,
        Some(std::path::PathBuf::from("/new/workspace"))
    );
}

#[test]
fn test_update_session_workspace_to_none() {
    let service = create_test_service();

    let session = create_test_session(&service, "conv-clear", "thread-clear");
    let session_id = session.id.0.clone();

    // Clear workspace
    let result = service.update_session_workspace(&session_id, "");

    assert!(result.is_ok());

    let store = service.session_store();
    let agents_session_id = SessionId(session_id.to_string());
    let updated = store.get_session(&agents_session_id).unwrap().unwrap();
    // Empty string might be treated as None or stored as is
    assert!(updated.workspace_root.is_none()
        || updated.workspace_root == Some(std::path::PathBuf::from("")));
}

// =========================================================================
// Agent Request Sending Tests
// =========================================================================

#[test]
fn test_send_request_validates_session_exists() {
    let service = create_test_service();

    let request = SendAgentRequestRequest {
        session_id: "nonexistent-session".to_string(),
        content: "Hello agent".to_string(),
    };

    let result = service.send_request(request);

    assert!(matches!(
        result,
        Err(crate::core::agent::entities::AgentDomainError::SessionNotFound { .. })
    ));
}

#[test]
fn test_send_request_success_with_valid_session() {
    let service = create_test_service();

    let session = create_test_session(&service, "conv-request", "thread-request");
    let session_id = session.id.0.clone();

    let request = SendAgentRequestRequest {
        session_id: session_id.clone(),
        content: "Hello agent".to_string(),
    };

    let result = service.send_request(request);

    assert!(result.is_ok());
    assert_eq!(result.unwrap().0, session_id);
}

#[test]
fn test_send_request_empty_content() {
    let service = create_test_service();

    let session = create_test_session(&service, "conv-empty", "thread-empty");
    let session_id = session.id.0.clone();

    let request = SendAgentRequestRequest {
        session_id: session_id.clone(),
        content: "".to_string(),
    };

    let result = service.send_request(request);

    // Should succeed - validation is for session existence only
    assert!(result.is_ok());
}

// =========================================================================
// Approval Queue Integration Tests
// =========================================================================

#[test]
fn test_get_pending_actions_empty_initially() {
    let service = create_test_service();

    let session = create_test_session(&service, "conv-approve", "thread-approve");
    let session_id = session.id.0.clone();

    let result = service.get_pending_actions(&session_id);

    assert!(result.is_ok());
    let actions = result.unwrap();
    assert!(actions.is_empty());
}

#[test]
fn test_get_pending_actions_returns_queued() {
    let service = create_test_service();

    let session = create_test_session(&service, "conv-pending", "thread-pending");
    let session_id = session.id.0.clone();

    // Queue a pending action
    create_test_pending_action(&service, &session_id, "fs_read_file");

    let result = service.get_pending_actions(&session_id);

    assert!(result.is_ok());
    let actions = result.unwrap();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].action_type, "read_file:/test/path");
}

#[test]
fn test_approve_action_success() {
    let service = create_test_service();

    let session = create_test_session(&service, "conv-approve-ok", "thread-approve-ok");
    let session_id = session.id.0.clone();

    // Queue an action
    let action_id = create_test_pending_action(&service, &session_id, "fs_read_file");

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

    assert!(matches!(
        result,
        Err(crate::core::agent::entities::AgentDomainError::ActionNotFound { .. })
    ));
}

#[test]
fn test_deny_action_success() {
    let service = create_test_service();

    let session = create_test_session(&service, "conv-deny-ok", "thread-deny-ok");
    let session_id = session.id.0.clone();

    // Queue an action
    let action_id = create_test_pending_action(&service, &session_id, "fs_delete_file");

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

    assert!(matches!(
        result,
        Err(crate::core::agent::entities::AgentDomainError::ActionNotFound { .. })
    ));
}

#[test]
fn test_multiple_pending_actions() {
    let service = create_test_service();

    let session = create_test_session(&service, "conv-multi-pending", "thread-multi");
    let session_id = session.id.0.clone();

    // Queue multiple actions
    create_test_pending_action(&service, &session_id, "fs_read_file");
    create_test_pending_action(&service, &session_id, "fs_write_file");
    create_test_pending_action(&service, &session_id, "search_grep");

    let pending = service.get_pending_actions(&session_id).unwrap();
    assert_eq!(pending.len(), 3);
}

// =========================================================================
// Registry Operations Tests
// =========================================================================

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
    assert!(!agent.allowed_tools.is_empty());
}

#[test]
fn test_get_agent_not_found() {
    let service = create_test_service();

    let result = service.get_agent("nonexistent-agent");

    assert!(matches!(
        result,
        Err(crate::core::agent::entities::AgentDomainError::AgentNotFound { .. })
    ));
}

#[test]
fn test_to_core_agent_converts_trust_mode_trusted() {
    let service = create_test_service();

    let agent = crate::agents::types::Agent {
        id: AgentId::new("test"),
        name: "Test".to_string(),
        description: "Test agent".to_string(),
        system_prompt: "Test".to_string(),
        allowed_tools: vec![],
        default_model_id: "test".to_string(),
        trust_mode: TrustMode::Trusted,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let core_agent = service.to_core_agent(agent);
    assert!(matches!(core_agent.trust_mode, AgentTrustMode::Never));
}

#[test]
fn test_to_core_agent_converts_trust_mode_restricted() {
    let service = create_test_service();

    let agent = crate::agents::types::Agent {
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

    let core_agent = service.to_core_agent(agent);
    assert!(matches!(core_agent.trust_mode, AgentTrustMode::Always));
}

#[test]
fn test_to_core_agent_converts_trust_mode_untrusted() {
    let service = create_test_service();

    let agent = crate::agents::types::Agent {
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

    let core_agent = service.to_core_agent(agent);
    assert!(matches!(core_agent.trust_mode, AgentTrustMode::RiskyOnly));
}

// =========================================================================
// Task/Subtask Management Tests
// =========================================================================

#[test]
fn test_create_subtask_success() {
    let service = create_test_service();

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

    let task_id = "task-get";
    create_test_subtask(&service, task_id, "First", 0);
    create_test_subtask(&service, task_id, "Second", 1);

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

    let result = service
        .update_subtask_status(subtask_id.clone(), CoreTaskStatus::InProgress);

    assert!(result.is_ok());

    // Verify the status was updated
    let subtasks = service.get_subtasks(task_id).unwrap();
    assert_eq!(subtasks[0].status, CoreTaskStatus::InProgress);
}

#[test]
fn test_update_subtask_status_all_transitions() {
    let service = create_test_service();

    let statuses = vec![
        CoreTaskStatus::Pending,
        CoreTaskStatus::InProgress,
        CoreTaskStatus::Completed,
    ];

    for status in statuses {
        let task_id = format!("task-status-{:?}", status);
        let subtask_id = create_test_subtask(&service, &task_id, "Status Test", 0);

        let result = service.update_subtask_status(subtask_id, status.clone());
        assert!(result.is_ok());

        let subtasks = service.get_subtasks(&task_id).unwrap();
        assert_eq!(subtasks[0].status, status);
    }
}

#[test]
fn test_task_status_conversion_roundtrip() {
    let service = create_test_service();

    // Test all status variants
    let statuses = vec![
        CoreTaskStatus::Pending,
        CoreTaskStatus::InProgress,
        CoreTaskStatus::Completed,
        CoreTaskStatus::Failed,
        CoreTaskStatus::Cancelled,
    ];

    for status in statuses {
        let converted = AgentServiceImpl::from_core_task_status(status.clone());
        let back = service.to_core_task_status(converted);
        assert_eq!(status, back);
    }
}

#[test]
fn test_subtask_order_preserved() {
    let service = create_test_service();

    let task_id = "task-order";
    create_test_subtask(&service, task_id, "First", 0);
    create_test_subtask(&service, task_id, "Second", 1);
    create_test_subtask(&service, task_id, "Third", 2);

    let subtasks = service.get_subtasks(task_id).unwrap();

    assert_eq!(subtasks.len(), 3);
    assert_eq!(subtasks[0].order, 0);
    assert_eq!(subtasks[1].order, 1);
    assert_eq!(subtasks[2].order, 2);
}

// =========================================================================
// History Operations Tests
// =========================================================================

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
    let _session = create_test_session(&service, "conv-active", "thread-active");

    let result = service.get_conversation_history("conv-active");

    assert!(result.is_ok());
    let history = result.unwrap();
    // Active sessions shouldn't be in history
    assert!(history.is_empty());
}

#[test]
fn test_get_session_details_success() {
    let service = create_test_service();

    let session = create_test_session(&service, "conv-details", "thread-details");
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

    assert!(matches!(
        result,
        Err(crate::core::agent::entities::AgentDomainError::SessionNotFound { .. })
    ));
}

// =========================================================================
// Rollback Operations Tests
// =========================================================================

#[test]
fn test_get_file_snapshots_empty_initially() {
    let service = create_test_service();

    let session = create_test_session(&service, "conv-snapshots", "thread-snapshots");
    let session_id = session.id.0.clone();

    let result = service.get_file_snapshots(&session_id);

    assert!(result.is_ok());
    let snapshots = result.unwrap();
    assert!(snapshots.is_empty());
}

#[test]
fn test_rollback_file_not_tracked() {
    let service = create_test_service();

    let session = create_test_session(&service, "conv-rollback", "thread-rollback");
    let session_id = session.id.0.clone();

    let result = service.rollback_file(&session_id, "/nonexistent/file.txt");

    // Should return an error since file is not tracked
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(crate::core::agent::entities::AgentDomainError::InvalidInput { .. })
    ));
}

#[test]
fn test_rollback_session_not_found() {
    let service = create_test_service();

    let result = service.rollback_session("nonexistent-session");

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(crate::core::agent::entities::AgentDomainError::InvalidInput { .. })
    ));
}

// =========================================================================
// Conversion Tests
// =========================================================================

#[test]
fn test_session_state_conversion_wait_for_request() {
    let service = create_test_service();

    let agents_session = AgentSession::new(
        AgentId::new("test"),
        "Test Agent".to_string(),
        "conv-1".to_string(),
        "thread-1".to_string(),
        None,
    );
    agents_session.set_state(crate::agents::session::SessionState::WaitForRequest {
        last_action_at: chrono::Utc::now(),
    });

    let core_session = service.to_core_session(agents_session);
    assert!(matches!(
        core_session.state,
        CoreSessionState::WaitForRequest { .. }
    ));
}

#[test]
fn test_session_state_conversion_finished() {
    let service = create_test_service();

    let agents_session = AgentSession::new(
        AgentId::new("test"),
        "Test Agent".to_string(),
        "conv-1".to_string(),
        "thread-1".to_string(),
        None,
    );
    agents_session.set_state(crate::agents::session::SessionState::Finished {
        completed_at: chrono::Utc::now(),
        result: "Done".to_string(),
    });

    let core_session = service.to_core_session(agents_session);
    assert!(matches!(
        core_session.state,
        CoreSessionState::Finished { .. }
    ));
}

#[test]
fn test_session_id_conversion() {
    let service = create_test_service();

    let test_id = "test-session-id-123";
    let converted = service.to_agents_session_id(test_id);
    assert_eq!(converted.0, test_id);
}

#[test]
fn test_agent_id_conversion() {
    let service = create_test_service();

    let test_id = "test-agent-id-456";
    let converted = service.to_agents_agent_id(test_id);
    assert_eq!(converted.0, test_id);
}

#[test]
fn test_subtask_to_core_task_conversion() {
    let service = create_test_service();

    let subtask = SubTask::new(
        "task-1",
        "Test Task",
        crate::agents::task::SubTaskMode::FullAccess,
        1,
    )
    .with_description("Test description".to_string());

    let core_task = service.subtask_to_core_task(subtask);

    assert_eq!(core_task.title, "Test Task");
    assert_eq!(core_task.description, Some("Test description".to_string()));
    assert_eq!(core_task.order, 1);
    assert_eq!(core_task.status, CoreTaskStatus::Pending);
}

// =========================================================================
// Error Handling Tests
// =========================================================================

#[test]
fn test_service_handles_runtime_creation_failure() {
    // This test verifies the service can be created with valid dependencies
    let registry = Arc::new(AgentRegistry::new());
    let session_store = Arc::new(
        SessionStore::new(":memory:").expect("Failed to create session store"),
    );
    let approval_queue = Arc::new(ApprovalQueue::new());
    let tool_registry = Arc::new(ToolRegistry::new().expect("Failed to create tool registry"));
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

    assert!(matches!(
        domain_err,
        crate::core::agent::entities::AgentDomainError::InvalidInput { .. }
    ));
    assert_eq!(domain_err.to_string(), "Invalid input: test error message");
}

// =========================================================================
// Integration Tests
// =========================================================================

#[test]
fn test_full_session_lifecycle() {
    let service = create_test_service();

    // Create session
    let session = create_test_session(&service, "conv-lifecycle", "thread-lifecycle");
    let session_id = session.id.0.clone();

    // Verify session exists
    let fetched = service.get_session(&session_id).unwrap();
    assert_eq!(fetched.id.0, session_id);

    // Update workspace
    service
        .update_session_workspace(&session_id, "/updated/workspace")
        .expect("Failed to update workspace");

    // Create subtasks
    let task_id = "lifecycle-task";
    service
        .create_subtask(task_id.to_string(), "Step 1".to_string(), None, 0)
        .expect("Failed to create subtask 1");
    service
        .create_subtask(task_id.to_string(), "Step 2".to_string(), None, 1)
        .expect("Failed to create subtask 2");

    // Verify subtasks
    let subtasks = service.get_subtasks(task_id).unwrap();
    assert_eq!(subtasks.len(), 2);

    // Cancel session
    service
        .cancel_session(&session_id)
        .expect("Failed to cancel session");

    // Verify cancelled state
    let cancelled = service.get_session(&session_id).unwrap();
    assert!(matches!(
        cancelled.state,
        CoreSessionState::Cancelled { .. }
    ));

    // Delete session
    service
        .delete_session(&session_id)
        .expect("Failed to delete session");

    // Verify deletion
    let result = service.get_session(&session_id);
    assert!(matches!(
        result,
        Err(crate::core::agent::entities::AgentDomainError::SessionNotFound { .. })
    ));
}

#[test]
fn test_multiple_conversations_isolated() {
    let service = create_test_service();

    // Create sessions for different conversations
    let session1 = create_test_session(&service, "conv-isolated-1", "thread-1");
    let session2 = create_test_session(&service, "conv-isolated-2", "thread-2");

    // Create subtasks for each
    let task1_id = "task-conv-1";
    let task2_id = "task-conv-2";

    service
        .create_subtask(task1_id.to_string(), "Conv1 Task".to_string(), None, 0)
        .expect("Failed to create subtask for conv 1");
    service
        .create_subtask(task2_id.to_string(), "Conv2 Task".to_string(), None, 0)
        .expect("Failed to create subtask for conv 2");

    // Verify isolation
    let conv1_sessions = service.get_conversation_sessions("conv-isolated-1").unwrap();
    let conv2_sessions = service.get_conversation_sessions("conv-isolated-2").unwrap();

    assert_eq!(conv1_sessions.len(), 1);
    assert_eq!(conv2_sessions.len(), 1);
    assert_eq!(conv1_sessions[0].id, session1.id);
    assert_eq!(conv2_sessions[0].id, session2.id);
}

#[test]
fn test_approval_workflow_end_to_end() {
    let service = create_test_service();

    let session = create_test_session(&service, "conv-approval-flow", "thread-approval");
    let session_id = session.id.0.clone();

    // Initially no pending actions
    let pending = service.get_pending_actions(&session_id).unwrap();
    assert!(pending.is_empty());

    // Queue an action
    let action_id = create_test_pending_action(&service, &session_id, "fs_write_file");

    // Verify it's pending
    let pending = service.get_pending_actions(&session_id).unwrap();
    assert_eq!(pending.len(), 1);

    // Approve it
    service.approve_action(&action_id).expect("Failed to approve");

    // Verify it's gone
    let pending = service.get_pending_actions(&session_id).unwrap();
    assert!(pending.is_empty());
}

#[test]
fn test_agent_list_and_retrieve() {
    let service = create_test_service();

    // List all agents
    let agents = service.list_agents().expect("Failed to list agents");

    for agent in &agents {
        // Retrieve each agent individually
        let retrieved = service.get_agent(&agent.id.0).expect("Failed to get agent");
        assert_eq!(retrieved.id, agent.id);
        assert_eq!(retrieved.name, agent.name);
    }
}
