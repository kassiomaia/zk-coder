//! Core types for pending agent clarifications.
//!
//! A clarification is the runtime-blocking question emitted by
//! `agent_narrate { intent: "question" }`. It carries the task and
//! subtask the runtime parked on so the coordinator can resume the same
//! work when the user replies — even across an app restart. See the
//! agent-clarification-resume plan for the broader flow.

use chrono::{DateTime, Utc};

/// An authoritative snapshot of a pending clarification, as persisted in
/// `agent_pending_clarifications`.
///
/// This is the cross-process shape — the same values the DB row holds —
/// so that a reply arriving on a fresh process can be routed and
/// resumed against the original task and subtask.
///
/// The process-local layer in `application::agent::clarification` caches
/// this record alongside the `oneshot::Sender` used on the fast-path
/// same-process resume.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingClarificationRecord {
    /// The session this clarification belongs to.
    pub session_id: String,
    /// The task that was in progress when the question was asked.
    pub task_id: String,
    /// The subtask in progress, if any. `None` when the coordinator was
    /// between subtasks (an orphan-safe state).
    pub subtask_id: Option<String>,
    /// The exact prompt the agent emitted with `intent = "question"`.
    pub prompt: String,
    /// UTC timestamp at which the question was asked.
    pub asked_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_record_is_value_equal() {
        let asked_at = DateTime::parse_from_rfc3339("2026-04-19T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let a = PendingClarificationRecord {
            session_id: "s".into(),
            task_id: "t".into(),
            subtask_id: Some("st".into()),
            prompt: "q".into(),
            asked_at,
        };
        let b = a.clone();
        assert_eq!(a, b);
    }
}
