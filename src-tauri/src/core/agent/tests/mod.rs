//! User story integration tests for subtask orchestration.
//!
//! These tests verify complete user workflows end-to-end.

mod common;
mod subtask_advancement;

use crate::core::agent::task::{SubTask, SubTaskMode, TaskStatus};
use crate::infrastructure::sqlite::repositories::agent::SqliteSessionRepository;
use common::{make_test_session, make_test_task};

// ------------------------------------------------------------------------
// User Story: Read mode subtask filters tools
// ------------------------------------------------------------------------

#[test]
fn user_story_read_mode_subtask_filters_tools() {
    // Story: Agent creates read-mode subtask, tools are filtered
    //
    // Steps:
    // 1. Create session
    // 2. Create subtask with Read mode
    // 3. Verify allowed_tools() excludes write tools
    // 4. Verify system prompt includes READ-ONLY instruction

    let store = SqliteSessionRepository::new(":memory:").unwrap();

    // Step 1: Create a session
    let session_id = make_test_session(&store);

    // Create task first (foreign key requirement)
    make_test_task(&store, "task-1", &session_id);

    // Step 2: Create subtask with Read mode
    let subtask = SubTask::new(
        "task-1".to_string(),
        "Explore codebase".to_string(),
        SubTaskMode::Read,
        0,
    );
    store.create_subtask(&subtask).unwrap();

    // Step 3: Verify allowed_tools() excludes write tools
    let allowed_tools = subtask.mode.allowed_tools();
    assert!(
        !allowed_tools.contains(&"fs_write_file"),
        "Read mode should not allow fs_write_file"
    );
    assert!(
        !allowed_tools.contains(&"system_execute"),
        "Read mode should not allow system_execute"
    );
    assert!(
        allowed_tools.contains(&"fs_read_file"),
        "Read mode should allow fs_read_file"
    );

    // Step 4: Verify instruction text includes READ-ONLY
    let instruction = subtask.mode.instruction_text();
    assert!(
        instruction.contains("READ-ONLY"),
        "Instruction should mention READ-ONLY mode"
    );
}

// ------------------------------------------------------------------------
// User Story: Subtask advancement updates status
// ------------------------------------------------------------------------

#[test]
fn user_story_subtask_advancement_updates_status() {
    // Story: Agent advances through multiple subtasks
    //
    // Steps:
    // 1. Create 3 subtasks (all Pending)
    // 2. Set first to InProgress
    // 3. Simulate agent_advance_to_next_subtask
    // 4. Verify: First Completed, second InProgress, third Pending

    let store = SqliteSessionRepository::new(":memory:").unwrap();
    let session_id = make_test_session(&store);

    // Create task first (foreign key requirement)
    make_test_task(&store, "task-1", &session_id);

    // Step 1: Create 3 subtasks
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

    // Step 2: Set first to InProgress
    store
        .update_subtask_status(&subtask1.id, &TaskStatus::InProgress)
        .unwrap();

    // Verify initial state
    let subtasks = store.get_subtasks("task-1").unwrap();
    assert_eq!(subtasks[0].status, TaskStatus::InProgress);
    assert_eq!(subtasks[1].status, TaskStatus::Pending);
    assert_eq!(subtasks[2].status, TaskStatus::Pending);
    assert!(
        subtasks[0].started_at.is_some(),
        "First subtask should have started_at set"
    );

    // Step 3: Simulate agent_advance_to_next_subtask
    // Complete first, start second
    store
        .update_subtask_status(&subtask1.id, &TaskStatus::Completed)
        .unwrap();
    store
        .update_subtask_status(&subtask2.id, &TaskStatus::InProgress)
        .unwrap();

    // Step 4: Verify final state
    let subtasks = store.get_subtasks("task-1").unwrap();
    assert_eq!(
        subtasks[0].status,
        TaskStatus::Completed,
        "First should be Completed"
    );
    assert!(
        subtasks[0].completed_at.is_some(),
        "First should have completed_at set"
    );
    assert_eq!(
        subtasks[1].status,
        TaskStatus::InProgress,
        "Second should be InProgress"
    );
    assert!(
        subtasks[1].started_at.is_some(),
        "Second should have started_at set"
    );
    assert_eq!(
        subtasks[2].status,
        TaskStatus::Pending,
        "Third should still be Pending"
    );
}

