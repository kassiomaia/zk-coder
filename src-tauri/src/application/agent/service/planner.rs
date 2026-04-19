//! Task planning service for single-shot LLM-based task planning.
//!
//! This service handles:
//! - Single-shot planning at task creation (title, description, subtasks)
//! - Replanning for interrupted tasks (review progress, update plan)
//! - Unified `plan_or_continue` entry point: given optional prior
//!   execution state, returns a decision (Fresh / Continue / Replan)
//!   so bootstrap can reuse an existing plan instead of regenerating
//!   one on resume.

use crate::application::agent::read_models::SessionHandoffSummary;
use crate::core::agent::memory::SessionMemory;
use crate::core::agent::task::{AgentTask, SubTask, SubTaskMode, TaskStatus};
use crate::core::llm::entities::{ChatMessage, ChatRequest, MessageContent, MessageRole};
use crate::infrastructure::agent::runtime::client::create_llm_client;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

/// Output token budget for single-shot planning calls.
///
/// The planner emits a JSON object whose size is dominated by the
/// subtask list. 4000 tokens comfortably fits ~15 subtasks with
/// descriptions; previous 2000-token cap truncated mid-JSON on plans
/// of 5+ subtasks with descriptions and produced the cryptic
/// "Could not locate a JSON object in planner response" error.
const PLANNER_MAX_OUTPUT_TOKENS: u32 = 4000;

/// Returns `true` when the provider's `finish_reason` indicates the
/// response was truncated because it hit the output token cap.
///
/// Values observed across providers (case-insensitive): OpenAI uses
/// `"length"`, Anthropic uses `"max_tokens"`, Google/Gemini uses
/// `"MAX_TOKENS"`. Any other value (including `None` and `"stop"`)
/// means the model finished cleanly and the downstream JSON parser
/// should run as normal.
fn finish_reason_indicates_truncation(reason: Option<&str>) -> bool {
    match reason {
        Some(value) => {
            let normalized = value.trim().to_ascii_lowercase();
            matches!(normalized.as_str(), "length" | "max_tokens")
        }
        None => false,
    }
}

/// Result of a single-shot planning operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    /// Generated task title (concise, clear)
    pub title: String,
    /// Generated task description (one-sentence summary)
    pub description: String,
    /// Whether this task requires subtask planning
    pub is_non_trivial: bool,
    /// Planned subtasks (if non-trivial)
    pub subtasks: Vec<SubtaskPlan>,
}

/// A single subtask in the plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskPlan {
    /// Subtask title
    pub title: String,
    /// Optional subtask description
    pub description: Option<String>,
    /// Execution order
    pub order: i32,
}

/// Result of a replan operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplanOutput {
    /// The decision made (replan, abort, continue)
    pub decision: ReplanAction,
    /// Updated plan (if replanning)
    pub updated_plan: Option<TaskPlan>,
    /// Indices of completed subtasks
    pub completed_indices: Vec<usize>,
    /// User-friendly summary message
    pub user_message: String,
    /// Next step index to resume from
    pub next_step_index: Option<usize>,
}

/// Action to take when reviewing an interrupted task.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReplanAction {
    /// Update and continue with the current task
    Replan,
    /// Abandon current task and start a new one
    Abort,
    /// Resume without changes
    Continue,
}

