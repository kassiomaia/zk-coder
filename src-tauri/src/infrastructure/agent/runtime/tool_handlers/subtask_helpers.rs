//! Helper functions for agent subtask management.
//!
//! This module contains utility functions for:
//! - Validating subtask plans
//! - Detecting code change intent
//! - Tracking subtask evidence and rationales
//! - Auto-decomposing failed subtasks
//! - Converting between task types

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

use log::warn;
use tauri::{AppHandle, Emitter};

use crate::core::agent::repositories::TaskRepository;
use crate::core::agent::session::AgentSession;
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;


/// Static tracker for plan retry attempts.
/// Tracks which session:task combinations have already retried planning.
pub static PLAN_RETRY_TRACKER: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

/// Static storage for subtask rationales.
/// Maps subtask IDs to their rationale explanations.
pub static SUBTASK_RATIONALES: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

/// Static tracker for subtask failure counts.
/// Maps subtask IDs to their failure count.
pub static SUBTASK_FAILURE_COUNTS: OnceLock<Mutex<HashMap<String, u32>>> = OnceLock::new();

/// Static tracker for decomposed subtasks.
/// Tracks which subtasks have already been auto-decomposed.
pub static DECOMPOSED_SUBTASKS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

/// Validates a subtask plan for correctness and quality.
///
/// Ensures that:
/// - Plan contains at least one subtask
/// - Each subtask has a unique, non-empty title
/// - Titles start with actionable verbs
/// - Code-change tasks include verification steps
pub fn validate_plan_subtasks(
    subtasks: &[serde_json::Value],
    requires_verification: bool,
) -> Result<(), String> {
    if subtasks.is_empty() {
        return Err("Plan must contain at least one subtask".to_string());
    }

    let actionable_verbs = [
        "analyze",
        "investigate",
        "identify",
        "understand",
        "trace",
        "inspect",
        "read",
        "search",
        "implement",
        "update",
        "refactor",
        "fix",
        "write",
        "create",
        "add",
        "remove",
        "rename",
        "run",
        "test",
        "verify",
        "build",
        "check",
        "debug",
        "review",
    ];
    let verification_terms = [
        "test",
        "verify",
        "validation",
        "regression",
        "check",
        "build",
        "compile",
        "lint",
    ];
    let mut seen = HashSet::new();
    let mut has_verification_step = false;

    for item in subtasks {
        let title = item
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "Each subtask must have a non-empty title".to_string())?;
        let normalized = title.to_lowercase();
        if !seen.insert(normalized.clone()) {
            return Err(format!("Duplicate subtask title found: '{title}'"));
        }

        let first_word = normalized
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_string();
        let first_word = first_word
            .trim_matches(|c: char| c.is_ascii_punctuation())
            .trim_matches(|c: char| c.is_ascii_digit())
            .to_string();
        if !actionable_verbs.contains(&first_word.as_str()) {
            return Err(format!(
                "Subtask '{title}' is not actionable; start titles with an action verb"
            ));
        }

        if verification_terms
            .iter()
            .any(|term| normalized.contains(term))
        {
            has_verification_step = true;
        }
    }

    if requires_verification && !has_verification_step {
        return Err(
            "Code-change intent detected. Add a verification subtask (test/verify/build/check)."
                .to_string(),
        );
    }

    Ok(())
}

/// Detects whether a task involves code changes based on:
/// - Keywords in the task title
/// - Previous tool calls (file writes, deletions, commands)
pub fn detect_code_change_intent(
    session: &AgentSession,
    task_id: &str,
    task_repo: &dyn TaskRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
) -> bool {
    let task_text = task_repo
        .get_session_tasks(&session.id.0)
        .ok()
        .and_then(|tasks| tasks.into_iter().find(|t| t.id == task_id))
        .map(|t| t.title.to_lowercase())
        .unwrap_or_default();

    let keywords = [
        "fix",
        "bug",
        "implement",
        "update",
        "write",
        "refactor",
        "create",
        "add",
        "remove",
        "rename",
        "build",
        "compile",
        "test",
    ];
    let heuristic_hit = keywords.iter().any(|k| task_text.contains(k));
    if heuristic_hit {
        return true;
    }

    telemetry_repo
        .get_tool_calls_for_session(&session.id)
        .ok()
        .is_some_and(|calls| {
            calls.into_iter().any(|c| {
                matches!(
                    c.tool_name.as_str(),
                    "fs_write_file" | "fs_delete_file" | "system_execute_command"
                ) && !c.is_error
            })
        })
}

