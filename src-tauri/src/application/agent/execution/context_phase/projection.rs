//! Domain → LLM payload projection helpers for the context phase.
//!
//! These builders turn durable runtime state (task goal, subtask list,
//! memorized failures, session memory) into the value objects consumed
//! by the context policy (`ExecutionBrief`, `MemoryReference`). They are
//! pure in the sense that identical inputs produce byte-identical
//! outputs — a prerequisite for `message_prefix_hash` stability across
//! steps.

use super::super::deps::ExecutionDeps;
use crate::core::agent::session::SessionId;
use crate::infrastructure::agent::runtime::context::{
    ExecutionBrief, MemorizedFailure, MemoryReference,
};

/// Synthesizes a deterministic `ExecutionBrief` (M4.1) from the current
/// runtime state. The brief is intentionally derived from durable /
/// inspectable state (the user's request, the subtask list, the
/// memorized-failure overlay) rather than a separate stateful cache —
/// so two calls on the same state produce byte-identical briefs and
/// `message_prefix_hash` stays stable until the underlying state
/// actually moves.
///
/// Workstream C.2: the `blocker:` field filters through the
/// subtask-scoped `memorized_failures` overlay so a failure from a
/// previous subtask does not leak into the active subtask's brief.
/// Only entries whose `subtask_id == current_subtask_id` or unscoped
/// entries are eligible; the most recent eligible entry wins.
///
/// Workstream D.1 (interim, pre-B): the `next:` field is hidden
/// unconditionally. The contract-driven transition-readiness gate
/// arrives with Workstream B; until then, surfacing the next subtask
/// title only encouraged lookahead blending (P4 in
/// `docs/planning/agent-fidelity/plan.md`).
pub(super) fn build_execution_brief(
    request_content: &str,
    active_subtask_title: Option<&str>,
    loaded_subtasks: &[crate::core::agent::task::SubTask],
    memorized_failures: &[MemorizedFailure],
    current_subtask_id: Option<&str>,
) -> ExecutionBrief {
    const RECENTLY_COMPLETED_LIMIT: usize = 3;

    let mut completed: Vec<(i32, String)> = loaded_subtasks
        .iter()
        .filter(|st| st.status == crate::core::agent::task::TaskStatus::Completed)
        .map(|st| (st.order, st.title.clone()))
        .collect();
    completed.sort_by_key(|(order, _)| *order);
    let skip = completed.len().saturating_sub(RECENTLY_COMPLETED_LIMIT);
    let recently_completed: Vec<String> = completed
        .into_iter()
        .skip(skip)
        .map(|(_, title)| title)
        .collect();

    // D.1 — `next:` is hidden unconditionally (interim pre-B). The
    // field stays on `ExecutionBrief` so Workstream B can re-enable it
    // once a contract-aware transition-readiness check exists without a
    // struct migration.
    let next_intended_action: Option<String> = None;

    let current_blocker = memorized_failures
        .iter()
        .rev()
        .find(
            |entry| match (entry.subtask_id.as_deref(), current_subtask_id) {
                (None, _) => true,
                (Some(entry_id), Some(current_id)) => entry_id == current_id,
                (Some(_), None) => false,
            },
        )
        .map(|entry| {
            format!(
                "{class}: {tool}",
                class = blocker_class_label(&entry.record),
                tool = entry.record.tool_name,
            )
        });

    ExecutionBrief {
        task_goal: request_content.to_string(),
        active_subtask: active_subtask_title.map(str::to_string),
        recently_completed,
        next_intended_action,
        current_blocker,
    }
}

/// Short label used by the execution brief's `current_blocker` line.
/// Kept narrow on purpose — the M4.2 failure-memory block carries the
/// full structured breakdown, so the brief only needs a pointer.
fn blocker_class_label(
    record: &crate::infrastructure::agent::runtime::context::FailureRecord,
) -> &'static str {
    record.class.label()
}

