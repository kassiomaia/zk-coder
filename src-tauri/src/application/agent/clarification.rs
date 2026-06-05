//! Clarification-pending registry (Workstream E.2 / E.3 →
//! agent-clarification-resume plan).
//!
//! When the agent emits `agent_narrate` with `intent = "question"`, the
//! runtime records the pending question here and pauses execution. The
//! registry is dual-layer:
//!
//! 1. **Process-local (fast path)** — an in-memory `HashMap` keyed by
//!    session, holding an optional `oneshot::Sender<String>` plus a
//!    cached copy of the persisted record. The sender is plumbed back
//!    to the coordinator so a live reply resumes the parked loop in
//!    the same process without re-reading the DB.
//! 2. **SQLite (authoritative)** — the `agent_pending_clarifications`
//!    table. Survives restarts so a reply arriving on a fresh process
//!    can be routed back to the same task and subtask. Accessed
//!    through a [`ClarificationRepository`] injected at app boot via
//!    [`init_clarification_store`].
//!
//! The legacy in-memory-only API
//! (`record_pending_clarification`/`pending_clarification`/`clear_pending_clarification`)
//! remains the entry point for the pre-resume step-gate flow — Phase 4
//! of the plan will migrate the coordinator to the dual-layer API and
//! remove it.
//!
//! Timeout behavior: the default wait is 30 minutes, configurable at
//! startup via the `CODER_CLARIFICATION_TIMEOUT_MINUTES` environment
//! variable. Values `<= 0` disable the timeout; malformed values fall
//! back to the default.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use chrono::{DateTime, Duration, Utc};
use tokio::sync::oneshot;

use crate::core::agent::clarification::PendingClarificationRecord;
use crate::core::agent::repositories::ClarificationRepository;

/// Default clarification timeout when
/// `CODER_CLARIFICATION_TIMEOUT_MINUTES` is unset or malformed.
pub const DEFAULT_CLARIFICATION_TIMEOUT_MINUTES: i64 = 30;

/// Environment variable consulted for the clarification timeout.
pub const CLARIFICATION_TIMEOUT_ENV: &str = "CODER_CLARIFICATION_TIMEOUT_MINUTES";

/// Legacy snapshot of a pending clarification, retained for callers that
/// only need the prompt and timestamp (e.g. the Phase-3 step gate).
///
/// Phase 4 of the agent-clarification-resume plan migrates callers to
/// [`PendingClarificationRecord`] and removes this struct.
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

impl From<&PendingClarificationRecord> for PendingClarification {
    fn from(record: &PendingClarificationRecord) -> Self {
        Self {
            prompt: record.prompt.clone(),
            asked_at: record.asked_at,
        }
    }
}

/// Outcome of [`deliver_reply`]. Tells the caller whether the reply
/// was forwarded to the in-process coordinator (fast path) or must be
/// rehydrated from the persisted state (post-restart path).
#[derive(Debug)]
pub enum ReplyDelivery {
    /// The reply was delivered on the live `oneshot::Sender`. The
    /// coordinator's `maybe_park_for_clarification` stage will wake
    /// up on its receiver and resume the loop in place.
    Live,
    /// There is no live sender — the process restarted after the pause.
    /// The caller must load the `ExecutionCheckpoint` and spawn a
    /// fresh coordinator seeded from it. `checkpoint_present` reports
    /// whether the matching checkpoint row is present; when `false`,
    /// the session's pause state is corrupt and should be repaired
    /// into [`SessionState::Error { recoverable: true }`](
    /// crate::core::agent::session::SessionState::Error).
    PostRestart {
        /// The persisted record the coordinator must resume against.
        record: PendingClarificationRecord,
        /// `true` when the corresponding checkpoint row exists.
        /// Phase 2 leaves this `false` unconditionally; Phase 3 wires
        /// the checkpoint repository in so the flag becomes meaningful.
        checkpoint_present: bool,
    },
}

