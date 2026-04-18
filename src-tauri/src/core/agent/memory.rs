//! Durable working-memory artifact for long-running agent sessions.
//!
//! A `SessionMemory` is a runtime-owned record — one per session — that
//! captures what the model has learned so far: the task goal, the active
//! plan, completed findings, open questions, and high-value extracted
//! facts. It is created when a task crosses a complexity threshold
//! (`is_non_trivial`) and survives compaction through deterministic
//! references in the packed context (see `apply_context_policy_v2`).
//!
//! Ownership stays in backend orchestration rather than being delegated
//! to ad hoc agent behavior: the runtime decides when the memory is
//! created, updated, or cleared. This keeps the artifact predictable and
//! testable. See `docs/planning/agent-context-engineering/plan.md`
//! (Milestone 3).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::repositories::RepositoryResult;
use super::session::SessionId;

/// Durable working-memory artifact for a single agent session.
///
/// The shape is intentionally flat: each field is independently
/// serializable so repository implementations can store them as JSON-
/// encoded columns without schema drift. Order of items in each `Vec`
/// must be preserved on round-trip because tests rely on deterministic
/// round-tripping (idempotent upsert).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMemory {
    /// The session this memory belongs to.
    pub session_id: SessionId,
    /// The task this memory was initialized for.
    pub task_id: String,
    /// The overall goal of the task, as given by the user (or as refined
    /// by the initial plan).
    pub task_goal: String,
    /// The active plan, usually a short textual summary of the current
    /// subtask list. Kept as free text so the packer can truncate to a
    /// token budget without schema churn.
    pub active_plan: String,
    /// Findings produced by completed subtasks. Append-only in normal
    /// operation; the runtime owns when entries are added.
    pub completed_findings: Vec<String>,
    /// Open questions the agent has flagged for itself. Used by the
    /// context packer to remind the model of unresolved items.
    pub open_questions: Vec<String>,
    /// High-value extracted facts (file paths, function names, version
    /// numbers, etc.) that must survive compaction.
    pub extracted_facts: Vec<String>,
    /// When this memory was first created.
    pub created_at: DateTime<Utc>,
    /// When this memory was last updated. Equal to `created_at` on a
    /// freshly created record.
    pub updated_at: DateTime<Utc>,
}

impl SessionMemory {
    /// Builds a fresh memory record keyed to a session and task with
    /// empty finding/question/fact lists.
    #[must_use]
    pub fn new(session_id: SessionId, task_id: String, task_goal: String, active_plan: String) -> Self {
        let now = Utc::now();
        Self {
            session_id,
            task_id,
            task_goal,
            active_plan,
            completed_findings: Vec::new(),
            open_questions: Vec::new(),
            extracted_facts: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Returns `true` when no findings, questions, or facts have been
    /// recorded yet — i.e., the memory is the bootstrap-time skeleton.
    /// Used by the context packer to decide whether the reference block
    /// is worth inlining.
    #[must_use]
    pub fn is_skeleton(&self) -> bool {
        self.completed_findings.is_empty()
            && self.open_questions.is_empty()
            && self.extracted_facts.is_empty()
    }

    /// Count of "knowledge items" across the three user-visible lists.
    /// Drives the compact summary in the context reference block.
    #[must_use]
    pub fn knowledge_item_count(&self) -> usize {
        self.completed_findings.len() + self.open_questions.len() + self.extracted_facts.len()
    }
}

/// Repository for `SessionMemory` records.
///
/// Implementations must provide:
///
/// 1. **Idempotent upsert** — calling `upsert` twice with the same
///    record produces the same row (apart from `updated_at` moving
///    forward).
/// 2. **Cleanup on session delete** — memory rows must be removed when
///    the owning session is removed. This is easiest to enforce at the
///    database level (e.g., `ON DELETE CASCADE`).
/// 3. **Explicit not-found** — `get` returns `Ok(None)` when no memory
///    exists for the session, not an error. A missing memory is the
///    normal case for trivial tasks.
pub trait MemoryRepository: Send + Sync {
    /// Fetches the current memory for a session, if one exists.
    fn get_memory(&self, session_id: &SessionId) -> RepositoryResult<Option<SessionMemory>>;

    /// Creates or updates the memory record for a session. Implementors
    /// must bump `updated_at` to the current wall clock on write.
    fn upsert_memory(&self, memory: &SessionMemory) -> RepositoryResult<()>;

    /// Removes the memory record for a session. A no-op when no record
    /// exists, so callers can use this as a cleanup step without
    /// branching on existence.
    fn delete_memory(&self, session_id: &SessionId) -> RepositoryResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(session_id: &str, task_id: &str) -> SessionMemory {
        SessionMemory::new(
            SessionId::from(session_id),
            task_id.to_string(),
            "ship the feature".to_string(),
            "subtask A; subtask B".to_string(),
        )
    }

    #[test]
    fn newly_created_memory_is_a_skeleton() {
        let memory = sample("sess-1", "task-1");
        assert!(memory.is_skeleton());
        assert_eq!(memory.knowledge_item_count(), 0);
    }

    #[test]
    fn memory_stops_being_a_skeleton_once_any_list_has_entries() {
        let mut memory = sample("sess-1", "task-1");
        memory.extracted_facts.push("auth.rs:42".to_string());
        assert!(!memory.is_skeleton());
        assert_eq!(memory.knowledge_item_count(), 1);
    }

    #[test]
    fn knowledge_item_count_sums_all_three_lists() {
        let mut memory = sample("sess-1", "task-1");
        memory.completed_findings.push("f1".to_string());
        memory.completed_findings.push("f2".to_string());
        memory.open_questions.push("q1".to_string());
        memory.extracted_facts.push("x1".to_string());
        memory.extracted_facts.push("x2".to_string());
        memory.extracted_facts.push("x3".to_string());
        assert_eq!(memory.knowledge_item_count(), 6);
    }

    #[test]
    fn fresh_memory_created_and_updated_at_match() {
        // The bootstrap contract depends on this: consumers of a
        // brand-new record should see a single wall-clock stamp.
        let memory = sample("sess-1", "task-1");
        assert_eq!(memory.created_at, memory.updated_at);
    }

    #[test]
    fn serialization_roundtrip_preserves_list_ordering() {
        // `upsert_memory` must round-trip without reordering entries.
        // Check the in-memory contract first so repo impls have a clear
        // target.
        let mut memory = sample("sess-1", "task-1");
        memory.completed_findings = vec!["A".into(), "B".into(), "C".into()];
        memory.open_questions = vec!["Q1".into(), "Q2".into()];
        memory.extracted_facts = vec!["fact-1".into()];

        let json = serde_json::to_string(&memory).expect("serialize");
        let decoded: SessionMemory = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.completed_findings, memory.completed_findings);
        assert_eq!(decoded.open_questions, memory.open_questions);
        assert_eq!(decoded.extracted_facts, memory.extracted_facts);
    }
}