/// Strips markdown code block wrappers from LLM responses.
///
/// LLMs often wrap JSON in ```json ... ``` blocks even when instructed not to.
/// This function extracts the actual content from within such blocks.
fn strip_markdown_code_block(content: &str) -> String {
    let content = content.trim();

    // If content is empty, return early
    if content.is_empty() {
        warn!("[Planner] strip_markdown_code_block called with empty content");
        return String::new();
    }

    // Check for ```json or ``` at the start
    if content.starts_with("```") {
        // Find the end of the opening fence line
        let after_opening_fence = if let Some(newline_pos) = content.find('\n') {
            &content[newline_pos + 1..]
        } else {
            // No newline after opening fence, treat entire thing as fence
            warn!("[Planner] Markdown fence found but no newline, returning empty");
            return String::new();
        };

        // Find the closing fence (```)
        let closing_fence = "\n```";
        if let Some(closing_pos) = after_opening_fence.find(closing_fence) {
            let result = after_opening_fence[..closing_pos].trim().to_string();
            // Safety check: if we get empty result but had content, log and return original
            if result.is_empty() && !after_opening_fence.trim().is_empty() {
                warn!("[Planner] Markdown stripping with newline fence resulted in empty, returning original content");
                return after_opening_fence.to_string();
            }
            return result;
        }

        // Also check for closing fence without leading newline
        let closing_fence_no_nl = "```";
        if let Some(closing_pos) = after_opening_fence.find(closing_fence_no_nl) {
            let result = after_opening_fence[..closing_pos].trim().to_string();
            // Safety check: if we get empty result but had content, log and return original
            if result.is_empty() && !after_opening_fence.trim().is_empty() {
                warn!("[Planner] Markdown stripping without newline fence resulted in empty, returning original content");
                return after_opening_fence.to_string();
            }
            return result;
        }

        // No closing fence found, return everything after opening fence
        let result = after_opening_fence.trim().to_string();
        if result.is_empty() && !after_opening_fence.trim().is_empty() {
            warn!("[Planner] No closing fence found but stripping resulted in empty, returning original content");
            return after_opening_fence.to_string();
        }
        return result;
    }

    // Check for closing fence at the end (even without opening fence)
    if let Some(fence_pos) = content.rfind("\n```") {
        let possible_content = &content[..fence_pos];
        // Only strip if there's meaningful content before it
        let trimmed = possible_content.trim();
        if !trimmed.is_empty() && !trimmed.starts_with("```") {
            info!("[Planner] Stripped closing fence from end of content");
            return trimmed.to_string();
        }
    }

    // No markdown wrapper found, return original
    info!("[Planner] No markdown wrapper found, returning original content");
    content.to_string()
}

/// Removes leading `<think>...</think>` blocks from an LLM response.
///
/// Some providers prepend reasoning wrapped in XML-like tags even when the user
/// requested plain JSON output. For planning, that reasoning is transport noise.
fn strip_leading_think_blocks(content: &str) -> String {
    let mut remaining = content.trim();

    loop {
        if !remaining.starts_with("<think>") {
            break;
        }

        let Some(end_tag_index) = remaining.find("</think>") else {
            warn!("[Planner] Unterminated <think> block found in planner response");
            break;
        };

        remaining = remaining[end_tag_index + "</think>".len()..].trim_start();
    }

    remaining.to_string()
}

