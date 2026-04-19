//! Deterministic block formatters and supporting value objects for the
//! `ContextPolicyV2` synthetic blocks.
//!
//! Owns the `MemoryReference` and `ExecutionBrief` value objects as
//! well as the formatters that turn them into byte-stable text bodies.
//! The formatters live here so the packer (`policy::apply_context_policy_v2`)
//! and the failure overlay (`failure::select_failure_memory_body`) share
//! a single source of truth for the prompt-facing copy.

use super::budget::{BUDGET_EXECUTION_BRIEF_TOKENS, BUDGET_MEMORY_REFERENCE_TOKENS};
use super::failure::FailureRecord;
use super::truncate_to_token_budget;

/// Compact descriptor of a session's durable working-memory artifact
/// (Milestone 3.2). Used by `ContextPolicyV2` to inline a deterministic
/// reference block instead of copying the full memory body into every
/// prompt.
///
/// All fields must be bounded: `task_goal` and `active_plan` are
/// truncated inside the packer; the three `_count` fields replace the
/// unbounded `Vec<String>` fields on `SessionMemory` so the block stays
/// within its token budget even as the memory grows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryReference {
    /// The session the memory is attached to.
    pub session_id: String,
    /// Task goal (short user-facing sentence). Truncated by the packer.
    pub task_goal: String,
    /// A textual summary of the active plan. Truncated by the packer.
    pub active_plan: String,
    /// Number of completed findings recorded.
    pub completed_findings_count: usize,
    /// Number of open questions recorded.
    pub open_questions_count: usize,
    /// Number of high-value extracted facts recorded.
    pub extracted_facts_count: usize,
    /// RFC 3339 timestamp of the last memory update. Stable across
    /// packing passes when the underlying memory has not changed, so it
    /// can be used to reason about cache hits.
    pub updated_at_rfc3339: String,
}

/// Compact "what matters now" snapshot inlined near the recent-context
/// tail by `apply_context_policy_v2` (Milestone 4.1). The brief
/// reintroduces the current objective close to the model's attention
/// tail so long-running execution does not lose its goal across
/// compaction. All free-text fields are truncated inside the packer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExecutionBrief {
    /// The overall task goal (short user-facing sentence).
    pub task_goal: String,
    /// Title of the currently in-progress subtask, if any.
    pub active_subtask: Option<String>,
    /// Short list of the most recent completed work items (successful
    /// tool calls, completed subtask titles). Order is oldest → newest.
    pub recently_completed: Vec<String>,
    /// The next intended action (typically the next pending subtask
    /// title). Empty when no further action is planned.
    pub next_intended_action: Option<String>,
    /// The current blocker, if any (usually a normalized error summary
    /// from `build_failure_memory`). Empty when unblocked.
    pub current_blocker: Option<String>,
}

/// Builds the deterministic text of the ContextPolicyV2 memory reference
/// block. The format is fixed so the same `MemoryReference` yields the
/// same bytes across packing passes — important for message-prefix hash
/// stability. Content is bounded to `BUDGET_MEMORY_REFERENCE_TOKENS`.
pub(super) fn format_memory_reference_block(memory: &MemoryReference) -> String {
    // Roughly split the budget between the two free-text fields and the
    // metadata line. The metadata line is short and fixed-shape, so
    // `BUDGET_MEMORY_REFERENCE_TOKENS - 60` is a generous cap for the
    // free-text portions combined.
    let goal_budget = BUDGET_MEMORY_REFERENCE_TOKENS / 3;
    let plan_budget = BUDGET_MEMORY_REFERENCE_TOKENS / 3;
    let goal = truncate_to_token_budget(&memory.task_goal, goal_budget);
    let plan = truncate_to_token_budget(&memory.active_plan, plan_budget);
    let mut out = String::from("ContextPolicyV2 session memory reference:\n");
    out.push_str(&format!(
        "session={} updated_at={} findings={} questions={} facts={}",
        memory.session_id,
        memory.updated_at_rfc3339,
        memory.completed_findings_count,
        memory.open_questions_count,
        memory.extracted_facts_count,
    ));
    if !goal.is_empty() {
        out.push_str("\ngoal: ");
        out.push_str(&goal);
    }
    if !plan.is_empty() {
        out.push_str("\nplan: ");
        out.push_str(&plan);
    }
    out
}

/// Builds the deterministic text of the ContextPolicyV2 execution brief
/// block (M4.1). Missing fields are omitted, not rendered with empty
/// values, so the block only grows when the state behind it exists.
/// The total block is capped at `BUDGET_EXECUTION_BRIEF_TOKENS`; each
/// free-text field gets a proportional sub-budget.
pub(super) fn format_execution_brief_block(brief: &ExecutionBrief) -> String {
    let per_field_budget = BUDGET_EXECUTION_BRIEF_TOKENS / 4;
    let mut out = String::from("ContextPolicyV2 execution brief:");
    let goal = truncate_to_token_budget(&brief.task_goal, per_field_budget);
    if !goal.is_empty() {
        out.push_str("\ngoal: ");
        out.push_str(&goal);
    }
    if let Some(ref active) = brief.active_subtask {
        let text = truncate_to_token_budget(active, per_field_budget);
        if !text.is_empty() {
            out.push_str("\nactive: ");
            out.push_str(&text);
        }
    }
    if !brief.recently_completed.is_empty() {
        let joined = brief
            .recently_completed
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("; ");
        let text = truncate_to_token_budget(&joined, per_field_budget);
        if !text.is_empty() {
            out.push_str("\nrecent: ");
            out.push_str(&text);
        }
    }
    if let Some(ref next) = brief.next_intended_action {
        let text = truncate_to_token_budget(next, per_field_budget);
        if !text.is_empty() {
            out.push_str("\nnext: ");
            out.push_str(&text);
        }
    }
    if let Some(ref blocker) = brief.current_blocker {
        let text = truncate_to_token_budget(blocker, per_field_budget);
        if !text.is_empty() {
            out.push_str("\nblocker: ");
            out.push_str(&text);
        }
    }
    out
}

/// Renders a single `FailureRecord` into the exact text emitted as the
/// `ContextPolicyV2 failure memory` block body. Kept separate from
/// `build_failure_memory` so the `MemorizedFailure` overlay (Workstream
/// C.1/C.2) can reuse the same copy without round-tripping through the
/// raw context scan.
pub fn render_failure_memory(record: &FailureRecord) -> String {
    let mut out = String::from("Last error:");
    out.push_str(&format!("\ntool: {}", record.tool_name));
    out.push_str(&format!("\nargs_fp: {}", record.args_fingerprint));
    out.push_str(&format!("\ntype: {}", record.class.label()));
    out.push_str(&format!("\nmessage: {}", record.message));
    out.push_str(&format!("\n{}", record.class.retry_line()));
    if record.repeat_detected {
        out.push_str("\nnote: identical call has failed before — do not retry unchanged");
    }
    out
}
