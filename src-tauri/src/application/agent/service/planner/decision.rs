//! Unified planner entry point — state-aware. Routes between the
//! single-shot planner (no prior state) and a resume-oriented planner
//! that decides whether to continue or replan an interrupted task.

use crate::application::agent::read_models::SessionHandoffSummary;
use crate::core::agent::memory::SessionMemory;
use crate::core::agent::task::{AgentTask, SubTask, TaskStatus};
use crate::core::config::ConfigRepository;
use crate::core::llm::entities::{ChatMessage, ChatRequest, MessageContent, MessageRole};
use crate::infrastructure::agent::runtime::client::create_llm_client;
use log::info;
use serde::{Deserialize, Serialize};

use super::parsing::{finish_reason_indicates_truncation, normalize_planning_json};
use super::single_shot::plan_new_task;
use super::{TaskPlan, UserIntentHint, PLANNER_MAX_OUTPUT_TOKENS};

/// The action the planner decided to take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlanAction {
    /// Create a brand-new task and subtask list.
    Fresh,
    /// Reuse the existing plan; resume at `next_subtask_order`.
    Continue,
    /// Replace the existing plan with the one in `PlanDecision::plan`.
    Replan,
}

/// Bundle of prior execution state for the resume/replan branches of
/// `plan_or_continue`. The planner reads this to produce a state-aware
/// decision — without it, planning is always treated as `Fresh`.
pub struct ExecutionState<'a> {
    /// The most recent task for the session.
    pub task: &'a AgentTask,
    /// Subtasks of `task`, in their persisted order.
    pub subtasks: &'a [SubTask],
    /// Handoff summary recorded when the session was last paused.
    pub handoff: Option<&'a SessionHandoffSummary>,
    /// Durable session memory, if any.
    pub memory: Option<&'a SessionMemory>,
    /// Which resume action the user asked for.
    pub user_intent_hint: UserIntentHint,
}

/// The planner's decision, returned from `plan_or_continue`.
///
/// - `Fresh` → `plan` carries the new task plan; the other fields are
///   unused.
/// - `Continue` → `plan` is `None`; `next_subtask_order` points at the
///   first non-completed subtask in the existing plan.
/// - `Replan` → `plan` carries a revised plan; `completed_indices`
///   records which of the old subtasks are already done;
///   `next_subtask_order` points into the new plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanDecision {
    /// The action to take.
    pub action: PlanAction,
    /// The plan payload, for `Fresh` and `Replan`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<TaskPlan>,
    /// Indices of prior subtasks proven done, for `Replan`.
    #[serde(default)]
    pub completed_indices: Vec<usize>,
    /// Order of the subtask the agent should resume at (in the new plan
    /// for `Replan`, in the existing plan for `Continue`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_subtask_order: Option<i32>,
    /// User-facing message describing the decision.
    pub user_message: String,
}

/// Unified planning entry point — state-aware.
///
/// When `state` is `None`, falls back to `plan_new_task` and wraps the
/// result as `PlanAction::Fresh`. When `state` is `Some`, runs a resume-
/// oriented planning prompt that includes the existing plan + handoff +
/// user intent, and returns `Continue` or `Replan`.
///
/// Bootstrap branches on the action:
/// - `Fresh` → create task + subtask rows from `plan`.
/// - `Continue` → keep existing rows, transition the subtask at
///   `next_subtask_order` to `InProgress`.
/// - `Replan` → reconcile existing rows with `plan`, then transition
///   the subtask at `next_subtask_order` to `InProgress`.
pub async fn plan_or_continue(
    config_repository: &dyn ConfigRepository,
    user_message: &str,
    state: Option<ExecutionState<'_>>,
    model_id: Option<&str>,
) -> Result<PlanDecision, String> {
    match state {
        None => {
            let plan = plan_new_task(config_repository, user_message, model_id).await?;
            Ok(PlanDecision {
                action: PlanAction::Fresh,
                user_message: plan.description.clone(),
                plan: Some(plan),
                completed_indices: Vec::new(),
                next_subtask_order: Some(0),
            })
        }
        Some(state) => decide_resume(config_repository, user_message, state, model_id).await,
    }
}

