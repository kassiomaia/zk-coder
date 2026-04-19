//! Subtask scope-drift detection (Workstream B.3 + B.4).
//!
//! A write-class tool call that targets a path not matching any
//! `path_pattern` in the active subtask's contract is flagged as
//! scope drift. By default the runtime only *soft-enforces* the
//! contract: it emits a `subtask_scope_drift` trace and injects a
//! correction system message into the next step's context so the
//! model is told exactly what happened and what its options are.
//!
//! Hard enforcement is gated behind the
//! `CODER_SUBTASK_CONTRACT_HARD_ENFORCEMENT` environment variable
//! (Workstream B.4). When enabled, the originating tool result is
//! rewritten to an error with reason `subtask_scope_drift`, sharing
//! the shape of `subtask_mode_disallowed` so failure classification
//! and telemetry treat the two uniformly.
//!
//! The module is self-contained: it depends only on the `globset`
//! crate for glob matching and the already-stored contract on
//! `SubTask`. All enforcement decisions happen after the tool has
//! already been dispatched (soft enforcement) or at dispatch time
//! (hard enforcement routed from `tool_phase`). The runtime never
//! executes a write-class tool a second time because of drift.

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde_json::Value as JsonValue;

use crate::core::agent::task::{ArtifactSpec, SubTaskContract};
use crate::core::llm::entities::ToolCall;
use crate::infrastructure::tools::OrchestratorToolResult;

/// Environment variable gate for B.4 hard enforcement.
///
/// When set to `"1"` / `"true"` (case-insensitive), drifting writes
/// are rejected with `subtask_scope_drift` instead of merely
/// generating a correction message. Defaults to disabled — the
/// feature ships behind the flag until soft enforcement shows the
/// model respects the correction.
const HARD_ENFORCEMENT_ENV: &str = "CODER_SUBTASK_CONTRACT_HARD_ENFORCEMENT";

/// Soft enforcement = trace + next-step correction, no rejection.
/// Hard enforcement = rewrite the tool result as a scope-drift error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnforcementMode {
    /// Emit trace, inject correction, let the tool result pass.
    Soft,
    /// Replace the tool result with a `subtask_scope_drift` error.
    Hard,
}

impl EnforcementMode {
    /// Reads the current enforcement mode from the environment.
    ///
    /// Kept as a function so tests and callers can override via
    /// `EnforcementMode::Soft` / `::Hard` directly without mutating
    /// global state. The runtime call-site uses `from_env()` to honor
    /// the deploy-time flag.
    #[must_use]
    pub fn from_env() -> Self {
        match std::env::var(HARD_ENFORCEMENT_ENV).as_deref() {
            Ok(val) => {
                let normalized = val.trim().to_ascii_lowercase();
                if matches!(normalized.as_str(), "1" | "true" | "yes" | "on") {
                    Self::Hard
                } else {
                    Self::Soft
                }
            }
            Err(_) => Self::Soft,
        }
    }

    /// Label emitted in the `enforcement` payload field of the
    /// `subtask_scope_drift` runtime trace.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Soft => "soft",
            Self::Hard => "hard",
        }
    }
}

/// Write-class classification. Any tool that produces a durable
/// artifact (file write, document finalize, etc.) is a write-class
/// tool for scope-drift purposes.
///
/// This is intentionally listed by exact tool name rather than
/// derived from `ActionType` so a new write tool requires a conscious
/// entry here — same defense-in-depth pattern as `SubTaskMode`
/// allowlists.
#[must_use]
pub fn is_write_class_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "fs_write_file"
            | "fs_replace_in_file"
            | "fs_apply_patch"
            | "pdf_finalize"
            | "docx_finalize"
            | "pptx_finalize"
    )
}

/// The observation produced by the drift evaluator. The caller
/// (tool_phase) uses it to drive trace emission, context injection,
/// and optional hard rejection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeDriftObservation {
    /// Name of the write-class tool that fired.
    pub tool_name: String,
    /// Path the tool targeted. Empty string when the tool does not
    /// declare an obvious path in its arguments (we still emit the
    /// trace for auditability, but the match against contract
    /// patterns is vacuous).
    pub written_path: String,
    /// Copy of the contract patterns that rejected the write. Sent
    /// back to the model verbatim in the correction message so the
    /// model sees exactly what it had committed to.
    pub expected_patterns: Vec<String>,
}

