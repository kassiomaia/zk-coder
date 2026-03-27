//! Main execution orchestration for agent runtime.

use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use log::{info, warn};
use serde_json::json;
use tauri::{AppHandle, Emitter};

use crate::core::agent::permissions::PermissionResolver;
use crate::core::agent::repositories::{SessionRepository, TaskRepository};
use crate::core::agent::session::{AgentSession, SessionId, SessionState};
use crate::core::agent::task::{
    AgentExecutionResult, AgentRequest, AgentTask, ExecutionStep, StepAction, StepResult,
    TaskStatus, UsageStats,
};
use crate::core::agent::types::TrustMode;
use crate::core::llm::entities::{ChatMessage as Message, ChatRequest, ToolCall, ToolDefinition};
use crate::infrastructure::agent::approval::ApprovalQueue;
use crate::infrastructure::agent::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::infrastructure::agent::file_snapshot_registry::FileSnapshotRegistry;
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::llm::LlmClient;
use crate::infrastructure::tools::context::ToolExecutionContext;
use crate::infrastructure::tools::ToolRegistry;

use super::client::{build_llm_context, chat_with_retry, create_llm_client, LlmRetryConfig};
use super::context::{
    apply_context_policy_v2, estimate_context_tokens, maybe_compact_context, ContextPolicyInput,
    COMPRESSION_THRESHOLD, CONTEXT_WINDOW,
};
use super::process_tool_calls_with_permissions;
use super::{add_to_recent_calls, detect_repetitive_loop, MAX_REPETITIONS};
use super::{derive_system_prompt, AgentRuntimeError, SubtaskSummary};

/// Maximum LLM retries for transient errors.
const LLM_MAX_RETRIES: usize = 2;

/// Base delay in ms for LLM retry backoff.
const LLM_RETRY_BASE_MS: u64 = 1000;

/// Default temperature for agent execution (coding-safe).
const DEFAULT_AGENT_TEMPERATURE: f32 = 0.0;

/// Environment override for agent temperature.
const ENV_AGENT_TEMPERATURE: &str = "CODER_AGENT_TEMPERATURE";
/// Safety limit for max agent loop steps.
const MAX_EXECUTION_STEPS: u32 = 120;

fn agent_temperature() -> f32 {
    let value = std::env::var(ENV_AGENT_TEMPERATURE)
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(DEFAULT_AGENT_TEMPERATURE);
    value.clamp(0.0, 2.0)
}

fn session_is_cancelled(
    session_repo: &dyn SessionRepository,
    session_id: &SessionId,
) -> Result<bool, AgentRuntimeError> {
    let session = session_repo
        .get_session(&crate::core::agent::entities::SessionId(session_id.0.clone()))
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;

    Ok(matches!(
        session.state,
        crate::core::agent::entities::SessionState::Cancelled { .. }
    ))
}

fn cancel_active_task(
    session_id: &str,
    task_id: &str,
    app_handle: &AppHandle,
    task_repo: &dyn TaskRepository,
) {
    let _ = task_repo.update_task_status(task_id, crate::core::agent::entities::TaskStatus::Cancelled);
    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::TaskUpdated {
            session_id: session_id.to_string(),
            task_id: task_id.to_string(),
            status: TaskStatus::Cancelled,
        },
    );
}

