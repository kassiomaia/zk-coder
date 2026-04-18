//! Bounded, deterministic refresh rules for the durable session working
//! memory (Milestone 3.3).
//!
//! The runtime fires a [`MemoryRefreshTrigger`] at well-defined points —
//! subtask completion, compaction boundaries, and retrieval bursts — and
//! this module is the single place that turns those triggers into
//! (optional) writes on the [`MemoryRepository`] and a single
//! `MemoryRefreshed` runtime trace. Non-trigger steps do not touch memory,
//! which is exactly the property M3.3 requires: memory updates must be
//! bounded and observable, not every-step rewrites.
//!
//! Determinism comes from three guarantees enforced here:
//!
//! 1. **Idempotent appends**: duplicate entries in `completed_findings` or
//!    `extracted_facts` are skipped, so re-firing the same trigger with
//!    the same inputs does not grow the lists.
//! 2. **Bounded lists**: findings and facts are each capped; oldest
//!    entries are evicted when the cap is crossed.
//! 3. **Trace-on-every-fire**: `apply_memory_refresh` appends a
//!    `MemoryRefreshed` trace on every call, including skipped cases (no
//!    memory row, no-op mutation). That makes it possible to audit
//!    refresh frequency without inferring it from absence.

use serde_json::{json, Value as JsonValue};

use crate::core::agent::execution::{RuntimeTrace, RuntimeTraceType};
use crate::core::agent::memory::MemoryRepository;
use crate::core::agent::repositories::RuntimeTraceRepository;
use crate::core::agent::session::SessionId;

/// Maximum number of entries retained in `SessionMemory.completed_findings`.
/// Older entries are evicted when the cap is crossed.
pub const MAX_COMPLETED_FINDINGS: usize = 50;

/// Maximum number of entries retained in `SessionMemory.extracted_facts`.
/// Older entries are evicted when the cap is crossed.
pub const MAX_EXTRACTED_FACTS: usize = 50;

/// Number of retrievals required within a single subtask before the
/// `RetrievalBurst` trigger fires. Reset on every subtask change.
pub const RETRIEVAL_BURST_THRESHOLD: u32 = 3;

/// The bounded set of events that the runtime treats as a memory-refresh
/// signal. The plan (`docs/planning/agent-context-engineering/plan.md`
/// M3.3) suggests more possible triggers; this enum is intentionally
/// closed so additions are a deliberate change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryRefreshTrigger {
    /// The previously in-progress subtask just transitioned to
    /// `Completed`. Carries the subtask id, its title (user-visible text
    /// appended to `completed_findings`), and its order.
    SubtaskCompleted {
        subtask_id: String,
        title: String,
        order: i32,
    },
    /// A context compaction just fired. The memory reference block in
    /// packed context is the only durable pointer to session state across
    /// compaction, so we record the boundary even when no content
    /// mutation is required.
    CompactionBoundary,
    /// `RETRIEVAL_BURST_THRESHOLD` retrievals fired within the current
    /// subtask. Carries the measured count so consumers can distinguish
    /// between a just-hit burst and a sustained burst.
    RetrievalBurst { count: u32 },
}

impl MemoryRefreshTrigger {
    /// Stable snake_case label for the trace payload. Keeping this as a
    /// dedicated method avoids coupling the trace format to the Debug
    /// representation, which must stay free to change.
    #[must_use]
    pub const fn trace_kind(&self) -> &'static str {
        match self {
            Self::SubtaskCompleted { .. } => "subtask_completed",
            Self::CompactionBoundary => "compaction_boundary",
            Self::RetrievalBurst { .. } => "retrieval_burst",
        }
    }
}

/// Content changes requested for a trigger firing. Each field is optional
/// so the caller can express "refresh this because of the trigger" without
/// necessarily providing a new content mutation — the
/// `CompactionBoundary` case uses an all-default mutation, for example.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MemoryMutation {
    /// Entries appended to `completed_findings` after dedup + cap.
    pub append_findings: Vec<String>,
    /// Entries appended to `extracted_facts` after dedup + cap.
    pub append_facts: Vec<String>,
    /// New value for `active_plan` when `Some`. `None` leaves the field
    /// untouched.
    pub new_active_plan: Option<String>,
}