/// Runs the resume-oriented planning LLM call and parses the decision.
async fn decide_resume(
    config_repository: &dyn ConfigRepository,
    user_message: &str,
    state: ExecutionState<'_>,
    model_id: Option<&str>,
) -> Result<PlanDecision, String> {
    let (client, actual_model) = create_llm_client(
        config_repository,
        model_id.map(std::string::ToString::to_string),
    )
    .await
    .map_err(|e| format!("Failed to create LLM client for resume planning: {e}"))?;

    let system_prompt = build_resume_system_prompt();
    let user_prompt = build_resume_user_prompt(user_message, &state);

    let request = ChatRequest::new(
        actual_model,
        vec![
            ChatMessage {
                role: MessageRole::System,
                content: MessageContent::Text(system_prompt),
                tool_calls: Vec::new(),
                tool_result: None,
            },
            ChatMessage {
                role: MessageRole::User,
                content: MessageContent::Text(user_prompt),
                tool_calls: Vec::new(),
                tool_result: None,
            },
        ],
    )
    .with_max_tokens(PLANNER_MAX_OUTPUT_TOKENS)
    .with_temperature(0.2);

    let response = client
        .chat(request)
        .await
        .map_err(|e| format!("LLM resume planning failed: {e}"))?;

    info!("[Planner] Raw resume decision: {}", &response.content);

    if finish_reason_indicates_truncation(response.finish_reason.as_deref()) {
        return Err(format!(
            "Resume planner output hit the {PLANNER_MAX_OUTPUT_TOKENS}-token cap before finishing \
             the JSON decision. Partial response: '{}'",
            response.content
        ));
    }

    let content = normalize_planning_json(&response.content)?;
    let decision: PlanDecision = serde_json::from_str(&content).map_err(|e| {
        format!("Failed to parse resume planner response as JSON: {e}\n\nResponse was: {content}")
    })?;

    validate_resume_decision(&decision, state.subtasks)?;
    Ok(decision)
}

/// Sanity-checks a resume decision so bootstrap can trust the shape.
fn validate_resume_decision(
    decision: &PlanDecision,
    existing_subtasks: &[SubTask],
) -> Result<(), String> {
    match decision.action {
        PlanAction::Fresh => Err(
            "Resume planner returned action=fresh, which is invalid when prior state exists"
                .to_string(),
        ),
        PlanAction::Continue => {
            if decision.plan.is_some() {
                return Err(
                    "Resume planner returned action=continue but also provided a new plan"
                        .to_string(),
                );
            }
            let order = decision.next_subtask_order.ok_or_else(|| {
                "Resume planner returned action=continue without a next_subtask_order".to_string()
            })?;
            if !existing_subtasks.iter().any(|s| s.order == order) {
                return Err(format!(
                    "Resume planner returned next_subtask_order={order} which is not present in \
                     the existing subtask list"
                ));
            }
            Ok(())
        }
        PlanAction::Replan => {
            if decision.plan.is_none() {
                return Err(
                    "Resume planner returned action=replan without an updated plan".to_string(),
                );
            }
            Ok(())
        }
    }
}

/// System prompt for the resume-oriented planner call.
fn build_resume_system_prompt() -> String {
    r#"You are the agent's resume planner. An agent session was interrupted (for example, the app was closed mid-execution). The session is now paused and the user has asked to continue.

You are given:
- the existing plan (title, description, subtasks, and their statuses)
- a handoff summary describing what was underway when the session paused
- the user's latest message
- a `userIntentHint` ("resume" means the user clicked Continue; "replan" means the user clicked Replan)

Your job: decide whether to KEEP the existing plan and resume from the first non-completed subtask, or REPLAN the remaining work.

Return a JSON object with this exact shape:
{
  "action": "continue" | "replan",
  "plan": null | { "title": "...", "description": "...", "is_non_trivial": true, "subtasks": [ { "title": "...", "description": "...", "order": 0 } ] },
  "completed_indices": [0, 1],
  "next_subtask_order": 2,
  "user_message": "Short, friendly summary for the user"
}

Decision logic:
- If `userIntentHint` is "resume" and the existing plan still fits the user's message, return `"action": "continue"`. Set `plan` to null. Set `next_subtask_order` to the order of the first subtask whose status is NOT "completed" (prefer the one already marked "in_progress" if it exists). Set `completed_indices` to the orders of already-completed subtasks. Keep `user_message` short.
- If `userIntentHint` is "replan", OR if the user's message reframes the task so the remaining subtasks no longer fit, return `"action": "replan"`. Provide a fresh `plan` that includes only the remaining work (do NOT re-list completed subtasks). Set `next_subtask_order` to 0 (the first subtask of the new plan). Set `completed_indices` to the orders of the existing subtasks that are already done.

Rules:
- Never return `"action": "fresh"` — this tool is for resume scenarios.
- `next_subtask_order` is required on both "continue" and "replan".
- For "continue", the order you return MUST exist in the existing subtask list.
- Return ONLY the JSON object. No `<think>` tags, no markdown fences, no prose.
"#.to_string()
}

