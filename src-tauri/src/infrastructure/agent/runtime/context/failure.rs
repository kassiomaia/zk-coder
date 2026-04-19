//! Failure classification, fingerprinting, and memorization.
//!
//! Owns the typed failure overlay that the tool phase appends after each
//! failed tool result, and the context-scan fallback that reconstructs a
//! `FailureRecord` from the raw message log when the overlay is empty.
//! The packer's failure-memory block (`policy::apply_context_policy_v2`)
//! reads from this module via `select_failure_memory_body`.

use serde_json::Value as JsonValue;

use crate::core::llm::entities::ChatMessage as Message;

use super::truncate_text;

/// Normalized classification of a tool failure. Drives the retryability
/// verdict carried by the M4.2 failure-memory block.
///
/// Workstream C.3 split the legacy `NotAllowed` variant into
/// `NotAllowedByMode` (the current subtask's mode rejected the tool —
/// retrying with the same subtask will fail identically) and
/// `NotAllowedByArgs` (the tool itself is permitted, but the arguments
/// hit a policy check — the caller must change arguments before
/// retrying). Each variant carries its own retry copy so the failure
/// memory no longer tells the model to "change args" when changing
/// args cannot possibly help.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    /// Target path / resource not found.
    FileNotFound,
    /// Access denied / not authorized.
    PermissionDenied,
    /// Operation exceeded its time budget.
    Timeout,
    /// Arguments failed validation (schema, format, range, etc.).
    ValidationFailed,
    /// The current subtask's mode does not permit this tool. Retrying
    /// with identical or adjusted arguments will fail identically until
    /// the subtask changes. The model must wait for a later subtask or
    /// mark this subtask `Failed`.
    NotAllowedByMode,
    /// The tool is allowed by the subtask, but the specific arguments
    /// were rejected by an argument-level policy (path outside
    /// workspace, forbidden pattern, etc.). Changing the arguments may
    /// allow a retry.
    NotAllowedByArgs,
    /// Default bucket when no specific class matches.
    Other,
}

impl FailureClass {
    /// Classifies an error message using bounded case-insensitive
    /// substring matching. Substring rules over parsing because tool
    /// errors arrive in many shapes; classification degrades to
    /// `Other` rather than failing.
    ///
    /// The `NotAllowedByMode` vs `NotAllowedByArgs` split (Workstream
    /// C.3) keys off the runtime's own error tag
    /// `subtask_mode_disallowed` — emitted by `tool_phase` when a tool
    /// name falls outside the active subtask's allowlist. Any other
    /// "not allowed" / "forbidden at argument level" phrasing falls
    /// into `NotAllowedByArgs`.
    pub(super) fn classify(error_text: &str) -> Self {
        let lower = error_text.to_lowercase();
        if lower.contains("subtask_mode_disallowed") {
            return Self::NotAllowedByMode;
        }
        if lower.contains("not allowed") {
            return Self::NotAllowedByArgs;
        }
        if lower.contains("no such file")
            || lower.contains("not found")
            || lower.contains("does not exist")
            || lower.contains("enoent")
        {
            return Self::FileNotFound;
        }
        if lower.contains("permission denied")
            || lower.contains("forbidden")
            || lower.contains("unauthorized")
            || lower.contains("eacces")
        {
            return Self::PermissionDenied;
        }
        if lower.contains("timed out") || lower.contains("timeout") {
            return Self::Timeout;
        }
        if lower.contains("invalid")
            || lower.contains("validation")
            || lower.contains("schema")
            || lower.contains("missing required")
        {
            return Self::ValidationFailed;
        }
        Self::Other
    }

    /// Stable snake_case label emitted into the failure-memory block.
    pub fn label(self) -> &'static str {
        match self {
            Self::FileNotFound => "file_not_found",
            Self::PermissionDenied => "permission_denied",
            Self::Timeout => "timeout",
            Self::ValidationFailed => "validation_failed",
            Self::NotAllowedByMode => "not_allowed_by_mode",
            Self::NotAllowedByArgs => "not_allowed_by_args",
            Self::Other => "other",
        }
    }

    /// The retry guidance line appended to the failure-memory block.
    ///
    /// Workstream C.3: the mode-disallowed case must not tell the model
    /// to "change args" — retrying with any arguments under the current
    /// subtask will produce the same rejection. The copy points the
    /// model at the two valid moves (wait for a later subtask or mark
    /// the current one `Failed`).
    pub(super) fn retry_line(self) -> &'static str {
        match self {
            Self::NotAllowedByMode => {
                "retry: do not retry — this tool is disallowed by the current subtask's mode. \
                 Wait for a later subtask or call agent_mark_subtask_status(Failed, reason)."
            }
            Self::NotAllowedByArgs => "retry: change arguments before retrying",
            Self::FileNotFound => "retry: change args before retry",
            Self::PermissionDenied => "retry: change args before retry",
            Self::ValidationFailed => "retry: change args before retry",
            Self::Timeout => "retry: safe to retry (likely transient)",
            Self::Other => "retry: safe to retry (likely transient)",
        }
    }
}

/// A single memorized tool-call failure, scoped to the subtask it
/// occurred in (Workstream C.1/C.2).
///
/// The packer's legacy path reconstructs a failure block by scanning
/// the raw `llm_context` for the most recent `is_error` tool result.
/// That scan has no notion of which subtask the failure belonged to,
/// so a failure from subtask N leaks into subtask N+1's prompt — the
/// exact regression captured in the 2026-04-18 trace (step 22
/// `fs_read_file` failure during subtask-2 surviving into subtask-3 at
/// step 35).
///
/// `MemorizedFailure` is the typed overlay that fixes the leak. The
/// tool phase appends a record after each failing tool result, capturing
/// the current `subtask_id`. The packer prefers this overlay over the
/// context scan when present, and filters to the records whose
/// `subtask_id == current_subtask_id` (or `None` for unscoped
/// failures). The raw-context scan remains as a fallback for replays,
/// tests, and crash recovery paths where the overlay is not populated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorizedFailure {
    /// Subtask that was active when this failure was recorded. `None`
    /// for failures that occurred outside any subtask (pre-planning,
    /// narration-only cycles, etc.); the filter treats unscoped
    /// failures as visible in every subtask.
    pub subtask_id: Option<String>,
    /// Failure record carrying tool name, class, args fingerprint,
    /// message, and repeat detection (mirrors the context-scan output).
    pub record: FailureRecord,
}

