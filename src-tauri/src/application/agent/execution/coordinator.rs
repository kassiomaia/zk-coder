//! Provides the application-owned execution coordinator for agents.
//!
//! This module contains the main `execute` function that orchestrates the
//! entire lifecycle of an agent's execution, from bootstrapping to completion.

use std::sync::Arc;
use tauri::AppHandle;

use crate::application::agent::harness::HarnessStateStore;
use crate::core::agent::execution::RuntimeTraceType;
use crate::core::agent::memory::MemoryRepository;
use crate::core::agent::permissions::PermissionResolver;
use crate::core::agent::repositories::{
    ExecutionCheckpointRepository, ExecutionRepository, RuntimeTraceRepository, SessionRepository,
    TaskRepository,
};
use crate::core::agent::session::AgentSession;
use crate::core::agent::task::{AgentExecutionResult, AgentRequest};
use crate::core::agent::types::TrustMode;
use crate::core::chat::services::ChatService;
use crate::core::config::ConfigRepository;
use crate::core::llm::entities::{
    ChatMessage as Message, MessageRole, ToolChoiceMode, ToolDefinition,
};
use crate::infrastructure::agent::approval::ApprovalQueue;
use crate::infrastructure::agent::file_snapshot_registry::FileSnapshotRegistry;
use crate::infrastructure::agent::runtime::AgentRuntimeError;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::tools::ToolRegistry;

use super::bootstrap::start_execution;
use super::context_phase::resolve_context;
use super::deps::ExecutionDeps;
use super::llm_phase::call_llm;
use super::park::{maybe_park_for_clarification, ParkOutcome};
use super::response_phase::{handle_response, ResponseDecision};
use super::step_gate::{begin_step, StepGateOutcome};
use super::tool_phase::{run_tools, ToolPhaseOutcome};
use super::yolo::ChatYoloModeProvider;

/// Canonical prompt appended to `llm_context` when the coordinator
/// enters the final-summary branch. Extracted to a constant so the
/// duplicate-push guard can compare the tail against the same byte
/// sequence the runtime emits, and so the wording is tunable in one
/// place.
pub(super) const FINAL_SUMMARY_PROMPT: &str = "All subtasks have been completed. Provide a concise summary of what was accomplished, key findings, and any important results.";

/// Returns `true` when the tail of `messages` is already the
/// canonical final-summary prompt.
///
/// Used by the final-summary branch to avoid pushing the same user
/// message twice on consecutive iterations — a regression observed
/// in the wild that produced two back-to-back `User` messages and
/// either an Anthropic 400 (role-alternation) or an `OpenAI` loop.
/// See `docs/issues/agent-final-summary-tool-call-discard/issue.md`.
pub(super) fn last_message_is_final_summary_prompt(messages: &[Message]) -> bool {
    messages
        .last()
        .filter(|message| matches!(message.role, MessageRole::User))
        .and_then(|message| message.as_text())
        .is_some_and(|text| text == FINAL_SUMMARY_PROMPT)
}

/// Outcome of a single final-summary iteration.
///
/// Mirrors the three actions the coordinator's main loop can take
/// after a normal step (return, restart, abort) but expressed as a
/// dedicated enum so the helper function does not need to construct
/// `ResponseDecision`s the caller already knows how to handle.
enum FinalSummaryOutcome {
    /// The LLM produced a clean text response and execution is done.
    Finished(AgentExecutionResult),
    /// Either the LLM emitted a tool call (rejected via the runtime
    /// allow-list) or `handle_response` returned
    /// `ContinueWithNextSubtask`. The main loop must restart.
    Continue,
    /// `run_tools` observed a session-cancelled state mid-batch.
    Cancelled,
}