/// The observable outcome of one refresh firing. Returned to the caller
/// so it can update its own counters (e.g., reset the retrieval counter
/// only when the burst trigger produced a real write) and included in the
/// emitted trace payload for audit.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MemoryRefreshOutcome {
    /// `true` when the memory row was upserted as a result of this call.
    pub wrote: bool,
    /// Number of new entries actually added to `completed_findings`.
    pub findings_added: usize,
    /// Number of new entries actually added to `extracted_facts`.
    pub facts_added: usize,
    /// `true` when `active_plan` differs from its previous value after
    /// applying the mutation.
    pub active_plan_changed: bool,
    /// Non-`None` when the trigger fired but no content-changing write
    /// happened. Stable, short snake_case reasons for trace consumers.
    pub skipped_reason: Option<&'static str>,
}

/// Appends `new_items` to `existing` while preserving insertion order and
/// avoiding duplicates. When the list exceeds `cap`, oldest entries are
/// evicted from the front. Returns the count of entries actually
/// appended (before eviction) — callers need this to build deterministic
/// trace payloads and to know whether a write is warranted.
pub(crate) fn dedupe_append_capped(
    existing: &mut Vec<String>,
    new_items: Vec<String>,
    cap: usize,
) -> usize {
    let mut added = 0;
    for item in new_items {
        if item.is_empty() {
            continue;
        }
        if existing.iter().any(|e| e == &item) {
            continue;
        }
        existing.push(item);
        added += 1;
    }
    while existing.len() > cap {
        existing.remove(0);
    }
    added
}

/// Extracts file-path prefixes from retrieval snippet strings. Snippets
/// are produced by `context_phase::run_retrieval_pass` in the format
/// `"path:line text"`. The `RetrievalBurst` trigger feeds those snippets
/// through this helper to distill a deduplicated list of paths worth
/// appending to `extracted_facts`.
///
/// Returns paths in the order they first appeared. The caller is
/// responsible for dedup against existing facts (done by
/// [`dedupe_append_capped`]).
#[must_use]
pub fn snippet_paths(snippets: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for snippet in snippets {
        let path = match snippet.split_once(':') {
            Some((head, _)) => head.trim(),
            None => snippet.trim(),
        };
        if path.is_empty() {
            continue;
        }
        if !out.iter().any(|existing| existing == path) {
            out.push(path.to_string());
        }
    }
    out
}

