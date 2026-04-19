//! Clarification-pending registry (Workstream E.2 / E.3).
//!
//! When the agent emits `agent_narrate` with `intent = "question"`, the
//! runtime records the pending question here and pauses execution. The
//! registry is session-scoped and process-local: restarting the backend
//! clears pending clarifications because they are tied to a live chat
//! turn, not to durable state.
//!
//! The registry has two consumers:
//!
//! 1. `permissions::handle_narrate` calls [`record_pending_clarification`]
//!    when a question is emitted.
//! 2. The execution step gate consults [`pending_clarification`] before
//!    every LLM call; if the state is set, the coordinator skips the
//!    LLM call and either returns control to the user (awaiting a
//!    reply) or triggers the timeout fallback.
//!
//! The user-reply path calls [`clear_pending_clarification`] from the
//! chat message-input handler so the next step resumes.
//!
//! Timeout behavior: the default wait is 30 minutes, configurable at
//! startup via the `CODER_CLARIFICATION_TIMEOUT_MINUTES` environment
//! variable. Values <= 0 disable the timeout; malformed values fall
//! back to the default.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use chrono::{DateTime, Duration, Utc};

/// Default clarification timeout when
/// `CODER_CLARIFICATION_TIMEOUT_MINUTES` is unset or malformed.
pub const DEFAULT_CLARIFICATION_TIMEOUT_MINUTES: i64 = 30;

/// Environment variable consulted for the clarification timeout.
pub const CLARIFICATION_TIMEOUT_ENV: &str = "CODER_CLARIFICATION_TIMEOUT_MINUTES";

/// Snapshot of a pending clarification for a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingClarification {
    /// Exact prompt the agent emitted with `intent = "question"`.
    pub prompt: String,
    /// UTC timestamp at which the question was asked.
    pub asked_at: DateTime<Utc>,
}

impl PendingClarification {
    /// Returns `true` when the pending clarification has exceeded the
    /// configured timeout. `now` is accepted explicitly so tests can
    /// drive the clock without mocking the system time.
    #[must_use]
    pub fn has_timed_out(&self, now: DateTime<Utc>, timeout_minutes: i64) -> bool {
        if timeout_minutes <= 0 {
            return false;
        }
        now - self.asked_at >= Duration::minutes(timeout_minutes)
    }
}

fn registry() -> &'static Mutex<HashMap<String, PendingClarification>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, PendingClarification>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Records a pending clarification for `session_id`.
///
/// Replaces any prior pending clarification for the same session — the
/// most recent question is the one the runtime waits on.
pub fn record_pending_clarification(session_id: &str, prompt: &str) {
    let mut guard = registry().lock().unwrap_or_else(|e| e.into_inner());
    guard.insert(
        session_id.to_string(),
        PendingClarification {
            prompt: prompt.to_string(),
            asked_at: Utc::now(),
        },
    );
}

/// Records a pending clarification with an explicit `asked_at` timestamp.
///
/// Used by the timeout tests to avoid the lexical dependency on
/// `Utc::now()`. Production code should prefer
/// [`record_pending_clarification`].
pub fn record_pending_clarification_at(
    session_id: &str,
    prompt: &str,
    asked_at: DateTime<Utc>,
) {
    let mut guard = registry().lock().unwrap_or_else(|e| e.into_inner());
    guard.insert(
        session_id.to_string(),
        PendingClarification {
            prompt: prompt.to_string(),
            asked_at,
        },
    );
}

/// Returns the pending clarification for `session_id`, if any.
#[must_use]
pub fn pending_clarification(session_id: &str) -> Option<PendingClarification> {
    let guard = registry().lock().unwrap_or_else(|e| e.into_inner());
    guard.get(session_id).cloned()
}

/// Clears the pending clarification for `session_id`.
///
/// Called by the chat message-input pipeline when the user sends a
/// reply, and by the step gate when the timeout fallback fires and
/// fails the subtask.
pub fn clear_pending_clarification(session_id: &str) {
    let mut guard = registry().lock().unwrap_or_else(|e| e.into_inner());
    guard.remove(session_id);
}

/// Returns the configured clarification timeout in minutes, consulting
/// `CODER_CLARIFICATION_TIMEOUT_MINUTES` at call time.
///
/// Values `<= 0` disable the timeout (the registry stays pending until
/// the user replies or the session is cancelled). Malformed values fall
/// back to [`DEFAULT_CLARIFICATION_TIMEOUT_MINUTES`].
#[must_use]
pub fn configured_timeout_minutes() -> i64 {
    match std::env::var(CLARIFICATION_TIMEOUT_ENV) {
        Ok(raw) => raw
            .trim()
            .parse::<i64>()
            .unwrap_or(DEFAULT_CLARIFICATION_TIMEOUT_MINUTES),
        Err(_) => DEFAULT_CLARIFICATION_TIMEOUT_MINUTES,
    }
}

