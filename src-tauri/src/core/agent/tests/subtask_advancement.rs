//! End-to-end test for subtask advancement through agent tools.

use crate::core::agent::task::{SubTask, SubTaskMode, TaskStatus};
use crate::infrastructure::sqlite::repositories::agent::SqliteSessionRepository;
use crate::infrastructure::tools::agent::AgentCategory;
use crate::infrastructure::tools::ToolCategory;

/// Verify the advance_to_next_subtask tool is registered and available.
#[test]
fn test_advance_to_next_subtask_tool_exists() {
    let category = AgentCategory;
    let tools = category.tools();

    let advance_tool = tools
        .iter()
        .find(|t| t.meta().id.to_string() == "agent_advance_to_next_subtask");

    assert!(
        advance_tool.is_some(),
        "agent_advance_to_next_subtask tool should exist"
    );

    let tool = advance_tool.unwrap();
    let meta = tool.meta();

    assert!(meta.description.contains("completed"));
    assert!(meta.description.contains("next subtask"));
}

/// Verify the mark_subtask_status tool is registered and available.
#[test]
fn test_mark_subtask_status_tool_exists() {
    let category = AgentCategory;
    let tools = category.tools();

    let mark_tool = tools
        .iter()
        .find(|t| t.meta().id.to_string() == "agent_mark_subtask_status");

    assert!(
        mark_tool.is_some(),
        "agent_mark_subtask_status tool should exist"
    );

    let tool = mark_tool.unwrap();
    let meta = tool.meta();

    assert!(meta.description.contains("status"));
    assert!(meta.description.contains("index"));
}

/// Simulate the complete advancement flow:
/// 1. Create subtasks
/// 2. Mark first as InProgress
/// 3. Simulate agent_advance_to_next_subtask call
/// 4. Verify: First Completed, second InProgress
#[test]
fn test_subtask_advancement_flow() {
    let store = SqliteSessionRepository::new(":memory:").unwrap();
    let session_id = crate::core::agent::tests::common::make_test_session(&store);

    // Create task first (foreign key requirement)
    crate::core::agent::tests::common::make_test_task(&store, "task-1", &session_id);

    // Create 3 subtasks
    let subtask1 = SubTask::new(
        "task-1".to_string(),
        "Explore".to_string(),
        SubTaskMode::Read,
        0,
    );
    let subtask2 = SubTask::new(
        "task-1".to_string(),
        "Fix".to_string(),
        SubTaskMode::Write,
        1,
    );
    let subtask3 = SubTask::new(
        "task-1".to_string(),
        "Test".to_string(),
        SubTaskMode::Execute,
        2,
    );

    store.create_subtask(&subtask1).unwrap();
    store.create_subtask(&subtask2).unwrap();
    store.create_subtask(&subtask3).unwrap();

    // Mark first as InProgress
    store
        .update_subtask_status(&subtask1.id, &TaskStatus::InProgress)
        .unwrap();

    // Verify initial state
    let subtasks = store.get_subtasks("task-1").unwrap();
    assert_eq!(subtasks[0].status, TaskStatus::InProgress);
    assert_eq!(subtasks[1].status, TaskStatus::Pending);
    assert_eq!(subtasks[2].status, TaskStatus::Pending);

    // Simulate agent_advance_to_next_subtask behavior
    // (this is what the runtime does when it intercepts the tool call)
    store
        .update_subtask_status(&subtask1.id, &TaskStatus::Completed)
        .unwrap();
    store
        .update_subtask_status(&subtask2.id, &TaskStatus::InProgress)
        .unwrap();

    // Verify advancement
    let subtasks = store.get_subtasks("task-1").unwrap();
    assert_eq!(subtasks[0].status, TaskStatus::Completed);
    assert_eq!(subtasks[1].status, TaskStatus::InProgress);
    assert_eq!(subtasks[2].status, TaskStatus::Pending);
}

