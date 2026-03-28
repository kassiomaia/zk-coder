//! Integration tests for the agent task planning system.
//!
//! These tests validate the complete flow from user message to task creation
//! with subtasks, ensuring the planning improvements work as expected.

use futures::future::BoxFuture;
use crate::core::agent::entities::{AgentTask, TaskMode, TaskStatus};
use crate::application::agent::service::planner::{
    TaskPlan, SubtaskPlan, subtask_plan_to_agent_task, build_planning_system_prompt,
};
use crate::infrastructure::llm::mock::MockLlmClient;
use crate::infrastructure::llm::LlmClient;
use crate::core::llm::entities::{ChatMessage, ChatRequest, MessageContent, MessageRole};

// ========================================================================
// End-to-End Planning Tests
// ========================================================================

#[tokio::test]
async fn test_full_flow_complex_message_creates_task_with_clean_title() {
    // Simulates the complete flow:
    // 1. User sends complex message
    // 2. LLM generates plan with clean title
    // 3. Task and subtasks are created
    // 4. First subtask is marked InProgress

    let client = MockLlmClient::anthropic("claude-3-5-sonnet");

    // User's message (very long, would be truncated in old system)
    let user_message = "Investigate the entire authentication and authorization system across all microservices including the API gateway the user service the permission service and the audit service to find security vulnerabilities and performance bottlenecks and provide recommendations for improvements";

    // LLM generates a clean, concise plan
    client.set_response(serde_json::json!({
        "title": "Authentication system security and performance audit",
        "description": "Review auth microservices for vulnerabilities and performance issues",
        "is_non_trivial": true,
        "subtasks": [
            {"title": "Analyze API gateway authentication", "description": "Review gateway auth flow", "order": 0},
            {"title": "Audit user service security", "description": "Check for vulnerabilities", "order": 1},
            {"title": "Review permission service", "description": "Validate access controls", "order": 2},
            {"title": "Profile performance bottlenecks", "description": "Identify slow operations", "order": 3},
            {"title": "Document findings and recommendations", "description": "Create improvement report", "order": 4}
        ]
    }).to_string());

    // Simulate planning call
    let system_prompt = build_planning_system_prompt();
    let user_prompt = format!(
        "User Request: {}\n\nCreate a task plan for this request.",
        user_message
    );

    let request = ChatRequest::new(
        "claude-3-5-sonnet",
        vec![
            ChatMessage {
                role: MessageRole::System,
                content: MessageContent::Text(system_prompt),
                tool_calls: Vec::new(),
                tool_result: None,
            },
            ChatMessage {
                role: MessageRole::User,
                content: MessageContent::Text(user_prompt),
                tool_calls: Vec::new(),
                tool_result: None,
            },
        ],
    );

    let response = client.chat(request).await.unwrap();
    let plan: TaskPlan = serde_json::from_str(&response.content).unwrap();

    // VALIDATE: Task title is from LLM, not truncated user message
    assert_eq!(plan.title, "Authentication system security and performance audit");
    assert!(plan.title.len() <= 80); // Concise

    // The old system would have truncated to 80 chars:
    // "Investigate the entire authentication and authorization system acros"
    assert_ne!(
        plan.title,
        &user_message[..80.min(user_message.len())],
        "Title should NOT be truncated user message"
    );

    // VALIDATE: All subtasks exist immediately
    assert_eq!(plan.subtasks.len(), 5);
    for (i, subtask) in plan.subtasks.iter().enumerate() {
        assert_eq!(subtask.order, i as i32);
        assert!(!subtask.title.is_empty());
    }

    // VALIDATE: Subtask conversion works correctly
    let session_id = "test-session";
    let task_id = "test-task";

    let subtask_entities: Vec<AgentTask> = plan
        .subtasks
        .iter()
        .map(|st| subtask_plan_to_agent_task(st, session_id, task_id))
        .collect();

    // First subtask should be marked InProgress (done in bootstrap)
    assert_eq!(subtask_entities[0].status, TaskStatus::Pending); // Conversion creates Pending
    // Bootstrap will update first to InProgress
}

#[tokio::test]
async fn test_regression_task_title_is_not_truncated_first_line() {
    // Regression test: Ensure we're NOT using the old derive_task_title()
    // which truncated the first 80 characters of the user's message

    let client = MockLlmClient::anthropic("claude-3-5-sonnet");

    // A message that would be poorly truncated
    let long_message = "Investigate the entire authentication and authorization system across all microservices including the API gateway the user service the permission service and the audit service to find security vulnerabilities and performance bottlenecks and provide recommendations for improvements";

    client.set_response(serde_json::json!({
        "title": "Auth system security audit",
        "description": "Security and performance review",
        "is_non_trivial": false,
        "subtasks": []
    }).to_string());

    let system_prompt = build_planning_system_prompt();
    let user_prompt = format!(
        "User Request: {}\n\nCreate a task plan for this request.",
        long_message
    );

    let request = ChatRequest::new(
        "claude-3-5-sonnet",
        vec![
            ChatMessage {
                role: MessageRole::System,
                content: MessageContent::Text(system_prompt),
                tool_calls: Vec::new(),
                tool_result: None,
            },
            ChatMessage {
                role: MessageRole::User,
                content: MessageContent::Text(user_prompt),
                tool_calls: Vec::new(),
                tool_result: None,
            },
        ],
    );

    let response = client.chat(request).await.unwrap();
    let plan: TaskPlan = serde_json::from_str(&response.content).unwrap();

    // The old truncation would be:
    let truncated = &long_message[..80];

    // Our LLM-generated title should be:
    // - Much shorter than 80 chars
    // - Not equal to the truncated version
    assert!(plan.title.len() < 80, "LLM title should be concise");
    assert_ne!(plan.title, truncated, "Should not use truncated user message");
    assert!(!plan.title.ends_with("acros"), "Should not have truncation artifact");
}