/// Test-only helper that resets the registry. Isolated here so the
/// mutex internals stay private to this module.
#[cfg(test)]
pub fn reset_for_tests() {
    let mut guard = registry().lock().unwrap_or_else(|e| e.into_inner());
    guard.clear();
}

/// Test-only serializer shared by every test suite that mutates the
/// clarification registry. Holding this mutex for the duration of a
/// test body prevents parallel test threads from racing on the
/// process-global registry. A panicking test drops the guard and
/// releases; poisoning is handled transparently.
#[cfg(test)]
#[must_use]
pub fn test_serializer() -> &'static std::sync::Mutex<()> {
    static SERIALIZER: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    SERIALIZER.get_or_init(|| std::sync::Mutex::new(()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // The registry is a process-global static. Tests use the public
    // `test_serializer()` guard to avoid races with the step_gate
    // clarification tests in a sibling module.

    #[test]
    fn record_and_read_round_trip() {
        let _g = test_serializer().lock().unwrap_or_else(|e| e.into_inner());
        reset_for_tests();
        record_pending_clarification("session-A", "What is the product name?");
        let pending = pending_clarification("session-A").expect("recorded pending clarification");
        assert_eq!(pending.prompt, "What is the product name?");
    }

    #[test]
    fn recording_a_second_question_replaces_the_first() {
        let _g = test_serializer().lock().unwrap_or_else(|e| e.into_inner());
        reset_for_tests();
        record_pending_clarification("session-B", "first question");
        record_pending_clarification("session-B", "second question");
        let pending = pending_clarification("session-B").expect("still pending");
        assert_eq!(pending.prompt, "second question");
    }

    #[test]
    fn clear_removes_the_pending_clarification() {
        let _g = test_serializer().lock().unwrap_or_else(|e| e.into_inner());
        reset_for_tests();
        record_pending_clarification("session-C", "a question");
        assert!(pending_clarification("session-C").is_some());
        clear_pending_clarification("session-C");
        assert!(pending_clarification("session-C").is_none());
    }

    #[test]
    fn pending_clarification_is_session_scoped() {
        let _g = test_serializer().lock().unwrap_or_else(|e| e.into_inner());
        reset_for_tests();
        record_pending_clarification("session-D1", "question D1");
        record_pending_clarification("session-D2", "question D2");
        let one = pending_clarification("session-D1").expect("D1 pending");
        let two = pending_clarification("session-D2").expect("D2 pending");
        assert_eq!(one.prompt, "question D1");
        assert_eq!(two.prompt, "question D2");
    }

    #[test]
    fn timeout_fires_after_configured_minutes() {
        let asked = Utc::now() - Duration::minutes(31);
        let pending = PendingClarification {
            prompt: "q".to_string(),
            asked_at: asked,
        };
        assert!(pending.has_timed_out(Utc::now(), 30));
    }

    #[test]
    fn timeout_does_not_fire_within_the_window() {
        let asked = Utc::now() - Duration::minutes(5);
        let pending = PendingClarification {
            prompt: "q".to_string(),
            asked_at: asked,
        };
        assert!(!pending.has_timed_out(Utc::now(), 30));
    }

    #[test]
    fn non_positive_timeout_disables_the_fallback() {
        let pending = PendingClarification {
            prompt: "q".to_string(),
            asked_at: Utc::now() - Duration::days(365),
        };
        assert!(!pending.has_timed_out(Utc::now(), 0));
        assert!(!pending.has_timed_out(Utc::now(), -5));
    }

    #[test]
    fn configured_timeout_reads_env_or_falls_back() {
        // Direct env manipulation is process-global; use a unique value
        // we can assert on, then restore.
        let prior = std::env::var(CLARIFICATION_TIMEOUT_ENV).ok();
        std::env::set_var(CLARIFICATION_TIMEOUT_ENV, "7");
        assert_eq!(configured_timeout_minutes(), 7);
        std::env::set_var(CLARIFICATION_TIMEOUT_ENV, "not-a-number");
        assert_eq!(
            configured_timeout_minutes(),
            DEFAULT_CLARIFICATION_TIMEOUT_MINUTES
        );
        match prior {
            Some(v) => std::env::set_var(CLARIFICATION_TIMEOUT_ENV, v),
            None => std::env::remove_var(CLARIFICATION_TIMEOUT_ENV),
        }
    }
}