/// Evaluates a successful tool call against the active subtask
/// contract. Returns `Some(observation)` only when:
///
/// 1. the tool is write-class
/// 2. the subtask has a non-empty contract
/// 3. none of the contract patterns match the extracted path
///
/// All other combinations return `None` (no drift).
#[must_use]
pub fn evaluate_scope_drift(
    call: &ToolCall,
    tool_result: &OrchestratorToolResult,
    contract: Option<&SubTaskContract>,
) -> Option<ScopeDriftObservation> {
    if tool_result.is_error {
        // Failed writes do not produce a durable artifact — there is
        // nothing on disk to drift. The failure path already surfaces
        // through the failure-memory overlay (Workstream C).
        return None;
    }
    if !is_write_class_tool(&call.name) {
        return None;
    }
    let Some(contract) = contract else {
        return None;
    };
    if contract.is_empty() {
        return None;
    }
    let written_path = extract_written_path(call, tool_result);
    let matcher = build_matcher(&contract.expected_artifacts);
    if !written_path.is_empty() && matcher.is_match(&written_path) {
        return None;
    }
    let expected_patterns: Vec<String> = contract
        .expected_artifacts
        .iter()
        .map(|spec| spec.path_pattern.clone())
        .collect();
    Some(ScopeDriftObservation {
        tool_name: call.name.clone(),
        written_path,
        expected_patterns,
    })
}

