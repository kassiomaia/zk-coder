//! Drift-aware nudge for repeated `fs_replace_in_file` misses
//! (Workstream γ.2).
//!
//! When two `fs_replace_in_file` calls targeting the same file miss
//! within a five-step window, the file contents are almost certainly
//! out of sync with the model's working snapshot. Silently letting the
//! model keep guessing burns steps and produces telemetry full of
//! "Replacement text not found" noise.
//!
//! This module mirrors the Workstream B.3 scope-drift correction:
//! soft enforcement only, no hard rejection. After the second miss in
//! the window, the evaluator emits a correction system message that
//! `tool_phase::run_tools` appends to the LLM context. The tracker is
//! cleared for the offending path on emit so a *third* miss does not
//! re-fire the same nudge — the tracker has to refill naturally before
//! another nudge is allowed.
//!
//! No tool is re-invoked, the miss result is left intact, and the
//! success path is untouched.

use crate::core::llm::entities::ToolCall;
use crate::infrastructure::tools::OrchestratorToolResult;

/// How many miss records we keep per task. A steady stream of
/// unrelated misses must not starve newer observations.
pub const REPLACE_FAILURE_TRACKER_CAP: usize = 16;

/// Sliding window (in step numbers) within which two misses on the
/// same file trigger the nudge. Matches the Workstream γ.2 spec.
pub const REPLACE_FAILURE_WINDOW_STEPS: u32 = 5;

/// Minimum number of misses on a single path within the window that
/// triggers a correction. Kept as a named constant so both the
/// evaluator and tests agree on the threshold.
const REPLACE_FAILURE_TRIGGER: usize = 2;

/// Tool name the tracker reacts to. Centralized so a future rename
/// propagates cleanly.
const REPLACE_TOOL_NAME: &str = "fs_replace_in_file";

/// Single recorded `fs_replace_in_file` miss. Stored on the
/// `ExecutionContext` across steps. The tracker reads the `step`
/// attribute to drop stale entries that fell outside the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceFailure {
    /// Target path of the replace call. Extracted from the tool
    /// arguments; empty-string paths are ignored (nothing to compare).
    pub path: String,
    /// Step number at which the miss happened. Compared against the
    /// current step to age out stale entries.
    pub step_number: u32,
}

/// Result of evaluating the tracker at the end of a tool phase.
///
/// `nudge` is populated when the latest miss was the
/// `REPLACE_FAILURE_TRIGGER`-th miss on the same path within the
/// window. The caller appends it as a system message to the LLM
/// context and moves on. `cleared_path` is reported for telemetry /
/// testing so the caller can prove the tracker really did drop the
/// offending entries on emit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceDriftOutcome {
    /// Correction body to inject into the next LLM step. Empty when
    /// the tracker did not fire.
    pub nudge: Option<String>,
    /// Path for which the nudge fired (if any). The tracker drops all
    /// entries with this path so the same correction does not re-fire
    /// on a later miss without a fresh pair.
    pub cleared_path: Option<String>,
}

impl ReplaceDriftOutcome {
    /// Convenience: no nudge produced.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            nudge: None,
            cleared_path: None,
        }
    }
}

/// Extracts the target path out of an `fs_replace_in_file` tool call.
/// Returns `None` when the call lacks a usable `path` argument.
#[must_use]
pub fn extract_replace_path(call: &ToolCall) -> Option<String> {
    if call.name != REPLACE_TOOL_NAME {
        return None;
    }
    let raw = call
        .arguments
        .get("path")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())?;
    if raw.is_empty() {
        None
    } else {
        Some(raw)
    }
}

/// Determines whether a `(call, result)` pair represents a failed
/// `fs_replace_in_file` invocation. Success paths and non-replace
/// tools return `false` — the tracker only records genuine misses.
#[must_use]
pub fn is_replace_miss(call: &ToolCall, result: &OrchestratorToolResult) -> bool {
    call.name == REPLACE_TOOL_NAME && result.is_error
}

/// Drops entries whose `step_number` falls outside the current window.
/// Called once at the start of drift evaluation so stale pairings do
/// not retroactively trigger a nudge.
pub fn prune_stale(tracker: &mut Vec<ReplaceFailure>, current_step: u32) {
    tracker.retain(|entry| {
        current_step.saturating_sub(entry.step_number) < REPLACE_FAILURE_WINDOW_STEPS
    });
}