/// Runs one iteration of the final-summary path: pushes the
/// canonical prompt (with idempotent guard), forces the runtime
/// into the no-tool-allowed state, calls the LLM, and dispatches
/// the response through the same `handle_response` + `run_tools`
/// pipeline the main loop uses.
///
/// Three layers of defense ensure the model does not call any tool
/// on this turn:
///   1. Prompt: the user message asks for a summary, not work.
///   2. Provider hint: `ToolChoiceMode::None` is forwarded to the LLM.
///   3. Runtime allow-list: `Some(Vec::new())` causes
///      `tool_phase::run_tools` to reject any slip-through call as
///      `subtask_mode_disallowed` so the model can self-correct.
///
/// Layer 3 was previously unreachable because a `_ => continue`
/// arm bypassed `run_tools` for any non-`Finished` response. The
/// extracted helper now uses the same exhaustive dispatch the
/// main loop uses, restoring the documented defense in depth.
/// See `docs/issues/agent-final-summary-tool-call-discard/issue.md`.
#[allow(clippy::too_many_arguments)]
async fn run_final_summary_step(
    deps: &ExecutionDeps<'_>,
    ctx: &mut super::state::ExecutionContext,
    session: &AgentSession,
    resolved: &super::state::ResolvedContext,
    client: &dyn crate::infrastructure::llm::LlmClient,
    permission_resolver: &PermissionResolver,
    yolo_mode_provider: &ChatYoloModeProvider<'_>,
) -> Result<FinalSummaryOutcome, AgentRuntimeError> {
    log::info!(
        "[AgentExecution] All subtasks completed for task {} – requesting finish summary",
        ctx.task_id
    );
    // Idempotent push: if the previous iteration already left the
    // canonical prompt at the tail (e.g. the LLM emitted a tool
    // call that `run_tools` rejected without producing an
    // intervening assistant turn), do not duplicate it. Two
    // consecutive `User` messages break Anthropic's role-
    // alternation contract and degrade other providers.
    if !last_message_is_final_summary_prompt(&ctx.llm_context) {
        ctx.llm_context.push(Message::user(FINAL_SUMMARY_PROMPT));
    }
    ctx.current_allowed_tool_names = Some(Vec::new());
    ctx.pending_tool_choice_override = Some(ToolChoiceMode::None);
    let _ = deps.tracer().append_trace(
        &ctx.session_id,
        &ctx.task_id,
        ctx.current_subtask_id.as_deref(),
        ctx.step_number,
        RuntimeTraceType::LlmResponseClassified,
        serde_json::json!({
            "forcedToolChoiceMode": "none",
            "reason": "all_subtasks_completed_final_summary",
        }),
    );
    let llm_result = call_llm(deps, ctx, resolved, client).await?;
    match handle_response(deps, ctx, session, llm_result).await? {
        ResponseDecision::Finished(result) => Ok(FinalSummaryOutcome::Finished(result)),
        ResponseDecision::ContinueWithNextSubtask => Ok(FinalSummaryOutcome::Continue),
        ResponseDecision::ExecuteTools(response) => {
            match run_tools(
                deps,
                ctx,
                session,
                response,
                permission_resolver,
                yolo_mode_provider,
            )
            .await?
            {
                ToolPhaseOutcome::Continue => Ok(FinalSummaryOutcome::Continue),
                ToolPhaseOutcome::Cancelled => Ok(FinalSummaryOutcome::Cancelled),
            }
        }
    }
}

