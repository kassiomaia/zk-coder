//! Tests for failure classification, fingerprinting, and the
//! subtask-scoped failure-memory overlay.

use serde_json::json;

use crate::core::llm::entities::{ChatMessage as Message, ToolCall};

use super::super::failure::{arguments_fingerprint, build_failure_memory};
use super::super::{
    select_failure_memory_body, FailureClass, MemorizedFailure,
};
use super::common::{context_with_failed_tool_call, failure_record};

#[test]
fn failure_class_classifies_common_errors() {
    assert_eq!(
        FailureClass::classify("No such file or directory (ENOENT)"),
        FailureClass::FileNotFound,
    );
    assert_eq!(
        FailureClass::classify("Permission denied"),
        FailureClass::PermissionDenied,
    );
    assert_eq!(
        FailureClass::classify("operation timed out after 30s"),
        FailureClass::Timeout,
    );
    assert_eq!(
        FailureClass::classify("Invalid argument: missing required field 'path'"),
        FailureClass::ValidationFailed,
    );
    assert_eq!(
        FailureClass::classify("subtask_mode_disallowed: tool not in allowed list"),
        FailureClass::NotAllowedByMode,
    );
    assert_eq!(
        FailureClass::classify("tool not allowed for this argument shape"),
        FailureClass::NotAllowedByArgs,
    );
    assert_eq!(
        FailureClass::classify("something unrelated"),
        FailureClass::Other,
    );
}

// --- C.3: NotAllowedByMode vs NotAllowedByArgs -------------------------
//
// The split exists so the failure-memory `retry:` line stops telling the
// model to "change args before retry" when changing args can never help
// (mode-disallowed). These tests lock in both the classifier split and
// the tailored retry copy.

#[test]
fn not_allowed_by_mode_classification_and_retry_copy() {
    let class = FailureClass::classify(
        "subtask_mode_disallowed: tool `fs_write_file` not permitted in Read mode",
    );
    assert_eq!(class, FailureClass::NotAllowedByMode);
    assert_eq!(class.label(), "not_allowed_by_mode");
    let retry = class.retry_line();
    assert!(retry.contains("do not retry"), "expected 'do not retry'; got: {retry}");
    assert!(
        retry.contains("Wait for a later subtask")
            || retry.contains("agent_mark_subtask_status"),
        "mode-disallowed retry copy must point at the two valid moves; got: {retry}",
    );
    assert!(
        !retry.contains("change arguments"),
        "mode-disallowed retry must NOT tell the model to change arguments; got: {retry}",
    );
}

#[test]
fn not_allowed_by_args_classification_and_retry_copy() {
    let class = FailureClass::classify("argument-level policy: this path is not allowed");
    assert_eq!(class, FailureClass::NotAllowedByArgs);
    assert_eq!(class.label(), "not_allowed_by_args");
    let retry = class.retry_line();
    assert!(
        retry.contains("change arguments"),
        "args-disallowed retry must tell the model to change arguments; got: {retry}",
    );
}

// --- C.1/C.2: subtask-scoped failure memory overlay ------------------

#[test]
fn select_failure_memory_prefers_current_subtask_overlay() {
    let memorized = vec![
        MemorizedFailure {
            subtask_id: Some("subtask-2".to_string()),
            record: failure_record("fs_read_file", FailureClass::FileNotFound),
        },
        MemorizedFailure {
            subtask_id: Some("subtask-3".to_string()),
            record: failure_record("fs_write_file", FailureClass::PermissionDenied),
        },
    ];
    let body = select_failure_memory_body(&memorized, Some("subtask-3"), &[])
        .expect("overlay should produce a body");
    assert!(body.contains("tool: fs_write_file"));
    assert!(body.contains("type: permission_denied"));
    assert!(!body.contains("fs_read_file"));
}

#[test]
fn select_failure_memory_filters_out_previous_subtask_failures() {
    // Regression: 2026-04-18 trace step 22 (subtask-2 failure)
    // must not surface in subtask-3's prompt (step 35).
    let memorized = vec![MemorizedFailure {
        subtask_id: Some("subtask-2".to_string()),
        record: failure_record("fs_read_file", FailureClass::FileNotFound),
    }];
    let body = select_failure_memory_body(&memorized, Some("subtask-3"), &[]);
    assert!(
        body.is_none(),
        "previous-subtask failure must not surface; got {body:?}",
    );
}

#[test]
fn select_failure_memory_surfaces_unscoped_entries_in_any_subtask() {
    let memorized = vec![MemorizedFailure {
        subtask_id: None,
        record: failure_record("search_grep", FailureClass::Timeout),
    }];
    let body = select_failure_memory_body(&memorized, Some("subtask-99"), &[])
        .expect("unscoped entry should surface");
    assert!(body.contains("search_grep"));
}

#[test]
fn select_failure_memory_falls_back_to_context_scan_when_overlay_is_empty() {
    let context = context_with_failed_tool_call(
        "fs_read_file",
        json!({"path": "/nope.txt"}),
        "No such file",
    );
    let body = select_failure_memory_body(&[], Some("subtask-1"), &context)
        .expect("context-scan fallback should fire");
    assert!(body.contains("fs_read_file"));
}