/// Evaluates the tracker after a tool phase and returns the correction
/// outcome.
///
/// Contract:
/// - Stale entries are pruned first (see [`prune_stale`]).
/// - New miss records from `(calls, results)` are appended.
/// - The tracker is capped at [`REPLACE_FAILURE_TRACKER_CAP`].
/// - If the *latest* appended entry is the
///   `REPLACE_FAILURE_TRIGGER`-th miss on its path inside the window,
///   the nudge is emitted *once* and every entry for that path is
///   dropped.
///
/// The nudge text is documented in `docs/planning/agent-fidelity-followup/plan.md`.
#[must_use]
pub fn evaluate_replace_drift(
    tracker: &mut Vec<ReplaceFailure>,
    calls: &[ToolCall],
    results: &[OrchestratorToolResult],
    current_step: u32,
) -> ReplaceDriftOutcome {
    prune_stale(tracker, current_step);

    let mut triggering_path: Option<String> = None;
    for (call, result) in calls.iter().zip(results.iter()) {
        if !is_replace_miss(call, result) {
            continue;
        }
        let Some(path) = extract_replace_path(call) else {
            continue;
        };
        tracker.push(ReplaceFailure {
            path: path.clone(),
            step_number: current_step,
        });
        if tracker.len() > REPLACE_FAILURE_TRACKER_CAP {
            let drop = tracker.len() - REPLACE_FAILURE_TRACKER_CAP;
            tracker.drain(0..drop);
        }
        let count = tracker
            .iter()
            .filter(|entry| {
                entry.path == path
                    && current_step.saturating_sub(entry.step_number) < REPLACE_FAILURE_WINDOW_STEPS
            })
            .count();
        if count >= REPLACE_FAILURE_TRIGGER && triggering_path.is_none() {
            triggering_path = Some(path);
        }
    }

    let Some(path) = triggering_path else {
        return ReplaceDriftOutcome::none();
    };
    tracker.retain(|entry| entry.path != path);
    ReplaceDriftOutcome {
        nudge: Some(build_correction_message(&path)),
        cleared_path: Some(path),
    }
}