/// Verify that advancing from the last subtask doesn't fail.
#[test]
fn test_all_subtasks_completion() {
    let store = SqliteSessionRepository::new(":memory:").unwrap();
    let session_id = crate::core::agent::tests::common::make_test_session(&store);

    crate::core::agent::tests::common::make_test_task(&store, "task-1", &session_id);

    // Create 2 subtasks
    let subtask1 = SubTask::new(
        "task-1".to_string(),
        "First".to_string(),
        SubTaskMode::Read,
        0,
    );
    let subtask2 = SubTask::new(
        "task-1".to_string(),
        "Last".to_string(),
        SubTaskMode::Write,
        1,
    );

    store.create_subtask(&subtask1).unwrap();
    store.create_subtask(&subtask2).unwrap();

    // Mark first as InProgress
    store
        .update_subtask_status(&subtask1.id, &TaskStatus::InProgress)
        .unwrap();

    // Advance to second
    store
        .update_subtask_status(&subtask1.id, &TaskStatus::Completed)
        .unwrap();
    store
        .update_subtask_status(&subtask2.id, &TaskStatus::InProgress)
        .unwrap();

    // Complete second (last subtask)
    store
        .update_subtask_status(&subtask2.id, &TaskStatus::Completed)
        .unwrap();

    // Verify all completed
    let subtasks = store.get_subtasks("task-1").unwrap();
    assert_eq!(subtasks[0].status, TaskStatus::Completed);
    assert_eq!(subtasks[1].status, TaskStatus::Completed);
}

/// Test that tool definitions include the new tools with correct IDs.
#[test]
fn test_tool_definitions_have_correct_ids() {
    use crate::infrastructure::tools::ToolRegistry;

    let registry = ToolRegistry::new().expect("ToolRegistry should initialize");
    let definitions = registry.definitions();

    // Find agent tools
    let agent_tools: Vec<_> = definitions
        .iter()
        .filter(|t| t.name.starts_with("agent_"))
        .collect();

    assert!(
        agent_tools
            .iter()
            .any(|t| t.name == "agent_advance_to_next_subtask"),
        "agent_advance_to_next_subtask should be in tool definitions"
    );
    assert!(
        agent_tools
            .iter()
            .any(|t| t.name == "agent_mark_subtask_status"),
        "agent_mark_subtask_status should be in tool definitions"
    );
}

/// Test that subtask modes permit the advance tool.
///
/// Restricted modes (Read / Write / Execute) must enumerate
/// `agent_advance_to_next_subtask` in their allowlist so the agent can
/// progress through subtasks without hitting `subtask_mode_disallowed`.
/// FullAccess carries no allowlist at all (returns `None`), so every
/// registered tool — including `agent_advance_to_next_subtask` — is
/// automatically allowed.
#[test]
fn test_subtask_modes_include_advance_tool() {
    let read_tools = SubTaskMode::Read
        .allowed_tools()
        .expect("Read mode must enforce an allowlist");
    let write_tools = SubTaskMode::Write
        .allowed_tools()
        .expect("Write mode must enforce an allowlist");
    let execute_tools = SubTaskMode::Execute
        .allowed_tools()
        .expect("Execute mode must enforce an allowlist");

    assert!(read_tools.contains(&"agent_advance_to_next_subtask"));
    assert!(write_tools.contains(&"agent_advance_to_next_subtask"));
    assert!(execute_tools.contains(&"agent_advance_to_next_subtask"));
    assert_eq!(
        SubTaskMode::FullAccess.allowed_tools(),
        None,
        "FullAccess must be unrestricted so agent_advance_to_next_subtask \
         (and every other registered tool) is automatically allowed"
    );
}

/// Test that subtask statuses can be updated via mark_subtask_status.
#[test]
fn test_mark_subtask_status_updates() {
    let store = SqliteSessionRepository::new(":memory:").unwrap();
    let session_id = crate::core::agent::tests::common::make_test_session(&store);

    crate::core::agent::tests::common::make_test_task(&store, "task-1", &session_id);

    // Create a subtask
    let subtask1 = SubTask::new(
        "task-1".to_string(),
        "First".to_string(),
        SubTaskMode::Read,
        0,
    );
    store.create_subtask(&subtask1).unwrap();

    // Mark as failed (simulating agent_mark_subtask_status)
    store
        .update_subtask_status(&subtask1.id, &TaskStatus::Failed)
        .unwrap();

    // Verify status changed
    let subtasks = store.get_subtasks("task-1").unwrap();
    assert_eq!(subtasks[0].status, TaskStatus::Failed);
}