#[test]
fn select_failure_memory_does_not_fallback_when_overlay_present_but_filtered_out() {
    // If the overlay is populated but no entry matches the current
    // subtask, do NOT fall back to the context scan: the scan is
    // subtask-blind and would reintroduce the leak C.2 is
    // explicitly designed to close.
    let memorized = vec![MemorizedFailure {
        subtask_id: Some("subtask-other".to_string()),
        record: failure_record("fs_read_file", FailureClass::FileNotFound),
    }];
    let context = context_with_failed_tool_call(
        "fs_read_file",
        json!({"path": "/nope.txt"}),
        "No such file",
    );
    let body = select_failure_memory_body(&memorized, Some("subtask-3"), &context);
    assert!(body.is_none());
}

#[test]
fn not_allowed_branches_render_distinct_failure_memory_blocks() {
    let mode_ctx = context_with_failed_tool_call(
        "fs_write_file",
        json!({"path": "/workspace/out.txt"}),
        "subtask_mode_disallowed: fs_write_file not in allowed list",
    );
    let args_ctx = context_with_failed_tool_call(
        "fs_write_file",
        json!({"path": "/etc/passwd"}),
        "tool not allowed for the given arguments",
    );
    let mode_block = build_failure_memory(&mode_ctx).expect("mode error present");
    let args_block = build_failure_memory(&args_ctx).expect("args error present");
    assert!(mode_block.contains("type: not_allowed_by_mode"));
    assert!(args_block.contains("type: not_allowed_by_args"));
    assert!(mode_block.contains("do not retry"));
    assert!(args_block.contains("change arguments"));
    assert!(!mode_block.contains("change arguments"));
}

#[test]
fn failure_memory_renders_structured_block_for_single_error() {
    let context = context_with_failed_tool_call(
        "fs_read_file",
        json!({"path": "/nope.txt"}),
        "No such file: /nope.txt",
    );
    let block = build_failure_memory(&context).expect("error present");
    assert!(block.starts_with("Last error:"));
    assert!(block.contains("tool: fs_read_file"));
    assert!(block.contains("type: file_not_found"));
    assert!(block.contains("args_fp: "));
    assert!(block.contains("message: No such file: /nope.txt"));
    assert!(block.contains("retry: change args before retry"));
    assert!(!block.contains("identical call has failed"));
}

#[test]
fn failure_memory_renders_retryable_verdict_for_timeouts() {
    let context = context_with_failed_tool_call(
        "search_grep",
        json!({"query": "auth"}),
        "operation timed out after 30s",
    );
    let block = build_failure_memory(&context).expect("error present");
    assert!(block.contains("type: timeout"));
    assert!(
        block.contains("retry: safe to retry (likely transient)"),
        "timeouts must be marked retryable; got {block}",
    );
}

#[test]
fn failure_memory_detects_identical_repeat_call() {
    let args = json!({"path": "/missing.txt"});
    let context = vec![
        Message::user("go"),
        Message::assistant_with_tool_calls(
            "try one".to_string(),
            vec![ToolCall {
                id: "call_a".to_string(),
                name: "fs_read_file".to_string(),
                arguments: args.clone(),
            }],
        ),
        Message::tool_result("call_a", json!({"error": "No such file"}), true),
        Message::assistant_with_tool_calls(
            "try again".to_string(),
            vec![ToolCall {
                id: "call_b".to_string(),
                name: "fs_read_file".to_string(),
                arguments: args,
            }],
        ),
        Message::tool_result("call_b", json!({"error": "No such file"}), true),
    ];
    let block = build_failure_memory(&context).expect("error present");
    assert!(
        block.contains("identical call has failed before — do not retry unchanged"),
        "repeat should surface anti-repeat guidance; got {block}",
    );
}

#[test]
fn failure_memory_does_not_flag_repeat_when_args_differ() {
    let context = vec![
        Message::user("go"),
        Message::assistant_with_tool_calls(
            "try one".to_string(),
            vec![ToolCall {
                id: "call_a".to_string(),
                name: "fs_read_file".to_string(),
                arguments: json!({"path": "/one.txt"}),
            }],
        ),
        Message::tool_result("call_a", json!({"error": "No such file"}), true),
        Message::assistant_with_tool_calls(
            "try another".to_string(),
            vec![ToolCall {
                id: "call_b".to_string(),
                name: "fs_read_file".to_string(),
                arguments: json!({"path": "/two.txt"}),
            }],
        ),
        Message::tool_result("call_b", json!({"error": "No such file"}), true),
    ];
    let block = build_failure_memory(&context).expect("error present");
    assert!(
        !block.contains("identical call has failed"),
        "different args must not trigger anti-repeat guidance; got {block}",
    );
}

#[test]
fn arguments_fingerprint_is_stable_across_calls() {
    let args = json!({"path": "/a.txt", "mode": "read"});
    let a = arguments_fingerprint(&args);
    let b = arguments_fingerprint(&args);
    assert_eq!(a, b);
    assert_eq!(a.len(), 8);
}

#[test]
fn arguments_fingerprint_distinguishes_different_args() {
    let a = arguments_fingerprint(&json!({"path": "/a.txt"}));
    let b = arguments_fingerprint(&json!({"path": "/b.txt"}));
    assert_ne!(a, b);
}