/// Checks if a subtask has evidence of work (file changes, commands executed).
pub fn has_subtask_evidence(
    session: &AgentSession,
    subtask_id: &str,
    telemetry_repo: &dyn AgentTelemetryRepository,
) -> bool {
    telemetry_repo
        .get_tool_calls_for_session(&session.id)
        .ok()
        .is_some_and(|calls| {
            calls.into_iter().any(|c| {
                c.subtask_id.as_deref() == Some(subtask_id)
                    && !c.is_error
                    && matches!(
                        c.tool_name.as_str(),
                        "fs_write_file" | "fs_delete_file" | "system_execute_command"
                    )
            })
        })
}

/// Stores a rationale for a subtask.
pub fn remember_subtask_rationale(subtask_id: &str, rationale: &str) {
    let map = SUBTASK_RATIONALES.get_or_init(|| Mutex::new(HashMap::new()));
    let mut lock = map.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    lock.insert(subtask_id.to_string(), rationale.to_string());
}

/// Checks if a subtask has a stored rationale.
pub fn has_subtask_rationale(subtask_id: &str) -> bool {
    let map = SUBTASK_RATIONALES.get_or_init(|| Mutex::new(HashMap::new()));
    let lock = map.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    lock.contains_key(subtask_id)
}

/// Increments the failure count for a subtask and returns the new count.
pub fn increment_subtask_failure_count(subtask_id: &str) -> u32 {
    let map = SUBTASK_FAILURE_COUNTS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut lock = map.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let entry = lock.entry(subtask_id.to_string()).or_insert(0);
    *entry += 1;
    *entry
}

/// Marks a subtask as decomposed (only once per subtask).
/// Returns true if this is the first time marking, false if already marked.
pub fn mark_subtask_decomposed_once(subtask_id: &str) -> bool {
    let set = DECOMPOSED_SUBTASKS.get_or_init(|| Mutex::new(HashSet::new()));
    let mut lock = set.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if lock.contains(subtask_id) {
        return false;
    }
    lock.insert(subtask_id.to_string());
    true
}

/// Auto-decomposes a failed subtask into three smaller subtasks:
/// 1. Analyze failure cause
/// 2. Implement focused fix
/// 3. Verify outcome
pub fn auto_decompose_failed_subtask(
    session: &AgentSession,
    task_id: &str,
    failed_subtask: &crate::core::agent::task::SubTask,
    app_handle: &AppHandle,
    task_repo: &dyn TaskRepository,
) {
    let existing = task_repo.get_subtasks(task_id).ok().unwrap_or_default();
    let max_order = existing.iter().map(|s| s.order).max().unwrap_or(0);
    let base_order = max_order + 1;
    let titles = [
        format!("Analyze failure cause: {}", failed_subtask.title),
        format!("Implement focused fix: {}", failed_subtask.title),
        format!("Verify outcome: {}", failed_subtask.title),
    ];

    let mut created_ids: Vec<String> = Vec::new();
    for (idx, title) in titles.iter().enumerate() {
        let mut subtask = crate::core::agent::task::SubTask::new(
            task_id.to_string(),
            title.clone(),
            crate::core::agent::task::SubTaskMode::FullAccess,
            base_order + idx as i32,
        );
        subtask.description = Some(format!(
            "Auto-decomposed from failed subtask '{}' ({})",
            failed_subtask.title, failed_subtask.id
        ));

        if let Err(e) = task_repo.create_subtask(&subtask) {
            warn!("[AgentRuntime] Failed to auto-create decomposed subtask: {e}");
            continue;
        }

        let subtask_id = subtask.id.clone();
        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::SubtaskCreated {
                session_id: session.id.0.clone(),
                subtask,
            },
        );
        created_ids.push(subtask_id);
    }

    if !created_ids.is_empty() {
        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::AgentNarration {
                session_id: session.id.0.clone(),
                conversation_id: session.conversation_id.clone(),
                thread_id: session.thread_id.clone(),
                content: format!(
                    "Auto-decomposed failed subtask into {} new subtasks",
                    created_ids.len()
                ),
                timestamp: chrono::Utc::now(),
            },
        );
    }
}