/// Executes an agent request within a session.
///
/// This is the main entry point. It:
/// 1. Resolves the LLM client from config
/// 2. Builds context from conversation history
/// 3. Runs the tool-calling loop with permission checks
/// 4. Emits events for each step
/// 5. Returns the final result
pub async fn execute(
    session: &AgentSession,
    request: AgentRequest,
    agent_system_prompt: &str,
    trust_mode: TrustMode,
    app_handle: &AppHandle,
    session_repo: &dyn SessionRepository,
    task_repo: &dyn TaskRepository,
    telemetry_repo: &dyn AgentTelemetryRepository,
    approval_queue: &Arc<ApprovalQueue>,
    tool_registry: &Arc<ToolRegistry>,
    file_snapshot_registry: &Arc<FileSnapshotRegistry>,
    all_tool_definitions: &[ToolDefinition],
) -> Result<AgentExecutionResult, AgentRuntimeError> {
    let session_id = session.id.0.clone();
    let permission_resolver = PermissionResolver::new(trust_mode);

    // Update session state to running
    update_session_state(
        session_repo,
        &session.id,
        SessionState::Running {
            current_step: 0,
            context_usage: 0.0,
        },
        app_handle,
    )?;

    // Emit execution started
    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::ExecutionStarted {
            session_id: session_id.clone(),
            request_id: uuid::Uuid::new_v4().to_string(),
        },
    );

    // Create and persist the task for this execution
    let task = AgentTask::new(session.id.clone(), request.clone());
    let core_task = crate::core::agent::entities::AgentTask {
        id: task.id.clone(),
        session_id: task.session_id.0.clone(),
        parent_task_id: None,
        title: derive_task_title(&task.request.content),
        // Persist the full latest request content for crash recovery / retry flows.
        description: Some(task.request.content.clone()),
        status: crate::core::agent::entities::TaskStatus::Pending,
        order: 0,
        mode: Some(crate::core::agent::entities::TaskMode::FullAccess),
    };
    if let Err(e) = task_repo.create_task(&core_task) {
        warn!("[AgentRuntime] Failed to persist task: {e}");
    }
    let task_id = task.id.clone();

    // Emit task created event
    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::TaskCreated {
            session_id: session_id.clone(),
            task,
        },
    );

    // Create LLM client
    let (client, model_name) = create_llm_client(app_handle).await?;

    // Build initial LLM context
    let mut llm_context = super::client::build_llm_context(session, &request, app_handle).await?;

    let mut steps: Vec<ExecutionStep> = Vec::new();
    let mut total_input_tokens: u32 = 0;
    let mut total_output_tokens: u32 = 0;
    let mut step_num: u32 = 0;

    // Track recent tool calls for loop detection: (tool_name, arguments_hash)
    let mut recent_tool_calls: Vec<(String, u64)> = Vec::new();

    // Track which subtask we last injected context for (to detect changes)
    let mut last_injected_subtask_order: Option<i32> = None;

    // Mark task as in progress
    let _ = task_repo.update_task_status(
        &task_id,
        crate::core::agent::entities::TaskStatus::InProgress,
    );

    // === EXECUTION PHASE ===
    // Agent runs until it has no more tool calls (decides it's done)
    loop {
        if session_is_cancelled(session_repo, &session.id)? {
            cancel_active_task(&session_id, &task_id, app_handle, task_repo);
            return Err(AgentRuntimeError::Cancelled);
        }

        step_num += 1;
        if step_num > MAX_EXECUTION_STEPS {
            let msg = format!(
                "Execution step limit reached ({}). Aborting to prevent runaway loop.",
                MAX_EXECUTION_STEPS
            );
            warn!("[AgentRuntime] {}", msg);
            update_session_state(
                session_repo,
                &session.id,
                SessionState::Error {
                    message: msg.clone(),
                    recoverable: true,
                },
                app_handle,
            )?;

            let _ = task_repo
                .update_task_status(&task_id, crate::core::agent::entities::TaskStatus::Failed);
            let _ = app_handle.emit(
                AGENT_EVENT_NAME,
                AgentEvent::TaskUpdated {
                    session_id: session_id.clone(),
                    task_id: task_id.clone(),
                    status: TaskStatus::Failed,
                },
            );
            return Err(AgentRuntimeError::LoopDetected);
        }
        info!(
            "[AgentRuntime] Step {} for session {}",
            step_num, session_id
        );

        // Calculate context usage percentage
        let estimated_tokens = estimate_context_tokens(&llm_context);
        let context_usage = (estimated_tokens as f64 / CONTEXT_WINDOW as f64 * 100.0).min(100.0);

        // Update session state with current context usage
        update_session_state(
            session_repo,
            &session.id,
            SessionState::Running {
                current_step: step_num,
                context_usage,
            },
            app_handle,
        )?;

        // Get current subtask for mode filtering and prompt derivation
        let (current_subtask_summary, tool_names) =
            if let Ok(subtasks) = task_repo.get_subtasks(&task_id) {
                let total = subtasks.len();
                if let Some(current) = subtasks
                    .iter()
                    .find(|st| st.status == crate::core::agent::entities::TaskStatus::InProgress)
                {
                    let summary = SubtaskSummary::from_subtask(current);
                    let allowed = summary.mode.allowed_tools();
                    info!(
                        "[AgentRuntime] Subtask '{}' ({:?} mode): {} tools available",
                        current.title,
                        summary.mode,
                        allowed.len()
                    );
                    (Some((summary, total)), allowed)
                } else {
                    (
                        None,
                        all_tool_definitions
                            .iter()
                            .map(|t| t.name.as_str())
                            .collect::<Vec<_>>(),
                    )
                }
            } else {
                (
                    None,
                    all_tool_definitions
                        .iter()
                        .map(|t| t.name.as_str())
                        .collect::<Vec<_>>(),
                )
            };

        // Detect and log subtask changes
        let current_order = current_subtask_summary
            .as_ref()
            .map(|(summary, _)| summary.order);
        let subtask_changed = last_injected_subtask_order != current_order;

        if subtask_changed {
            match (last_injected_subtask_order, current_order) {
                (None, Some(order)) => {
                    info!("[AgentRuntime] Subtask changed: None → Some({})", order);
                }
                (Some(prev), Some(curr)) => {
                    info!(
                        "[AgentRuntime] Subtask changed: Some({}) → Some({})",
                        prev, curr
                    );
                }
                (Some(prev), None) => {
                    info!(
                        "[AgentRuntime] Subtask changed: Some({}) → None (no active subtask)",
                        prev
                    );
                }
                (None, None) => {}
            }
        }

        let usage_ratio = estimate_context_tokens(&llm_context) as f64 / CONTEXT_WINDOW as f64;
        let should_retrieve = usage_ratio >= COMPRESSION_THRESHOLD || subtask_changed;
        let retrieval_snippets = if should_retrieve {
            run_retrieval_pass(
                tool_registry,
                app_handle,
                session,
                &request.content,
                current_subtask_summary
                    .as_ref()
                    .map(|(summary, _)| summary.title.as_str()),
            )
            .await
        } else {
            Vec::new()
        };

        let policy_input = ContextPolicyInput {
            force_pack: subtask_changed,
            task_goal: Some(request.content.clone()),
            active_subtask: current_subtask_summary
                .as_ref()
                .map(|(summary, _)| summary.title.clone()),
            retrieval_snippets,
        };
        if let Some(compaction) = apply_context_policy_v2(&mut llm_context, &policy_input) {
            info!(
                "[AgentRuntime] ContextPolicyV2 packed: {} messages → {}, {} tokens → {}",
                compaction.before_count,
                compaction.after_count,
                compaction.before_tokens,
                compaction.after_tokens
            );
            let _ = app_handle.emit(
                AGENT_EVENT_NAME,
                AgentEvent::ContextCompacted {
                    session_id: session_id.clone(),
                    conversation_id: session.conversation_id.clone(),
                    before_count: compaction.before_count,
                    after_count: compaction.after_count,
                    before_tokens: compaction.before_tokens,
                    after_tokens: compaction.after_tokens,
                    timestamp: Utc::now(),
                },
            );
            let _ = compaction.needs_reinjection;
        } else if usage_ratio >= COMPRESSION_THRESHOLD {
            // Fallback to legacy compaction if v2 didn't apply for any reason.
            if let Some(compaction) = maybe_compact_context(&mut llm_context) {
                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::ContextCompacted {
                        session_id: session_id.clone(),
                        conversation_id: session.conversation_id.clone(),
                        before_count: compaction.before_count,
                        after_count: compaction.after_count,
                        before_tokens: compaction.before_tokens,
                        after_tokens: compaction.after_tokens,
                        timestamp: Utc::now(),
                    },
                );
            }
        }
        last_injected_subtask_order = current_order;

        // Derive system prompt with subtask context
        let system_prompt = derive_system_prompt(agent_system_prompt, &current_subtask_summary);
        info!(
            "[AgentRuntime] System prompt length: {}{}",
            system_prompt.len(),
            if current_subtask_summary.is_some() {
                " (with subtask context)"
            } else {
                ""
            }
        );

        // Filter tool definitions based on current mode
        let tool_definitions: Vec<ToolDefinition> = all_tool_definitions
            .iter()
            .filter(|t| tool_names.contains(&t.name.as_str()))
            .cloned()
            .collect();

        // Build chat request
        let mut chat_request = ChatRequest::new(&model_name, llm_context.clone())
            .with_system(&system_prompt)
            .with_temperature(agent_temperature())
            .with_max_tokens(4096);
        if !tool_definitions.is_empty() {
            chat_request = chat_request.with_tools(tool_definitions.clone());
        }

        // Call LLM with retry
        let start = Instant::now();
        let retry_config = LlmRetryConfig {
            max_retries: LLM_MAX_RETRIES,
            base_delay_ms: LLM_RETRY_BASE_MS,
        };
        let response = chat_with_retry(client.as_ref(), &chat_request, &retry_config).await?;
        let duration_ms = start.elapsed().as_millis() as u64;

        // Track usage
        if let Some(usage) = &response.usage {
            total_input_tokens += usage.input_tokens;
            total_output_tokens += usage.output_tokens;
        }

        // Get current subtask_id for execution step linkage
        let current_subtask_id = if let Ok(subtasks) = task_repo.get_subtasks(&task_id) {
            subtasks
                .iter()
                .find(|st| st.status == crate::core::agent::entities::TaskStatus::InProgress)
                .map(|st| st.id.clone())
        } else {
            None
        };

        // Record LLM step with full token details
        let llm_step = ExecutionStep {
            step_number: step_num,
            action: StepAction::LlmRequest {
                model: response.model.clone(),
            },
            result: Some(StepResult {
                success: true,
                output: serde_json::json!({
                    "content_length": response.content.len(),
                    "tool_calls": response.tool_calls.len(),
                }),
                tokens_used: response.usage.as_ref().map(|u| u.total_tokens),
                input_tokens: response.usage.as_ref().map(|u| u.input_tokens),
                output_tokens: response.usage.as_ref().map(|u| u.output_tokens),
                duration_ms,
            }),
            timestamp: Utc::now(),
        };

        // Persist execution step to database
        if let Err(e) = telemetry_repo.create_execution_step(
            &llm_step,
            &session_id,
            Some(&task_id),
            current_subtask_id.as_deref(),
        ) {
            warn!("[AgentRuntime] Failed to persist execution step: {e}");
        }

        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::ExecutionStep {
                session_id: session_id.clone(),
                step: llm_step.clone(),
            },
        );
        steps.push(llm_step);

        // If no tool calls, we're done - agent has completed its task
        if response.tool_calls.is_empty() {
            if session_is_cancelled(session_repo, &session.id)? {
                cancel_active_task(&session_id, &task_id, app_handle, task_repo);
                return Err(AgentRuntimeError::Cancelled);
            }
            return finalize_execution(
                response.content,
                &session_id,
                &session.conversation_id,
                &session.thread_id,
                &task_id,
                steps,
                total_input_tokens,
                total_output_tokens,
                app_handle,
                session_repo,
                telemetry_repo,
                task_repo,
            )
            .await;
        }

        // Check for repetitive action loops
        for tool_call in &response.tool_calls {
            if tool_call.name == "agent_plan_subtasks" {
                let seen_before = recent_tool_calls
                    .iter()
                    .any(|(name, _)| name == "agent_plan_subtasks");
                if seen_before {
                    warn!(
                        "[AgentRuntime] Repeated planning call detected: agent_plan_subtasks called again; treating as idempotent and continuing"
                    );
                }
            }
            if let Some(repeat_count) =
                detect_repetitive_loop(tool_call, &recent_tool_calls, MAX_REPETITIONS)
            {
                let msg = format!(
                    "Detected repetitive loop: tool '{}' called with same arguments {} times",
                    tool_call.name,
                    repeat_count + 1
                );
                warn!("[AgentRuntime] {}", msg);

                update_session_state(
                    session_repo,
                    &session.id,
                    SessionState::Error {
                        message: msg.clone(),
                        recoverable: true,
                    },
                    app_handle,
                )?;

                // Mark task as failed
                let _ = task_repo
                    .update_task_status(&task_id, crate::core::agent::entities::TaskStatus::Failed);
                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::TaskUpdated {
                        session_id: session_id.clone(),
                        task_id: task_id.clone(),
                        status: TaskStatus::Failed,
                    },
                );
                return Err(AgentRuntimeError::LoopDetected);
            }
        }

        // Add current tool calls to history
        add_to_recent_calls(&response.tool_calls, &mut recent_tool_calls, 10);

        // Process tool calls with permission checks
        llm_context.push(Message::assistant_with_tool_calls(
            response.content,
            response.tool_calls.clone(),
        ));

        let tool_results = process_tool_calls_with_permissions(
            &response.tool_calls,
            session,
            &task_id,
            &permission_resolver,
            app_handle,
            approval_queue,
            tool_registry,
            file_snapshot_registry,
            session_repo,
            telemetry_repo,
            task_repo,
        )
        .await?;

        if session_is_cancelled(session_repo, &session.id)? {
            cancel_active_task(&session_id, &task_id, app_handle, task_repo);
            return Err(AgentRuntimeError::Cancelled);
        }

        // Append tool results to context
        for result in &tool_results {
            llm_context.push(Message::tool_result(
                result.call_id.clone(),
                result.output.clone(),
                result.is_error,
            ));
        }
    }
}

