//! Per-session runtime scope for the agent permissions pipeline.
//!
//! Owns the small collections that were previously held in four
//! process-global `OnceLock<Mutex<_>>` trackers inside `permissions.rs`:
//!
//! - plan-retry fingerprints (single-retry window per
//!   `(session, task)` when plan validation fails)
//! - subtask rationales (set by `agent_mark_subtask_status` and later
//!   consulted by `agent_advance_to_next_subtask`)
//! - subtask failure counts (incremented on every `status = "failed"`
//!   mark, read to decide whether to auto-decompose)
//! - decomposed subtasks (so a given subtask cannot be auto-decomposed
//!   more than once within a session; see `auto_decompose_failed_subtask`)
//!
//! Scope lifetime is tied to [`ExecutionContext`]: it is constructed at
//! bootstrap, threaded through the runtime as `&mut`, and dropped when
//! the session ends. This fixes a confirmed multi-session correctness
//! bug documented in `docs/planning/permissions-decomposition/plan.md`.

use std::collections::{HashMap, HashSet};

use crate::core::agent::session::SessionId;

/// Runtime state that must survive across tool batches within a single
/// execution session but must not leak across sessions. No `Mutex`, no
/// `OnceLock`: all access goes through a single `&mut` borrow owned by
/// [`ExecutionContext`].
#[derive(Debug, Default)]
pub(crate) struct RuntimeSessionScope {
    session_id: Option<SessionId>,
    plan_retries: HashSet<String>,
    subtask_rationales: HashMap<String, String>,
    subtask_failure_counts: HashMap<String, u32>,
    decomposed_subtasks: HashSet<String>,
}

impl RuntimeSessionScope {
    /// Creates an empty scope tagged with the owning session's id.
    #[must_use]
    pub(crate) fn new(session_id: SessionId) -> Self {
        Self {
            session_id: Some(session_id),
            ..Self::default()
        }
    }

    /// Returns the scope's owning session id. Useful for diagnostics.
    #[cfg(test)]
    pub(crate) fn session_id(&self) -> Option<&SessionId> {
        self.session_id.as_ref()
    }

    /// Records a rationale written by `agent_mark_subtask_status`.
    /// Overwrites any previous rationale for the same subtask id.
    pub(crate) fn record_rationale(&mut self, subtask_id: &str, rationale: &str) {
        self.subtask_rationales
            .insert(subtask_id.to_string(), rationale.to_string());
    }

    /// Returns `true` when a rationale has been recorded for the given
    /// subtask within this session.
    pub(crate) fn has_rationale(&self, subtask_id: &str) -> bool {
        self.subtask_rationales.contains_key(subtask_id)
    }

    /// Increments and returns the new failure count for the given
    /// subtask id.
    pub(crate) fn increment_failure(&mut self, subtask_id: &str) -> u32 {
        let entry = self
            .subtask_failure_counts
            .entry(subtask_id.to_string())
            .or_insert(0);
        *entry += 1;
        *entry
    }

    /// Attempts to mark the given subtask as auto-decomposed. Returns
    /// `true` when the mark was set for the first time, `false` when
    /// the subtask was already decomposed within this session.
    pub(crate) fn try_mark_decomposed(&mut self, subtask_id: &str) -> bool {
        if self.decomposed_subtasks.contains(subtask_id) {
            return false;
        }
        self.decomposed_subtasks.insert(subtask_id.to_string());
        true
    }

    /// Attempts to mark a plan-retry window for the given fingerprint
    /// (usually `"{session_id}:{task_id}"`). Returns `true` when the
    /// fingerprint was inserted for the first time, `false` when a
    /// retry has already been observed within this session.
    pub(crate) fn mark_plan_retry(&mut self, fingerprint: &str) -> bool {
        if self.plan_retries.contains(fingerprint) {
            return false;
        }
        self.plan_retries.insert(fingerprint.to_string());
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_scope_is_empty_but_tracks_session_id() {
        let scope = RuntimeSessionScope::new(SessionId("session-1".into()));
        assert_eq!(scope.session_id().map(|s| s.0.as_str()), Some("session-1"));
        assert!(!scope.has_rationale("sub-1"));
    }

    #[test]
    fn default_scope_has_no_session_id() {
        let scope = RuntimeSessionScope::default();
        assert!(scope.session_id().is_none());
    }

    #[test]
    fn record_and_query_rationale_round_trips() {
        let mut scope = RuntimeSessionScope::new(SessionId("s".into()));
        scope.record_rationale("sub-1", "because tests failed");
        assert!(scope.has_rationale("sub-1"));
        assert!(!scope.has_rationale("sub-2"));
    }

    #[test]
    fn record_rationale_overwrites_existing_entry() {
        let mut scope = RuntimeSessionScope::new(SessionId("s".into()));
        scope.record_rationale("sub", "first");
        scope.record_rationale("sub", "second");
        assert!(scope.has_rationale("sub"));
    }

    #[test]
    fn increment_failure_counts_from_zero() {
        let mut scope = RuntimeSessionScope::new(SessionId("s".into()));
        assert_eq!(scope.increment_failure("sub"), 1);
        assert_eq!(scope.increment_failure("sub"), 2);
        assert_eq!(scope.increment_failure("sub"), 3);
        assert_eq!(scope.increment_failure("other"), 1);
    }

    #[test]
    fn try_mark_decomposed_is_idempotent() {
        let mut scope = RuntimeSessionScope::new(SessionId("s".into()));
        assert!(scope.try_mark_decomposed("sub"));
        assert!(!scope.try_mark_decomposed("sub"));
        assert!(scope.try_mark_decomposed("other"));
    }

    #[test]
    fn mark_plan_retry_is_idempotent() {
        let mut scope = RuntimeSessionScope::new(SessionId("s".into()));
        assert!(scope.mark_plan_retry("s:task-1"));
        assert!(!scope.mark_plan_retry("s:task-1"));
        assert!(scope.mark_plan_retry("s:task-2"));
    }

    #[test]
    fn scopes_are_independent_across_sessions() {
        // Guards the multi-session correctness bug fix: the new scope
        // is per-session, so two scopes do not share state.
        let mut a = RuntimeSessionScope::new(SessionId("a".into()));
        let mut b = RuntimeSessionScope::new(SessionId("b".into()));
        assert!(a.try_mark_decomposed("shared-subtask-id"));
        assert!(b.try_mark_decomposed("shared-subtask-id"));
        assert!(!a.try_mark_decomposed("shared-subtask-id"));
    }
}
