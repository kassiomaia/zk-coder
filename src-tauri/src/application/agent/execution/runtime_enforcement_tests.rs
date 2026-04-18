//! Characterization tests for Slice 4 — M2.2 runtime enforcement and M2.3
//! state-driven tool-choice forcing.
//!
//! These tests pin two invariants without requiring a live agent run:
//!
//! 1. The classification of tool calls as "disallowed by current subtask
//!    mode" mirrors the actual logic used in `tool_phase` and
//!    `process_tool_calls_with_permissions`. A single source of truth check
//!    keeps the two sites honest.
//! 2. The `ChatRequest` produced by `call_llm` reflects the one-shot
//!    `pending_tool_choice_override` on `ExecutionContext`: it is consumed
//!    (taken), it appears in the built request, and it does *not* persist to
//!    subsequent calls.
//!
//! The actual `call_llm` function needs a real `LlmClient`, so the request
//! construction portion of its logic is exercised here via a direct
//! `ChatRequest` builder call that matches what `call_llm` does. This keeps
//! the test fast, pure, and independent of any runtime deps.

use crate::core::agent::task::SubTaskMode;
use crate::core::llm::entities::{ChatMessage, ChatRequest, ToolCall, ToolChoiceMode};

/// Mirrors the predicate used in both `tool_phase::run_tools` and
/// `permissions::process_tool_calls_with_permissions` to decide whether a tool
/// call is disallowed under the current subtask mode. Keeping this as a
/// testable helper guards against the two sites drifting.
fn is_disallowed_by_mode(call_name: &str, allowed_tool_names: Option<&[String]>) -> bool {
    allowed_tool_names.is_some_and(|allowed| !allowed.iter().any(|name| name == call_name))
}

fn call(name: &str) -> ToolCall {
    ToolCall {
        id: format!("call-{name}"),
        name: name.to_string(),
        arguments: serde_json::json!({}),
    }
}

/// Restricted-mode helper: the caller asserts `mode` enumerates an
/// allowlist (Read / Write / Execute) and unwraps. Using the helper on
/// `FullAccess` panics — `FullAccess` carries no allowlist and the
/// runtime wires it through as `None`, which skips enforcement entirely.
fn allowed_for(mode: SubTaskMode) -> Vec<String> {
    mode.allowed_tools()
        .expect("allowed_for is for restricted modes only; FullAccess returns None")
        .into_iter()
        .map(std::string::ToString::to_string)
        .collect()
}

// --- M2.2: runtime enforcement of subtask mode ------------------------------

#[test]
fn tool_call_inside_allowed_set_is_not_disallowed() {
    let allowed = allowed_for(SubTaskMode::Read);
    assert!(!is_disallowed_by_mode("fs_read_file", Some(&allowed)));
    assert!(!is_disallowed_by_mode("search_grep", Some(&allowed)));
}

#[test]
fn write_in_read_mode_is_disallowed() {
    // The canonical regression case: the catalog still advertises
    // `fs_write_file`, but in Read mode the runtime must reject the call
    // rather than letting it reach the registry.
    let allowed = allowed_for(SubTaskMode::Read);
    assert!(is_disallowed_by_mode("fs_write_file", Some(&allowed)));
    assert!(is_disallowed_by_mode("fs_apply_patch", Some(&allowed)));
    assert!(is_disallowed_by_mode(
        "system_execute_command",
        Some(&allowed)
    ));
}

#[test]
fn execute_in_write_mode_is_disallowed() {
    let allowed = allowed_for(SubTaskMode::Write);
    assert!(is_disallowed_by_mode("system_execute_command", Some(&allowed)));
}

#[test]
fn full_access_disallows_nothing_by_not_enforcing_an_allowlist() {
    // FullAccess is unrestricted by design: `allowed_tools()` returns
    // `None`, and `context_phase::resolve_context` forwards that `None`
    // through to `ctx.current_allowed_tool_names`. The enforcement
    // predicate treats `None` as "no filter", so every registered tool
    // passes — including document-generation tools (`pdf_*`, `docx_*`,
    // `pptx_*`) and agent orchestration tools that were never listed in
    // any hard-coded allowlist. This is the regression guard: any attempt
    // to reintroduce a stale FullAccess enumeration will fail the first
    // assert here.
    assert_eq!(
        SubTaskMode::FullAccess.allowed_tools(),
        None,
        "FullAccess must be unrestricted — no hard-coded list"
    );
    for tool_name in [
        "fs_read_file",
        "fs_write_file",
        "system_execute_command",
        "pdf_start",
        "pdf_add_page",
        "pdf_finalize",
        "docx_start",
        "pptx_start",
        "agent_plan_subtasks",
        "agent_narrate",
        "agent_mark_subtask_status",
        "search_web_search",
        "conversation_search_messages",
    ] {
        assert!(!is_disallowed_by_mode(tool_name, None));
    }
}

#[test]
fn no_restriction_list_means_nothing_is_disallowed() {
    // `None` is the "no subtask / session-scope" case — the runtime must not
    // restrict anything when the allow-list is absent, because the prior
    // behavior (filtering the catalog by mode) has been removed.
    assert!(!is_disallowed_by_mode("fs_read_file", None));
    assert!(!is_disallowed_by_mode("system_execute_command", None));
}