/// The main execution loop for an agent.
///
/// This function coordinates the entire execution process, which consists of the
/// following steps in a loop:
/// 1.  **Bootstrap**: Initializes the execution context, plans the task, and sets up the environment.
/// 2.  **Step Gate**: Checks for termination conditions before starting a new step.
/// 3.  **Context Resolution**: Prepares the context for the LLM call.
/// 4.  **LLM Call**: Makes a call to the Large Language Model.
/// 5.  **Response Handling**: Processes the LLM's response, which may lead to finishing the execution or calling tools.
/// 6.  **Tool Execution**: If necessary, executes the tools requested by the LLM.
///
/// # Arguments
///
/// * `session` - The agent session for which the execution is running.
/// * `request` - The user's request that initiated the execution.
/// * `agent_system_prompt` - The base system prompt for the agent.
/// * `trust_mode` - The trust mode for the execution.
/// * `app_handle` - A handle to the Tauri application.
/// * `session_repo` - The repository for managing agent sessions.
/// * `task_repo` - The repository for managing agent tasks.
/// * `telemetry_repo` - The repository for agent telemetry.
/// * `execution_repo` - The repository for execution checkpoints.
/// * `trace_repo` - The repository for runtime traces.
/// * `approval_queue` - The queue for handling pending approvals.
/// * `tool_registry` - The registry of available tools.
/// * `file_snapshot_registry` - The registry for file snapshots.
/// * `all_tool_definitions` - A slice of all available tool definitions.
///
/// # Returns
///
/// A `Result` containing the `AgentExecutionResult` on success, or an `AgentRuntimeError` on failure.
#[allow(clippy::too_many_arguments)]
pub async fn execute(
    session: &AgentSession,
    request: AgentRequest,
    agent_system_prompt: &str,
    trust_mode: TrustMode,
    app_handle: &AppHandle,
    session_repo: &dyn SessionRepository,
    chat_service: &dyn ChatService,
    config_repository: &dyn ConfigRepository,
    task_repo: &dyn TaskRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
    execution_repo: &dyn ExecutionRepository,
    execution_checkpoint_repo: &dyn ExecutionCheckpointRepository,
    trace_repo: &dyn RuntimeTraceRepository,
    memory_repo: &dyn MemoryRepository,
    approval_queue: &Arc<ApprovalQueue>,
    tool_registry: &Arc<ToolRegistry>,
    file_snapshot_registry: &Arc<FileSnapshotRegistry>,
    harness_state_store: &Arc<dyn HarnessStateStore>,
    all_tool_definitions: &[ToolDefinition],
) -> Result<AgentExecutionResult, AgentRuntimeError> {
    let deps = ExecutionDeps {
        app_handle,
        session_repo,
        chat_service,
        config_repository,
        task_repo,
        telemetry_repo,
        execution_repo,
        execution_checkpoint_repo,
        trace_repo,
        memory_repo,
        approval_queue,
        tool_registry,
        file_snapshot_registry,
        harness_state_store,
        all_tool_definitions,
    };

    let bootstrap = start_execution(&deps, session, &request, trust_mode).await?;
    let mut ctx = bootstrap.context;
    let client = bootstrap.client;
    let permission_resolver = PermissionResolver::new(trust_mode);
    let yolo_mode_provider = ChatYoloModeProvider::new(deps.chat_service);

    loop {
        match begin_step(&deps, &mut ctx)? {
            StepGateOutcome::Proceed => {}
            StepGateOutcome::Cancelled => return Err(AgentRuntimeError::Cancelled),
        }

        let resolved = resolve_context(
            &deps,
            &mut ctx,
            session,
            &request,
            agent_system_prompt,
            client.as_ref(),
        )
        .await?;

        if resolved.all_subtasks_completed {
            match run_final_summary_step(
                &deps,
                &mut ctx,
                session,
                &resolved,
                client.as_ref(),
                &permission_resolver,
                &yolo_mode_provider,
            )
            .await?
            {
                FinalSummaryOutcome::Finished(result) => return Ok(result),
                FinalSummaryOutcome::Continue => continue,
                FinalSummaryOutcome::Cancelled => {
                    return Err(AgentRuntimeError::Cancelled);
                }
            }
        }

        let llm_result = call_llm(&deps, &mut ctx, &resolved, client.as_ref()).await?;

        match handle_response(&deps, &mut ctx, session, llm_result).await? {
            ResponseDecision::Finished(result) => return Ok(result),
            ResponseDecision::ContinueWithNextSubtask => continue,
            ResponseDecision::ExecuteTools(response) => {
                match run_tools(
                    &deps,
                    &mut ctx,
                    session,
                    response,
                    &permission_resolver,
                    &yolo_mode_provider,
                )
                .await?
                {
                    ToolPhaseOutcome::Continue => {}
                    ToolPhaseOutcome::Cancelled => return Err(AgentRuntimeError::Cancelled),
                }
            }
        }

        // Park if the agent emitted `agent_narrate { intent: "question" }`
        // during the tool phase (agent-clarification-resume plan). The
        // check is cheap when there is no pending clarification; no
        // extra LLM call is made while parked.
        match maybe_park_for_clarification(&deps, &mut ctx).await? {
            ParkOutcome::Continue | ParkOutcome::Replied => {}
            ParkOutcome::TimedOut => {
                return Err(AgentRuntimeError::ClarificationTimedOut);
            }
            ParkOutcome::Cancelled => return Err(AgentRuntimeError::Cancelled),
        }
    }
}

#[cfg(test)]
mod final_summary_guard_tests {
    //! Regression tests for the duplicate-push guard added to the
    //! final-summary branch.
    //!
    //! See `docs/issues/agent-final-summary-tool-call-discard/issue.md`
    //! for the original defect: when the LLM responded to the final-
    //! summary prompt with a tool call, the coordinator skipped
    //! `run_tools` and re-pushed the same `User` prompt on the next
    //! iteration — producing two consecutive `User` messages that
    //! Anthropic rejects with HTTP 400.