/// Builds the correction system-message body injected into the next
/// step's context. Soft enforcement uses this wording verbatim so the
/// model sees the same consequence language documented in
/// `system-prompt-architecture.md`.
#[must_use]
pub fn build_correction_message(observation: &ScopeDriftObservation) -> String {
    let patterns = if observation.expected_patterns.is_empty() {
        "(none declared)".to_string()
    } else {
        observation
            .expected_patterns
            .iter()
            .map(|p| format!("`{p}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let target = if observation.written_path.is_empty() {
        "(no path recorded)".to_string()
    } else {
        format!("`{}`", observation.written_path)
    };
    format!(
        "Subtask scope drift: the last `{tool}` write went to {target}, which is outside this \
         subtask's declared expected outputs ({patterns}). Either rescope the work to fit the \
         contract or call `agent_mark_subtask_status` with `Failed` and a reason.",
        tool = observation.tool_name,
    )
}

/// Builds the `subtask_scope_drift` error payload used when hard
/// enforcement rewrites the tool result. Shape mirrors
/// `subtask_mode_disallowed` so the classifier in
/// `FailureClass::classify` can route it into the same `NotAllowed`
/// branch without special-casing.
#[must_use]
pub fn hard_enforced_error_result(
    call: &ToolCall,
    observation: &ScopeDriftObservation,
) -> OrchestratorToolResult {
    let patterns = observation
        .expected_patterns
        .iter()
        .map(|p| serde_json::Value::String(p.clone()))
        .collect::<Vec<_>>();
    OrchestratorToolResult {
        call_id: call.id.clone(),
        name: call.name.clone(),
        output: serde_json::json!({
            "error": format!(
                "Tool `{tool}` attempted to write `{path}`, which is outside this \
                 subtask's declared expected outputs. Rescope the work, or mark the \
                 subtask Failed with a reason.",
                tool = observation.tool_name,
                path = if observation.written_path.is_empty() {
                    "(no path recorded)"
                } else {
                    &observation.written_path
                },
            ),
            "reason": "subtask_scope_drift",
            "writtenPath": observation.written_path,
            "expectedPatterns": patterns,
        }),
        is_error: true,
    }
}

/// Extracts the most useful "path" from the tool call and its
/// result. Different write-class tools carry the path in different
/// fields; the walk below covers every current registered shape:
///
/// - `fs_write_file`, `fs_replace_in_file`, `fs_apply_patch` —
///   arguments `path` / `file_path`
/// - `pdf_finalize`, `docx_finalize`, `pptx_finalize` — the result
///   object's `path` field (the renderer writes to a deterministic
///   artifacts folder; that path is the shipped artifact).
///
/// Returns the empty string when no path is available. The caller
/// still evaluates drift in that case — the contract has declared
/// expectations and the tool produced something untracked, which is
/// itself a signal worth surfacing.
fn extract_written_path(call: &ToolCall, tool_result: &OrchestratorToolResult) -> String {
    if let Some(path) = read_str(&call.arguments, "path") {
        return path;
    }
    if let Some(path) = read_str(&call.arguments, "file_path") {
        return path;
    }
    if let Some(path) = read_str(&tool_result.output, "path") {
        return path;
    }
    String::new()
}

fn read_str(value: &JsonValue, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn build_matcher(specs: &[ArtifactSpec]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for spec in specs {
        if let Ok(glob) = Glob::new(&spec.path_pattern) {
            builder.add(glob);
        }
    }
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::task::{ArtifactKind, ArtifactSpec, SubTaskContract};
    use serde_json::json;

    fn tool_call(name: &str, args: JsonValue) -> ToolCall {
        ToolCall {
            id: format!("call-{name}"),
            name: name.to_string(),
            arguments: args,
        }
    }

    fn ok_result(name: &str, output: JsonValue) -> OrchestratorToolResult {
        OrchestratorToolResult {
            call_id: format!("call-{name}"),
            name: name.to_string(),
            output,
            is_error: false,
        }
    }

    fn file_contract(pattern: &str) -> SubTaskContract {
        SubTaskContract {
            expected_artifacts: vec![ArtifactSpec {
                path_pattern: pattern.to_string(),
                kind: ArtifactKind::File,
            }],
        }
    }

    #[test]
    fn write_outside_contract_is_drift() {
        // Replay of the 2026-04-18 session (plan.md P2): active
        // subtask says "social media copy", but the call writes to
        // an `email/*` path. That is scope drift.
        let contract = file_contract("docs/social/*.md");
        let call = tool_call(
            "fs_write_file",
            json!({"path": "docs/email/launch.md", "content": "x"}),
        );
        let result = ok_result("fs_write_file", json!({"path": "docs/email/launch.md"}));
        let obs =
            evaluate_scope_drift(&call, &result, Some(&contract)).expect("drift must trigger");
        assert_eq!(obs.tool_name, "fs_write_file");
        assert_eq!(obs.written_path, "docs/email/launch.md");
        assert_eq!(obs.expected_patterns, vec!["docs/social/*.md".to_string()]);
    }

    #[test]
    fn write_matching_contract_is_not_drift() {
        let contract = file_contract("docs/social/*.md");
        let call = tool_call(
            "fs_write_file",
            json!({"path": "docs/social/twitter-launch.md", "content": "x"}),
        );
        let result = ok_result(
            "fs_write_file",
            json!({"path": "docs/social/twitter-launch.md"}),
        );
        assert!(evaluate_scope_drift(&call, &result, Some(&contract)).is_none());
    }

    #[test]
    fn recursive_glob_pattern_matches_nested_paths() {
        let contract = file_contract("reports/**/*.md");
        let call = tool_call(
            "fs_write_file",
            json!({"path": "reports/2026/q1/summary.md", "content": "x"}),
        );
        let result = ok_result("fs_write_file", json!({}));
        assert!(evaluate_scope_drift(&call, &result, Some(&contract)).is_none());
    }

    #[test]
    fn read_class_tools_never_trigger_drift() {
        // The detector only fires on write-class tools. Reads, even
        // to weird paths, must never produce a drift observation.
        let contract = file_contract("docs/**");
        let call = tool_call("fs_read_file", json!({"path": "src/main.rs"}));
        let result = ok_result("fs_read_file", json!({"content": ""}));
        assert!(evaluate_scope_drift(&call, &result, Some(&contract)).is_none());
    }

    #[test]
    fn failed_tool_results_never_trigger_drift() {
        let contract = file_contract("docs/**");
        let call = tool_call("fs_write_file", json!({"path": "out.bin", "content": ""}));
        let result = OrchestratorToolResult {
            call_id: "call-fs_write_file".to_string(),
            name: "fs_write_file".to_string(),
            output: json!({"error": "boom"}),
            is_error: true,
        };
        assert!(evaluate_scope_drift(&call, &result, Some(&contract)).is_none());
    }

    #[test]
    fn absent_contract_is_a_no_op() {
        let call = tool_call("fs_write_file", json!({"path": "x.md", "content": ""}));
        let result = ok_result("fs_write_file", json!({}));
        assert!(evaluate_scope_drift(&call, &result, None).is_none());
    }

    #[test]
    fn empty_contract_is_a_no_op() {
        let contract = SubTaskContract {
            expected_artifacts: vec![],
        };
        let call = tool_call("fs_write_file", json!({"path": "x.md", "content": ""}));
        let result = ok_result("fs_write_file", json!({}));
        assert!(evaluate_scope_drift(&call, &result, Some(&contract)).is_none());
    }

    #[test]
    fn pdf_finalize_reads_path_from_result_when_arguments_lack_it() {
        // `pdf_finalize` receives a `pdf_id` in arguments but emits
        // the final path in the result object. The extractor must
        // walk the result to find it.
        let contract = file_contract("artifacts/reports/*.pdf");
        let call = tool_call("pdf_finalize", json!({"pdf_id": "pdf_123"}));
        let result = ok_result(
            "pdf_finalize",
            json!({"pdf_id": "pdf_123", "path": "artifacts/reports/launch.pdf"}),
        );
        assert!(
            evaluate_scope_drift(&call, &result, Some(&contract)).is_none(),
            "pdf_finalize path from result must match the contract"
        );

        let drifted = ok_result(
            "pdf_finalize",
            json!({"pdf_id": "pdf_123", "path": "artifacts/scratch/draft.pdf"}),
        );
        let obs = evaluate_scope_drift(&call, &drifted, Some(&contract))
            .expect("out-of-contract path must trigger");
        assert_eq!(obs.written_path, "artifacts/scratch/draft.pdf");
    }

    #[test]
    fn write_without_any_path_still_surfaces_as_drift() {
        // Defense-in-depth: the model invokes a write-class tool
        // without declaring a path. The contract expected
        // something; the tool produced nothing we can audit.
        // Surfacing drift keeps the signal honest rather than
        // silently passing.
        let contract = file_contract("docs/**/*.md");
        let call = tool_call("fs_write_file", json!({"content": "stranded"}));
        let result = ok_result("fs_write_file", json!({}));
        let obs =
            evaluate_scope_drift(&call, &result, Some(&contract)).expect("must surface drift");
        assert_eq!(obs.written_path, "");
    }

    #[test]
    fn correction_message_cites_tool_path_and_patterns() {
        let obs = ScopeDriftObservation {
            tool_name: "fs_write_file".to_string(),
            written_path: "email/outreach.md".to_string(),
            expected_patterns: vec!["docs/social/*.md".to_string()],
        };
        let msg = build_correction_message(&obs);
        assert!(msg.contains("fs_write_file"));
        assert!(msg.contains("email/outreach.md"));
        assert!(msg.contains("docs/social/*.md"));
        assert!(msg.contains("agent_mark_subtask_status"));
    }

    #[test]
    fn hard_enforced_error_carries_structured_reason() {
        let call = tool_call(
            "fs_write_file",
            json!({"path": "email/outreach.md", "content": "x"}),
        );
        let obs = ScopeDriftObservation {
            tool_name: "fs_write_file".to_string(),
            written_path: "email/outreach.md".to_string(),
            expected_patterns: vec!["docs/social/*.md".to_string()],
        };
        let err = hard_enforced_error_result(&call, &obs);
        assert!(err.is_error);
        assert_eq!(
            err.output.get("reason").and_then(|v| v.as_str()),
            Some("subtask_scope_drift"),
        );
        assert_eq!(
            err.output.get("writtenPath").and_then(|v| v.as_str()),
            Some("email/outreach.md"),
        );
        let patterns = err
            .output
            .get("expectedPatterns")
            .and_then(|v| v.as_array())
            .expect("expectedPatterns array");
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].as_str(), Some("docs/social/*.md"));
    }

    #[test]
    fn enforcement_mode_defaults_to_soft_when_flag_unset() {
        // Snapshot the env var so the test is self-contained. The
        // `remove_var` after the assertion restores a clean state.
        let prior = std::env::var(HARD_ENFORCEMENT_ENV).ok();
        std::env::remove_var(HARD_ENFORCEMENT_ENV);
        assert_eq!(EnforcementMode::from_env(), EnforcementMode::Soft);
        if let Some(value) = prior {
            std::env::set_var(HARD_ENFORCEMENT_ENV, value);
        }
    }

    #[test]
    fn enforcement_mode_hard_when_flag_is_truthy() {
        let prior = std::env::var(HARD_ENFORCEMENT_ENV).ok();
        std::env::set_var(HARD_ENFORCEMENT_ENV, "1");
        assert_eq!(EnforcementMode::from_env(), EnforcementMode::Hard);
        std::env::set_var(HARD_ENFORCEMENT_ENV, "true");
        assert_eq!(EnforcementMode::from_env(), EnforcementMode::Hard);
        std::env::set_var(HARD_ENFORCEMENT_ENV, "TRUE");
        assert_eq!(EnforcementMode::from_env(), EnforcementMode::Hard);
        std::env::set_var(HARD_ENFORCEMENT_ENV, "no");
        assert_eq!(EnforcementMode::from_env(), EnforcementMode::Soft);
        if let Some(value) = prior {
            std::env::set_var(HARD_ENFORCEMENT_ENV, value);
        } else {
            std::env::remove_var(HARD_ENFORCEMENT_ENV);
        }
    }

    /// Simulates the core loop that `tool_phase::apply_scope_drift_detection`
    /// runs across the tool batch, minus the trace/deps coupling. This
    /// is the shape-level invariant: one correction per drifting call,
    /// hard mode mutates the result in place, successful in-contract
    /// writes are left untouched.
    fn apply_enforcement(
        calls: &[ToolCall],
        results: &mut [OrchestratorToolResult],
        contract: Option<&SubTaskContract>,
        mode: EnforcementMode,
    ) -> Vec<String> {
        let mut corrections = Vec::new();
        for (call, result) in calls.iter().zip(results.iter_mut()) {
            let Some(obs) = evaluate_scope_drift(call, result, contract) else {
                continue;
            };
            if mode == EnforcementMode::Hard {
                *result = hard_enforced_error_result(call, &obs);
            }
            corrections.push(build_correction_message(&obs));
        }
        corrections
    }

    #[test]
    fn soft_enforcement_appends_correction_without_rewriting_the_result() {
        let contract = file_contract("docs/social/*.md");
        let calls = vec![
            tool_call(
                "fs_write_file",
                json!({"path": "docs/email/outreach.md", "content": "x"}),
            ),
            tool_call("fs_read_file", json!({"path": "README.md"})),
        ];
        let mut results = vec![
            ok_result("fs_write_file", json!({"path": "docs/email/outreach.md"})),
            ok_result("fs_read_file", json!({"content": ""})),
        ];
        let corrections =
            apply_enforcement(&calls, &mut results, Some(&contract), EnforcementMode::Soft);
        assert_eq!(corrections.len(), 1, "exactly one drift must fire");
        assert!(corrections[0].contains("fs_write_file"));
        // Soft mode leaves the original success untouched.
        assert!(!results[0].is_error, "soft mode must not rewrite results");
        assert_eq!(
            results[0].output.get("path").and_then(|v| v.as_str()),
            Some("docs/email/outreach.md"),
        );
    }

    #[test]
    fn hard_enforcement_rewrites_drifting_result_to_error() {
        let contract = file_contract("docs/social/*.md");
        let calls = vec![tool_call(
            "fs_write_file",
            json!({"path": "docs/email/outreach.md", "content": "x"}),
        )];
        let mut results = vec![ok_result(
            "fs_write_file",
            json!({"path": "docs/email/outreach.md"}),
        )];
        let corrections =
            apply_enforcement(&calls, &mut results, Some(&contract), EnforcementMode::Hard);
        assert_eq!(corrections.len(), 1);
        assert!(
            results[0].is_error,
            "hard mode must rewrite a drifting result into an error"
        );
        assert_eq!(
            results[0].output.get("reason").and_then(|v| v.as_str()),
            Some("subtask_scope_drift"),
        );
    }

    #[test]
    fn enforcement_is_a_no_op_when_contract_is_absent() {
        let calls = vec![tool_call(
            "fs_write_file",
            json!({"path": "whatever.md", "content": ""}),
        )];
        let mut results = vec![ok_result("fs_write_file", json!({"path": "whatever.md"}))];
        let corrections = apply_enforcement(&calls, &mut results, None, EnforcementMode::Hard);
        assert!(corrections.is_empty());
        assert!(!results[0].is_error);
    }

    #[test]
    fn is_write_class_tool_covers_document_finalizers() {
        for name in [
            "fs_write_file",
            "fs_replace_in_file",
            "fs_apply_patch",
            "pdf_finalize",
            "docx_finalize",
            "pptx_finalize",
        ] {
            assert!(is_write_class_tool(name), "{name} must be write-class");
        }
        for name in [
            "fs_read_file",
            "search_grep",
            "agent_emit_status",
            "pdf_add_page",
            "docx_add_section",
            "pptx_add_slide",
        ] {
            assert!(
                !is_write_class_tool(name),
                "{name} must not be write-class — drafts do not produce shipped artifacts"
            );
        }
    }
}