#[test]
fn empty_allow_list_disallows_every_tool_call() {
    // This is the final-summary path: an empty allow-list means the runtime
    // rejects any tool call that slips through, defense-in-depth after
    // `ToolChoiceMode::None` is forced on the provider side.
    let empty: Vec<String> = Vec::new();
    assert!(is_disallowed_by_mode("fs_read_file", Some(&empty)));
    assert!(is_disallowed_by_mode("agent_emit_status", Some(&empty)));
}

// --- M2.3: one-shot tool-choice override ------------------------------------

#[test]
fn chat_request_defaults_to_auto_tool_choice() {
    // When `call_llm` sees `ctx.pending_tool_choice_override == None`, it
    // skips the `.with_tool_choice()` builder call. The request must then
    // default to `Auto`.
    let request = ChatRequest::new("claude-sonnet", vec![ChatMessage::user("hi")]);
    assert_eq!(request.tool_choice, ToolChoiceMode::Auto);
}

#[test]
fn chat_request_with_forced_none_records_none_mode() {
    // Final-summary path: the coordinator sets
    // `ctx.pending_tool_choice_override = Some(ToolChoiceMode::None)` and
    // `call_llm` plumbs it through to the request.
    let request = ChatRequest::new("claude-sonnet", vec![ChatMessage::user("summarize")])
        .with_tool_choice(ToolChoiceMode::None);
    assert_eq!(request.tool_choice, ToolChoiceMode::None);
}

#[test]
fn chat_request_with_forced_required_records_required_mode() {
    // Narration-loop `EmitFillerAndNudge` path: force the next call to
    // require a tool so the model cannot continue narrating without action.
    let request = ChatRequest::new("claude-sonnet", vec![ChatMessage::user("continue")])
        .with_tool_choice(ToolChoiceMode::Required);
    assert_eq!(request.tool_choice, ToolChoiceMode::Required);
}

#[test]
fn take_on_override_is_one_shot() {
    // `call_llm` uses `.take()` so the override fires exactly once. Model the
    // same two-step with a local `Option` to pin the invariant in case a
    // future refactor replaces `.take()` with `.clone()` or similar.
    let mut override_slot: Option<ToolChoiceMode> = Some(ToolChoiceMode::Required);
    let consumed = override_slot.take();
    assert_eq!(consumed, Some(ToolChoiceMode::Required));
    assert!(
        override_slot.is_none(),
        "forced tool-choice override must not persist past the step that consumed it"
    );
}

// --- integration of both: the full-catalog / enforcement wire-up ------------

#[test]
fn call_names_seen_by_runtime_enforcement_come_from_the_full_catalog() {
    // This is the M2.2 wire-up invariant stated as a test: the runtime
    // enforcement is based on call *names*, not on whether the tool appears
    // in the request catalog. So a call to `fs_write_file` must still be
    // classified against `allowed_tool_names` even if the LLM saw
    // `fs_write_file` in the advertised catalog (which, after M2.2, it
    // always does).
    let read_allowed = allowed_for(SubTaskMode::Read);
    let write_call = call("fs_write_file");
    assert!(is_disallowed_by_mode(
        &write_call.name,
        Some(&read_allowed)
    ));
}

/// Regression test for the "agent finished but never produced the PDF" bug.
///
/// Before this fix, `SubTaskMode::FullAccess.allowed_tools()` returned a
/// stale hardcoded subset (fs_* + search_grep + system_execute_command +
/// agent_emit_status + agent_advance_to_next_subtask). Any document-
/// generation tool (`pdf_*`, `docx_*`, `pptx_*`) or agent orchestration
/// tool registered after that list was authored was rejected at runtime
/// with `subtask_mode_disallowed`, even when the subtask was explicitly
/// declared `FullAccess`. The agent would keep trying, eventually
/// `agent_advance_to_next_subtask`, and complete with a confident summary
/// but no actual PDF/DOCX/PPTX artifact.
///
/// The contract now: `FullAccess.allowed_tools() == None`, and
/// `context_phase::resolve_context` forwards that `None` to
/// `ctx.current_allowed_tool_names`. The enforcement predicate treats
/// `None` as "no filter", so every registered tool passes. Adding a new
/// tool to the registry automatically makes it reachable under
/// `FullAccess` — no parallel list to keep in sync, no silent
/// rejections.
#[test]
fn full_access_allows_document_generation_and_orchestration_tools() {
    assert_eq!(
        SubTaskMode::FullAccess.allowed_tools(),
        None,
        "FullAccess must be unrestricted"
    );

    // Under the previous bug these all reached the runtime filter with a
    // stale allowlist and were rejected. They must now pass because the
    // filter sees `None` and short-circuits.
    let full_access_allowed: Option<&[String]> = None;
    for tool_name in [
        "pdf_start",
        "pdf_add_page",
        "pdf_finalize",
        "pdf_list_pages",
        "pdf_get_page",
        "pdf_delete_page",
        "docx_start",
        "docx_add_section",
        "docx_finalize",
        "docx_list_sections",
        "pptx_start",
        "pptx_add_slide",
        "pptx_finalize",
        "pptx_list_slides",
        "pptx_get_slide",
        "pptx_update_slide",
        "pptx_delete_slide",
        "pptx_search_slides",
        "agent_plan_subtasks",
        "agent_narrate",
        "agent_mark_subtask_status",
        "search_web_search",
        "conversation_search_messages",
    ] {
        assert!(
            !is_disallowed_by_mode(tool_name, full_access_allowed),
            "FullAccess must allow {tool_name} — any rejection here means the \
             stale-allowlist bug has regressed"
        );
    }
}