/// Typed failure modes for the dual-layer API.
#[derive(Debug, thiserror::Error)]
pub enum ClarificationError {
    /// No pending clarification exists for the session. Distinct from
    /// [`Self::ReplyAlreadyDelivered`] so callers can tell a genuine
    /// "nothing to reply to" apart from a double-delivery race.
    #[error("no pending clarification for session {0}")]
    NotPending(String),
    /// A reply was already delivered for this clarification. The second
    /// caller must not attempt the resume flow again.
    #[error("clarification reply already delivered for session {0}")]
    ReplyAlreadyDelivered(String),
    /// The clarification store has not been initialized — [`init_clarification_store`]
    /// was never called. Should only happen in tests that forgot to
    /// inject a repository.
    #[error("clarification store not initialized")]
    StoreUninitialized,
    /// Underlying repository failure. Carries the string form of the
    /// domain error to stay free of upward dependencies.
    #[error("clarification repository error: {0}")]
    Repository(String),
}

/// Process-local entry for a pending clarification. Holds the reply
/// channel and a cached copy of the persisted record for fast reads.
struct ChannelEntry {
    /// Reply sender. `None` means either (a) the entry was populated
    /// from a post-restart repo read (no live coordinator to notify)
    /// or (b) the sender was consumed by a live reply. The `delivered`
    /// flag disambiguates these two cases.
    sender: Option<oneshot::Sender<String>>,
    /// Matching receiver. Owned by the registry between `record_…` and
    /// the coordinator's `take_reply_receiver` call. `None` once the
    /// coordinator has picked it up (or when the entry was populated
    /// from a post-restart repo read with no live channel).
    receiver: Option<oneshot::Receiver<String>>,
    /// `true` once a reply has been delivered through this entry.
    /// Second attempts observe this and return
    /// [`ClarificationError::ReplyAlreadyDelivered`] even though the
    /// sender slot has been drained.
    delivered: bool,
    /// Snapshot of the persisted record, cached for callers that only
    /// need to read it without hitting SQLite on every query.
    record: PendingClarificationRecord,
}

fn legacy_registry() -> &'static Mutex<HashMap<String, PendingClarification>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, PendingClarification>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn channel_registry() -> &'static Mutex<HashMap<String, ChannelEntry>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, ChannelEntry>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn repository_slot() -> &'static Mutex<Option<Arc<dyn ClarificationRepository>>> {
    static SLOT: OnceLock<Mutex<Option<Arc<dyn ClarificationRepository>>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

/// Injects the authoritative clarification repository. Called once
/// from the app bootstrap so the dual-layer API has somewhere to
/// persist pause state. A subsequent call overwrites the slot —
/// production only calls this once, but `cfg(test)` suites rely on
/// the replacement semantics to isolate per-test repositories.
pub fn init_clarification_store(repo: Arc<dyn ClarificationRepository>) {
    let mut guard = repository_slot().lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(repo);
}

fn repository() -> Result<Arc<dyn ClarificationRepository>, ClarificationError> {
    repository_slot()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .ok_or(ClarificationError::StoreUninitialized)
}

// ========================================================================
// Legacy in-memory API (pre-resume; step_gate flow — removed in Phase 4)
// ========================================================================

