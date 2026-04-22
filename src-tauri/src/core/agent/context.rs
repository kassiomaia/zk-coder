//! Context-compaction domain contracts.
//!
//! Strategies for deciding whether and how to shrink the LLM context
//! window live behind the [`ContextCompactionStrategy`] trait. Concrete
//! implementations live in the infrastructure layer (see
//! `infrastructure/agent/runtime/context/strategies/`); the runtime
//! dispatcher in `infrastructure::agent::runtime::context` selects a
//! strategy and delegates both the trigger check and the compaction
//! itself to it.
//!
//! Keeping the trait in `core` means new strategies can be added
//! without touching the dispatcher (OCP) and the runtime depends on
//! this abstraction rather than a specific implementation (DIP).

use crate::core::llm::entities::ChatMessage;

/// Qualitative classification of a compaction pass. Callers rely on
/// this to decide whether to surface the event to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextPackOutcome {
    /// Packing produced a measurable token reduction.
    Compacted,
    /// Packing changed the context structure but did not reduce tokens meaningfully.
    Reorganized,
    /// Packing produced no change in counts or tokens.
    Unchanged,
}

/// Before/after metrics plus an outcome label describing a single
/// compaction pass. Returned by the `compact` arm of a strategy so the
/// caller can trace the pass and optionally emit a user-visible event.
#[derive(Debug, Clone)]
pub struct CompactionDetails {
    /// Qualitative outcome of the packing pass.
    pub outcome: ContextPackOutcome,
    /// Number of messages before compaction.
    pub before_count: usize,
    /// Number of messages after compaction.
    pub after_count: usize,
    /// Estimated tokens before compaction.
    pub before_tokens: usize,
    /// Estimated tokens after compaction.
    pub after_tokens: usize,
    /// Whether reinjection of context is needed after compaction.
    pub needs_reinjection: bool,
}

/// Read-only snapshot of the runtime state the trigger predicate
/// consults. Borrowed from the live runtime so no cloning happens on
/// the hot path.
#[derive(Debug, Clone, Copy)]
pub struct ContextState<'a> {
    /// The current in-memory message list.
    pub messages: &'a [ChatMessage],
    /// The model name — drives the token window and adaptive threshold.
    pub model_name: &'a str,
}

/// Strategy for deciding whether and how to compact the LLM context
/// window on every step.
///
/// The runtime calls [`Self::should_compact`] first; only if it
/// returns `true` does it invoke [`Self::compact`]. Implementations
/// encapsulate both the trigger predicate (token threshold, message
/// count, time-based, …) and the compaction algorithm itself.
pub trait ContextCompactionStrategy: Send + Sync {
    /// Decides whether the current context warrants compaction.
    fn should_compact(&self, state: &ContextState<'_>) -> bool;

    /// Performs compaction in place. Returns `Some` when the context
    /// was modified (including no-op reorganisations classified as
    /// `Unchanged`), `None` when the strategy decided against mutating
    /// it after all (e.g. a secondary guard like "too few messages to
    /// compact" fired).
    fn compact(&self, context: &mut Vec<ChatMessage>) -> Option<CompactionDetails>;
}