// ------------------------------------------------------------------------
// User Story: Tool calls link to current subtask
// ------------------------------------------------------------------------

#[test]
fn user_story_tool_calls_link_to_current_subtask() {
    // Story: Tool calls made during subtask execution are linked to that subtask
    //
    // Steps:
    // 1. Create session and subtask
    // 2. Insert tool call with subtask_id
    // 3. Query tool_calls_for_session
    // 4. Verify subtask_id matches

    let store = SqliteSessionRepository::new(":memory:").unwrap();
    let session_id = make_test_session(&store);

    // Create task first (foreign key requirement)
    make_test_task(&store, "task-1", &session_id);

    // Step 1: Create session and subtask
    let subtask = SubTask::new(
        "task-1".to_string(),
        "Read file".to_string(),
        SubTaskMode::Read,
        0,
    );
    store.create_subtask(&subtask).unwrap();

    // Step 2: Insert tool call with subtask_id
    let tool_call_id = "tool-123";
    store
        .insert_tool_call(
            &session_id,
            tool_call_id,
            "fs_read_file",
            &serde_json::json!({"path": "/src/main.rs"}),
            Some(&subtask.id),
        )
        .unwrap();

    // Step 3: Query tool calls for session
    let tool_calls = store.get_tool_calls_for_session(&session_id).unwrap();

    // Step 4: Verify linkage
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].id, tool_call_id);
    assert_eq!(tool_calls[0].tool_name, "fs_read_file");
    assert_eq!(
        tool_calls[0].subtask_id,
        Some(subtask.id),
        "Tool call should be linked to subtask"
    );

    // Verify we can trace back from tool call to subtask
    let linked_subtask_id = tool_calls[0].subtask_id.as_ref().unwrap();
    let subtasks = store.get_subtasks("task-1").unwrap();
    let linked_subtask = subtasks.iter().find(|s| s.id == *linked_subtask_id);
    assert!(
        linked_subtask.is_some(),
        "Should be able to find linked subtask"
    );
    assert_eq!(linked_subtask.unwrap().title, "Read file");
}

// ------------------------------------------------------------------------
// User Story: Mixed mode subtasks
// ------------------------------------------------------------------------

#[test]
fn user_story_mixed_mode_subtasks() {
    // Story: Agent creates subtasks with different modes for different phases
    //
    // Steps:
    // 1. Create Read mode subtask (exploration)
    // 2. Create Write mode subtask (implementation)
    // 3. Verify each has correct mode and tool permissions

    let store = SqliteSessionRepository::new(":memory:").unwrap();
    let session_id = make_test_session(&store);

    // Create task first (foreign key requirement)
    make_test_task(&store, "task-1", &session_id);

    // Step 1: Create Read mode subtask
    let explore_subtask = SubTask::new(
        "task-1".to_string(),
        "Explore codebase structure".to_string(),
        SubTaskMode::Read,
        0,
    );
    store.create_subtask(&explore_subtask).unwrap();

    // Step 2: Create Write mode subtask
    let fix_subtask = SubTask::new(
        "task-1".to_string(),
        "Fix the bug".to_string(),
        SubTaskMode::Write,
        1,
    );
    store.create_subtask(&fix_subtask).unwrap();

    // Step 3: Verify modes and permissions
    let subtasks = store.get_subtasks("task-1").unwrap();
    assert_eq!(subtasks.len(), 2);

    // Explore subtask (Read mode)
    assert_eq!(subtasks[0].mode, SubTaskMode::Read);
    let explore_tools = subtasks[0].mode.allowed_tools();
    assert!(!explore_tools.contains(&"fs_write_file"));
    assert!(!explore_tools.contains(&"system_execute"));

    // Fix subtask (Write mode)
    assert_eq!(subtasks[1].mode, SubTaskMode::Write);
    let fix_tools = subtasks[1].mode.allowed_tools();
    assert!(fix_tools.contains(&"fs_write_file"));
    assert!(!fix_tools.contains(&"system_execute"));
}

// ------------------------------------------------------------------------
// User Story: Subtask with description
// ------------------------------------------------------------------------

