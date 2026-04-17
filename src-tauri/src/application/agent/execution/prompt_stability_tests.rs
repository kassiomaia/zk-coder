//! Characterization tests for M1.3 — prompt assembly stabilization.
//!
//! These tests lock in the invariant that identical session state produces
//! identical prepared-request hashes, and that subtask transitions change
//! only the expected request components. See
//! `docs/planning/agent-context-engineering/measurement-harness.md` for the
//! full list of runtime transitions that are allowed to change each hash.

use serde_json::json;

use crate::core::agent::task::{SubTaskMode, TaskStatus};
use crate::core::llm::entities::{ChatMessage, ToolDefinition};
use crate::infrastructure::agent::runtime::request_shape::{
    message_prefix_hash, system_prompt_hash, tool_catalog_hash,
};
use crate::infrastructure::agent::runtime::{derive_system_prompt, SubtaskSummary};

const BASE_PROMPT: &str = "You are a senior engineering agent. Be careful.";

fn summary(title: &str, mode: SubTaskMode, status: TaskStatus, order: i32) -> SubtaskSummary {
    SubtaskSummary {
        title: title.to_string(),
        mode,
        status,
        order,
    }
}

fn tool(name: &str, description: &str) -> ToolDefinition {
    ToolDefinition {
        name: name.to_string(),
        description: description.to_string(),
        input_schema: json!({"type": "object"}),
    }
}

fn full_catalog() -> Vec<ToolDefinition> {
    // Canonical (sorted-by-name) order as produced by ToolRegistry::definitions()
    // after M1.1.
    vec![
        tool("agent_advance_to_next_subtask", "Advance subtask"),
        tool("agent_emit_status", "Emit status"),
        tool("fs_apply_patch", "Apply a patch"),
        tool("fs_fuzzy_find_files", "Fuzzy find files"),
        tool("fs_glob_files", "Glob files"),
        tool("fs_list_files", "List files"),
        tool("fs_read_file", "Read a file"),
        tool("fs_replace_in_file", "Replace in file"),
        tool("fs_stat_path", "Stat a path"),
        tool("fs_write_file", "Write a file"),
        tool("search_grep", "Grep search"),
        tool("system_execute_command", "Execute command"),
    ]
}

/// Mirrors the filter in `context_phase::resolve_context` that narrows the
/// canonical catalog to the current subtask's allowed tools. Returns tools
/// in canonical order because `Vec::iter().filter(...)` preserves order.
fn filter_to_mode(catalog: &[ToolDefinition], mode: &SubTaskMode) -> Vec<ToolDefinition> {
    let allowed = mode.allowed_tools();
    catalog
        .iter()
        .filter(|t| allowed.iter().any(|name| *name == t.name))
        .cloned()
        .collect()
}

fn user(content: &str) -> ChatMessage {
    ChatMessage::user(content)
}

// --- system_prompt_hash stability -------------------------------------------

#[test]
fn identical_session_state_produces_identical_system_prompt_hash() {
    let s = summary("Review auth", SubTaskMode::Read, TaskStatus::InProgress, 0);
    let first = derive_system_prompt(BASE_PROMPT, &Some((s.clone(), 3)));
    let second = derive_system_prompt(BASE_PROMPT, &Some((s, 3)));
    assert_eq!(first, second);
    assert_eq!(system_prompt_hash(&first), system_prompt_hash(&second));
}

#[test]
fn no_subtask_state_produces_identical_system_prompt_hash_across_invocations() {
    let a = derive_system_prompt(BASE_PROMPT, &None);
    let b = derive_system_prompt(BASE_PROMPT, &None);
    assert_eq!(system_prompt_hash(&a), system_prompt_hash(&b));
}

#[test]
fn subtask_transition_changes_system_prompt_hash() {
    let before = summary(
        "Review auth",
        SubTaskMode::Read,
        TaskStatus::InProgress,
        0,
    );
    let after = summary(
        "Patch auth",
        SubTaskMode::Write,
        TaskStatus::InProgress,
        1,
    );
    let prompt_before = derive_system_prompt(BASE_PROMPT, &Some((before, 3)));
    let prompt_after = derive_system_prompt(BASE_PROMPT, &Some((after, 3)));
    assert_ne!(
        system_prompt_hash(&prompt_before),
        system_prompt_hash(&prompt_after),
        "expected system prompt hash to move when the active subtask changes",
    );
}