async fn run_retrieval_pass(
    tool_registry: &ToolRegistry,
    app_handle: &AppHandle,
    session: &AgentSession,
    request_content: &str,
    active_subtask_title: Option<&str>,
) -> Vec<String> {
    let query = build_retrieval_query(request_content, active_subtask_title);
    if query.is_empty() {
        return Vec::new();
    }

    let call = ToolCall {
        id: uuid::Uuid::new_v4().to_string(),
        name: "search_grep".to_string(),
        arguments: json!({
            "query": query,
            "max_results": 12,
            "case_sensitive": false
        }),
    };
    let context = ToolExecutionContext::new(&session.conversation_id)
        .with_thread(session.thread_id.clone())
        .with_session(session.id.0.clone());
    let context = if let Some(workspace) = &session.workspace_root {
        context.with_workspace(workspace.clone())
    } else {
        context
    };

    let result = tool_registry
        .execute(call, context, app_handle.clone())
        .await;
    if result.is_error {
        return Vec::new();
    }

    result
        .output
        .get("matches")
        .and_then(|v| v.as_array())
        .map(|matches| {
            matches
                .iter()
                .take(8)
                .filter_map(|m| {
                    let path = m.get("path").and_then(|v| v.as_str())?;
                    let line = m.get("line").and_then(|v| v.as_u64()).unwrap_or(0);
                    let text = m.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    Some(format!("{}:{} {}", path, line, text.trim()))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn build_retrieval_query(request_content: &str, active_subtask_title: Option<&str>) -> String {
    let mut raw = String::new();
    raw.push_str(request_content);
    raw.push(' ');
    if let Some(title) = active_subtask_title {
        raw.push_str(title);
    }

    let stop_words = [
        "the", "and", "for", "with", "this", "that", "from", "into", "about", "please", "would",
        "should", "could", "must", "task",
    ];
    let mut keywords: Vec<String> = Vec::new();
    for token in raw.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-') {
        let t = token.trim().to_lowercase();
        if t.len() < 4 {
            continue;
        }
        if stop_words.contains(&t.as_str()) {
            continue;
        }
        if keywords.iter().any(|k| k == &t) {
            continue;
        }
        keywords.push(t);
        if keywords.len() >= 5 {
            break;
        }
    }
    keywords.join("|")
}

fn derive_task_title(content: &str) -> String {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return "General request".to_string();
    }
    let lower = trimmed.to_lowercase();
    let is_greeting = matches!(
        lower.as_str(),
        "hi" | "hello" | "hey" | "yo" | "hiya" | "sup" | "hello!" | "hi!" | "hey!"
    );
    if is_greeting || trimmed.len() < 16 {
        return "General request".to_string();
    }

    const MAX_LEN: usize = 80;
    if trimmed.len() <= MAX_LEN {
        return trimmed.to_string();
    }
    let mut out = trimmed[..MAX_LEN].to_string();
    out.push_str("...");
    out
}

/// Finalizes the agent execution and returns the result.
async fn finalize_execution(
    content: String,
    session_id: &str,
    conversation_id: &str,
    thread_id: &str,
    task_id: &str,
    steps: Vec<ExecutionStep>,
    total_input_tokens: u32,
    total_output_tokens: u32,
    app_handle: &AppHandle,
    session_repo: &dyn SessionRepository,
    _telemetry_repo: &dyn AgentTelemetryRepository,
    task_repo: &dyn TaskRepository,
) -> Result<AgentExecutionResult, AgentRuntimeError> {
    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::StreamingContent {
            session_id: session_id.to_string(),
            conversation_id: conversation_id.to_string(),
            thread_id: thread_id.to_string(),
            chunk: content.clone(),
            is_final: true,
        },
    );

    let result = AgentExecutionResult {
        content: content.clone(),
        steps,
        usage: Some(UsageStats {
            input_tokens: total_input_tokens,
            output_tokens: total_output_tokens,
            total_tokens: total_input_tokens + total_output_tokens,
        }),
    };

    // Update task to completed
    if let Err(e) =
        task_repo.update_task_status(task_id, crate::core::agent::entities::TaskStatus::Completed)
    {
        warn!("[AgentRuntime] Failed to update task status: {e}");
    }
    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::TaskUpdated {
            session_id: session_id.to_string(),
            task_id: task_id.to_string(),
            status: TaskStatus::Completed,
        },
    );

    // Update session to finished
    update_session_state(
        session_repo,
        &SessionId(session_id.to_string()),
        SessionState::Finished {
            completed_at: Utc::now(),
        },
        app_handle,
    )?;

    // Emit execution complete event
    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::ExecutionComplete {
            session_id: session_id.to_string(),
            conversation_id: conversation_id.to_string(),
            thread_id: thread_id.to_string(),
            result: result.clone(),
        },
    );

    Ok(result)
}

