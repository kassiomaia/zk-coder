//! Context management utilities for agent runtime.
//!
//! Provides context compaction and token estimation for managing the
//! LLM context window during agent execution. The logic is split across
//! four focused submodules:
//!
//! - [`budget`] — per-model context-window sizes, the adaptive
//!   compaction threshold, the `ContextPolicyV2` per-block token
//!   budgets, and the `estimate_*_tokens` family.
//! - [`policy`] — V1 compaction (`maybe_compact_context`), V2
//!   deterministic packing (`apply_context_policy_v2`), outcome
//!   classification, and the dialogue-tail collector that protects
//!   `tool_use` / `tool_result` pairings.
//! - [`failure`] — `FailureClass`, `FailureRecord`, `MemorizedFailure`,
//!   the subtask-scoped overlay selector, the context-scan fallback,
//!   and the arguments fingerprint used for repeat-call detection.
//! - [`render`] — `MemoryReference` and `ExecutionBrief` value objects
//!   plus the deterministic block formatters shared by the packer and
//!   the failure overlay.
//!
//! All previously-`pub` names remain reachable via
//! `infrastructure::agent::runtime::context::<Name>` through the
//! `pub use` re-exports below; callers in the application layer do not
//! need to know about the submodules.

mod budget;
mod failure;
mod policy;
mod render;

pub use budget::{
    adaptive_compression_threshold, estimate_context_tokens, estimate_message_tokens,
    model_context_window, BUDGET_ACTIVE_SUBTASK_TOKENS, BUDGET_EXECUTION_BRIEF_TOKENS,
    BUDGET_FAILURE_MEMORY_TOKENS, BUDGET_FILE_RELEVANCE_TOKENS, BUDGET_MEMORY_REFERENCE_TOKENS,
    BUDGET_RECENT_DIALOGUE_TOKENS, BUDGET_TASK_SUMMARY_TOKENS, COMPRESSION_THRESHOLD,
    CONTEXT_WINDOW, IMAGE_TOKEN_COST, KEEP_RECENT_MESSAGES, KEEP_RECENT_TAIL_MESSAGES,
    PER_MESSAGE_OVERHEAD, PER_TOOL_CALL_FRAMING, PER_TOOL_RESULT_FRAMING,
};
pub use failure::{
    extract_failure_record, select_failure_memory_body, FailureClass, FailureRecord,
    MemorizedFailure,
};
pub use policy::{
    apply_context_policy_v2, maybe_compact_context, CompactionDetails, ContextPackOutcome,
    ContextPolicyInput, MIN_SAVINGS_RATIO, MIN_SAVINGS_TOKENS, TINY_CONTEXT_RATIO,
};
pub use render::{render_failure_memory, ExecutionBrief, MemoryReference};

/// Shared text truncator used across the submodules. Thin wrapper over
/// `infrastructure::utils::truncate_text` so the submodules don't need
/// to reach across layer boundaries directly.
pub(super) fn truncate_text(text: &str, max_len: usize) -> String {
    crate::infrastructure::utils::truncate_text(text, max_len)
}

/// Truncates `text` so its estimated token cost stays under `max_tokens`,
/// trimming leading/trailing whitespace first. Converts the token budget
/// into a character budget using the same `len / 4` rule the estimator
/// uses, then delegates to `truncate_text`.
pub(super) fn truncate_to_token_budget(text: &str, max_tokens: usize) -> String {
    let max_chars = max_tokens.saturating_mul(4);
    truncate_text(text.trim(), max_chars)
}

#[cfg(test)]
mod tests;
