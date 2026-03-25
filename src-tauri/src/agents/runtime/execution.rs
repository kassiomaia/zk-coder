//! Main execution orchestration for agent runtime.

use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use log::{info, warn};
use tauri::{AppHandle, Emitter};

use crate::agents::approval::ApprovalQueue;
use crate::agents::events::{AgentEvent, AGENT_EVENT_NAME};
use crate::agents::permissions::PermissionResolver;
use crate::agents::rollback::RollbackStore;
use crate::agents::session::{AgentSession, SessionId, SessionState};
use crate::agents::store::SessionStore;
use crate::agents::task::{
    AgentExecutionResult, AgentRequest, AgentTask, ExecutionStep, StepAction, StepResult,
    TaskStatus, UsageStats,
};
use crate::agents::types::TrustMode;
use crate::llm::{ChatRequest, LlmClient, Message};
use crate::tools::ToolRegistry;

use super::client::{chat_with_retry, create_llm_client, LlmRetryConfig, build_llm_context};
use super::context::{estimate_context_tokens, maybe_compact_context, CONTEXT_WINDOW};
use super::{derive_system_prompt, SubtaskSummary, AgentRuntimeError};
use super::{add_to_recent_calls, detect_repetitive_loop, MAX_REPETITIONS};
use super::process_tool_calls_with_permissions;

/// Maximum LLM retries for transient errors.
const LLM_MAX_RETRIES: usize = 2;