/// Update session state and emit corresponding event.
fn update_session_state(
    session_repo: &dyn SessionRepository,
    session_id: &SessionId,
    state: SessionState,
    app_handle: &AppHandle,
) -> Result<(), AgentRuntimeError> {
    let core_state: crate::core::agent::entities::SessionState = state.clone().into();
    session_repo
        .update_state(
            &crate::core::agent::entities::SessionId(session_id.0.clone()),
            &core_state,
        )
        .map_err(|e| AgentRuntimeError::Repository(e.to_string()))?;

    let _ = app_handle.emit(
        AGENT_EVENT_NAME,
        AgentEvent::SessionStateChanged {
            session_id: session_id.0.clone(),
            state,
        },
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::session::SessionId;
    use crate::core::agent::task::{AgentRequest, TaskStatus};
    use crate::core::agent::types::TrustMode;
    use serde_json::json;

    #[test]
    fn test_constants() {
        assert_eq!(LLM_MAX_RETRIES, 2);
        assert_eq!(LLM_RETRY_BASE_MS, 1000);
    }

    #[test]
    fn test_update_session_state_running() {
        // This test verifies the session state update logic
        // In a real scenario, this would use a mock store
        let state = SessionState::Running {
            current_step: 5,
            context_usage: 45.5,
        };

        match state {
            SessionState::Running {
                current_step,
                context_usage,
            } => {
                assert_eq!(current_step, 5);
                assert_eq!(context_usage, 45.5);
            }
            _ => panic!("Expected Running state"),
        }
    }

    #[test]
    fn test_session_state_finished() {
        let state = SessionState::Finished {
            completed_at: Utc::now(),
        };

        match state {
            SessionState::Finished { completed_at } => {
                assert!(completed_at <= Utc::now());
            }
            _ => panic!("Expected Finished state"),
        }
    }

    #[test]
    fn test_session_state_error() {
        let state = SessionState::Error {
            message: "Test error".to_string(),
            recoverable: true,
        };

        match state {
            SessionState::Error {
                message,
                recoverable,
            } => {
                assert_eq!(message, "Test error");
                assert!(recoverable);
            }
            _ => panic!("Expected Error state"),
        }
    }

    #[test]
    fn test_session_state_transitions() {
        // Test that session states can be created and converted
        let running = SessionState::Running {
            current_step: 1,
            context_usage: 10.0,
        };

        let finished = SessionState::Finished {
            completed_at: Utc::now(),
        };

        let error = SessionState::Error {
            message: "Something went wrong".to_string(),
            recoverable: false,
        };

        // Verify all states can be created
        assert!(matches!(running, SessionState::Running { .. }));
        assert!(matches!(finished, SessionState::Finished { .. }));
        assert!(matches!(error, SessionState::Error { .. }));
    }

    #[test]
    fn test_agent_request_creation() {
        let request = AgentRequest {
            content: "Test content".to_string(),
            attachments: vec![],
            context: None,
        };

        assert_eq!(request.content, "Test content");
        assert!(request.attachments.is_empty());
        assert!(request.context.is_none());
    }

    #[test]
    fn test_agent_request_with_context() {
        let request = AgentRequest {
            content: "Test content".to_string(),
            attachments: vec![],
            context: Some(crate::core::agent::task::RequestContext {
                file_path: Some("/path/to/file.rs".to_string()),
                selection: Some("selected text".to_string()),
            }),
        };

        assert!(request.context.is_some());
        let ctx = request.context.unwrap();
        assert_eq!(ctx.file_path, Some("/path/to/file.rs".to_string()));
        assert_eq!(ctx.selection, Some("selected text".to_string()));
    }

    #[test]
    fn test_task_status_serialization() {
        // Verify task statuses can be serialized and deserialized
        let statuses = vec![
            TaskStatus::Pending,
            TaskStatus::InProgress,
            TaskStatus::WaitingApproval,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Cancelled,
        ];

        for status in statuses {
            let serialized = serde_json::to_string(&status).unwrap();
            let deserialized: TaskStatus = serde_json::from_str(&serialized).unwrap();
            assert_eq!(status, deserialized);
        }
    }

    #[test]
    fn test_execution_step_creation() {
        let step = ExecutionStep {
            step_number: 1,
            action: StepAction::LlmRequest {
                model: "gpt-4".to_string(),
            },
            result: Some(StepResult {
                success: true,
                output: json!({"test": "data"}),
                tokens_used: Some(100),
                input_tokens: Some(50),
                output_tokens: Some(50),
                duration_ms: 1000,
            }),
            timestamp: Utc::now(),
        };

        assert_eq!(step.step_number, 1);
        assert!(step.result.is_some());
        let result = step.result.unwrap();
        assert!(result.success);
        assert_eq!(result.tokens_used, Some(100));
    }

    #[test]
    fn test_step_action_serialization() {
        let actions = vec![
            StepAction::LlmRequest {
                model: "test-model".to_string(),
            },
            StepAction::ToolCall {
                tool: "fs_read_file".to_string(),
                arguments: json!({"path": "/test.txt"}),
            },
        ];

        for action in actions {
            let serialized = serde_json::to_string(&action).unwrap();
            let deserialized: StepAction = serde_json::from_str(&serialized).unwrap();
            // We can't directly compare StepAction due to Value differences
            // but we can verify it round-trips
            match (&action, &deserialized) {
                (StepAction::LlmRequest { model: a }, StepAction::LlmRequest { model: b }) => {
                    assert_eq!(a, b);
                }
                (StepAction::ToolCall { tool: a, .. }, StepAction::ToolCall { tool: b, .. }) => {
                    assert_eq!(a, b);
                }
                _ => panic!("Action types don't match"),
            }
        }
    }

    #[test]
    fn test_usage_stats() {
        let stats = crate::core::agent::task::UsageStats {
            input_tokens: 1000,
            output_tokens: 500,
            total_tokens: 1500,
        };

        assert_eq!(stats.input_tokens, 1000);
        assert_eq!(stats.output_tokens, 500);
        assert_eq!(stats.total_tokens, 1500);
    }

    #[test]
    fn test_agent_execution_result() {
        let result = crate::core::agent::task::AgentExecutionResult {
            content: "Final answer".to_string(),
            steps: vec![],
            usage: Some(crate::core::agent::task::UsageStats {
                input_tokens: 100,
                output_tokens: 50,
                total_tokens: 150,
            }),
        };

        assert_eq!(result.content, "Final answer");
        assert!(result.steps.is_empty());
        assert!(result.usage.is_some());
    }

    // Additional state transition and edge case tests
    #[test]
    fn test_session_state_error_message_content() {
        let state = SessionState::Error {
            message: "Loop detected".to_string(),
            recoverable: true,
        };

        // Verify error message is preserved
        match state {
            SessionState::Error { message, .. } => {
                assert_eq!(message, "Loop detected");
                assert!(message.len() > 0);
            }
            _ => panic!("Expected Error state"),
        }
    }

    #[test]
    fn test_task_status_terminal_states() {
        // Test that terminal states are correctly identified
        let completed = TaskStatus::Completed;
        let failed = TaskStatus::Failed;
        let cancelled = TaskStatus::Cancelled;

        // These are terminal states
        let is_terminal_completed = matches!(
            completed,
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
        );
        let is_terminal_failed = matches!(
            failed,
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
        );
        let is_terminal_cancelled = matches!(
            cancelled,
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
        );

        assert!(is_terminal_completed);
        assert!(is_terminal_failed);
        assert!(is_terminal_cancelled);
    }

    #[test]
    fn test_task_status_non_terminal_states() {
        // Test that non-terminal states are correctly identified
        let pending = TaskStatus::Pending;
        let in_progress = TaskStatus::InProgress;
        let waiting = TaskStatus::WaitingApproval;

        let is_terminal_pending = matches!(
            pending,
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
        );
        let is_terminal_in_progress = matches!(
            in_progress,
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
        );
        let is_terminal_waiting = matches!(
            waiting,
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
        );

        assert!(!is_terminal_pending);
        assert!(!is_terminal_in_progress);
        assert!(!is_terminal_waiting);
    }

    #[test]
    fn test_execution_step_without_result() {
        let step = ExecutionStep {
            step_number: 5,
            action: StepAction::LlmRequest {
                model: "claude-3".to_string(),
            },
            result: None,
            timestamp: Utc::now(),
        };

        assert_eq!(step.step_number, 5);
        assert!(step.result.is_none());
    }

    #[test]
    fn test_execution_step_failed_result() {
        let step = ExecutionStep {
            step_number: 2,
            action: StepAction::ToolCall {
                tool: "fs_write_file".to_string(),
                arguments: json!({"path": "/tmp/test.txt", "content": "hello"}),
            },
            result: Some(StepResult {
                success: false,
                output: json!({"error": "Permission denied"}),
                tokens_used: Some(0),
                input_tokens: Some(0),
                output_tokens: Some(0),
                duration_ms: 50,
            }),
            timestamp: Utc::now(),
        };

        let result = step.result.unwrap();
        assert!(!result.success);
        assert_eq!(result.duration_ms, 50);
    }

    #[test]
    fn test_execution_step_with_token_tracking() {
        let step = ExecutionStep {
            step_number: 1,
            action: StepAction::LlmRequest {
                model: "gpt-4".to_string(),
            },
            result: Some(StepResult {
                success: true,
                output: json!({}),
                tokens_used: Some(500),
                input_tokens: Some(300),
                output_tokens: Some(200),
                duration_ms: 1500,
            }),
            timestamp: Utc::now(),
        };

        let result = step.result.unwrap();
        // Verify token calculation
        assert_eq!(result.tokens_used, Some(500));
        assert_eq!(result.input_tokens, Some(300));
        assert_eq!(result.output_tokens, Some(200));
        assert_eq!(
            result.input_tokens.unwrap() + result.output_tokens.unwrap(),
            500
        );
    }

    #[test]
    fn test_agent_task_status_progression() {
        // Test status progression through the lifecycle
        let statuses = vec![
            TaskStatus::Pending,
            TaskStatus::InProgress,
            TaskStatus::WaitingApproval,
            TaskStatus::InProgress, // Back to in progress after approval
            TaskStatus::Completed,
        ];

        // Verify we can create and compare each status
        for status in &statuses {
            let _ = *status;
        }

        assert_eq!(statuses[0], TaskStatus::Pending);
        assert_eq!(statuses[1], TaskStatus::InProgress);
        assert_eq!(statuses[4], TaskStatus::Completed);
    }

    #[test]
    fn test_session_id_equality() {
        let id1 = SessionId("session-123".to_string());
        let id2 = SessionId("session-123".to_string());
        let id3 = SessionId("session-456".to_string());

        assert_eq!(id1.0, id2.0);
        assert_ne!(id1.0, id3.0);
    }

    #[test]
    fn test_step_action_tool_call_with_various_arguments() {
        let actions = vec![
            StepAction::ToolCall {
                tool: "fs_read_file".to_string(),
                arguments: json!({"path": "/test.txt"}),
            },
            StepAction::ToolCall {
                tool: "fs_write_file".to_string(),
                arguments: json!({"path": "/out.txt", "content": "hello world"}),
            },
            StepAction::ToolCall {
                tool: "system_execute_command".to_string(),
                arguments: json!({"command": "cargo test", "cwd": "/project"}),
            },
        ];

        for action in actions {
            let serialized = serde_json::to_string(&action).unwrap();
            let deserialized: StepAction = serde_json::from_str(&serialized).unwrap();

            match (&action, &deserialized) {
                (
                    StepAction::ToolCall {
                        tool: t1,
                        arguments: a1,
                    },
                    StepAction::ToolCall {
                        tool: t2,
                        arguments: a2,
                    },
                ) => {
                    assert_eq!(t1, t2);
                    // Arguments should have the same structure
                    if let (Some(obj1), Some(obj2)) = (a1.as_object(), a2.as_object()) {
                        assert_eq!(
                            obj1.keys().collect::<Vec<_>>(),
                            obj2.keys().collect::<Vec<_>>()
                        );
                    }
                }
                _ => panic!("ToolCall actions should round-trip"),
            }
        }
    }

    #[test]
    fn test_agent_execution_result_with_steps() {
        let steps = vec![
            ExecutionStep {
                step_number: 1,
                action: StepAction::LlmRequest {
                    model: "gpt-4".to_string(),
                },
                result: Some(StepResult {
                    success: true,
                    output: json!({}),
                    tokens_used: Some(100),
                    input_tokens: Some(50),
                    output_tokens: Some(50),
                    duration_ms: 500,
                }),
                timestamp: Utc::now(),
            },
            ExecutionStep {
                step_number: 2,
                action: StepAction::ToolCall {
                    tool: "fs_read_file".to_string(),
                    arguments: json!({"path": "/test.txt"}),
                },
                result: Some(StepResult {
                    success: true,
                    output: json!({"content": "file content"}),
                    tokens_used: None,
                    input_tokens: None,
                    output_tokens: None,
                    duration_ms: 100,
                }),
                timestamp: Utc::now(),
            },
        ];

        let result = AgentExecutionResult {
            content: "Task completed successfully".to_string(),
            steps: steps.clone(),
            usage: Some(UsageStats {
                input_tokens: 50,
                output_tokens: 50,
                total_tokens: 100,
            }),
        };

        assert_eq!(result.steps.len(), 2);
        assert_eq!(result.steps[0].step_number, 1);
        assert_eq!(result.steps[1].step_number, 2);
    }

    #[test]
    fn test_usage_stats_total_calculation() {
        let stats = UsageStats {
            input_tokens: 1234,
            output_tokens: 5678,
            total_tokens: 6912,
        };

        assert_eq!(stats.total_tokens, stats.input_tokens + stats.output_tokens);
    }

    #[test]
    fn test_execution_step_timestamp_ordering() {
        let step1 = ExecutionStep {
            step_number: 1,
            action: StepAction::LlmRequest {
                model: "gpt-4".to_string(),
            },
            result: None,
            timestamp: Utc::now(),
        };

        // Small delay to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(10));

        let step2 = ExecutionStep {
            step_number: 2,
            action: StepAction::LlmRequest {
                model: "gpt-4".to_string(),
            },
            result: None,
            timestamp: Utc::now(),
        };

        assert!(step2.timestamp > step1.timestamp);
        assert!(step2.step_number > step1.step_number);
    }

    #[test]
    fn test_agent_request_with_multiple_attachments() {
        let request = AgentRequest {
            content: "Review these files".to_string(),
            attachments: vec![
                crate::core::agent::task::Attachment {
                    name: "file1.rs".to_string(),
                    content: "content1".to_string(),
                    content_type: "text/plain".to_string(),
                },
                crate::core::agent::task::Attachment {
                    name: "file2.rs".to_string(),
                    content: "content2".to_string(),
                    content_type: "text/plain".to_string(),
                },
                crate::core::agent::task::Attachment {
                    name: "image.png".to_string(),
                    content: "base64data".to_string(),
                    content_type: "image/png".to_string(),
                },
            ],
            context: None,
        };

        assert_eq!(request.attachments.len(), 3);
        assert_eq!(request.attachments[0].name, "file1.rs");
        assert_eq!(request.attachments[2].content_type, "image/png");
    }

    #[test]
    fn test_trust_mode_variants() {
        let modes = vec![
            TrustMode::Trusted,
            TrustMode::Untrusted,
            TrustMode::Restricted,
        ];

        for mode in modes {
            let serialized = serde_json::to_string(&mode).unwrap();
            let deserialized: TrustMode = serde_json::from_str(&serialized).unwrap();
            assert_eq!(mode, deserialized);
        }
    }

    #[test]
    fn test_agent_runtime_error_from_loop_detected() {
        let error = AgentRuntimeError::LoopDetected;
        assert!(error.to_string().contains("repetitive") || error.to_string().contains("loop"));

        // Test Display implementation
        let display_string = format!("{}", error);
        assert!(!display_string.is_empty());
    }

    #[test]
    fn test_agent_runtime_error_from_action_denied() {
        let error = AgentRuntimeError::ActionDenied;
        assert!(error.to_string().contains("denied"));

        let display_string = format!("{}", error);
        assert!(!display_string.is_empty());
    }

    #[test]
    fn test_agent_runtime_error_from_approval_timeout() {
        let error = AgentRuntimeError::ApprovalTimeout;
        assert!(error.to_string().contains("timeout"));

        let display_string = format!("{}", error);
        assert!(!display_string.is_empty());
    }

    #[test]
    fn test_subtask_summary_properties() {
        use crate::infrastructure::agent::runtime::types::SubtaskSummary;

        let summary = SubtaskSummary {
            title: "Test subtask".to_string(),
            mode: crate::core::agent::entities::TaskMode::Read,
            status: TaskStatus::InProgress.into(),
            order: 2,
        };

        assert_eq!(summary.title, "Test subtask");
        assert!(matches!(
            summary.mode,
            crate::core::agent::entities::TaskMode::Read
        ));
        assert_eq!(summary.status, TaskStatus::InProgress.into());
        assert_eq!(summary.order, 2);
    }

    // === Session State Management Tests ===

    #[test]
    fn test_session_state_launching() {
        let state = SessionState::Launching;
        assert!(matches!(state, SessionState::Launching));
    }

    #[test]
    fn test_session_state_wait_for_request() {
        let state = SessionState::WaitForRequest {
            last_action_at: chrono::Utc::now(),
        };
        assert!(matches!(state, SessionState::WaitForRequest { .. }));
    }

    #[test]
    fn test_session_state_user_input_required() {
        let action_ids = vec!["action-1".to_string(), "action-2".to_string()];
        let state = SessionState::UserInputRequired {
            pending_action_ids: action_ids.clone(),
        };

        match state {
            SessionState::UserInputRequired { pending_action_ids } => {
                assert_eq!(pending_action_ids.len(), 2);
                assert_eq!(pending_action_ids[0], "action-1");
            }
            _ => panic!("Expected UserInputRequired state"),
        }
    }

    #[test]
    fn test_session_state_cancelled() {
        let state = SessionState::Cancelled {
            reason: "User cancelled".to_string(),
        };

        match state {
            SessionState::Cancelled { reason } => {
                assert_eq!(reason, "User cancelled");
            }
            _ => panic!("Expected Cancelled state"),
        }
    }

    #[test]
    fn test_session_state_running_fields() {
        let state = SessionState::Running {
            current_step: 42,
            context_usage: 87.5,
        };

        match state {
            SessionState::Running {
                current_step,
                context_usage,
            } => {
                assert_eq!(current_step, 42);
                assert_eq!(context_usage, 87.5);
            }
            _ => panic!("Expected Running state"),
        }
    }

    #[test]
    fn test_session_state_finished_completed_at() {
        let now = Utc::now();
        let state = SessionState::Finished { completed_at: now };

        match state {
            SessionState::Finished { completed_at } => {
                assert_eq!(completed_at, now);
            }
            _ => panic!("Expected Finished state"),
        }
    }

    #[test]
    fn test_session_state_all_variants() {
        // Ensure all session state variants can be created
        let _ = SessionState::Launching;
        let _ = SessionState::WaitForRequest {
            last_action_at: Utc::now(),
        };
        let _ = SessionState::Running {
            current_step: 0,
            context_usage: 0.0,
        };
        let _ = SessionState::UserInputRequired {
            pending_action_ids: vec![],
        };
        let _ = SessionState::Finished {
            completed_at: Utc::now(),
        };
        let _ = SessionState::Cancelled {
            reason: "test".to_string(),
        };
        let _ = SessionState::Error {
            message: "error".to_string(),
            recoverable: false,
        };
    }

    // === Agent Execution Result Tests ===

    #[test]
    fn test_agent_execution_result_empty_steps() {
        let result = AgentExecutionResult {
            content: "Done".to_string(),
            steps: vec![],
            usage: Some(UsageStats {
                input_tokens: 0,
                output_tokens: 0,
                total_tokens: 0,
            }),
        };

        assert!(result.steps.is_empty());
        assert_eq!(result.content, "Done");
    }

    #[test]
    fn test_agent_execution_result_no_usage() {
        let result = AgentExecutionResult {
            content: "Quick answer".to_string(),
            steps: vec![],
            usage: None,
        };

        assert!(result.usage.is_none());
        assert_eq!(result.content, "Quick answer");
    }

    #[test]
    fn test_agent_execution_result_multiple_steps() {
        let steps = vec![
            ExecutionStep {
                step_number: 1,
                action: StepAction::LlmRequest {
                    model: "gpt-4".to_string(),
                },
                result: Some(StepResult {
                    success: true,
                    output: json!({}),
                    tokens_used: Some(100),
                    input_tokens: Some(50),
                    output_tokens: Some(50),
                    duration_ms: 1000,
                }),
                timestamp: Utc::now(),
            },
            ExecutionStep {
                step_number: 2,
                action: StepAction::ToolCall {
                    tool: "fs_read_file".to_string(),
                    arguments: json!({"path": "/test.txt"}),
                },
                result: Some(StepResult {
                    success: true,
                    output: json!({"content": "file content"}),
                    tokens_used: None,
                    input_tokens: None,
                    output_tokens: None,
                    duration_ms: 100,
                }),
                timestamp: Utc::now(),
            },
            ExecutionStep {
                step_number: 3,
                action: StepAction::LlmRequest {
                    model: "gpt-4".to_string(),
                },
                result: Some(StepResult {
                    success: true,
                    output: json!({}),
                    tokens_used: Some(50),
                    input_tokens: Some(30),
                    output_tokens: Some(20),
                    duration_ms: 800,
                }),
                timestamp: Utc::now(),
            },
        ];

        let result = AgentExecutionResult {
            content: "Final result".to_string(),
            steps,
            usage: Some(UsageStats {
                input_tokens: 80,
                output_tokens: 70,
                total_tokens: 150,
            }),
        };

        assert_eq!(result.steps.len(), 3);
        assert_eq!(result.usage.as_ref().unwrap().total_tokens, 150);
    }

    // === Step Action Tests ===

    #[test]
    fn test_step_action_llm_request() {
        let action = StepAction::LlmRequest {
            model: "claude-3-opus".to_string(),
        };

        match action {
            StepAction::LlmRequest { model } => {
                assert_eq!(model, "claude-3-opus");
            }
            _ => panic!("Expected LlmRequest action"),
        }
    }

    #[test]
    fn test_step_action_tool_call() {
        let action = StepAction::ToolCall {
            tool: "agent_emit_status".to_string(),
            arguments: json!({"message": "Working on it..."}),
        };

        match action {
            StepAction::ToolCall { tool, arguments } => {
                assert_eq!(tool, "agent_emit_status");
                assert_eq!(arguments["message"], "Working on it...");
            }
            _ => panic!("Expected ToolCall action"),
        }
    }

    // === Execution Loop Constants Tests ===

    #[test]
    fn test_execution_loop_constants() {
        // Verify the execution loop constants are set correctly
        assert_eq!(LLM_MAX_RETRIES, 2);
        assert_eq!(LLM_RETRY_BASE_MS, 1000);
    }

    #[test]
    fn test_retry_delay_calculation() {
        // Test exponential backoff: base_delay_ms * (1 << attempt)
        let base_delay_ms = LLM_RETRY_BASE_MS;

        let delay_0 = base_delay_ms * (1u64 << 0);
        let delay_1 = base_delay_ms * (1u64 << 1);
        let delay_2 = base_delay_ms * (1u64 << 2);

        assert_eq!(delay_0, 1000);
        assert_eq!(delay_1, 2000);
        assert_eq!(delay_2, 4000);
    }

    #[test]
    fn test_retry_delay_within_bounds() {
        // Ensure delays don't overflow for reasonable attempts
        let base_delay_ms = LLM_RETRY_BASE_MS;

        for attempt in 0..=LLM_MAX_RETRIES {
            let delay = base_delay_ms.saturating_mul(1u64 << attempt);
            assert!(
                delay > 0,
                "Delay should be positive for attempt {}",
                attempt
            );
        }
    }

    // === Agent Request Tests ===

    #[test]
    fn test_agent_request_empty() {
        let request = AgentRequest {
            content: "".to_string(),
            attachments: vec![],
            context: None,
        };

        assert_eq!(request.content, "");
        assert!(request.attachments.is_empty());
        assert!(request.context.is_none());
    }

    #[test]
    fn test_agent_request_with_long_content() {
        let long_content = "a".repeat(10000);
        let request = AgentRequest {
            content: long_content.clone(),
            attachments: vec![],
            context: None,
        };

        assert_eq!(request.content.len(), 10000);
    }

    #[test]
    fn test_agent_request_serialization() {
        let request = AgentRequest {
            content: "Test".to_string(),
            attachments: vec![],
            context: Some(crate::core::agent::task::RequestContext {
                file_path: Some("/path/to/file.rs".to_string()),
                selection: Some("selected text".to_string()),
            }),
        };

        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: AgentRequest = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.content, "Test");
        assert!(deserialized.context.is_some());
    }

    // === Attachment Tests ===

    #[test]
    fn test_attachment_creation() {
        let attachment = crate::core::agent::task::Attachment {
            name: "test.txt".to_string(),
            content: "hello world".to_string(),
            content_type: "text/plain".to_string(),
        };

        assert_eq!(attachment.name, "test.txt");
        assert_eq!(attachment.content, "hello world");
        assert_eq!(attachment.content_type, "text/plain");
    }

    #[test]
    fn test_attachment_with_binary_content() {
        let attachment = crate::core::agent::task::Attachment {
            name: "image.png".to_string(),
            content: "base64encodeddata".to_string(),
            content_type: "image/png".to_string(),
        };

        assert_eq!(attachment.content_type, "image/png");
    }

    #[test]
    fn test_attachment_serialization() {
        let attachment = crate::core::agent::task::Attachment {
            name: "code.rs".to_string(),
            content: "fn main() {}".to_string(),
            content_type: "text/x-rust".to_string(),
        };

        let serialized = serde_json::to_string(&attachment).unwrap();
        let deserialized: crate::core::agent::task::Attachment =
            serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.name, "code.rs");
        assert_eq!(deserialized.content, "fn main() {}");
    }

    // === RequestContext Tests ===

    #[test]
    fn test_request_context_full() {
        let ctx = crate::core::agent::task::RequestContext {
            file_path: Some("/project/src/main.rs".to_string()),
            selection: Some("fn main() {}".to_string()),
        };

        assert_eq!(ctx.file_path, Some("/project/src/main.rs".to_string()));
        assert_eq!(ctx.selection, Some("fn main() {}".to_string()));
    }

    #[test]
    fn test_request_context_only_file_path() {
        let ctx = crate::core::agent::task::RequestContext {
            file_path: Some("/project/src/main.rs".to_string()),
            selection: None,
        };

        assert!(ctx.file_path.is_some());
        assert!(ctx.selection.is_none());
    }

    #[test]
    fn test_request_context_empty() {
        let ctx = crate::core::agent::task::RequestContext {
            file_path: None,
            selection: None,
        };

        assert!(ctx.file_path.is_none());
        assert!(ctx.selection.is_none());
    }

    // === Execution Step Edge Cases ===

    #[test]
    fn test_execution_step_zero_tokens() {
        let step = ExecutionStep {
            step_number: 1,
            action: StepAction::ToolCall {
                tool: "test".to_string(),
                arguments: json!({}),
            },
            result: Some(StepResult {
                success: true,
                output: json!({}),
                tokens_used: Some(0),
                input_tokens: Some(0),
                output_tokens: Some(0),
                duration_ms: 0,
            }),
            timestamp: Utc::now(),
        };

        assert_eq!(step.result.as_ref().unwrap().tokens_used, Some(0));
    }

    #[test]
    fn test_execution_step_large_duration() {
        let step = ExecutionStep {
            step_number: 1,
            action: StepAction::LlmRequest {
                model: "slow-model".to_string(),
            },
            result: Some(StepResult {
                success: true,
                output: json!({}),
                tokens_used: Some(1000),
                input_tokens: Some(500),
                output_tokens: Some(500),
                duration_ms: 60_000, // 1 minute
            }),
            timestamp: Utc::now(),
        };

        assert_eq!(step.result.as_ref().unwrap().duration_ms, 60_000);
    }

    #[test]
    fn test_execution_step_complex_output() {
        let complex_output = json!({
            "files": ["file1.rs", "file2.rs"],
            "changes": 42,
            "errors": [],
            "warnings": ["unused variable"]
        });

        let step = ExecutionStep {
            step_number: 5,
            action: StepAction::ToolCall {
                tool: "analyze_code".to_string(),
                arguments: json!({}),
            },
            result: Some(StepResult {
                success: true,
                output: complex_output,
                tokens_used: None,
                input_tokens: None,
                output_tokens: None,
                duration_ms: 500,
            }),
            timestamp: Utc::now(),
        };

        let output = step.result.unwrap().output;
        assert_eq!(output["files"].as_array().unwrap().len(), 2);
        assert_eq!(output["changes"], 42);
    }

    // === Usage Stats Edge Cases ===

    #[test]
    fn test_usage_stats_only_input() {
        let stats = UsageStats {
            input_tokens: 1000,
            output_tokens: 0,
            total_tokens: 1000,
        };

        assert_eq!(stats.input_tokens, 1000);
        assert_eq!(stats.output_tokens, 0);
        assert_eq!(stats.total_tokens, 1000);
    }

    #[test]
    fn test_usage_stats_only_output() {
        let stats = UsageStats {
            input_tokens: 0,
            output_tokens: 500,
            total_tokens: 500,
        };

        assert_eq!(stats.input_tokens, 0);
        assert_eq!(stats.output_tokens, 500);
    }

    #[test]
    fn test_usage_stats_large_values() {
        let stats = UsageStats {
            input_tokens: 100_000,
            output_tokens: 50_000,
            total_tokens: 150_000,
        };

        assert_eq!(stats.total_tokens, 150_000);
    }

    // === Trust Mode Tests ===

    #[test]
    fn test_trust_mode_trusted() {
        let mode = TrustMode::Trusted;
        let serialized = serde_json::to_string(&mode).unwrap();
        assert!(serialized.contains("trusted"));
    }

    #[test]
    fn test_trust_mode_untrusted() {
        let mode = TrustMode::Untrusted;
        let serialized = serde_json::to_string(&mode).unwrap();
        assert!(serialized.contains("untrusted"));
    }

    #[test]
    fn test_trust_mode_restricted() {
        let mode = TrustMode::Restricted;
        let serialized = serde_json::to_string(&mode).unwrap();
        assert!(serialized.contains("restricted"));
    }

    // === Error Type Edge Cases ===

    #[test]
    fn test_agent_runtime_error_with_empty_message() {
        let error = AgentRuntimeError::Llm("".to_string());
        assert!(error.to_string().contains("LLM error"));
    }

    #[test]
    fn test_agent_runtime_error_with_long_message() {
        let long_msg = "x".repeat(1000);
        let error = AgentRuntimeError::Repository(long_msg.clone());
        assert!(error.to_string().contains("Repository error"));
    }

    #[test]
    fn test_agent_runtime_error_with_special_characters() {
        let special_msg = "Error: \n\t\r\nInternal failure";
        let error = AgentRuntimeError::Config(special_msg.to_string());
        let error_str = error.to_string();
        assert!(error_str.contains("Config error"));
    }

    // === Session ID Tests ===

    #[test]
    fn test_session_id_with_special_characters() {
        let id = SessionId("session-123_456.test".to_string());
        assert_eq!(id.0, "session-123_456.test");
    }

    #[test]
    fn test_session_id_empty() {
        let id = SessionId("".to_string());
        assert_eq!(id.0, "");
    }

    #[test]
    fn test_session_id_with_uuid() {
        let uuid_str = uuid::Uuid::new_v4().to_string();
        let id = SessionId(uuid_str.clone());
        assert_eq!(id.0, uuid_str);
    }

    // === Step Number Tests ===

    #[test]
    fn test_execution_step_number_overflow_protection() {
        let step = ExecutionStep {
            step_number: u32::MAX,
            action: StepAction::LlmRequest {
                model: "test".to_string(),
            },
            result: None,
            timestamp: Utc::now(),
        };

        assert_eq!(step.step_number, u32::MAX);
    }

    #[test]
    fn test_execution_step_sequence() {
        let steps: Vec<ExecutionStep> = (1..=10)
            .map(|i| ExecutionStep {
                step_number: i,
                action: StepAction::LlmRequest {
                    model: "test".to_string(),
                },
                result: None,
                timestamp: Utc::now(),
            })
            .collect();

        for (i, step) in steps.iter().enumerate() {
            assert_eq!(step.step_number, (i + 1) as u32);
        }
    }

    // === Tool Call Integration Tests ===

    #[test]
    fn test_tool_call_creation() {
        let tool_call = ToolCall {
            id: "call-123".to_string(),
            name: "fs_read_file".to_string(),
            arguments: json!({"path": "/test.txt"}),
        };

        assert_eq!(tool_call.id, "call-123");
        assert_eq!(tool_call.name, "fs_read_file");
        assert_eq!(tool_call.arguments["path"], "/test.txt");
    }

    #[test]
    fn test_tool_call_empty_arguments() {
        let tool_call = ToolCall {
            id: "call-456".to_string(),
            name: "agent_emit_status".to_string(),
            arguments: json!({}),
        };

        assert_eq!(tool_call.arguments.as_object().unwrap().len(), 0);
    }

    #[test]
    fn test_tool_call_nested_arguments() {
        let tool_call = ToolCall {
            id: "call-789".to_string(),
            name: "complex_tool".to_string(),
            arguments: json!({
                "options": {
                    "deep": {
                        "value": 42
                    }
                },
                "items": ["a", "b", "c"]
            }),
        };

        assert_eq!(tool_call.arguments["options"]["deep"]["value"], 42);
        assert_eq!(tool_call.arguments["items"].as_array().unwrap().len(), 3);
    }

    // === Message Context Tests ===

    #[test]
    fn test_message_user_creation() {
        let msg = Message::user("Hello, world!");
        assert_eq!(msg.role, crate::core::llm::entities::MessageRole::User);
    }

    #[test]
    fn test_message_system_creation() {
        let msg = Message::system("You are helpful.");
        assert_eq!(msg.role, crate::core::llm::entities::MessageRole::System);
    }

    #[test]
    fn test_message_assistant_creation() {
        let msg = Message::assistant("Response");
        assert_eq!(msg.role, crate::core::llm::entities::MessageRole::Assistant);
    }

    #[test]
    fn test_message_tool_result_creation() {
        let msg = Message::tool_result("call-123", serde_json::json!("Success"), false);
        assert_eq!(msg.role, crate::core::llm::entities::MessageRole::Tool);
    }

    #[test]
    fn test_message_tool_result_error() {
        let msg = Message::tool_result("call-456", serde_json::json!("Error occurred"), true);
        assert!(msg.tool_result.is_some());
        assert!(msg.tool_result.unwrap().is_error);
    }
}