/// Records a pending clarification for `session_id` in the in-memory
/// registry (legacy path).
///
/// Replaces any prior pending clarification for the same session — the
/// most recent question is the one the runtime waits on.
pub fn record_pending_clarification(session_id: &str, prompt: &str) {
    let mut guard = legacy_registry().lock().unwrap_or_else(|e| e.into_inner());
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
pub fn record_pending_clarification_at(session_id: &str, prompt: &str, asked_at: DateTime<Utc>) {
    let mut guard = legacy_registry().lock().unwrap_or_else(|e| e.into_inner());
    guard.insert(
        session_id.to_string(),
        PendingClarification {
            prompt: prompt.to_string(),
            asked_at,
        },
    );
}

/// Returns the pending clarification for `session_id` from the
/// in-memory registry, if any (legacy path).
#[must_use]
pub fn pending_clarification(session_id: &str) -> Option<PendingClarification> {
    let guard = legacy_registry().lock().unwrap_or_else(|e| e.into_inner());
    guard.get(session_id).cloned()
}

/// Clears the pending clarification for `session_id` from the in-memory
/// registry (legacy path).
pub fn clear_pending_clarification(session_id: &str) {
    let mut guard = legacy_registry().lock().unwrap_or_else(|e| e.into_inner());
    guard.remove(session_id);
}

// ========================================================================
// Dual-layer API (Phase 2: agent-clarification-resume plan)
// ========================================================================

/// Records a pending clarification in both the authoritative DB row
/// and the process-local channel registry, returning the receiver half
/// of the live reply channel.
///
/// The coordinator's park stage (Phase 4) will `tokio::select!` on the
/// receiver to wake on a reply. Replaces any prior pending
/// clarification for `session_id` in both layers.
pub fn record_pending_clarification_persistent(
    session_id: &str,
    task_id: &str,
    subtask_id: Option<&str>,
    prompt: &str,
) -> Result<(), ClarificationError> {
    let record = PendingClarificationRecord {
        session_id: session_id.to_string(),
        task_id: task_id.to_string(),
        subtask_id: subtask_id.map(str::to_string),
        prompt: prompt.to_string(),
        asked_at: Utc::now(),
    };
    repository()?
        .upsert_pending(&record)
        .map_err(|e| ClarificationError::Repository(e.to_string()))?;

    let (tx, rx) = oneshot::channel::<String>();
    let mut guard = channel_registry().lock().unwrap_or_else(|e| e.into_inner());
    // A second question supersedes any prior one; dropping the previous
    // sender delivers `RecvError` to the stale receiver so the prior
    // park arm exits cleanly rather than hanging.
    guard.insert(
        session_id.to_string(),
        ChannelEntry {
            sender: Some(tx),
            receiver: Some(rx),
            delivered: false,
            record,
        },
    );
    Ok(())
}

/// Returns every pending clarification across all sessions — used by
/// the rehydrate path at app startup (agent-clarification-resume
/// plan, Phase 6).
pub fn list_pending_clarifications() -> Result<Vec<PendingClarificationRecord>, ClarificationError>
{
    repository()?
        .list_pending()
        .map_err(|e| ClarificationError::Repository(e.to_string()))
}

/// Takes the reply receiver for `session_id`, if one is registered.
///
/// Called by the coordinator's park stage when it observes a pending
/// clarification and intends to wait on the live reply channel. Once
/// taken, the receiver is removed from the registry — the sender
/// stays, so `deliver_reply` can still forward on it and a subsequent
/// park cannot accidentally double-await.
pub fn take_reply_receiver(session_id: &str) -> Option<oneshot::Receiver<String>> {
    let mut guard = channel_registry().lock().unwrap_or_else(|e| e.into_inner());
    guard
        .get_mut(session_id)
        .and_then(|entry| entry.receiver.take())
}

/// Returns the pending clarification record for `session_id`, consulting
/// the process-local cache first and falling back to the authoritative
/// DB row.
///
/// A `Some` result from this function means the session is paused;
/// `None` means it is not. Both layers agree on this after every write
/// (both are cleared atomically on reply and timeout).
pub fn pending_clarification_record(
    session_id: &str,
) -> Result<Option<PendingClarificationRecord>, ClarificationError> {
    // Fast path: cached entry from the live registry.
    if let Some(record) = {
        let guard = channel_registry().lock().unwrap_or_else(|e| e.into_inner());
        guard.get(session_id).map(|entry| entry.record.clone())
    } {
        return Ok(Some(record));
    }
    // Slow path: consult the repository. Populates the cache lazily so
    // the subsequent query can hit the fast path; we leave the
    // `sender` slot as `None` because no live coordinator is parked on
    // this session in the current process (post-restart path).
    let record = repository()?
        .get_pending(session_id)
        .map_err(|e| ClarificationError::Repository(e.to_string()))?;
    if let Some(ref r) = record {
        let mut guard = channel_registry().lock().unwrap_or_else(|e| e.into_inner());
        guard
            .entry(session_id.to_string())
            .or_insert_with(|| ChannelEntry {
                sender: None,
                receiver: None,
                delivered: false,
                record: r.clone(),
            });
    }
    Ok(record)
}

/// Outcome of the live-channel probe inside [`deliver_reply`].
enum LiveDelivery {
    /// The reply was sent on the live `oneshot::Sender`.
    Sent,
    /// The sender was drained earlier. Caller must treat as already-delivered.
    AlreadyDelivered,
    /// The sender was present but the receiver was dropped — fall back
    /// to the post-restart path so the caller can rebuild from the checkpoint.
    ReceiverDropped,
    /// No process-local entry exists — fall back to the post-restart path.
    Absent,
}

/// Delivers a user reply to the pending clarification.
///
/// On the live fast-path the reply is sent on the `oneshot::Sender`
/// and the caller gets [`ReplyDelivery::Live`]; the channel entry
/// stays around with a drained sender so a second attempt returns
/// [`ClarificationError::ReplyAlreadyDelivered`]. On the post-restart
/// path the caller receives the persisted record so it can seed a
/// fresh coordinator from the checkpoint; the `checkpoint_present`
/// flag is populated in Phase 3.
///
/// This function does *not* clear either layer — the caller owns the
/// subsequent transactional clear so the state transition (session
/// state → `Running`, clarification row deleted, checkpoint deleted)
/// stays atomic.
pub fn deliver_reply(session_id: &str, content: &str) -> Result<ReplyDelivery, ClarificationError> {
    // Attempt live delivery first. Scope the lock tightly so we do not
    // hold it across the fallible repo read below.
    let live_outcome = {
        let mut guard = channel_registry().lock().unwrap_or_else(|e| e.into_inner());
        match guard.get_mut(session_id) {
            None => LiveDelivery::Absent,
            Some(entry) => match entry.sender.take() {
                None => {
                    if entry.delivered {
                        LiveDelivery::AlreadyDelivered
                    } else {
                        // Sender was never registered — this entry was
                        // populated from a post-restart repo read.
                        LiveDelivery::Absent
                    }
                }
                Some(sender) => match sender.send(content.to_string()) {
                    Ok(()) => {
                        entry.delivered = true;
                        LiveDelivery::Sent
                    }
                    Err(_) => LiveDelivery::ReceiverDropped,
                },
            },
        }
    };

    match live_outcome {
        LiveDelivery::Sent => return Ok(ReplyDelivery::Live),
        LiveDelivery::AlreadyDelivered => {
            return Err(ClarificationError::ReplyAlreadyDelivered(
                session_id.to_string(),
            ));
        }
        LiveDelivery::Absent | LiveDelivery::ReceiverDropped => {
            // Fall through to the post-restart path.
        }
    }

    // Post-restart path: consult the authoritative repository.
    let record = repository()?
        .get_pending(session_id)
        .map_err(|e| ClarificationError::Repository(e.to_string()))?;
    match record {
        Some(record) => Ok(ReplyDelivery::PostRestart {
            record,
            // Phase 3 wires the checkpoint repo in to compute this
            // honestly. Today the post-restart caller does not yet
            // rebuild a coordinator from a checkpoint, so the flag
            // stays `false` and the caller must short-circuit to
            // "repair" until Phase 3 lands.
            checkpoint_present: false,
        }),
        None => Err(ClarificationError::NotPending(session_id.to_string())),
    }
}

/// Clears the pending clarification from both layers in one call.
///
/// Called on resume (after a successful reply) and on timeout. Does
/// nothing when the session has no pending entry — that is not an
/// error, because the caller may not know whether a prior cleanup
/// already fired (e.g. from a concurrent cancel path).
pub fn clear_pending_clarification_persistent(session_id: &str) -> Result<(), ClarificationError> {
    {
        let mut guard = channel_registry().lock().unwrap_or_else(|e| e.into_inner());
        guard.remove(session_id);
    }
    repository()?
        .clear_pending(session_id)
        .map_err(|e| ClarificationError::Repository(e.to_string()))?;
    Ok(())
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

/// Test-only helper that resets both layers of the registry and the
/// repo slot. Isolated here so the internals stay private to this
/// module.
#[cfg(test)]
pub fn reset_for_tests() {
    {
        let mut guard = legacy_registry().lock().unwrap_or_else(|e| e.into_inner());
        guard.clear();
    }
    {
        let mut guard = channel_registry().lock().unwrap_or_else(|e| e.into_inner());
        guard.clear();
    }
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
    use crate::infrastructure::sqlite::repositories::agent::test_support::TestAgentRepos;
    use crate::infrastructure::sqlite::repositories::agent::SqliteClarificationRepository;
    use std::sync::Arc;

    fn with_test_repo() -> Arc<SqliteClarificationRepository> {
        let repos = TestAgentRepos::in_memory().unwrap();
        // Satisfy the FK on agent_sessions for the sessions our tests touch.
        {
            let conn = repos.session.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO agent_sessions (id, agent_id, agent_name, conversation_id, thread_id, state, created_at, updated_at, needs_review)
                 VALUES (?1, 'a', 'Agent', 'c', 't', '{\"type\":\"running\",\"currentStep\":0,\"contextUsage\":0.0}', ?2, ?2, 0)",
                rusqlite::params!["session-A", "2026-04-19T00:00:00Z"],
            ).unwrap();
            conn.execute(
                "INSERT INTO agent_sessions (id, agent_id, agent_name, conversation_id, thread_id, state, created_at, updated_at, needs_review)
                 VALUES (?1, 'a', 'Agent', 'c', 't', '{\"type\":\"running\",\"currentStep\":0,\"contextUsage\":0.0}', ?2, ?2, 0)",
                rusqlite::params!["session-B", "2026-04-19T00:00:00Z"],
            ).unwrap();
        }
        let clarification = repos.clarification.clone();
        init_clarification_store(clarification.clone() as Arc<dyn ClarificationRepository>);
        clarification
    }

    // ---- Legacy in-memory API ----

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

    // ---- Dual-layer API (Phase 2) ----

    #[test]
    fn persistent_record_round_trips_through_db_and_cache() {
        let _g = test_serializer().lock().unwrap_or_else(|e| e.into_inner());
        reset_for_tests();
        let _repo = with_test_repo();
        record_pending_clarification_persistent("session-A", "task-1", Some("sub-1"), "prompt")
            .expect("record");
        let record = pending_clarification_record("session-A")
            .expect("repo ok")
            .expect("record present");
        assert_eq!(record.task_id, "task-1");
        assert_eq!(record.subtask_id.as_deref(), Some("sub-1"));
        assert_eq!(record.prompt, "prompt");
    }

    #[tokio::test]
    async fn deliver_reply_live_forwards_on_the_channel() {
        let _g = test_serializer().lock().unwrap_or_else(|e| e.into_inner());
        reset_for_tests();
        let _repo = with_test_repo();
        record_pending_clarification_persistent("session-A", "task-1", None, "prompt")
            .expect("record");
        let rx = take_reply_receiver("session-A").expect("receiver available");
        match deliver_reply("session-A", "my reply").expect("deliver") {
            ReplyDelivery::Live => {}
            other => panic!("expected Live, got {other:?}"),
        }
        let received = rx.await.expect("channel not dropped");
        assert_eq!(received, "my reply");
    }

    #[test]
    fn deliver_reply_second_attempt_returns_already_delivered() {
        let _g = test_serializer().lock().unwrap_or_else(|e| e.into_inner());
        reset_for_tests();
        let _repo = with_test_repo();
        record_pending_clarification_persistent("session-A", "task-1", None, "prompt")
            .expect("record");
        let _rx = take_reply_receiver("session-A");
        deliver_reply("session-A", "first reply").expect("first ok");
        match deliver_reply("session-A", "second reply") {
            Err(ClarificationError::ReplyAlreadyDelivered(_)) => {}
            other => panic!("expected ReplyAlreadyDelivered, got {other:?}"),
        }
    }

    #[test]
    fn deliver_reply_with_no_pending_returns_not_pending() {
        let _g = test_serializer().lock().unwrap_or_else(|e| e.into_inner());
        reset_for_tests();
        let _repo = with_test_repo();
        match deliver_reply("session-A", "reply") {
            Err(ClarificationError::NotPending(id)) => assert_eq!(id, "session-A"),
            other => panic!("expected NotPending, got {other:?}"),
        }
    }

    #[test]
    fn deliver_reply_post_restart_returns_the_record_from_db() {
        let _g = test_serializer().lock().unwrap_or_else(|e| e.into_inner());
        reset_for_tests();
        let repo = with_test_repo();
        // Simulate a post-restart state: the DB row exists but no
        // channel entry is registered for the session.
        repo.upsert_pending(&PendingClarificationRecord {
            session_id: "session-A".into(),
            task_id: "task-1".into(),
            subtask_id: Some("sub-1".into()),
            prompt: "prompt".into(),
            asked_at: Utc::now(),
        })
        .unwrap();
        match deliver_reply("session-A", "reply").expect("deliver ok") {
            ReplyDelivery::PostRestart {
                record,
                checkpoint_present,
            } => {
                assert_eq!(record.task_id, "task-1");
                assert_eq!(record.subtask_id.as_deref(), Some("sub-1"));
                assert!(!checkpoint_present); // Phase 2 default
            }
            other => panic!("expected PostRestart, got {other:?}"),
        }
    }

    #[test]
    fn clear_persistent_removes_from_both_layers() {
        let _g = test_serializer().lock().unwrap_or_else(|e| e.into_inner());
        reset_for_tests();
        let repo = with_test_repo();
        record_pending_clarification_persistent("session-A", "task-1", None, "prompt")
            .expect("record");
        clear_pending_clarification_persistent("session-A").expect("clear ok");
        assert!(repo.get_pending("session-A").unwrap().is_none());
        assert!(pending_clarification_record("session-A").unwrap().is_none());
    }

    #[test]
    fn persistent_session_scoping_isolates_sessions() {
        let _g = test_serializer().lock().unwrap_or_else(|e| e.into_inner());
        reset_for_tests();
        let _repo = with_test_repo();
        record_pending_clarification_persistent("session-A", "task-1", None, "qA")
            .expect("record A");
        record_pending_clarification_persistent("session-B", "task-2", None, "qB")
            .expect("record B");
        clear_pending_clarification_persistent("session-A").expect("clear A");
        assert!(pending_clarification_record("session-A").unwrap().is_none());
        let b = pending_clarification_record("session-B")
            .expect("B repo ok")
            .expect("B still pending");
        assert_eq!(b.prompt, "qB");
    }

    #[test]
    fn take_reply_receiver_returns_receiver_once() {
        let _g = test_serializer().lock().unwrap_or_else(|e| e.into_inner());
        reset_for_tests();
        let _repo = with_test_repo();
        record_pending_clarification_persistent("session-A", "task-1", None, "prompt")
            .expect("record");
        assert!(take_reply_receiver("session-A").is_some());
        assert!(take_reply_receiver("session-A").is_none());
    }
}