/// Structured breakdown of the most recent failure, used by
/// `build_failure_memory` to emit a bounded, typed M4.2 block and by
/// callers that want the components individually (for example the
/// execution brief's `current_blocker` field).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureRecord {
    /// Failing tool name (or `"tool"` if the paired call could not be
    /// located in context).
    pub tool_name: String,
    /// Stable 8-char fingerprint of the call's arguments.
    pub args_fingerprint: String,
    /// Normalized class of the failure.
    pub class: FailureClass,
    /// Truncated error message extracted from the tool result.
    pub message: String,
    /// True when a previous call with the same tool + args fingerprint
    /// already failed earlier in the same context.
    pub repeat_detected: bool,
}

/// Short stable fingerprint of a tool-call arguments JSON blob. The
/// output is 8 lowercase hex chars derived from the canonicalized JSON
/// so equivalent argument objects produce the same fingerprint across
/// runs. Used by M4.2 to detect "identical call retried" patterns.
pub(super) fn arguments_fingerprint(args: &JsonValue) -> String {
    let canonical = serde_json::to_string(args).unwrap_or_default();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in canonical.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    format!("{:08x}", hash as u32)
}

/// Walks the context backwards to find the most recent `is_error` tool
/// result and extract a structured `FailureRecord`. Returns `None` when
/// no tool error is visible. Detects repeat-failure patterns by
/// scanning earlier context for the same `tool + args_fingerprint`
/// pair paired with another error result.
pub fn extract_failure_record(context: &[Message]) -> Option<FailureRecord> {
    let mut call_map: std::collections::HashMap<String, (String, JsonValue)> =
        std::collections::HashMap::new();
    for msg in context {
        for call in &msg.tool_calls {
            call_map.insert(call.id.clone(), (call.name.clone(), call.arguments.clone()));
        }
    }

    // Error results (call_id, error_text) collected oldest → newest so
    // we can scan for prior occurrences of the latest error's
    // tool/args fingerprint.
    let mut error_results: Vec<(String, String)> = Vec::new();
    for msg in context {
        if let Some(result) = &msg.tool_result {
            if result.is_error {
                let error_text = result
                    .output
                    .get("error")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string)
                    .unwrap_or_else(|| truncate_text(&result.output.to_string(), 160));
                error_results.push((result.call_id.clone(), error_text));
            }
        }
    }
    let (last_call_id, last_error_text) = error_results.last()?.clone();
    let (tool_name, args) = call_map
        .get(&last_call_id)
        .cloned()
        .unwrap_or_else(|| ("tool".to_string(), JsonValue::Null));
    let args_fingerprint = arguments_fingerprint(&args);
    let class = FailureClass::classify(&last_error_text);
    let message = truncate_text(&last_error_text, 160);

    let mut repeat_detected = false;
    for (call_id, _error) in error_results.iter().take(error_results.len() - 1) {
        if let Some((prev_tool, prev_args)) = call_map.get(call_id) {
            if prev_tool == &tool_name && arguments_fingerprint(prev_args) == args_fingerprint {
                repeat_detected = true;
                break;
            }
        }
    }

    Some(FailureRecord {
        tool_name,
        args_fingerprint,
        class,
        message,
        repeat_detected,
    })
}

/// Builds a failure-memory block from the raw context by extracting the
/// most recent failure and rendering it via `render_failure_memory`.
/// Used as the legacy-path fallback when no `MemorizedFailure` overlay
/// is available; production paths populate the overlay and call
/// `select_failure_memory_body`.
pub(super) fn build_failure_memory(context: &[Message]) -> Option<String> {
    let record = extract_failure_record(context)?;
    Some(super::render::render_failure_memory(&record))
}

/// Chooses the failure-memory body for the current step, preferring the
/// subtask-scoped `memorized_failures` overlay (Workstream C.1/C.2)
/// over the raw context scan.
///
/// Filter rule (C.2): only entries whose `subtask_id == current_subtask_id`
/// or whose `subtask_id` is `None` (unscoped) are eligible. The most
/// recent eligible entry wins. When no entry passes the filter, the
/// packer falls back to `build_failure_memory(context)` — this keeps
/// legacy callers and crash-recovery paths working unchanged.
///
/// The fallback intentionally does *not* re-filter the raw scan by
/// subtask: that scan is subtask-blind and is only reached when no
/// typed overlay is available. Production paths that set up
/// `memorized_failures` (via the tool phase) never hit the fallback.
pub fn select_failure_memory_body(
    memorized_failures: &[MemorizedFailure],
    current_subtask_id: Option<&str>,
    context: &[Message],
) -> Option<String> {
    let scoped = memorized_failures
        .iter()
        .rev()
        .find(|entry| match (entry.subtask_id.as_deref(), current_subtask_id) {
            (None, _) => true,
            (Some(entry_id), Some(current_id)) => entry_id == current_id,
            (Some(_), None) => false,
        });
    if let Some(entry) = scoped {
        return Some(super::render::render_failure_memory(&entry.record));
    }
    if memorized_failures.is_empty() {
        return build_failure_memory(context);
    }
    None
}
