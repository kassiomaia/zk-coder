//! Shared helpers for per-tool handler modules.
//!
//! Kept at handler-module scope so each handler can depend on a tiny,
//! testable surface rather than the full permissions pipeline.

use crate::core::agent::repositories::TaskRepository;
use crate::core::agent::session::AgentSession;
use crate::core::agent::task::{SubTask, TaskFraming, TaskStatus};
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;

/// Normalizes a repository-loaded [`SubTask`] for emission as a
/// `SubtaskCreated`/`SubtaskUpdated` event. Ensures the `task_id`
/// field is populated so frontend consumers can route the event
/// without an extra query.
pub(crate) fn core_task_to_event_subtask(mut subtask: SubTask, task_id: &str) -> SubTask {
    if subtask.task_id.is_empty() {
        subtask.task_id = task_id.to_string();
    }
    subtask
}

/// Maps the `status` argument of `agent_mark_subtask_status` to the
/// matching core [`TaskStatus`] pair (core state + event state).
///
/// Returns `None` when the string is not a recognized status.
pub(super) fn parse_status(status: &str) -> Option<(TaskStatus, TaskStatus)> {
    match status {
        "pending" => Some((TaskStatus::Pending, TaskStatus::Pending)),
        "in_progress" => Some((TaskStatus::InProgress, TaskStatus::InProgress)),
        "completed" => Some((TaskStatus::Completed, TaskStatus::Completed)),
        "failed" => Some((TaskStatus::Failed, TaskStatus::Failed)),
        _ => None,
    }
}

/// Workstream H.2.b — parse and validate the optional top-level
/// `framing` argument from an `agent_plan_subtasks` call.
///
/// Returns `Ok(None)` when no framing is declared (legacy behavior).
/// Returns `Ok(Some(_))` when a valid framing is supplied. Returns
/// `Err(msg)` when the framing is malformed or fails the validation
/// rules in [`TaskFraming::validate`].
pub(super) fn parse_and_validate_framing(
    raw: Option<&serde_json::Value>,
) -> Result<Option<TaskFraming>, String> {
    let Some(raw) = raw else { return Ok(None) };
    if raw.is_null() {
        return Ok(None);
    }
    let framing: TaskFraming = serde_json::from_value(raw.clone()).map_err(|e| {
        format!(
            "framing argument is malformed — expected an object with `role`, `voice_constraints`, optional `domain_priors`, optional `forbidden_patterns`: {e}"
        )
    })?;
    framing.validate()?;
    Ok(Some(framing))
}

/// Detects whether the active task is likely to require file or
/// command-execution work. Used by `handle_plan_subtasks` to decide
/// whether the plan must include a verification step.
pub(super) fn detect_code_change_intent(
    session: &AgentSession,
    task_id: &str,
    task_repo: &dyn TaskRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
) -> bool {
    let task_text = task_repo
        .get_session_tasks(&session.id.0)
        .ok()
        .and_then(|tasks| tasks.into_iter().find(|t| t.id == task_id))
        .and_then(|t| t.title.map(|title| title.to_lowercase()))
        .unwrap_or_default();

    let keywords = [
        "fix",
        "bug",
        "implement",
        "update",
        "write",
        "refactor",
        "create",
        "add",
        "remove",
        "rename",
        "build",
        "compile",
        "test",
    ];
    let heuristic_hit = keywords.iter().any(|k| task_text.contains(k));
    if heuristic_hit {
        return true;
    }

    telemetry_repo
        .get_tool_calls_for_session(&session.id)
        .ok()
        .map(|calls| {
            calls.into_iter().any(|c| {
                matches!(
                    c.tool_name.as_str(),
                    "fs_write_file" | "fs_delete_file" | "system_execute_command"
                ) && !c.is_error
            })
        })
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Tests migrated from `permissions.rs` for `parse_and_validate_framing`.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod framing_parse_tests {
    //! Workstream H.2.b — planner framing validation.
    use super::*;
    use crate::core::agent::task::DomainPriors;

    #[test]
    fn absent_framing_parses_as_none() {
        assert!(parse_and_validate_framing(None).unwrap().is_none());
    }

    #[test]
    fn null_framing_parses_as_none() {
        let null = serde_json::Value::Null;
        assert!(parse_and_validate_framing(Some(&null)).unwrap().is_none());
    }

    #[test]
    fn valid_communication_framing_parses_and_persists_shape() {
        let json = serde_json::json!({
            "role": "senior communication professional",
            "voiceConstraints": [
                "one claim per paragraph",
                "no filler openers"
            ],
            "domainPriors": {"hints": ["press-release voice"]},
            "forbiddenPatterns": ["(?i)game[- ]changing"]
        });
        let framing = parse_and_validate_framing(Some(&json))
            .unwrap()
            .expect("framing should be parsed");
        assert_eq!(framing.role, "senior communication professional");
        assert_eq!(framing.voice_constraints.len(), 2);
        assert_eq!(
            framing.domain_priors,
            Some(DomainPriors {
                hints: vec!["press-release voice".to_string()]
            }),
        );
        assert_eq!(framing.forbidden_patterns.len(), 1);
    }

    #[test]
    fn framing_with_single_voice_constraint_is_rejected() {
        let json = serde_json::json!({
            "role": "writer",
            "voiceConstraints": ["professional and clear"]
        });
        let err = parse_and_validate_framing(Some(&json)).unwrap_err();
        assert!(err.contains("voice_constraints"));
    }

    #[test]
    fn framing_with_invalid_forbidden_regex_is_rejected() {
        let json = serde_json::json!({
            "role": "writer",
            "voiceConstraints": ["a", "b"],
            "forbiddenPatterns": ["[unclosed"]
        });
        let err = parse_and_validate_framing(Some(&json)).unwrap_err();
        assert!(err.contains("forbidden_patterns"));
    }

    #[test]
    fn framing_missing_required_fields_is_rejected_with_helpful_message() {
        let json = serde_json::json!({
            "voiceConstraints": ["a", "b"]
        });
        let err = parse_and_validate_framing(Some(&json)).unwrap_err();
        assert!(
            err.contains("role") || err.contains("malformed"),
            "error message must point at the malformed shape: {err}",
        );
    }

    #[test]
    fn handler_branch_accepts_communication_framing_shape() {
        let communication = serde_json::json!({
            "role": "senior communication professional",
            "voiceConstraints": [
                "one claim per paragraph",
                "no adjectives without a concrete number"
            ],
            "domainPriors": {"hints": ["press-release voice", "B2B SaaS launch copy"]},
            "forbiddenPatterns": [
                "(?i)pioneering",
                "(?i)game[- ]changing",
                "(?i)unprecedented"
            ]
        });
        let framing = parse_and_validate_framing(Some(&communication))
            .expect("communication framing must validate")
            .expect("framing present");
        assert_eq!(framing.role, "senior communication professional");
        assert_eq!(framing.voice_constraints.len(), 2);
        assert_eq!(framing.forbidden_patterns.len(), 3);
    }
}