/// User prompt for the resume-oriented planner call — packs the full
/// state snapshot the model needs to make its decision.
fn build_resume_user_prompt(user_message: &str, state: &ExecutionState<'_>) -> String {
    let subtasks_json = state
        .subtasks
        .iter()
        .map(|s| {
            serde_json::json!({
                "order": s.order,
                "title": s.title,
                "description": s.description,
                "status": match s.status {
                    TaskStatus::Pending => "pending",
                    TaskStatus::InProgress => "in_progress",
                    TaskStatus::WaitingApproval => "waiting_approval",
                    TaskStatus::Completed => "completed",
                    TaskStatus::Failed => "failed",
                    TaskStatus::Cancelled => "cancelled",
                },
            })
        })
        .collect::<Vec<_>>();

    let plan = serde_json::json!({
        "title": state.task.title.clone().unwrap_or_default(),
        "description": state.task.description.clone().unwrap_or_default(),
        "subtasks": subtasks_json,
    });

    let handoff = state.handoff.map(|h| {
        serde_json::json!({
            "lastCompletedIncrement": h.last_completed_increment,
            "currentObjective": h.current_objective,
            "nextRecommendedAction": h.next_recommended_action,
            "openIssues": h.open_issues,
        })
    });

    let memory = state.memory.map(|m| {
        serde_json::json!({
            "taskGoal": m.task_goal,
            "activePlan": m.active_plan,
        })
    });

    let intent = match state.user_intent_hint {
        UserIntentHint::Resume => "resume",
        UserIntentHint::Replan => "replan",
        UserIntentHint::Fresh => "resume", // defensive: should not be reached
    };

    let payload = serde_json::json!({
        "userIntentHint": intent,
        "userMessage": user_message,
        "existingPlan": plan,
        "handoff": handoff,
        "memory": memory,
    });

    format!(
        "Session state and user request (JSON):\n{}\n\nDecide whether to continue or replan.",
        serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string())
    )
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{completed_subtask, in_progress_subtask, pending_subtask};
    use super::super::TaskPlan;
    use super::*;

    #[test]
    fn plan_decision_deserializes_continue_payload_from_resume_planner() {
        // Exact on-the-wire shape the resume planner is asked to emit in
        // build_resume_system_prompt. The deserializer must accept it.
        let raw = r#"{
            "action": "continue",
            "plan": null,
            "completed_indices": [0],
            "next_subtask_order": 1,
            "user_message": "Picking up where we left off"
        }"#;
        let decision: PlanDecision = serde_json::from_str(raw).expect("continue shape parses");
        assert_eq!(decision.action, PlanAction::Continue);
        assert!(decision.plan.is_none());
        assert_eq!(decision.completed_indices, vec![0]);
        assert_eq!(decision.next_subtask_order, Some(1));
    }

    #[test]
    fn plan_decision_deserializes_replan_payload_from_resume_planner() {
        let raw = r#"{
            "action": "replan",
            "plan": {
                "title": "Revised plan",
                "description": "Updated remaining work",
                "is_non_trivial": true,
                "subtasks": [
                    { "title": "New step A", "description": null, "order": 0 }
                ]
            },
            "completed_indices": [0, 1],
            "next_subtask_order": 0,
            "user_message": "Revised the remaining work"
        }"#;
        let decision: PlanDecision = serde_json::from_str(raw).expect("replan shape parses");
        assert_eq!(decision.action, PlanAction::Replan);
        let plan = decision.plan.expect("replan carries a plan");
        assert_eq!(plan.title, "Revised plan");
        assert_eq!(plan.subtasks.len(), 1);
        assert_eq!(decision.completed_indices, vec![0, 1]);
        assert_eq!(decision.next_subtask_order, Some(0));
    }

    #[test]
    fn validate_resume_decision_rejects_fresh_when_state_is_some() {
        let decision = PlanDecision {
            action: PlanAction::Fresh,
            plan: None,
            completed_indices: vec![],
            next_subtask_order: Some(0),
            user_message: String::new(),
        };
        let existing = vec![pending_subtask("any", 0)];
        let err = validate_resume_decision(&decision, &existing)
            .expect_err("fresh decision must fail against prior state");
        assert!(err.contains("fresh"), "unexpected error: {err}");
    }

    #[test]
    fn validate_resume_decision_rejects_continue_without_next_order() {
        let decision = PlanDecision {
            action: PlanAction::Continue,
            plan: None,
            completed_indices: vec![],
            next_subtask_order: None,
            user_message: String::new(),
        };
        let existing = vec![pending_subtask("first", 0)];
        let err = validate_resume_decision(&decision, &existing)
            .expect_err("continue without cursor must fail");
        assert!(err.contains("next_subtask_order"));
    }

    #[test]
    fn validate_resume_decision_rejects_continue_with_unknown_order() {
        let decision = PlanDecision {
            action: PlanAction::Continue,
            plan: None,
            completed_indices: vec![],
            next_subtask_order: Some(42),
            user_message: String::new(),
        };
        let existing = vec![pending_subtask("first", 0)];
        let err = validate_resume_decision(&decision, &existing)
            .expect_err("continue targeting a missing order must fail");
        assert!(err.contains("42"));
    }

    #[test]
    fn validate_resume_decision_rejects_continue_with_plan_payload() {
        let decision = PlanDecision {
            action: PlanAction::Continue,
            plan: Some(TaskPlan {
                title: "stray".to_string(),
                description: String::new(),
                is_non_trivial: false,
                subtasks: vec![],
            }),
            completed_indices: vec![],
            next_subtask_order: Some(0),
            user_message: String::new(),
        };
        let existing = vec![pending_subtask("first", 0)];
        let err = validate_resume_decision(&decision, &existing)
            .expect_err("continue + plan must fail (contradictory)");
        assert!(err.contains("continue"));
    }

    #[test]
    fn validate_resume_decision_accepts_continue_with_matching_order() {
        let decision = PlanDecision {
            action: PlanAction::Continue,
            plan: None,
            completed_indices: vec![0],
            next_subtask_order: Some(1),
            user_message: "resume".to_string(),
        };
        let existing = vec![completed_subtask("done", 0), in_progress_subtask("now", 1)];
        assert!(validate_resume_decision(&decision, &existing).is_ok());
    }

    #[test]
    fn validate_resume_decision_rejects_replan_without_plan() {
        let decision = PlanDecision {
            action: PlanAction::Replan,
            plan: None,
            completed_indices: vec![],
            next_subtask_order: Some(0),
            user_message: String::new(),
        };
        let existing = vec![pending_subtask("first", 0)];
        let err = validate_resume_decision(&decision, &existing)
            .expect_err("replan without plan must fail");
        assert!(err.contains("replan"));
    }

    #[test]
    fn build_resume_user_prompt_embeds_subtask_statuses_and_intent() {
        // The resume planner must see the actual statuses so it can
        // identify which subtask is still InProgress and which ones
        // are already done. If we accidentally dropped the `status`
        // field, the model would have to guess the cursor.
        use crate::core::agent::session::SessionId;
        use crate::core::agent::task::{AgentRequest, AgentTask, RequestContext};
        let mut task = AgentTask::new(
            SessionId("s1".to_string()),
            AgentRequest {
                content: "Original request".to_string(),
                attachments: vec![],
                context: Some(RequestContext {
                    file_path: None,
                    selection: Some("desc".to_string()),
                }),
            },
        );
        task.id = "task-1".to_string();
        task.title = Some("Investigate bug".to_string());
        task.description = Some("Investigate bug in detail".to_string());

        let subtasks = vec![
            completed_subtask("Scan files", 0),
            in_progress_subtask("Read suspect module", 1),
            pending_subtask("Write fix", 2),
        ];

        let state = ExecutionState {
            task: &task,
            subtasks: &subtasks,
            handoff: None,
            memory: None,
            user_intent_hint: UserIntentHint::Resume,
        };

        let prompt = build_resume_user_prompt("Please keep going", &state);
        // Must include every status literal (the model needs them to
        // pick the cursor).
        assert!(prompt.contains("completed"));
        assert!(prompt.contains("in_progress"));
        assert!(prompt.contains("pending"));
        // Must carry the user intent hint + user message.
        assert!(prompt.contains("\"userIntentHint\": \"resume\""));
        assert!(prompt.contains("Please keep going"));
        // Must carry the subtask titles so the model can reference them.
        assert!(prompt.contains("Scan files"));
        assert!(prompt.contains("Read suspect module"));
    }
}