/// Builds the correction system-message body. Kept as a free function
/// so tests can lock the exact wording without needing to thread an
/// observation through a struct.
#[must_use]
pub fn build_correction_message(path: &str) -> String {
    format!(
        "The last two replacements on `{path}` failed with \"Replacement text not found\". \
         The file is almost certainly out of sync with your working model. Read it fresh \
         with `fs_read_file` before your next edit."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn call(path: &str) -> ToolCall {
        ToolCall {
            id: format!("call-{path}"),
            name: REPLACE_TOOL_NAME.to_string(),
            arguments: json!({ "path": path, "replacements": [{ "old": "x", "new": "y" }] }),
        }
    }

    fn miss_result(path: &str) -> OrchestratorToolResult {
        OrchestratorToolResult {
            call_id: format!("call-{path}"),
            name: REPLACE_TOOL_NAME.to_string(),
            output: json!({ "error": "Replacement text not found in ..." }),
            is_error: true,
        }
    }

    fn success_result(path: &str) -> OrchestratorToolResult {
        OrchestratorToolResult {
            call_id: format!("call-{path}"),
            name: REPLACE_TOOL_NAME.to_string(),
            output: json!({ "path": path, "changed": true }),
            is_error: false,
        }
    }

    #[test]
    fn first_miss_on_a_file_does_not_trigger_nudge() {
        let mut tracker = Vec::new();
        let calls = vec![call("src/lib.rs")];
        let results = vec![miss_result("src/lib.rs")];
        let outcome = evaluate_replace_drift(&mut tracker, &calls, &results, 3);
        assert!(outcome.nudge.is_none(), "first miss must not nudge");
        assert!(outcome.cleared_path.is_none());
        assert_eq!(tracker.len(), 1);
    }

    #[test]
    fn second_miss_on_same_file_within_window_fires_once_and_clears_tracker() {
        let mut tracker = vec![ReplaceFailure {
            path: "src/lib.rs".to_string(),
            step_number: 10,
        }];
        let calls = vec![call("src/lib.rs")];
        let results = vec![miss_result("src/lib.rs")];
        let outcome = evaluate_replace_drift(&mut tracker, &calls, &results, 12);
        let nudge = outcome.nudge.expect("second miss must nudge");
        assert!(nudge.contains("`src/lib.rs`"));
        assert!(nudge.contains("fs_read_file"));
        assert!(nudge.contains("out of sync"));
        assert_eq!(outcome.cleared_path.as_deref(), Some("src/lib.rs"));
        assert!(
            tracker.iter().all(|e| e.path != "src/lib.rs"),
            "tracker must drop entries for the cleared path"
        );
    }

    #[test]
    fn misses_on_different_files_do_not_fire_nudge() {
        let mut tracker = vec![ReplaceFailure {
            path: "src/a.rs".to_string(),
            step_number: 5,
        }];
        let calls = vec![call("src/b.rs")];
        let results = vec![miss_result("src/b.rs")];
        let outcome = evaluate_replace_drift(&mut tracker, &calls, &results, 6);
        assert!(outcome.nudge.is_none());
        assert_eq!(tracker.len(), 2);
    }

    #[test]
    fn stale_entry_outside_window_is_pruned_before_evaluation() {
        // The prior miss on src/lib.rs happened 6 steps ago — outside
        // the 5-step window. A fresh miss must *not* combine with the
        // pruned entry.
        let mut tracker = vec![ReplaceFailure {
            path: "src/lib.rs".to_string(),
            step_number: 5,
        }];
        let calls = vec![call("src/lib.rs")];
        let results = vec![miss_result("src/lib.rs")];
        let outcome = evaluate_replace_drift(&mut tracker, &calls, &results, 11);
        assert!(
            outcome.nudge.is_none(),
            "stale pair must not trigger — it's outside the window"
        );
        // One fresh entry remains; the stale one was pruned.
        assert_eq!(tracker.len(), 1);
        assert_eq!(tracker[0].step_number, 11);
    }

    #[test]
    fn three_misses_in_window_produce_exactly_one_nudge() {
        // Two misses on step 10 plus a third on step 11 — the nudge
        // must fire exactly once and the tracker must empty for that
        // path so a fourth miss can re-arm cleanly.
        let mut tracker = Vec::new();
        let calls_round_1 = vec![call("src/lib.rs"), call("src/lib.rs")];
        let results_round_1 = vec![miss_result("src/lib.rs"), miss_result("src/lib.rs")];
        let first = evaluate_replace_drift(&mut tracker, &calls_round_1, &results_round_1, 10);
        assert!(first.nudge.is_some(), "first pair must nudge");
        assert!(tracker.is_empty(), "tracker must be cleared after firing");

        let calls_round_2 = vec![call("src/lib.rs")];
        let results_round_2 = vec![miss_result("src/lib.rs")];
        let second = evaluate_replace_drift(&mut tracker, &calls_round_2, &results_round_2, 11);
        assert!(
            second.nudge.is_none(),
            "single post-nudge miss must not re-fire"
        );
    }

    #[test]
    fn successful_replace_does_not_record_failure() {
        let mut tracker = vec![ReplaceFailure {
            path: "src/lib.rs".to_string(),
            step_number: 10,
        }];
        let calls = vec![call("src/lib.rs")];
        let results = vec![success_result("src/lib.rs")];
        let outcome = evaluate_replace_drift(&mut tracker, &calls, &results, 11);
        assert!(outcome.nudge.is_none());
        assert_eq!(tracker.len(), 1, "success path must not append a record");
    }

    #[test]
    fn other_tool_misses_are_ignored() {
        let mut tracker = vec![ReplaceFailure {
            path: "src/lib.rs".to_string(),
            step_number: 10,
        }];
        let other = ToolCall {
            id: "call-other".to_string(),
            name: "fs_write_file".to_string(),
            arguments: json!({ "path": "src/lib.rs", "content": "x" }),
        };
        let other_miss = OrchestratorToolResult {
            call_id: "call-other".to_string(),
            name: "fs_write_file".to_string(),
            output: json!({ "error": "boom" }),
            is_error: true,
        };
        let outcome = evaluate_replace_drift(&mut tracker, &[other], &[other_miss], 11);
        assert!(outcome.nudge.is_none());
        assert_eq!(tracker.len(), 1);
    }

    #[test]
    fn extract_replace_path_rejects_empty_and_non_replace_tools() {
        let none_tool = ToolCall {
            id: "x".to_string(),
            name: "fs_read_file".to_string(),
            arguments: json!({ "path": "x.md" }),
        };
        assert!(extract_replace_path(&none_tool).is_none());

        let empty_path = ToolCall {
            id: "x".to_string(),
            name: REPLACE_TOOL_NAME.to_string(),
            arguments: json!({ "path": "   " }),
        };
        assert!(extract_replace_path(&empty_path).is_none());
    }

    #[test]
    fn tracker_respects_cap() {
        // Push more misses than the cap; the tracker must never
        // exceed it.
        let mut tracker = Vec::new();
        for step in 0..(REPLACE_FAILURE_TRACKER_CAP + 5) {
            let path = format!("src/file_{step}.rs");
            let calls = vec![call(&path)];
            let results = vec![miss_result(&path)];
            let _ = evaluate_replace_drift(&mut tracker, &calls, &results, step as u32);
        }
        assert!(tracker.len() <= REPLACE_FAILURE_TRACKER_CAP);
    }
}