#[test]
fn user_story_subtask_with_description() {
    // Story: Agent creates subtask with detailed description
    //
    // Steps:
    // 1. Create subtask with description
    // 2. Verify description is persisted

    let store = SqliteSessionRepository::new(":memory:").unwrap();
    let session_id = make_test_session(&store);

    // Create task first (foreign key requirement)
    make_test_task(&store, "task-1", &session_id);

    // Step 1: Create subtask with description
    let subtask = SubTask::new(
        "task-1".to_string(),
        "Analyze auth bug".to_string(),
        SubTaskMode::Read,
        0,
    )
    .with_description("Examine the authentication flow in src/auth.rs".to_string());

    store.create_subtask(&subtask).unwrap();

    // Step 2: Verify description persisted
    let fetched = store.get_subtasks("task-1").unwrap();
    assert_eq!(fetched.len(), 1);
    assert_eq!(
        fetched[0].description,
        Some("Examine the authentication flow in src/auth.rs".to_string())
    );
}

// ------------------------------------------------------------------------
// User Story: Tool call result and error tracking
// ------------------------------------------------------------------------

#[test]
fn user_story_tool_call_result_and_error_tracking() {
    // Story: Tool calls track success/failure and duration
    //
    // Steps:
    // 1. Create tool call
    // 2. Update with successful result
    // 3. Verify result, error flag, duration stored

    let store = SqliteSessionRepository::new(":memory:").unwrap();
    let session_id = make_test_session(&store);

    // Step 1: Create tool call
    let tool_call_id = "tool-456";
    store
        .insert_tool_call(
            &session_id,
            tool_call_id,
            "fs_read_file",
            &serde_json::json!({"path": "/test.txt"}),
            None,
        )
        .unwrap();

    // Step 2: Update with result
    let result = serde_json::json!({"content": "Hello, World!", "lines": 1});
    store
        .update_tool_call_result(tool_call_id, &result, false, Some(42))
        .unwrap();

    // Step 3: Verify stored data
    let tool_calls = store.get_tool_calls_for_session(&session_id).unwrap();
    assert_eq!(tool_calls.len(), 1);

    let tc = &tool_calls[0];
    assert_eq!(tc.result, Some(result));
    assert_eq!(tc.is_error, false);
    assert_eq!(tc.duration_ms, Some(42));
}

// ------------------------------------------------------------------------
// User Story: Multiple tool calls per subtask
// ------------------------------------------------------------------------

#[test]
fn user_story_multiple_tool_calls_per_subtask() {
    // Story: Subtask has multiple tool calls during execution
    //
    // Steps:
    // 1. Create subtask
    // 2. Make multiple tool calls with same subtask_id
    // 3. Verify all linked correctly

    let store = SqliteSessionRepository::new(":memory:").unwrap();
    let session_id = make_test_session(&store);

    // Create task first (foreign key requirement)
    make_test_task(&store, "task-1", &session_id);

    // Step 1: Create subtask
    let subtask = SubTask::new(
        "task-1".to_string(),
        "Read multiple files".to_string(),
        SubTaskMode::Read,
        0,
    );
    store.create_subtask(&subtask).unwrap();

    // Step 2: Make multiple tool calls
    store
        .insert_tool_call(
            &session_id,
            "tool-1",
            "fs_read_file",
            &serde_json::json!({"path": "/file1.txt"}),
            Some(&subtask.id),
        )
        .unwrap();

    store
        .insert_tool_call(
            &session_id,
            "tool-2",
            "fs_read_file",
            &serde_json::json!({"path": "/file2.txt"}),
            Some(&subtask.id),
        )
        .unwrap();

    store
        .insert_tool_call(
            &session_id,
            "tool-3",
            "search_grep",
            &serde_json::json!({"query": "TODO"}),
            Some(&subtask.id),
        )
        .unwrap();

    // Step 3: Verify all linked to same subtask
    let tool_calls = store.get_tool_calls_for_session(&session_id).unwrap();
    assert_eq!(tool_calls.len(), 3);

    for tc in &tool_calls {
        assert_eq!(
            tc.subtask_id,
            Some(subtask.id.clone()),
            "All tool calls should link to the subtask"
        );
    }

    // Verify chronological order
    assert_eq!(tool_calls[0].id, "tool-1");
    assert_eq!(tool_calls[1].id, "tool-2");
    assert_eq!(tool_calls[2].id, "tool-3");
}