    use super::{last_message_is_final_summary_prompt, FINAL_SUMMARY_PROMPT};
    use crate::core::llm::entities::{ChatMessage as Message, ToolCall};
    use serde_json::json;

    fn push_prompt_if_needed(messages: &mut Vec<Message>) {
        if !last_message_is_final_summary_prompt(messages) {
            messages.push(Message::user(FINAL_SUMMARY_PROMPT));
        }
    }

    #[test]
    fn empty_context_pushes_prompt() {
        let mut messages: Vec<Message> = Vec::new();
        push_prompt_if_needed(&mut messages);
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0].as_text(),
            Some(FINAL_SUMMARY_PROMPT),
            "first iteration must seed the prompt"
        );
    }

    #[test]
    fn second_consecutive_iteration_does_not_duplicate_prompt() {
        // Replays the bug: iteration N pushed the prompt, the LLM
        // emitted a tool call that was discarded by `_ => continue`,
        // iteration N+1 reaches the same branch with the prompt still
        // at the tail. Without the guard the prompt would be appended
        // again, producing back-to-back `User` messages.
        let mut messages: Vec<Message> = vec![Message::user(FINAL_SUMMARY_PROMPT)];
        push_prompt_if_needed(&mut messages);
        assert_eq!(
            messages.len(),
            1,
            "guard must skip the duplicate push when the tail already has the prompt"
        );
    }

    #[test]
    fn intervening_assistant_turn_allows_a_new_push() {
        // After `run_tools` lands the assistant + tool-result messages
        // (Option A in the plan), the prompt is no longer at the tail.
        // The next final-summary iteration must push a fresh prompt so
        // the LLM sees a coherent "summarize" request.
        let mut messages: Vec<Message> = vec![
            Message::user(FINAL_SUMMARY_PROMPT),
            Message::assistant_with_tool_calls(
                "I'll narrate progress.",
                vec![ToolCall {
                    id: "call-1".to_string(),
                    name: "agent_narrate".to_string(),
                    arguments: json!({"message": "summarizing"}),
                }],
            ),
            Message::tool_result(
                "call-1",
                json!({
                    "error": "Tool 'agent_narrate' is not permitted under the current subtask mode.",
                    "reason": "subtask_mode_disallowed",
                }),
                true,
            ),
        ];
        push_prompt_if_needed(&mut messages);
        assert_eq!(
            messages.len(),
            4,
            "guard must allow a fresh push when the tail is no longer the prompt"
        );
        assert_eq!(
            messages.last().and_then(Message::as_text),
            Some(FINAL_SUMMARY_PROMPT)
        );
    }

    #[test]
    fn assistant_text_at_tail_allows_a_new_push() {
        // Realistic shape after a clean text response from the model:
        // the assistant turn lands at the tail. The next final-summary
        // entry must seed a fresh prompt.
        let mut messages: Vec<Message> = vec![
            Message::user(FINAL_SUMMARY_PROMPT),
            Message::assistant("Here's the summary..."),
        ];
        push_prompt_if_needed(&mut messages);
        assert_eq!(messages.len(), 3);
        assert_eq!(
            messages.last().and_then(Message::as_text),
            Some(FINAL_SUMMARY_PROMPT)
        );
    }

    #[test]
    fn user_message_with_different_text_does_not_block_push() {
        // The guard must compare byte-for-byte, not just on role —
        // an unrelated user reply at the tail does not satisfy the
        // "already at the tail" precondition.
        let mut messages: Vec<Message> = vec![Message::user("ping")];
        push_prompt_if_needed(&mut messages);
        assert_eq!(messages.len(), 2);
        assert_eq!(
            messages.last().and_then(Message::as_text),
            Some(FINAL_SUMMARY_PROMPT)
        );
    }

    #[test]
    fn final_summary_prompt_is_stable_string() {
        // Pin the exact wording the runtime emits so the
        // `disallowed-by-mode` correction (which the LLM sees on the
        // next step) and any future telemetry filter that wants to
        // recognise this prompt have a single, byte-stable target.
        // Bumping the wording requires updating this assertion
        // deliberately.
        assert_eq!(
            FINAL_SUMMARY_PROMPT,
            "All subtasks have been completed. Provide a concise summary of what was accomplished, key findings, and any important results."
        );
    }
}