/// Loads the durable session memory (if any) and condenses it into a
/// `MemoryReference` suitable for inlining in the packed context. The
/// reference carries only counts + short truncated strings so the block
/// stays within its token budget even when the memory body grows
/// unbounded (M3.2).
pub(super) fn lookup_memory_reference(
    deps: &ExecutionDeps<'_>,
    session_id: &str,
) -> Option<MemoryReference> {
    let session_id = SessionId::from(session_id);
    match deps.memory_repo.get_memory(&session_id) {
        Ok(Some(memory)) => Some(MemoryReference {
            session_id: memory.session_id.0.clone(),
            task_goal: memory.task_goal.clone(),
            active_plan: memory.active_plan.clone(),
            completed_findings_count: memory.completed_findings.len(),
            open_questions_count: memory.open_questions.len(),
            extracted_facts_count: memory.extracted_facts.len(),
            updated_at_rfc3339: memory.updated_at.to_rfc3339(),
        }),
        Ok(None) => None,
        Err(err) => {
            log::warn!(
                "[AgentRuntime] Failed to load session memory for {session_id}: {err}",
                session_id = session_id
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::core::agent::task::{SubTask, SubTaskMode, TaskStatus};
    use crate::infrastructure::agent::runtime::context::{FailureClass, FailureRecord};

    fn sample_failure(tool_name: &str, class: FailureClass) -> FailureRecord {
        FailureRecord {
            tool_name: tool_name.to_string(),
            args_fingerprint: "deadbeef".to_string(),
            class,
            message: "bounded error".to_string(),
            repeat_detected: false,
        }
    }

    fn sample_subtask(id: &str, title: &str, order: i32, status: TaskStatus) -> SubTask {
        let mut st = SubTask::new(
            "task-1".to_string(),
            title.to_string(),
            SubTaskMode::Write,
            order,
        );
        st.id = id.to_string();
        st.status = status;
        st
    }

    #[test]
    fn brief_blocker_filters_failures_from_previous_subtask() {
        // Replay of the 2026-04-18 trace sequence (plan.md P3):
        // subtask-2 records an fs_read_file file_not_found failure;
        // the task advances to subtask-3; subtask-3's brief must not
        // mention the subtask-2 blocker.
        let memorized = vec![MemorizedFailure {
            subtask_id: Some("subtask-2".to_string()),
            record: sample_failure("fs_read_file", FailureClass::FileNotFound),
        }];
        let subtasks = vec![
            sample_subtask("subtask-2", "read repo", 1, TaskStatus::Completed),
            sample_subtask("subtask-3", "write draft", 2, TaskStatus::InProgress),
        ];

        let brief = build_execution_brief(
            "create release communication files",
            Some("write draft"),
            &subtasks,
            &memorized,
            Some("subtask-3"),
        );

        assert!(
            brief.current_blocker.is_none(),
            "subtask-2 failure must not leak into subtask-3's brief; got {:?}",
            brief.current_blocker,
        );
    }

    #[test]
    fn brief_blocker_surfaces_failure_from_current_subtask() {
        let memorized = vec![MemorizedFailure {
            subtask_id: Some("subtask-3".to_string()),
            record: sample_failure("fs_write_file", FailureClass::PermissionDenied),
        }];
        let subtasks = vec![sample_subtask(
            "subtask-3",
            "write draft",
            2,
            TaskStatus::InProgress,
        )];

        let brief = build_execution_brief(
            "goal",
            Some("write draft"),
            &subtasks,
            &memorized,
            Some("subtask-3"),
        );

        let blocker = brief.current_blocker.expect("blocker must surface");
        assert!(blocker.contains("permission_denied"), "got {blocker}");
        assert!(blocker.contains("fs_write_file"), "got {blocker}");
    }

    #[test]
    fn brief_blocker_surfaces_unscoped_failures_in_any_subtask() {
        let memorized = vec![MemorizedFailure {
            subtask_id: None,
            record: sample_failure("search_grep", FailureClass::Timeout),
        }];
        let subtasks = vec![sample_subtask(
            "subtask-7",
            "review",
            0,
            TaskStatus::InProgress,
        )];

        let brief = build_execution_brief(
            "goal",
            Some("review"),
            &subtasks,
            &memorized,
            Some("subtask-7"),
        );

        assert!(brief.current_blocker.is_some());
    }

    #[test]
    fn brief_blocker_picks_most_recent_eligible_entry() {
        // Two failures for the same subtask, latest should win.
        let memorized = vec![
            MemorizedFailure {
                subtask_id: Some("subtask-3".to_string()),
                record: sample_failure("fs_read_file", FailureClass::FileNotFound),
            },
            MemorizedFailure {
                subtask_id: Some("subtask-3".to_string()),
                record: sample_failure("fs_write_file", FailureClass::ValidationFailed),
            },
        ];
        let subtasks = vec![sample_subtask(
            "subtask-3",
            "write draft",
            2,
            TaskStatus::InProgress,
        )];

        let brief = build_execution_brief(
            "goal",
            Some("write draft"),
            &subtasks,
            &memorized,
            Some("subtask-3"),
        );

        let blocker = brief.current_blocker.expect("blocker must surface");
        assert!(blocker.contains("validation_failed"), "got {blocker}");
        assert!(blocker.contains("fs_write_file"), "got {blocker}");
    }

    #[test]
    fn brief_next_is_hidden_unconditionally_pre_workstream_b() {
        // D.1 (interim): even when a pending subtask exists, the brief
        // must not surface `next:`. The field stays on the struct so
        // Workstream B can re-enable it with a transition-readiness
        // gate without a migration.
        let subtasks = vec![
            sample_subtask("subtask-1", "active", 0, TaskStatus::InProgress),
            sample_subtask("subtask-2", "pending next", 1, TaskStatus::Pending),
        ];

        let brief =
            build_execution_brief("goal", Some("active"), &subtasks, &[], Some("subtask-1"));

        assert!(
            brief.next_intended_action.is_none(),
            "D.1: `next:` must be hidden until B lands; got {:?}",
            brief.next_intended_action,
        );
    }

    #[test]
    fn brief_without_memorized_failures_has_no_blocker() {
        let subtasks = vec![sample_subtask(
            "subtask-1",
            "active",
            0,
            TaskStatus::InProgress,
        )];

        let brief =
            build_execution_brief("goal", Some("active"), &subtasks, &[], Some("subtask-1"));

        assert!(brief.current_blocker.is_none());
    }
}