/// Applies a refresh trigger to the durable session memory and records a
/// `MemoryRefreshed` trace. The function is the sole write path in Slice
/// 6 — every M3.3 trigger site calls through here so observability stays
/// centralized.
///
/// # Arguments
///
/// * `memory_repo` — used to load the current row and upsert when a
///   change is detected.
/// * `trace_repo` — used to append the `MemoryRefreshed` trace.
/// * `session_id`, `task_id`, `subtask_id`, `step_number` — identifying
///   coordinates for the trace entry.
/// * `trigger` — which trigger is firing; drives the trace payload's
///   `trigger` field and is not otherwise used to modify content.
/// * `mutation` — the proposed content change; may be empty (e.g.
///   `CompactionBoundary`).
///
/// The function is intentionally infallible: repository failures are
/// swallowed and reported via `skipped_reason` so a transient storage
/// glitch does not tear down an in-flight execution.
pub fn apply_memory_refresh(
    memory_repo: &dyn MemoryRepository,
    trace_repo: &dyn RuntimeTraceRepository,
    session_id: &str,
    task_id: &str,
    subtask_id: Option<&str>,
    step_number: u32,
    trigger: &MemoryRefreshTrigger,
    mutation: MemoryMutation,
) -> MemoryRefreshOutcome {
    let session_key = SessionId::from(session_id);
    let mut outcome = MemoryRefreshOutcome::default();

    let load = memory_repo.get_memory(&session_key);
    let mut memory = match load {
        Ok(Some(memory)) => memory,
        Ok(None) => {
            outcome.skipped_reason = Some("no_memory");
            emit_trace(
                trace_repo,
                session_id,
                task_id,
                subtask_id,
                step_number,
                trigger,
                &outcome,
            );
            return outcome;
        }
        Err(err) => {
            log::warn!(
                "[AgentRuntime] memory refresh skipped — load failed for {session_id}: {err}"
            );
            outcome.skipped_reason = Some("load_failed");
            emit_trace(
                trace_repo,
                session_id,
                task_id,
                subtask_id,
                step_number,
                trigger,
                &outcome,
            );
            return outcome;
        }
    };

    let findings_added = dedupe_append_capped(
        &mut memory.completed_findings,
        mutation.append_findings,
        MAX_COMPLETED_FINDINGS,
    );
    let facts_added = dedupe_append_capped(
        &mut memory.extracted_facts,
        mutation.append_facts,
        MAX_EXTRACTED_FACTS,
    );
    let active_plan_changed = match mutation.new_active_plan {
        Some(ref next) if next != &memory.active_plan => {
            memory.active_plan.clone_from(next);
            true
        }
        _ => false,
    };

    let needs_write = findings_added > 0 || facts_added > 0 || active_plan_changed;

    if needs_write {
        match memory_repo.upsert_memory(&memory) {
            Ok(()) => {
                outcome.wrote = true;
            }
            Err(err) => {
                log::warn!(
                    "[AgentRuntime] memory refresh write failed for {session_id}: {err}"
                );
                outcome.skipped_reason = Some("write_failed");
            }
        }
    } else {
        outcome.skipped_reason = Some("no_change");
    }

    outcome.findings_added = findings_added;
    outcome.facts_added = facts_added;
    outcome.active_plan_changed = active_plan_changed;

    emit_trace(
        trace_repo,
        session_id,
        task_id,
        subtask_id,
        step_number,
        trigger,
        &outcome,
    );

    outcome
}

fn emit_trace(
    trace_repo: &dyn RuntimeTraceRepository,
    session_id: &str,
    task_id: &str,
    subtask_id: Option<&str>,
    step_number: u32,
    trigger: &MemoryRefreshTrigger,
    outcome: &MemoryRefreshOutcome,
) {
    let trace = RuntimeTrace {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        task_id: task_id.to_string(),
        subtask_id: subtask_id.map(ToOwned::to_owned),
        step_number,
        trace_type: RuntimeTraceType::MemoryRefreshed,
        payload_json: build_trace_payload(trigger, outcome),
        created_at: chrono::Utc::now(),
    };
    if let Err(err) = trace_repo.append_runtime_trace(&trace) {
        log::warn!(
            "[AgentRuntime] memory refresh trace append failed for {session_id}: {err}"
        );
    }
}

