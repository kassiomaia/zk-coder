//! Shared fixtures for the `context` submodule test suites.

use serde_json::{json, Value as JsonValue};

use crate::core::llm::entities::{ChatMessage as Message, MessageContent, MessageRole, ToolCall};

use super::super::{ExecutionBrief, FailureClass, FailureRecord, MemoryReference};

pub(super) fn large_json_string(approx_bytes: usize) -> String {
    "x".repeat(approx_bytes)
}

pub(super) fn sample_memory_reference() -> MemoryReference {
    MemoryReference {
        session_id: "sess-42".to_string(),
        task_goal: "investigate auth regression".to_string(),
        active_plan: "review auth.rs; patch login flow; add tests".to_string(),
        completed_findings_count: 2,
        open_questions_count: 1,
        extracted_facts_count: 3,
        updated_at_rfc3339: "2026-04-17T12:00:00+00:00".to_string(),
    }
}

pub(super) fn memory_block_from(messages: &[Message]) -> Option<&str> {
    messages.iter().find_map(|m| match &m.content {
        MessageContent::Text(text)
            if m.role == MessageRole::System
                && text.starts_with("ContextPolicyV2 session memory reference:") =>
        {
            Some(text.as_str())
        }
        _ => None,
    })
}

pub(super) fn build_above_threshold_context() -> Vec<Message> {
    // Large enough to exceed the 70% threshold for gpt-4 so the
    // packer runs without needing `force_pack`.
    let padding = "x".repeat(400);
    let mut context = vec![Message::system("System")];
    for i in 0..1_000 {
        context.push(Message::user(format!("msg {i} {padding}")));
    }
    context
}

pub(super) fn assistant_with_call(id: &str, args_padding_bytes: usize) -> Message {
    let content = "x".repeat(args_padding_bytes);
    Message::assistant_with_tool_calls(
        "writing".to_string(),
        vec![ToolCall {
            id: id.to_string(),
            name: "fs_write_file".to_string(),
            arguments: json!({"path": "docs/foo.md", "content": content}),
        }],
    )
}

pub(super) fn tool_result_for(id: &str) -> Message {
    Message::tool_result(id, json!({"bytes_written": 42}), false)
}

pub(super) fn orphan_tool_result_ids(tail: &[Message]) -> Vec<String> {
    let assistant_ids: std::collections::HashSet<String> = tail
        .iter()
        .filter(|m| m.role == MessageRole::Assistant)
        .flat_map(|m| m.tool_calls.iter().map(|tc| tc.id.clone()))
        .collect();
    tail.iter()
        .filter(|m| m.role == MessageRole::Tool)
        .filter_map(|m| m.tool_result.as_ref().map(|r| r.call_id.clone()))
        .filter(|id| !assistant_ids.contains(id))
        .collect()
}

pub(super) fn sample_execution_brief() -> ExecutionBrief {
    ExecutionBrief {
        task_goal: "fix failing login flow".to_string(),
        active_subtask: Some("patch token refresh".to_string()),
        recently_completed: vec![
            "audit auth.rs".to_string(),
            "repro failing test".to_string(),
        ],
        next_intended_action: Some("add regression test".to_string()),
        current_blocker: None,
    }
}

pub(super) fn brief_block_from(messages: &[Message]) -> Option<&str> {
    messages.iter().find_map(|m| match &m.content {
        MessageContent::Text(text)
            if m.role == MessageRole::System
                && text.starts_with("ContextPolicyV2 execution brief:") =>
        {
            Some(text.as_str())
        }
        _ => None,
    })
}

pub(super) fn context_with_failed_tool_call(
    tool_name: &str,
    args: JsonValue,
    error_message: &str,
) -> Vec<Message> {
    vec![
        Message::user("go"),
        Message::assistant_with_tool_calls(
            "running".to_string(),
            vec![ToolCall {
                id: "call_err_1".to_string(),
                name: tool_name.to_string(),
                arguments: args,
            }],
        ),
        Message::tool_result("call_err_1", json!({ "error": error_message }), true),
    ]
}

pub(super) fn failure_record(tool_name: &str, class: FailureClass) -> FailureRecord {
    FailureRecord {
        tool_name: tool_name.to_string(),
        args_fingerprint: "feedface".to_string(),
        class,
        message: "bounded error".to_string(),
        repeat_detected: false,
    }
}