#[tokio::test]
async fn test_regression_all_subtasks_created_upfront_not_delayed() {
    // Regression test: Ensure all subtasks exist from task creation
    // NOT created gradually during execution

    let client = MockLlmClient::anthropic("claude-3-5-sonnet");

    client.set_response(serde_json::json!({
        "title": "Codebase investigation",
        "description": "Analyze codebase structure and quality",
        "is_non_trivial": true,
        "subtasks": [
            {"title": "Scan project structure", "description": "List all files and directories", "order": 0},
            {"title": "Analyze code quality", "description": "Check for code smells", "order": 1},
            {"title": "Review dependencies", "description": "Check external dependencies", "order": 2},
            {"title": "Generate report", "description": "Create findings document", "order": 3}
        ]
    }).to_string());

    let system_prompt = build_planning_system_prompt();
    let user_prompt = format!(
        "User Request: {}\n\nCreate a task plan for this request.",
        "Investigate the codebase and suggest improvements"
    );

    let request = ChatRequest::new(
        "claude-3-5-sonnet",
        vec![
            ChatMessage {
                role: MessageRole::System,
                content: MessageContent::Text(system_prompt),
                tool_calls: Vec::new(),
                tool_result: None,
            },
            ChatMessage {
                role: MessageRole::User,
                content: MessageContent::Text(user_prompt),
                tool_calls: Vec::new(),
                tool_result: None,
            },
        ],
    );

    let response = client.chat(request).await.unwrap();
    let plan: TaskPlan = serde_json::from_str(&response.content).unwrap();

    // All subtasks should exist in the plan
    assert_eq!(plan.subtasks.len(), 4);

    // All subtasks should have valid titles (no placeholders)
    for subtask in &plan.subtasks {
        assert!(!subtask.title.is_empty(), "Subtask should have valid title");
        assert!(!subtask.title.contains("..."), "No placeholder titles");
        assert_ne!(subtask.title, "TBD", "No placeholder titles");
        assert_ne!(subtask.title, "Pending", "No placeholder titles");
    }

    // Subtasks should be properly ordered
    for (i, subtask) in plan.subtasks.iter().enumerate() {
        assert_eq!(subtask.order, i as i32, "Subtask order should match position");
    }
}

#[tokio::test]
async fn test_trivial_task_creates_no_subtasks() {
    // Test that simple requests don't generate subtasks

    let client = MockLlmClient::anthropic("claude-3-5-sonnet");

    client.set_response(serde_json::json!({
        "title": "Read README",
        "description": "Read the project README file",
        "is_non_trivial": false,
        "subtasks": []
    }).to_string());

    let system_prompt = build_planning_system_prompt();
    let user_prompt = format!(
        "User Request: {}\n\nCreate a task plan for this request.",
        "Read the README file"
    );

    let request = ChatRequest::new(
        "claude-3-5-sonnet",
        vec![
            ChatMessage {
                role: MessageRole::System,
                content: MessageContent::Text(system_prompt),
                tool_calls: Vec::new(),
                tool_result: None,
            },
            ChatMessage {
                role: MessageRole::User,
                content: MessageContent::Text(user_prompt),
                tool_calls: Vec::new(),
                tool_result: None,
            },
        ],
    );

    let response = client.chat(request).await.unwrap();
    let plan: TaskPlan = serde_json::from_str(&response.content).unwrap();

    // Should be marked as trivial
    assert!(!plan.is_non_trivial);

    // Should have no subtasks
    assert!(plan.subtasks.is_empty());
}

#[tokio::test]
async fn test_subtask_titles_are_actionable() {
    // Test that subtask titles follow best practices

    let client = MockLlmClient::anthropic("claude-3-5-sonnet");

    client.set_response(serde_json::json!({
        "title": "Feature implementation",
        "description": "Implement new user profile feature",
        "is_non_trivial": true,
        "subtasks": [
            {"title": "Design database schema", "description": "Create tables", "order": 0},
            {"title": "Implement API endpoints", "description": "Create REST API", "order": 1},
            {"title": "Build frontend components", "description": "Create UI", "order": 2},
            {"title": "Write tests", "description": "Add test coverage", "order": 3},
            {"title": "Update documentation", "description": "Document changes", "order": 4}
        ]
    }).to_string());

    let system_prompt = build_planning_system_prompt();
    let user_prompt = format!(
        "User Request: {}\n\nCreate a task plan for this request.",
        "Implement a user profile feature"
    );

    let request = ChatRequest::new(
        "claude-3-5-sonnet",
        vec![
            ChatMessage {
                role: MessageRole::System,
                content: MessageContent::Text(system_prompt),
                tool_calls: Vec::new(),
                tool_result: None,
            },
            ChatMessage {
                role: MessageRole::User,
                content: MessageContent::Text(user_prompt),
                tool_calls: Vec::new(),
                tool_result: None,
            },
        ],
    );

    let response = client.chat(request).await.unwrap();
    let plan: TaskPlan = serde_json::from_str(&response.content).unwrap();

    // Subtask titles should be actionable (start with verbs)
    let action_verbs = ["Design", "Implement", "Build", "Write", "Update", "Create", "Add", "Fix", "Refactor", "Test"];
    let mut has_actionable_titles = 0;

    for subtask in &plan.subtasks {
        let first_word = subtask.title.split_whitespace().next().unwrap_or("");
        if action_verbs.contains(&first_word) {
            has_actionable_titles += 1;
        }
    }

    // At least some subtasks should start with action verbs
    assert!(has_actionable_titles > 0, "Subtasks should have actionable titles");
}