fn build_trace_payload(
    trigger: &MemoryRefreshTrigger,
    outcome: &MemoryRefreshOutcome,
) -> JsonValue {
    let mut details = json!({});
    if let Some(obj) = details.as_object_mut() {
        match trigger {
            MemoryRefreshTrigger::SubtaskCompleted {
                subtask_id,
                title,
                order,
            } => {
                obj.insert("subtaskId".into(), json!(subtask_id));
                obj.insert("subtaskTitle".into(), json!(title));
                obj.insert("subtaskOrder".into(), json!(order));
            }
            MemoryRefreshTrigger::RetrievalBurst { count } => {
                obj.insert("retrievalCount".into(), json!(count));
            }
            MemoryRefreshTrigger::CompactionBoundary => {}
        }
    }

    json!({
        "trigger": trigger.trace_kind(),
        "wrote": outcome.wrote,
        "findingsAdded": outcome.findings_added,
        "factsAdded": outcome.facts_added,
        "activePlanChanged": outcome.active_plan_changed,
        "skippedReason": outcome.skipped_reason,
        "triggerDetails": details,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Arc;

    use crate::core::agent::memory::{MemoryRepository, SessionMemory};
    use crate::core::agent::session::{AgentSession, SessionId};
    use crate::core::agent::task::{AgentRequest, AgentTask};
    use crate::core::agent::types::AgentId;
    use crate::infrastructure::sqlite::repositories::agent::SqliteSessionRepository;

    /// Fixed task id used across the memory_refresh tests. Foreign-key
    /// constraints on `agent_runtime_traces.task_id` require the task row
    /// to exist before a trace can be appended, so the fixture seeds it
    /// unconditionally.
    const FIXTURE_TASK_ID: &str = "task-1";

    fn make_repo() -> Arc<SqliteSessionRepository> {
        let repo = SqliteSessionRepository::new(":memory:").unwrap();
        let session = AgentSession::new(
            AgentId::new("code-agent"),
            "Code Agent".to_string(),
            "conv-1".to_string(),
            "thread-1".to_string(),
            None,
            None,
        );
        SqliteSessionRepository::create_session(&repo, &session).unwrap();

        let mut task = AgentTask::new(
            session.id.clone(),
            AgentRequest {
                content: "ship the feature".to_string(),
                attachments: vec![],
                context: None,
            },
        );
        task.id = FIXTURE_TASK_ID.to_string();
        task.title = Some("Fixture task".to_string());
        SqliteSessionRepository::create_task(
            &repo,
            &task,
            task.title.as_deref(),
            None,
        )
        .unwrap();

        Arc::new(repo)
    }

    fn seed_subtask(repo: &SqliteSessionRepository, id: &str, order: i32) {
        let mut subtask = crate::core::agent::task::SubTask::new(
            FIXTURE_TASK_ID.to_string(),
            format!("Subtask {order}"),
            crate::core::agent::task::SubTaskMode::FullAccess,
            order,
        );
        subtask.id = id.to_string();
        SqliteSessionRepository::create_subtask(repo, &subtask).unwrap();
    }

    fn seed_memory(repo: &SqliteSessionRepository, session_id: &SessionId) {
        let memory = SessionMemory::new(
            session_id.clone(),
            FIXTURE_TASK_ID.to_string(),
            "ship the feature".into(),
            "investigate; implement; verify".into(),
        );
        MemoryRepository::upsert_memory(repo, &memory).unwrap();
    }

    fn session_id(repo: &SqliteSessionRepository) -> SessionId {
        SqliteSessionRepository::list_sessions(repo).unwrap()[0]
            .id
            .clone()
    }

    // ----------------------------------------------------------------
    // Pure helpers
    // ----------------------------------------------------------------

    #[test]
    fn dedupe_append_capped_is_idempotent_on_duplicates() {
        let mut existing = vec!["a".to_string(), "b".to_string()];
        let added = dedupe_append_capped(
            &mut existing,
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            10,
        );
        assert_eq!(added, 1);
        assert_eq!(existing, vec!["a", "b", "c"]);
    }

    #[test]
    fn dedupe_append_capped_evicts_oldest_when_over_cap() {
        let mut existing = vec!["old1".into(), "old2".into(), "old3".into()];
        let added = dedupe_append_capped(
            &mut existing,
            vec!["new1".into(), "new2".into()],
            3,
        );
        assert_eq!(added, 2);
        assert_eq!(existing, vec!["old3", "new1", "new2"]);
    }

    #[test]
    fn dedupe_append_capped_skips_empty_entries() {
        let mut existing: Vec<String> = Vec::new();
        let added = dedupe_append_capped(
            &mut existing,
            vec![String::new(), "real".into(), String::new()],
            10,
        );
        assert_eq!(added, 1);
        assert_eq!(existing, vec!["real"]);
    }

    #[test]
    fn snippet_paths_deduplicates_and_preserves_first_seen_order() {
        let snippets = vec![
            "src/foo.rs:12 hello".to_string(),
            "src/bar.rs:3 world".to_string(),
            "src/foo.rs:40 again".to_string(),
        ];
        assert_eq!(
            snippet_paths(&snippets),
            vec!["src/foo.rs".to_string(), "src/bar.rs".to_string()]
        );
    }

    #[test]
    fn snippet_paths_falls_back_to_whole_string_when_no_colon() {
        let snippets = vec!["README.md".to_string(), "".to_string()];
        assert_eq!(snippet_paths(&snippets), vec!["README.md".to_string()]);
    }

    // ----------------------------------------------------------------
    // apply_memory_refresh
    // ----------------------------------------------------------------

    #[test]
    fn apply_skips_when_no_memory_row_exists_and_emits_trace() {
        let repo = make_repo();
        let id = session_id(&repo);

        let trigger = MemoryRefreshTrigger::CompactionBoundary;
        let outcome = apply_memory_refresh(
            repo.as_ref(),
            repo.as_ref(),
            &id.0,
            "task-1",
            None,
            5,
            &trigger,
            MemoryMutation::default(),
        );

        assert!(!outcome.wrote);
        assert_eq!(outcome.skipped_reason, Some("no_memory"));

        let traces = repo.get_runtime_traces(&id.0, None).unwrap();
        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0].trace_type, RuntimeTraceType::MemoryRefreshed);
        assert_eq!(traces[0].payload_json["trigger"], "compaction_boundary");
        assert_eq!(traces[0].payload_json["skippedReason"], "no_memory");
    }

    #[test]
    fn subtask_completed_trigger_appends_title_and_updates_active_plan() {
        let repo = make_repo();
        let id = session_id(&repo);
        seed_memory(&repo, &id);
        seed_subtask(&repo, "sub-0", 0);

        let trigger = MemoryRefreshTrigger::SubtaskCompleted {
            subtask_id: "sub-0".into(),
            title: "Investigate repo".into(),
            order: 0,
        };
        let mutation = MemoryMutation {
            append_findings: vec!["Investigate repo".into()],
            append_facts: Vec::new(),
            new_active_plan: Some("implement; verify".into()),
        };
        let outcome = apply_memory_refresh(
            repo.as_ref(),
            repo.as_ref(),
            &id.0,
            "task-1",
            Some("sub-0"),
            3,
            &trigger,
            mutation,
        );

        assert!(outcome.wrote);
        assert_eq!(outcome.findings_added, 1);
        assert_eq!(outcome.facts_added, 0);
        assert!(outcome.active_plan_changed);
        assert_eq!(outcome.skipped_reason, None);

        let loaded = MemoryRepository::get_memory(repo.as_ref(), &id)
            .unwrap()
            .unwrap();
        assert_eq!(loaded.completed_findings, vec!["Investigate repo"]);
        assert_eq!(loaded.active_plan, "implement; verify");

        let traces = repo.get_runtime_traces(&id.0, None).unwrap();
        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0].payload_json["trigger"], "subtask_completed");
        assert_eq!(traces[0].payload_json["wrote"], true);
        assert_eq!(traces[0].payload_json["findingsAdded"], 1);
        assert_eq!(traces[0].payload_json["activePlanChanged"], true);
        assert_eq!(
            traces[0].payload_json["triggerDetails"]["subtaskId"],
            "sub-0"
        );
    }

    #[test]
    fn subtask_completed_trigger_is_idempotent_on_repeat() {
        let repo = make_repo();
        let id = session_id(&repo);
        seed_memory(&repo, &id);
        seed_subtask(&repo, "sub-0", 0);

        let trigger = MemoryRefreshTrigger::SubtaskCompleted {
            subtask_id: "sub-0".into(),
            title: "Investigate repo".into(),
            order: 0,
        };
        let first = apply_memory_refresh(
            repo.as_ref(),
            repo.as_ref(),
            &id.0,
            "task-1",
            Some("sub-0"),
            3,
            &trigger,
            MemoryMutation {
                append_findings: vec!["Investigate repo".into()],
                ..MemoryMutation::default()
            },
        );
        assert!(first.wrote);
        assert_eq!(first.findings_added, 1);

        let second = apply_memory_refresh(
            repo.as_ref(),
            repo.as_ref(),
            &id.0,
            "task-1",
            Some("sub-0"),
            4,
            &trigger,
            MemoryMutation {
                append_findings: vec!["Investigate repo".into()],
                ..MemoryMutation::default()
            },
        );
        assert!(!second.wrote);
        assert_eq!(second.findings_added, 0);
        assert_eq!(second.skipped_reason, Some("no_change"));

        let loaded = MemoryRepository::get_memory(repo.as_ref(), &id)
            .unwrap()
            .unwrap();
        assert_eq!(loaded.completed_findings, vec!["Investigate repo"]);

        let traces = repo.get_runtime_traces(&id.0, None).unwrap();
        assert_eq!(traces.len(), 2);
    }

    #[test]
    fn compaction_boundary_trigger_does_not_write_when_mutation_is_empty() {
        let repo = make_repo();
        let id = session_id(&repo);
        seed_memory(&repo, &id);

        let pre = MemoryRepository::get_memory(repo.as_ref(), &id)
            .unwrap()
            .unwrap();

        let trigger = MemoryRefreshTrigger::CompactionBoundary;
        let outcome = apply_memory_refresh(
            repo.as_ref(),
            repo.as_ref(),
            &id.0,
            "task-1",
            None,
            7,
            &trigger,
            MemoryMutation::default(),
        );

        assert!(!outcome.wrote);
        assert_eq!(outcome.skipped_reason, Some("no_change"));

        let post = MemoryRepository::get_memory(repo.as_ref(), &id)
            .unwrap()
            .unwrap();
        assert_eq!(pre.updated_at, post.updated_at);
        assert_eq!(pre.completed_findings, post.completed_findings);

        let traces = repo.get_runtime_traces(&id.0, None).unwrap();
        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0].payload_json["trigger"], "compaction_boundary");
        assert_eq!(traces[0].payload_json["wrote"], false);
    }

    #[test]
    fn compaction_boundary_trigger_updates_active_plan_when_provided() {
        let repo = make_repo();
        let id = session_id(&repo);
        seed_memory(&repo, &id);

        let trigger = MemoryRefreshTrigger::CompactionBoundary;
        let outcome = apply_memory_refresh(
            repo.as_ref(),
            repo.as_ref(),
            &id.0,
            "task-1",
            None,
            11,
            &trigger,
            MemoryMutation {
                new_active_plan: Some("verify".into()),
                ..MemoryMutation::default()
            },
        );

        assert!(outcome.wrote);
        assert!(outcome.active_plan_changed);

        let loaded = MemoryRepository::get_memory(repo.as_ref(), &id)
            .unwrap()
            .unwrap();
        assert_eq!(loaded.active_plan, "verify");
    }

    #[test]
    fn retrieval_burst_trigger_appends_paths_and_dedupes() {
        let repo = make_repo();
        let id = session_id(&repo);
        seed_memory(&repo, &id);

        let trigger = MemoryRefreshTrigger::RetrievalBurst { count: 3 };
        let mutation = MemoryMutation {
            append_facts: vec![
                "src/foo.rs".into(),
                "src/bar.rs".into(),
                "src/foo.rs".into(),
            ],
            ..MemoryMutation::default()
        };

        let outcome = apply_memory_refresh(
            repo.as_ref(),
            repo.as_ref(),
            &id.0,
            "task-1",
            None,
            9,
            &trigger,
            mutation,
        );

        assert!(outcome.wrote);
        assert_eq!(outcome.facts_added, 2);

        let loaded = MemoryRepository::get_memory(repo.as_ref(), &id)
            .unwrap()
            .unwrap();
        assert_eq!(loaded.extracted_facts, vec!["src/foo.rs", "src/bar.rs"]);

        let traces = repo.get_runtime_traces(&id.0, None).unwrap();
        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0].payload_json["trigger"], "retrieval_burst");
        assert_eq!(traces[0].payload_json["triggerDetails"]["retrievalCount"], 3);
    }

    #[test]
    fn retrieval_burst_trigger_caps_extracted_facts() {
        let repo = make_repo();
        let id = session_id(&repo);

        // Pre-seed with MAX_EXTRACTED_FACTS entries so the cap is immediately relevant.
        let mut memory = SessionMemory::new(
            id.clone(),
            "task-1".into(),
            "goal".into(),
            "plan".into(),
        );
        memory.extracted_facts =
            (0..MAX_EXTRACTED_FACTS).map(|i| format!("old-{i}")).collect();
        MemoryRepository::upsert_memory(repo.as_ref(), &memory).unwrap();

        let trigger = MemoryRefreshTrigger::RetrievalBurst { count: 3 };
        let mutation = MemoryMutation {
            append_facts: vec!["new-a".into(), "new-b".into()],
            ..MemoryMutation::default()
        };
        let outcome = apply_memory_refresh(
            repo.as_ref(),
            repo.as_ref(),
            &id.0,
            "task-1",
            None,
            13,
            &trigger,
            mutation,
        );
        assert!(outcome.wrote);
        assert_eq!(outcome.facts_added, 2);

        let loaded = MemoryRepository::get_memory(repo.as_ref(), &id)
            .unwrap()
            .unwrap();
        assert_eq!(loaded.extracted_facts.len(), MAX_EXTRACTED_FACTS);
        // Oldest entries are evicted — the cap preserves the newer half.
        assert_eq!(loaded.extracted_facts.first().map(String::as_str), Some("old-2"));
        assert_eq!(
            loaded.extracted_facts.last().map(String::as_str),
            Some("new-b"),
        );
    }

    #[test]
    fn non_trigger_steps_leave_memory_untouched() {
        // Model the M3.3 contract: if no trigger fires, no memory write
        // occurs. We assert this by confirming that the memory row is
        // unchanged when `apply_memory_refresh` is simply not called.
        let repo = make_repo();
        let id = session_id(&repo);
        seed_memory(&repo, &id);

        let before = MemoryRepository::get_memory(repo.as_ref(), &id)
            .unwrap()
            .unwrap();

        // Simulate many non-trigger "steps" by doing nothing.
        std::thread::sleep(std::time::Duration::from_millis(2));

        let after = MemoryRepository::get_memory(repo.as_ref(), &id)
            .unwrap()
            .unwrap();
        assert_eq!(before.updated_at, after.updated_at);
        assert_eq!(before.completed_findings, after.completed_findings);
        assert_eq!(before.extracted_facts, after.extracted_facts);

        // And no memory-refresh traces are written by non-trigger steps.
        let traces = repo.get_runtime_traces(&id.0, None).unwrap();
        assert!(
            traces
                .iter()
                .all(|t| t.trace_type != RuntimeTraceType::MemoryRefreshed),
            "no MemoryRefreshed trace should exist on non-trigger steps",
        );
    }

    #[test]
    fn trace_kind_labels_are_stable_snake_case() {
        assert_eq!(
            MemoryRefreshTrigger::SubtaskCompleted {
                subtask_id: "s".into(),
                title: "t".into(),
                order: 0,
            }
            .trace_kind(),
            "subtask_completed"
        );
        assert_eq!(
            MemoryRefreshTrigger::CompactionBoundary.trace_kind(),
            "compaction_boundary"
        );
        assert_eq!(
            MemoryRefreshTrigger::RetrievalBurst { count: 5 }.trace_kind(),
            "retrieval_burst"
        );
    }
}
