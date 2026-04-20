//! Context management utilities for agent runtime.
//!
//! Provides context compaction and token estimation for managing the
//! LLM context window during agent execution. The logic is split across
//! focused submodules:
//!
//! - [`budget`] — per-model context-window sizes, the adaptive
//!   compaction threshold, the `ContextPolicyV2` per-block token
//!   budgets, and the `estimate_*_tokens` family.
//! - [`policy`] — V2 deterministic packing (`apply_context_policy_v2`),
//!   outcome classification, and the dialogue-tail collector that
//!   protects `tool_use` / `tool_result` pairings.
//! - [`strategies`] — concrete
//!   [`crate::core::agent::context::ContextCompactionStrategy`]
//!   implementations (V1 token-threshold today); the
//!   [`maybe_compact_context`] dispatcher below selects one and
//!   delegates both trigger and compaction to it.
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
//! need to know about the submodules. The `CompactionDetails` and
//! `ContextPackOutcome` value objects now live in
//! `core::agent::context`; they are re-exported here so the existing
//! import surface keeps working.

mod budget;
mod failure;
mod policy;
mod render;
mod strategies;

pub use crate::core::agent::context::{
    CompactionDetails, ContextCompactionStrategy, ContextPackOutcome, ContextState,
};
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
    apply_context_policy_v2, ContextPolicyInput, MIN_SAVINGS_RATIO, MIN_SAVINGS_TOKENS,
    TINY_CONTEXT_RATIO,
};
pub use render::{render_failure_memory, ExecutionBrief, MemoryReference};
pub use strategies::TokenThresholdStrategy;

use crate::core::llm::entities::ChatMessage as Message;

/// Thin dispatcher that delegates to the default
/// [`TokenThresholdStrategy`]. Runs `should_compact` first and only
/// invokes `compact` when the predicate agrees. Returns `None` when
/// either guard short-circuits the path, matching the legacy
/// `maybe_compact_context` contract.
pub fn maybe_compact_context(
    context: &mut Vec<Message>,
    model_name: &str,
) -> Option<CompactionDetails> {
    maybe_compact_with_strategy(context, model_name, &TokenThresholdStrategy)
}

/// Shared implementation used by [`maybe_compact_context`] and by
/// tests that want to exercise an explicit strategy. Borrowing
/// `&dyn ContextCompactionStrategy` means a new strategy does not
/// require a second dispatcher.
pub fn maybe_compact_with_strategy(
    context: &mut Vec<Message>,
    model_name: &str,
    strategy: &dyn ContextCompactionStrategy,
) -> Option<CompactionDetails> {
    let should_compact = {
        let state = ContextState {
            messages: context,
            model_name,
        };
        strategy.should_compact(&state)
    };
    if !should_compact {
        return None;
    }
    strategy.compact(context)
}

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