/// Base delay in ms for LLM retry backoff.
const LLM_RETRY_BASE_MS: u64 = 1000;

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
    session_store: &SessionStore,
    approval_queue: &Arc<ApprovalQueue>,
    tool_registry: &Arc<ToolRegistry>,
    rollback_store: &Arc<RollbackStore>,
    all_tool_definitions: &[crate::llm::ToolDefinition],
) -> Result<AgentExecutionResult, AgentRuntimeError> {
    let session_id = session.id.0.clone();
    let permission_resolver = PermissionResolver::new(trust_mode);

    // Update session state to running
    update_session_state(
        session_store,
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
    if let Err(e) = session_store.create_task(&task) {
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
    let _ = session_store.update_task_status(&task_id, &TaskStatus::InProgress);

    // === EXECUTION PHASE ===
    // Agent runs until it has no more tool calls (decides it's done)
    loop {
        step_num += 1;
        info!("[AgentRuntime] Step {} for session {}", step_num, session_id);

        // Calculate context usage percentage
        let estimated_tokens = estimate_context_tokens(&llm_context);
        let context_usage = (estimated_tokens as f64 / CONTEXT_WINDOW as f64 * 100.0).min(100.0);

        // Auto-compact context if it's getting too large
        if let Some(compaction) = maybe_compact_context(&mut llm_context) {
            info!(
                "[AgentRuntime] Context compacted: {} messages → {}, {} tokens → {}",
                compaction.before_count,
                compaction.after_count,
                compaction.before_tokens,
                compaction.after_tokens
            );

            // Emit compaction event for frontend to show as chat message
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

            // If subtask context was removed, reset tracking to force re-injection
            if compaction.needs_reinjection {
                if let Some(current) = last_injected_subtask_order {
                    info!(
                        "[AgentRuntime] Re-injecting subtask context for subtask {}",
                        current
                    );
                    last_injected_subtask_order = None;
                }
            }
        }

        // Update session state with current context usage
        update_session_state(
            session_store,
            &session.id,
            SessionState::Running {
                current_step: step_num,
                context_usage,
            },
            app_handle,
        )?;

        // Get current subtask for mode filtering and prompt derivation
        let (current_subtask_summary, tool_names) =
            if let Ok(subtasks) = session_store.get_subtasks(&task_id) {
                let total = subtasks.len();
                if let Some(current) = subtasks.iter().find(|st| st.status == TaskStatus::InProgress) {
                    let summary = SubtaskSummary::from_subtask(current);
                    let allowed = current.mode.allowed_tools();
                    info!(
                        "[AgentRuntime] Subtask '{}' ({} mode): {} tools available",
                        current.title,
                        current.mode.icon(),
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
        let current_order = current_subtask_summary.as_ref().map(|(summary, _)| summary.order);

        if last_injected_subtask_order != current_order {
            match (last_injected_subtask_order, current_order) {
                (None, Some(order)) => {
                    info!("[AgentRuntime] Subtask changed: None → Some({})", order);
                }
                (Some(prev), Some(curr)) => {
                    info!("[AgentRuntime] Subtask changed: Some({}) → Some({})", prev, curr);
                }
                (Some(prev), None) => {
                    info!(
                        "[AgentRuntime] Subtask changed: Some({}) → None (no active subtask)",
                        prev
                    );
                }
                (None, None) => {}
            }
            last_injected_subtask_order = current_order;
        }

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
        let tool_definitions: Vec<crate::llm::ToolDefinition> = all_tool_definitions
            .iter()
            .filter(|t| tool_names.contains(&t.name.as_str()))
            .cloned()
            .collect();

        // Build chat request
        let mut chat_request = ChatRequest::new(&model_name, llm_context.clone())
            .with_system(&system_prompt)
            .with_temperature(0.7)
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
        let current_subtask_id = if let Ok(subtasks) = session_store.get_subtasks(&task_id) {
            subtasks
                .iter()
                .find(|st| st.status == TaskStatus::InProgress)
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
        if let Err(e) = session_store.create_execution_step(
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
            return finalize_execution(
                client.as_ref(),
                &chat_request,
                &session_id,
                &session.conversation_id,
                &session.thread_id,
                &task_id,
                steps,
                total_input_tokens,
                total_output_tokens,
                app_handle,
                session_store,
            )
            .await;
        }

        // Check for repetitive action loops
        for tool_call in &response.tool_calls {
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
                    session_store,
                    &session.id,
                    SessionState::Error {
                        message: msg.clone(),
                        recoverable: true,
                    },
                    app_handle,
                )?;

                // Mark task as failed
                let _ = session_store.update_task_status(&task_id, &TaskStatus::Failed);
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
            rollback_store,
            session_store,
        )
        .await?;

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

/// Finalizes the agent execution and returns the result.
async fn finalize_execution(
    client: &dyn LlmClient,
    request: &ChatRequest,
    session_id: &str,
    conversation_id: &str,
    thread_id: &str,
    task_id: &str,
    steps: Vec<ExecutionStep>,
    mut total_input_tokens: u32,
    mut total_output_tokens: u32,
    app_handle: &AppHandle,
    session_store: &SessionStore,
) -> Result<AgentExecutionResult, AgentRuntimeError> {
    use futures::StreamExt;

    // Use streaming for the final response to show real-time progress
    let mut stream_request = request.clone();
    stream_request.stream = true;

    let mut full_content = String::new();
    let mut final_usage = None;

    let mut stream = client.chat_stream(stream_request);

    while let Some(chunk_result) = StreamExt::next(&mut stream).await {
        match chunk_result {
            Ok(chunk) => {
                if let Some(ref delta) = chunk.delta {
                    if let Some(ref content) = delta.content {
                        // Emit streaming content events
                        tokio::spawn({
                            let session_id = session_id.to_string();
                            let conversation_id = conversation_id.to_string();
                            let thread_id = thread_id.to_string();
                            let chunk_content = content.clone();
                            let app_handle = app_handle.clone();

                            async move {
                                let _ = app_handle.emit(
                                    AGENT_EVENT_NAME,
                                    AgentEvent::StreamingContent {
                                        session_id,
                                        conversation_id,
                                        thread_id,
                                        chunk: chunk_content,
                                        is_final: false,
                                    },
                                );
                            }
                        });

                        full_content.push_str(content);
                    }
                }

                if let Some(usage) = chunk.usage {
                    final_usage = Some(usage);
                }
            }
            Err(e) => {
                warn!("[AgentRuntime] Stream error: {}", e);
                // Continue to try to get what we have
            }
        }
    }

    // Update usage with streaming response
    if let Some(usage) = &final_usage {
        total_input_tokens += usage.input_tokens;
        total_output_tokens += usage.output_tokens;
    }

    let result = AgentExecutionResult {
        content: full_content.clone(),
        steps,
        usage: Some(UsageStats {
            input_tokens: total_input_tokens,
            output_tokens: total_output_tokens,
            total_tokens: total_input_tokens + total_output_tokens,
        }),
    };

    // Update task to completed
    if let Err(e) = session_store.update_task_status(task_id, &TaskStatus::Completed) {
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
        session_store,
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
    session_store: &SessionStore,
    session_id: &SessionId,
    state: SessionState,
    app_handle: &AppHandle,
) -> Result<(), AgentRuntimeError> {
    session_store
        .update_state(session_id, &state)
        .map_err(|e| AgentRuntimeError::Store(e.to_string()))?;

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

    #[test]
    fn test_constants() {
        assert_eq!(LLM_MAX_RETRIES, 2);
        assert_eq!(LLM_RETRY_BASE_MS, 1000);
    }
}