/// Extracts the first complete top-level JSON object from a response string.
///
/// This tolerates explanatory text before or after the JSON body while avoiding
/// partial-object parsing. It only returns object-shaped JSON because planner
/// responses are required to be objects.
fn extract_first_json_object(content: &str) -> Option<String> {
    let bytes = content.as_bytes();
    let mut start_index = None;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escape = false;

    for (index, byte) in bytes.iter().enumerate() {
        match byte {
            b'\\' if in_string => {
                escape = !escape;
            }
            b'"' if !escape => {
                in_string = !in_string;
            }
            _ => {
                escape = false;

                if in_string {
                    continue;
                }

                match byte {
                    b'{' => {
                        if start_index.is_none() {
                            start_index = Some(index);
                        }
                        depth += 1;
                    }
                    b'}' if depth > 0 => {
                        depth -= 1;
                        if depth == 0 {
                            let start = start_index?;
                            return Some(content[start..=index].trim().to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    None
}

/// Normalizes an LLM response into the JSON object expected by the planner.
///
/// The planner accepts provider-specific wrappers such as `<think>` blocks,
/// fenced code blocks, or prose surrounding the JSON payload.
fn normalize_planning_json(content: &str) -> Result<String, String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err("LLM returned empty response. This may indicate a configuration issue with your LLM provider.".to_string());
    }

    let without_think = strip_leading_think_blocks(trimmed);
    let without_markdown = strip_markdown_code_block(&without_think);

    if without_markdown.trim().is_empty() {
        return Err(format!(
            "LLM returned empty response after normalization. Raw response was: '{}'",
            content
        ));
    }

    if let Some(json) = extract_first_json_object(&without_markdown) {
        return Ok(json);
    }

    Err(format!(
        "Could not locate a JSON object in planner response. Normalized response was: '{}'",
        without_markdown
    ))
}

/// Create a task plan using single-shot LLM planning.
///
/// This generates a title, description, and optionally subtasks all in one LLM call.
pub async fn plan_new_task(
    app_handle: &AppHandle,
    user_message: &str,
    model_id: Option<&str>,
) -> Result<TaskPlan, String> {
    let (client, actual_model) =
        create_llm_client(app_handle, model_id.map(std::string::ToString::to_string))
            .await
            .map_err(|e| format!("Failed to create LLM client for planning: {e}"))?;

    // Build the planning prompt
    let system_prompt = build_planning_system_prompt();

    let user_prompt =
        format!("User Request: {user_message}\n\nCreate a task plan for this request.");

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
    .with_temperature(0.3);

    // Execute LLM call
    let response = client
        .chat(request)
        .await
        .map_err(|e| format!("LLM planning failed: {e}"))?;

    // Log raw response for debugging
    info!("[Planner] Raw LLM response: {}", &response.content);

    // Detect truncation before JSON extraction so the user sees a
    // targeted error instead of the cryptic "Could not locate a JSON
    // object" that falls out of parsing a half-written object.
    if finish_reason_indicates_truncation(response.finish_reason.as_deref()) {
        return Err(format!(
            "Planner output hit the {PLANNER_MAX_OUTPUT_TOKENS}-token cap before finishing the \
             JSON plan. Ask for a narrower task or raise the planner token budget. \
             Partial response: '{}'",
            response.content
        ));
    }

    let content = normalize_planning_json(&response.content)?;

    // Log stripped content for debugging
    info!("[Planner] Stripped content: {}", &content);

    // Parse JSON response
    let plan: TaskPlan = serde_json::from_str(&content).map_err(|e| {
        format!("Failed to parse planning response as JSON: {e}\n\nResponse was: {content}")
    })?;

    Ok(plan)
}

/// Review an interrupted task and decide on next action.
///
/// This is called when a user sends a message while a task is incomplete.
pub async fn review_and_replan(
    app_handle: &AppHandle,
    current_task: &crate::core::agent::task::AgentTask,
    subtasks: &[SubTask],
    user_message: &str,
    progress_summary: &str,
    model_id: Option<&str>,
) -> Result<ReplanOutput, String> {
    let (client, actual_model) =
        create_llm_client(app_handle, model_id.map(std::string::ToString::to_string))
            .await
            .map_err(|e| format!("Failed to create LLM client for replanning: {e}"))?;

    // Build current plan summary
    let current_plan = TaskPlan {
        title: current_task.title.clone().unwrap_or_default(),
        description: current_task
            .request
            .context
            .as_ref()
            .and_then(|ctx| ctx.selection.clone())
            .unwrap_or_default(),
        is_non_trivial: !subtasks.is_empty(),
        subtasks: subtasks
            .iter()
            .map(|st| SubtaskPlan {
                title: st.title.clone(),
                description: st.description.clone(),
                order: st.order,
            })
            .collect(),
    };

    let system_prompt = build_replan_system_prompt();

    let user_prompt = format!(
        "Current Plan:\n{}\n\nProgress Summary:\n{}\n\nUser's New Request:\n{}\n\nAnalyze and decide what to do next.",
        serde_json::to_string(&current_plan).unwrap_or_default(),
        progress_summary,
        user_message
    );

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
    );

    let response = client
        .chat(request)
        .await
        .map_err(|e| format!("LLM replan failed: {e}"))?;

    let content = normalize_planning_json(&response.content)?;

    // Parse JSON response
    let output: ReplanOutput = serde_json::from_str(&content).map_err(|e| {
        format!("Failed to parse replan response as JSON: {e}\n\nResponse was: {content}")
    })?;

    Ok(output)
}

/// Build the system prompt for single-shot task planning.
#[must_use]
pub fn build_planning_system_prompt() -> String {
    r#"You are a task planning assistant. Your job is to analyze a user's request and create a clear, actionable plan.

Generate a JSON response with this exact structure:
{
  "title": "Brief, clear task title (3-8 words)",
  "description": "One-sentence summary of what will be done",
  "is_non_trivial": true or false,
  "subtasks": [
    {
      "title": "Step 1: ...",
      "description": "What this step accomplishes",
      "order": 0
    }
  ]
}

Rules:
- Title should be concise and descriptive (e.g., "Codebase investigation and improvement analysis")
- Description should be one clear sentence
- Set is_non_trivial to false for: greetings, acknowledgements, simple questions, single file reads
- For non-trivial tasks, create 3-8 subtasks
- Each subtask should be independently achievable
- Order subtasks logically (0, 1, 2, ...)
- Keep subtask titles actionable (start with verbs)

Return ONLY the JSON object, nothing else. Do not include reasoning tags like <think> or markdown fences."#
        .to_string()
}

/// Build the system prompt for replanning an interrupted task.
#[must_use]
pub fn build_replan_system_prompt() -> String {
    r#"You are reviewing an interrupted agent task. The user has sent a new message while a task was still in progress.

Your job is to:
1. Understand what was being worked on
2. Review what progress was made
3. Understand the user's new request
4. Decide whether to continue, replan, or abort the current task

Generate a JSON response with this exact structure:
{
  "decision": "REPLAN" | "ABORT" | "CONTINUE",
  "completed_indices": [0, 1, ...],
  "updated_plan": {
    "title": "...",
    "description": "...",
    "is_non_trivial": true,
    "subtasks": [...]
  },
  "user_message": "Friendly summary for the user",
  "next_step_index": 2
}

Decision logic:
- ABORT: User's request is completely unrelated (e.g., was coding, now asking about cooking)
- REPLAN: User is still interested in the same topic but with new context/requirements
- CONTINUE: User's message is a minor clarification or confirmation

For REPLAN:
- Include completed_indices (which subtasks are already done)
- Create updated_plan with remaining work as new subtasks
- Set next_step_index to where to resume

For ABORT or CONTINUE:
- updated_plan can be null
- Set next_step_index to null

Return ONLY the JSON object, nothing else. Do not include reasoning tags like <think> or markdown fences."#
        .to_string()
}

/// Convert a `SubtaskPlan` to an `AgentTask` entity.
#[must_use]
pub fn subtask_plan_to_agent_task(
    subtask_plan: &SubtaskPlan,
    _session_id: &str,
    parent_task_id: &str,
) -> SubTask {
    SubTask {
        id: uuid::Uuid::new_v4().to_string(),
        task_id: parent_task_id.to_string(),
        title: subtask_plan.title.clone(),
        description: subtask_plan.description.clone(),
        mode: SubTaskMode::FullAccess,
        status: TaskStatus::Pending,
        order: subtask_plan.order,
        contract: None,
        created_at: chrono::Utc::now(),
        started_at: None,
        completed_at: None,
    }
}

// =========================================================================
// Unified planner entry point (state-aware)
// =========================================================================

/// Hint from the user's explicit action — Continue vs Replan vs Fresh —
/// carried alongside the request content so the planner can bias its
/// decision without inventing that signal from text cues.
///
/// `Fresh` is the default for first-run execution (no prior state).
/// `Resume` is set by the Continue button. `Replan` is set by the Replan
/// button. The planner is still free to override — for example, if a
/// `Resume` hint is paired with a user message that clearly changes the
/// task scope, returning `PlanAction::Replan` is valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserIntentHint {
    /// No prior execution state — treat as a brand-new task.
    Fresh,
    /// User wants to resume the existing plan from the current cursor.
    Resume,
    /// User wants the planner to revise the existing plan.
    Replan,
}

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
    app_handle: &AppHandle,
    user_message: &str,
    state: Option<ExecutionState<'_>>,
    model_id: Option<&str>,
) -> Result<PlanDecision, String> {
    match state {
        None => {
            let plan = plan_new_task(app_handle, user_message, model_id).await?;
            Ok(PlanDecision {
                action: PlanAction::Fresh,
                user_message: plan.description.clone(),
                plan: Some(plan),
                completed_indices: Vec::new(),
                next_subtask_order: Some(0),
            })
        }
        Some(state) => decide_resume(app_handle, user_message, state, model_id).await,
    }
}

/// Runs the resume-oriented planning LLM call and parses the decision.
async fn decide_resume(
    app_handle: &AppHandle,
    user_message: &str,
    state: ExecutionState<'_>,
    model_id: Option<&str>,
) -> Result<PlanDecision, String> {
    let (client, actual_model) =
        create_llm_client(app_handle, model_id.map(std::string::ToString::to_string))
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
                    "Resume planner returned action=replan without an updated plan".to_string()
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
    use super::*;
    use crate::infrastructure::llm::mock::MockLlmClient;
    use crate::infrastructure::llm::LlmClient;

    // ========================================================================
    // Serialization Tests (existing)
    // ========================================================================

    #[test]
    fn test_task_plan_serialization() {
        let plan = TaskPlan {
            title: "Test Task".to_string(),
            description: "A test task".to_string(),
            is_non_trivial: true,
            subtasks: vec![SubtaskPlan {
                title: "Step 1".to_string(),
                description: Some("First step".to_string()),
                order: 0,
            }],
        };

        let json = serde_json::to_string(&plan).unwrap();
        assert!(json.contains("Test Task"));
        assert!(json.contains("Step 1"));
    }

    #[test]
    fn test_replan_output_serialization() {
        let output = ReplanOutput {
            decision: ReplanAction::Replan,
            updated_plan: Some(TaskPlan {
                title: "Updated Task".to_string(),
                description: "Updated".to_string(),
                is_non_trivial: true,
                subtasks: vec![],
            }),
            completed_indices: vec![0, 1],
            user_message: "Continuing work".to_string(),
            next_step_index: Some(2),
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("replan"));
        assert!(json.contains("Continuing work"));
    }

    #[test]
    fn test_replan_action_serialization() {
        let action = ReplanAction::Replan;
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("replan"));
    }

    #[test]
    fn test_subtask_plan_to_agent_task() {
        let plan = SubtaskPlan {
            title: "Test subtask".to_string(),
            description: Some("Test description".to_string()),
            order: 0,
        };

        let task = subtask_plan_to_agent_task(&plan, "session-123", "task-456");

        assert_eq!(task.title, "Test subtask");
        assert_eq!(task.description, Some("Test description".to_string()));
        assert_eq!(task.task_id, "task-456");
        assert_eq!(task.order, 0);
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(matches!(task.mode, SubTaskMode::FullAccess));
    }

    // ========================================================================
    // Response Normalization Tests
    // ========================================================================

    #[test]
    fn test_strip_markdown_code_block_with_json_wrapper() {
        // Test the exact format the LLM returned
        let wrapped = r#"```json
{
  "title": "Test",
  "description": "Test description",
  "is_non_trivial": false,
  "subtasks": []
}
```"#;

        let stripped = strip_markdown_code_block(wrapped);
        let parsed: serde_json::Value = serde_json::from_str(&stripped).unwrap();

        assert_eq!(parsed["title"], "Test");
        assert!(!stripped.starts_with("```"));
        assert!(!stripped.contains("json"));
    }

    #[test]
    fn test_strip_markdown_code_block_with_plain_wrapper() {
        let wrapped = r#"```
{
  "title": "Test"
}
```"#;

        let stripped = strip_markdown_code_block(wrapped);
        let parsed: serde_json::Value = serde_json::from_str(&stripped).unwrap();

        assert_eq!(parsed["title"], "Test");
        assert!(!stripped.starts_with("```"));
    }

    #[test]
    fn test_strip_markdown_code_block_plain_json() {
        // Plain JSON should pass through unchanged
        let plain = r#"{"title": "Test", "description": "Test"}"#;

        let stripped = strip_markdown_code_block(plain);

        assert_eq!(stripped, plain);
    }

    #[test]
    fn test_strip_markdown_code_block_with_newlines() {
        // Test with extra newlines around the JSON (like the actual LLM response)
        let wrapped = r#"```json
{
  "title": "Test"
}
```"#;

        let stripped = strip_markdown_code_block(wrapped);
        let parsed: serde_json::Value = serde_json::from_str(&stripped).unwrap();

        assert_eq!(parsed["title"], "Test");
    }

    #[test]
    fn test_strip_markdown_code_block_no_closing_fence() {
        // Some LLMs might forget the closing fence
        let wrapped = r#"```json
{
  "title": "Test"
}"#;

        let stripped = strip_markdown_code_block(wrapped);
        let parsed: serde_json::Value = serde_json::from_str(&stripped).unwrap();

        assert_eq!(parsed["title"], "Test");
    }

    #[test]
    fn test_strip_leading_think_blocks_removes_reasoning_preamble() {
        let wrapped = r#"<think>Reasoning goes here</think>

{"title":"Test"}"#;

        let stripped = strip_leading_think_blocks(wrapped);

        assert_eq!(stripped, r#"{"title":"Test"}"#);
    }

    #[test]
    fn test_extract_first_json_object_ignores_prefix_and_suffix_text() {
        let content = r#"Some prose before
{"title":"Test","description":"Example","is_non_trivial":false,"subtasks":[]}
Some prose after"#;

        let extracted = extract_first_json_object(content).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&extracted).unwrap();

        assert_eq!(parsed["title"], "Test");
        assert_eq!(parsed["is_non_trivial"], false);
    }

    #[test]
    fn test_normalize_planning_json_handles_think_and_fenced_json() {
        let wrapped = r#"<think>The user said hi, so this is trivial.</think>

```json
{
  "title": "Greeting Response",
  "description": "Acknowledge the user's greeting",
  "is_non_trivial": false,
  "subtasks": []
}
```"#;

        let normalized = normalize_planning_json(wrapped).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&normalized).unwrap();

        assert_eq!(parsed["title"], "Greeting Response");
        assert_eq!(parsed["is_non_trivial"], false);
    }

    #[test]
    fn finish_reason_truncation_detector_covers_every_provider_spelling() {
        // OpenAI spelling.
        assert!(finish_reason_indicates_truncation(Some("length")));
        // Anthropic spelling.
        assert!(finish_reason_indicates_truncation(Some("max_tokens")));
        // Google / Gemini spelling (upper-case) — normalized lower.
        assert!(finish_reason_indicates_truncation(Some("MAX_TOKENS")));
        // Surrounding whitespace from stray provider quirks.
        assert!(finish_reason_indicates_truncation(Some("  length  ")));
    }

    #[test]
    fn finish_reason_truncation_detector_rejects_clean_finishes() {
        assert!(!finish_reason_indicates_truncation(None));
        assert!(!finish_reason_indicates_truncation(Some("stop")));
        assert!(!finish_reason_indicates_truncation(Some("tool_calls")));
        assert!(!finish_reason_indicates_truncation(Some("end_turn")));
        assert!(!finish_reason_indicates_truncation(Some("")));
    }

    #[test]
    fn test_normalize_planning_json_rejects_missing_json() {
        // Prose that survives `<think>` stripping but contains no JSON object —
        // exercises the "Could not locate a JSON object" branch of
        // `normalize_planning_json`. An input like `<think>...</think>` alone
        // would strip to empty and hit the empty-after-normalization branch
        // instead, which is covered elsewhere.
        let error =
            normalize_planning_json("I'm sorry, I can't help with that request.").unwrap_err();

        assert!(
            error.contains("Could not locate a JSON object"),
            "expected missing-JSON error, got: {error}"
        );
    }

    // ========================================================================
    // Behavior Tests with Mock LLM
    // ========================================================================

    /// Helper function to test planning with a mock client
    async fn plan_new_task_with_mock(
        client: &MockLlmClient,
        user_message: &str,
    ) -> Result<TaskPlan, String> {
        let system_prompt = build_planning_system_prompt();
        let user_prompt = format!(
            "User Request: {}\n\nCreate a task plan for this request.",
            user_message
        );

        let request = ChatRequest::new(
            "test-model",
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
        );

        let response = client
            .chat(request)
            .await
            .map_err(|e| format!("LLM error: {e}"))?;

        let content = normalize_planning_json(&response.content)?;

        let plan: TaskPlan = serde_json::from_str(&content).map_err(|e| {
            format!(
                "Failed to parse planning response as JSON: {}\n\nResponse was: {}",
                e, content
            )
        })?;

        Ok(plan)
    }

    /// Helper function to test replanning with a mock client
    async fn review_and_replan_with_mock(
        client: &MockLlmClient,
        current_plan: &TaskPlan,
        progress_summary: &str,
        user_message: &str,
    ) -> Result<ReplanOutput, String> {
        let system_prompt = build_replan_system_prompt();
        let user_prompt = format!(
            "Current Plan:\n{}\n\nProgress Summary:\n{}\n\nUser's New Request:\n{}\n\nAnalyze and decide what to do next.",
            serde_json::to_string(current_plan).unwrap_or_default(),
            progress_summary,
            user_message
        );

        let request = ChatRequest::new(
            "test-model",
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
        );

        let response = client
            .chat(request)
            .await
            .map_err(|e| format!("LLM error: {e}"))?;

        let content = normalize_planning_json(&response.content)?;

        let output: ReplanOutput = serde_json::from_str(&content).map_err(|e| {
            format!(
                "Failed to parse replan response as JSON: {}\n\nResponse was: {}",
                e, content
            )
        })?;

        Ok(output)
    }

    #[tokio::test]
    async fn test_plan_new_task_with_simple_request() {
        let client = MockLlmClient::anthropic("test-model");
        client.set_response(
            serde_json::json!({
                "title": "Read README file",
                "description": "Read the README.md file",
                "is_non_trivial": false,
                "subtasks": []
            })
            .to_string(),
        );

        let plan = plan_new_task_with_mock(&client, "Read the README file")
            .await
            .unwrap();

        assert_eq!(plan.title, "Read README file");
        assert_eq!(plan.description, "Read the README.md file");
        assert!(!plan.is_non_trivial);
        assert!(plan.subtasks.is_empty());
    }

    #[tokio::test]
    async fn test_plan_new_task_with_complex_request() {
        let client = MockLlmClient::anthropic("test-model");
        client.set_response(serde_json::json!({
            "title": "Codebase investigation and analysis",
            "description": "Investigate the codebase and identify improvement opportunities",
            "is_non_trivial": true,
            "subtasks": [
                {"title": "Scan project structure", "description": "List all source files", "order": 0},
                {"title": "Analyze dependencies", "description": "Review external dependencies", "order": 1},
                {"title": "Identify improvements", "description": "List potential improvements", "order": 2}
            ]
        }).to_string());

        let plan =
            plan_new_task_with_mock(&client, "Investigate codebase and suggest improvements")
                .await
                .unwrap();

        assert!(plan.title.len() <= 80);
        assert!(plan.is_non_trivial);
        assert_eq!(plan.subtasks.len(), 3);
        assert_eq!(plan.subtasks[0].order, 0);
        assert_eq!(plan.subtasks[1].order, 1);
        assert_eq!(plan.subtasks[2].order, 2);
    }

    #[tokio::test]
    async fn test_plan_new_task_handles_reasoning_wrapped_json() {
        let client = MockLlmClient::anthropic("test-model");
        client.set_response(
            r#"<think>The user just said hi, so this task is trivial.</think>

```json
{
  "title": "Respond to Greeting",
  "description": "Acknowledge the user's simple greeting with a friendly response",
  "is_non_trivial": false,
  "subtasks": []
}
```"#
                .to_string(),
        );

        let plan = plan_new_task_with_mock(&client, "Hi").await.unwrap();

        assert_eq!(plan.title, "Respond to Greeting");
        assert!(!plan.is_non_trivial);
        assert!(plan.subtasks.is_empty());
    }

    #[tokio::test]
    async fn test_plan_new_task_handles_invalid_json() {
        let client = MockLlmClient::anthropic("test-model");
        client.set_response("This is not valid JSON".to_string());

        let result = plan_new_task_with_mock(&client, "Some request").await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        // `normalize_planning_json` rejects responses with no JSON object before
        // `serde_json::from_str` ever runs, so either error-path is an acceptable
        // failure mode for "not valid JSON".
        assert!(
            error.contains("Failed to parse") || error.contains("Could not locate a JSON object"),
            "expected parse or locate error, got: {error}"
        );
    }

    #[tokio::test]
    async fn test_review_and_replan_abort_decision() {
        let client = MockLlmClient::anthropic("test-model");
        client.set_response(
            serde_json::json!({
                "decision": "abort",
                "completed_indices": [0],
                "updated_plan": null,
                "user_message": "Switching to cooking topic",
                "next_step_index": null
            })
            .to_string(),
        );

        let current_task = create_test_task("Investigate Rust code");
        let subtasks = vec![create_test_subtask("Scan files", 0)];

        let current_plan = build_plan_from_task(&current_task, &subtasks);

        let output = review_and_replan_with_mock(
            &client,
            &current_plan,
            "Completed 1/3 subtasks",
            "How do I cook pasta?",
        )
        .await
        .unwrap();

        assert_eq!(output.decision, ReplanAction::Abort);
        assert!(output.updated_plan.is_none());
        assert!(output.next_step_index.is_none());
    }

    #[tokio::test]
    async fn test_review_and_replan_continue_decision() {
        let client = MockLlmClient::anthropic("test-model");
        client.set_response(
            serde_json::json!({
                "decision": "continue",
                "completed_indices": [],
                "updated_plan": null,
                "user_message": "Continuing with clarification",
                "next_step_index": null
            })
            .to_string(),
        );

        let current_task = create_test_task("Fix bug in auth");
        let current_plan = build_plan_from_task(&current_task, &[]);

        let output = review_and_replan_with_mock(
            &client,
            &current_plan,
            "In progress on subtask 1",
            "Yes, proceed with the fix",
        )
        .await
        .unwrap();

        assert_eq!(output.decision, ReplanAction::Continue);
    }

    #[tokio::test]
    async fn test_review_and_replan_updates_plan() {
        let client = MockLlmClient::anthropic("test-model");
        client.set_response(serde_json::json!({
            "decision": "replan",
            "completed_indices": [0, 1],
            "updated_plan": {
                "title": "Codebase investigation with performance focus",
                "description": "Investigate and optimize performance",
                "is_non_trivial": true,
                "subtasks": [
                    {"title": "Profile database queries", "description": "Find slow queries", "order": 0}
                ]
            },
            "user_message": "Adding performance optimization to investigation",
            "next_step_index": 0
        }).to_string());

        let current_task = create_test_task("Investigate codebase");
        let subtasks = vec![
            create_test_subtask("Scan files", 0),
            create_test_subtask("Analyze structure", 1),
        ];
        let current_plan = build_plan_from_task(&current_task, &subtasks);

        let output = review_and_replan_with_mock(
            &client,
            &current_plan,
            "Completed scanning and analysis",
            "Also focus on performance",
        )
        .await
        .unwrap();

        assert_eq!(output.decision, ReplanAction::Replan);
        assert!(output.updated_plan.is_some());
        assert_eq!(output.completed_indices, vec![0, 1]);
        assert_eq!(output.next_step_index, Some(0));
        assert!(output.user_message.contains("performance"));
    }

    #[tokio::test]
    async fn test_review_and_replan_handles_reasoning_wrapped_json() {
        let client = MockLlmClient::anthropic("test-model");
        client.set_response(
            r#"<think>The request is related, so replanning is appropriate.</think>

{
  "decision": "replan",
  "completed_indices": [0],
  "updated_plan": {
    "title": "Investigate planner parsing",
    "description": "Harden planner parsing against reasoning wrappers",
    "is_non_trivial": true,
    "subtasks": [
      {
        "title": "Add parser normalization",
        "description": "Strip wrapper content before JSON parsing",
        "order": 0
      }
    ]
  },
  "user_message": "Replanning around the parsing failure",
  "next_step_index": 0
}"#
            .to_string(),
        );

        let current_task = create_test_task("Investigate planner");
        let current_plan = build_plan_from_task(&current_task, &[]);

        let output = review_and_replan_with_mock(
            &client,
            &current_plan,
            "Captured the failing provider response",
            "Focus on parsing wrappers first",
        )
        .await
        .unwrap();

        assert_eq!(output.decision, ReplanAction::Replan);
        assert_eq!(output.completed_indices, vec![0]);
        assert_eq!(output.next_step_index, Some(0));
        assert_eq!(
            output.updated_plan.as_ref().map(|plan| plan.title.as_str()),
            Some("Investigate planner parsing")
        );
    }

    // ========================================================================
    // Test Helper Functions
    // ========================================================================

    fn create_test_task(title: &str) -> crate::core::agent::task::AgentTask {
        use crate::core::agent::session::SessionId;
        use crate::core::agent::task::{AgentRequest, AgentTask, RequestContext};
        let mut task = AgentTask::new(
            SessionId("session-1".to_string()),
            AgentRequest {
                content: title.to_string(),
                attachments: vec![],
                context: Some(RequestContext {
                    file_path: None,
                    selection: Some("Test task".to_string()),
                }),
            },
        );
        task.id = "task-1".to_string();
        task.title = Some(title.to_string());
        task.status = TaskStatus::InProgress;
        task
    }

    fn create_test_subtask(title: &str, order: i32) -> SubTask {
        let mut subtask = SubTask::new(
            "task-1".to_string(),
            title.to_string(),
            SubTaskMode::FullAccess,
            order,
        );
        subtask.id = format!("subtask-{}", order);
        subtask.status = TaskStatus::Completed;
        subtask
    }

    fn build_plan_from_task(
        task: &crate::core::agent::task::AgentTask,
        subtasks: &[SubTask],
    ) -> TaskPlan {
        TaskPlan {
            title: task.title.clone().unwrap_or_default(),
            description: task
                .request
                .context
                .as_ref()
                .and_then(|ctx| ctx.selection.clone())
                .unwrap_or_default(),
            is_non_trivial: !subtasks.is_empty(),
            subtasks: subtasks
                .iter()
                .map(|st| SubtaskPlan {
                    title: st.title.clone(),
                    description: st.description.clone(),
                    order: st.order,
                })
                .collect(),
        }
    }

    // ========================================================================
    // Unified planner (plan_or_continue) tests
    // ========================================================================

    fn pending_subtask(title: &str, order: i32) -> SubTask {
        let mut subtask = SubTask::new(
            "task-1".to_string(),
            title.to_string(),
            SubTaskMode::FullAccess,
            order,
        );
        subtask.id = format!("subtask-{order}");
        subtask.status = TaskStatus::Pending;
        subtask
    }

    fn in_progress_subtask(title: &str, order: i32) -> SubTask {
        let mut s = pending_subtask(title, order);
        s.status = TaskStatus::InProgress;
        s
    }

    fn completed_subtask(title: &str, order: i32) -> SubTask {
        let mut s = pending_subtask(title, order);
        s.status = TaskStatus::Completed;
        s
    }

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