#[test]
fn subtask_moving_out_of_in_progress_reverts_system_prompt_hash_to_base() {
    // The helper only inlines subtask context when status is InProgress.
    // A subtask flipping to Completed must therefore produce the same hash as
    // `None` — and *not* the in-progress variant's hash.
    let in_progress = summary(
        "Review auth",
        SubTaskMode::Read,
        TaskStatus::InProgress,
        0,
    );
    let completed = summary(
        "Review auth",
        SubTaskMode::Read,
        TaskStatus::Completed,
        0,
    );
    let in_progress_prompt = derive_system_prompt(BASE_PROMPT, &Some((in_progress, 3)));
    let completed_prompt = derive_system_prompt(BASE_PROMPT, &Some((completed, 3)));
    let base_prompt = derive_system_prompt(BASE_PROMPT, &None);

    assert_eq!(
        system_prompt_hash(&completed_prompt),
        system_prompt_hash(&base_prompt),
    );
    assert_ne!(
        system_prompt_hash(&completed_prompt),
        system_prompt_hash(&in_progress_prompt),
    );
}

// --- tool_catalog_hash stability --------------------------------------------

#[test]
fn identical_filter_over_canonical_catalog_produces_identical_tool_catalog_hash() {
    let catalog = full_catalog();
    let first = filter_to_mode(&catalog, &SubTaskMode::Read);
    let second = filter_to_mode(&catalog, &SubTaskMode::Read);
    assert_eq!(tool_catalog_hash(&first), tool_catalog_hash(&second));
}

#[test]
fn subtask_transition_preserves_tool_catalog_hash_when_mode_is_unchanged() {
    // Two different subtasks of the same mode must filter down to the same
    // tool set, so the tool catalog hash must not move on transition.
    let catalog = full_catalog();
    let a = filter_to_mode(&catalog, &SubTaskMode::Read);
    let b = filter_to_mode(&catalog, &SubTaskMode::Read);
    assert_eq!(tool_catalog_hash(&a), tool_catalog_hash(&b));
}

#[test]
fn subtask_mode_transition_changes_tool_catalog_hash() {
    // Current (pre-M2.2) behavior: a mode change resizes the allowed set and
    // therefore legitimately changes the catalog hash. Lock this in so any
    // unintended change in the filtering logic is caught.
    let catalog = full_catalog();
    let read_set = filter_to_mode(&catalog, &SubTaskMode::Read);
    let write_set = filter_to_mode(&catalog, &SubTaskMode::Write);
    assert_ne!(tool_catalog_hash(&read_set), tool_catalog_hash(&write_set));
}

// --- message_prefix_hash stability ------------------------------------------

#[test]
fn appending_a_single_new_message_leaves_prefix_hash_unchanged() {
    // This is the cache-warmth invariant: the prepared request for step N+1
    // should share its prefix with step N, modulo an appended tail message.
    let shared = vec![
        user("please investigate login"),
        user("start by reading auth.rs"),
    ];
    let mut step_n = shared.clone();
    step_n.push(user("tool call for step N"));
    let mut step_n1 = shared.clone();
    step_n1.push(user("tool call for step N+1"));

    assert_eq!(
        message_prefix_hash(&step_n),
        message_prefix_hash(&step_n1),
        "appending a new tail message must not move the prefix hash",
    );
}

#[test]
fn mutating_an_earlier_message_changes_prefix_hash() {
    let original = vec![
        user("please investigate login"),
        user("start by reading auth.rs"),
        user("current tail"),
    ];
    let mutated = vec![
        user("please investigate login"),
        user("start by reading login.rs"),
        user("current tail"),
    ];
    assert_ne!(
        message_prefix_hash(&original),
        message_prefix_hash(&mutated),
    );
}
