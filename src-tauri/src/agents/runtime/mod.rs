//! Agent runtime for executing requests with permission-checked tool calls.
//!
//! This module is organized into submodules for better maintainability:
//!
//! - [`client`] - LLM client creation and utilities
//! - [`context`] - Context compaction and token estimation
//! - [`loop_detection`] - Repetitive loop detection
//! - [`execution`] - Main execution orchestration (in this file)

pub mod client;
pub mod context;
pub mod loop_detection;

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use log::{info, warn};
use tauri::{AppHandle, Emitter};

use crate::llm::{
    ChatRequest, LlmClient, Message, ToolCall,
};
use crate::tools::context::ToolExecutionContext;
use crate::tools::OrchestratorToolResult;
use crate::tools::ToolRegistry;

use super::approval::{ApprovalEvent, ApprovalQueue};
use super::events::{AgentEvent, AGENT_EVENT_NAME};
use super::permissions::PermissionResolver;
use super::rollback::RollbackStore;
use super::session::{AgentSession, PendingAction, SessionId, SessionState};
use super::store::SessionStore;
use super::task::{
    AgentExecutionResult, AgentRequest, AgentTask, ExecutionStep, StepAction, StepResult,
    TaskStatus, UsageStats,
};
use super::types::{ActionType, PermissionLevel, TrustMode};

// Re-export submodule utilities
pub use client::{chat_with_retry, create_llm_client, LlmRetryConfig};
pub use context::{estimate_context_tokens, maybe_compact_context, CompactionDetails,
            COMPRESSION_THRESHOLD, CONTEXT_WINDOW, KEEP_RECENT_MESSAGES};
pub use loop_detection::{add_to_recent_calls, detect_repetitive_loop, hash_tool_arguments,
                        LOOP_SKIP_TOOLS, MAX_REPETITIONS};

use client::build_llm_context;

/// Maximum LLM retries for transient errors.
const LLM_MAX_RETRIES: usize = 2;

/// Base delay in ms for LLM retry backoff.
const LLM_RETRY_BASE_MS: u64 = 1000;

/// Timeout for waiting on user approval (seconds).
const APPROVAL_TIMEOUT_SECS: u64 = 300;

/// Agent runtime errors.
#[derive(Debug, thiserror::Error)]
pub enum AgentRuntimeError {
    #[error("Agent not found: {0}")]
    AgentNotFound(String),
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("LLM error: {0}")]
    Llm(String),
    #[error("Detected repetitive action loop")]
    LoopDetected,
    #[error("Action denied by user")]
    ActionDenied,
    #[error("Action timed out waiting for approval")]
    ApprovalTimeout,
    #[error("Store error: {0}")]
    Store(String),
    #[error("Config error: {0}")]
    Config(String),
}

/// Summary of subtask data needed for prompt derivation and tracking.
#[derive(Debug, Clone)]
struct SubtaskSummary {
    title: String,
    mode: super::task::SubTaskMode,
    status: TaskStatus,
    order: i32,
}

impl SubtaskSummary {
    fn from_subtask(subtask: &super::task::SubTask) -> Self {
        Self {
            title: subtask.title.clone(),
            mode: subtask.mode.clone(),
            status: subtask.status.clone(),
            order: subtask.order,
        }
    }
}

/// The agent runtime orchestrates LLM + tool execution with permissions.
pub struct AgentRuntime {
    session_store: Arc<SessionStore>,
    approval_queue: Arc<ApprovalQueue>,
    tool_registry: Arc<ToolRegistry>,
    rollback_store: Arc<RollbackStore>,
}

impl AgentRuntime {
    /// Create a new agent runtime.
    pub fn new(
        session_store: Arc<SessionStore>,
        approval_queue: Arc<ApprovalQueue>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Self {
        Self {
            session_store,
            approval_queue,
            tool_registry,
            rollback_store: Arc::new(RollbackStore::new()),
        }
    }

    /// Get the rollback store for external access (e.g., Tauri commands).
    pub fn rollback_store(&self) -> &Arc<RollbackStore> {
        &self.rollback_store
    }
}

impl AgentRuntime {
    /// Execute an agent request within a session.
    ///
    /// This is the main entry point. It:
    /// 1. Resolves the LLM client from config
    /// 2. Builds context from conversation history
    /// 3. Runs the tool-calling loop with permission checks
    /// 4. Emits events for each step
    /// 5. Returns the final result
    pub async fn execute(
        &self,
        session: &AgentSession,
        request: AgentRequest,
        agent_system_prompt: &str,
        trust_mode: TrustMode,
        app_handle: &AppHandle,
    ) -> Result<AgentExecutionResult, AgentRuntimeError> {
        // Get all tool definitions (will be filtered per subtask mode in the loop)
        let all_tool_definitions = self.tool_registry.definitions();
        let session_id = session.id.0.clone();
        let permission_resolver = PermissionResolver::new(trust_mode);

        // Update session state to running
        self.update_session_state(
            &session.id,
            SessionState::Running { current_step: 0, context_usage: 0.0 },
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
        if let Err(e) = self.session_store.create_task(&task) {
            warn!("[AgentRuntime] Failed to persist task: {e}");
        }
        let task_id = task.id.clone();

        // Emit task created event
        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::TaskCreated {
                session_id: session_id.clone(),
                task: task,
            },
        );

        // Create LLM client
        let (client, model_name) = create_llm_client(app_handle).await?;

        // Build initial LLM context
        let mut llm_context = build_llm_context(session, &request, app_handle).await?;

        let mut steps: Vec<ExecutionStep> = Vec::new();
        let mut total_input_tokens: u32 = 0;
        let mut total_output_tokens: u32 = 0;
        let mut step_num: u32 = 0;

        // Track recent tool calls for loop detection: (tool_name, arguments_hash)
        let mut recent_tool_calls: Vec<(String, u64)> = Vec::new();

        // Track which subtask we last injected context for (to detect changes)
        let mut last_injected_subtask_order: Option<i32> = None;

        // Mark task as in progress
        let _ = self.session_store.update_task_status(&task_id, &TaskStatus::InProgress);

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
                    compaction.before_count, compaction.after_count, compaction.before_tokens, compaction.after_tokens
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
                        info!("[AgentRuntime] Re-injecting subtask context for subtask {}", current);
                        // Reset to None so change detection will trigger re-injection
                        last_injected_subtask_order = None;
                    }
                }
            }

            // Update session state with current context usage
            self.update_session_state(
                &session.id,
                SessionState::Running { current_step: step_num, context_usage },
                app_handle,
            )?;

            // Get current subtask for mode filtering and prompt derivation
            let (current_subtask_summary, tool_names) = if let Ok(subtasks) = self.session_store.get_subtasks(&task_id) {
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
                    (None, all_tool_definitions.iter().map(|t| t.name.as_str()).collect::<Vec<_>>())
                }
            } else {
                (None, all_tool_definitions.iter().map(|t| t.name.as_str()).collect::<Vec<_>>())
            };

            // Detect and log subtask changes
            let current_order = current_subtask_summary.as_ref()
                .map(|(summary, _)| summary.order);

            if last_injected_subtask_order != current_order {
                match (last_injected_subtask_order, current_order) {
                    (None, Some(order)) => {
                        info!("[AgentRuntime] Subtask changed: None → Some({})", order);
                    }
                    (Some(prev), Some(curr)) => {
                        info!("[AgentRuntime] Subtask changed: Some({}) → Some({})", prev, curr);
                    }
                    (Some(prev), None) => {
                        info!("[AgentRuntime] Subtask changed: Some({}) → None (no active subtask)", prev);
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
                if current_subtask_summary.is_some() { " (with subtask context)" } else { "" }
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
            let start = std::time::Instant::now();
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
            let current_subtask_id = if let Ok(subtasks) = self.session_store.get_subtasks(&task_id) {
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
            if let Err(e) = self.session_store.create_execution_step(
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
                return self.finalize_execution(
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
                ).await;
            }

            // Check for repetitive action loops
            for tool_call in &response.tool_calls {
                if let Some(repeat_count) = detect_repetitive_loop(
                    tool_call,
                    &recent_tool_calls,
                    MAX_REPETITIONS,
                ) {
                    let msg = format!(
                        "Detected repetitive loop: tool '{}' called with same arguments {} times",
                        tool_call.name, repeat_count + 1
                    );
                    warn!("[AgentRuntime] {}", msg);

                    self.update_session_state(
                        &session.id,
                        SessionState::Error {
                            message: msg.clone(),
                            recoverable: true,
                        },
                        app_handle,
                    )?;

                    // Mark task as failed
                    let _ = self.session_store.update_task_status(&task_id, &TaskStatus::Failed);
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

            let tool_results = self
                .process_tool_calls_with_permissions(
                    &response.tool_calls,
                    session,
                    &task_id,
                    &permission_resolver,
                    app_handle,
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
        &self,
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
        if let Err(e) = self.session_store.update_task_status(task_id, &TaskStatus::Completed) {
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
        self.update_session_state(
            &crate::agents::session::SessionId(session_id.to_string()),
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
}

impl AgentRuntime {
    /// Process tool calls with permission checks, requesting approval as needed.
    async fn process_tool_calls_with_permissions(
        &self,
        tool_calls: &[ToolCall],
        session: &AgentSession,
        task_id: &str,
        permission_resolver: &PermissionResolver,
        app_handle: &AppHandle,
    ) -> Result<Vec<OrchestratorToolResult>, AgentRuntimeError> {
        let mut results = Vec::with_capacity(tool_calls.len());

        for call in tool_calls {
            let action_type = infer_action_type(&call.name, &call.arguments);
            let permission = permission_resolver.check(&call.name, &action_type);

            // Generate unique ID, persist, and emit tool call event
            let tool_call_id = uuid::Uuid::new_v4().to_string();

            // Get current subtask_id for tool call linkage
            let current_subtask_id = if let Ok(subtasks) = self.session_store.get_subtasks(&task_id) {
                subtasks
                    .iter()
                    .find(|st| st.status == TaskStatus::InProgress)
                    .map(|st| st.id.clone())
            } else {
                None
            };

            if let Err(e) = self.session_store.insert_tool_call(
                &session.id,
                &tool_call_id,
                &call.name,
                &call.arguments,
                current_subtask_id.as_deref(),
            ) {
                warn!("[AgentRuntime] Failed to persist tool call: {e}");
            }

            let _ = app_handle.emit(
                AGENT_EVENT_NAME,
                AgentEvent::ToolCall {
                    session_id: session.id.0.clone(),
                    tool_call_id: tool_call_id.clone(),
                    tool: call.name.clone(),
                    arguments: call.arguments.clone(),
                },
            );

            let result = match permission {
                PermissionLevel::AutoRun => {
                    // Execute directly using the tool registry
                    self.execute_tool(call, session, task_id, app_handle).await
                }
                PermissionLevel::ApprovalRequired => {
                    // Queue for approval and wait
                    self.execute_with_approval(call, session, task_id, app_handle)
                        .await?
                }
                PermissionLevel::Disabled => OrchestratorToolResult {
                    call_id: call.id.clone(),
                    name: call.name.clone(),
                    output: serde_json::json!({
                        "error": "This tool is disabled by the agent's permission policy"
                    }),
                    is_error: true,
                },
            };

            // Persist result and emit tool result event
            if let Err(e) = self.session_store.update_tool_call_result(
                &tool_call_id,
                &result.output,
                result.is_error,
                None,
            ) {
                warn!("[AgentRuntime] Failed to persist tool result: {e}");
            }

            let _ = app_handle.emit(
                AGENT_EVENT_NAME,
                AgentEvent::ToolResult {
                    session_id: session.id.0.clone(),
                    tool_call_id: tool_call_id.clone(),
                    tool: call.name.clone(),
                    result: result.output.clone(),
                    is_error: result.is_error,
                },
            );

            results.push(result);
        }

        Ok(results)
    }

    /// Execute a tool call using the tool registry.
    async fn execute_tool(
        &self,
        call: &ToolCall,
        session: &AgentSession,
        _task_id: &str,
        app_handle: &AppHandle,
    ) -> OrchestratorToolResult {
        // Capture file state before write operations for rollback
        if call.name == "fs_write_file" {
            if let Some(path) = call.arguments.get("path").and_then(|v| v.as_str()) {
                if let Some(ref workspace) = session.workspace_root {
                    let target = if std::path::Path::new(path).is_absolute() {
                        std::path::PathBuf::from(path)
                    } else {
                        workspace.join(path)
                    };
                    self.rollback_store.capture(&session.id.0, &target);
                }
            }
        }

        let context = ToolExecutionContext::new(&session.conversation_id)
            .with_thread(session.thread_id.clone())
            .with_session(session.id.0.clone());
        let context = if let Some(ref workspace) = session.workspace_root {
            context.with_workspace(workspace.clone())
        } else {
            context
        };

        let llm_call = crate::llm::ToolCall {
            id: call.id.clone(),
            name: call.name.clone(),
            arguments: call.arguments.clone(),
        };

        let result: crate::llm::ToolResult = self
            .tool_registry
            .execute(llm_call, context, app_handle.clone())
            .await;

        // Emit file touched event for successful file operations
        if !result.is_error {
            match call.name.as_str() {
                "fs_write_file" => {
                    if let Some(path) = call.arguments.get("path").and_then(|v| v.as_str()) {
                        // Generate diff for post-session review
                        let diff = if let Some(workspace_root) = &session.workspace_root {
                            // Get the original content from snapshot (if available)
                            let original_content = self
                                .rollback_store
                                .get_snapshot_content(&session.id.0, path);

                            // Read current file content
                            let current_content = std::fs::read_to_string(workspace_root.join(path)).ok();

                            // Generate diff between original and current
                            match (original_content, current_content) {
                                (Some(original), Some(current)) => {
                                    Some(generate_diff_from_content(path, &original, &current))
                                }
                                (None, Some(current)) => {
                                    // New file - show entire content as addition
                                    let line_count = current.lines().count();
                                    let lines: String = current
                                        .lines()
                                        .map(|line| format!("+{}", line))
                                        .collect::<Vec<_>>()
                                        .join("\n");
                                    Some(format!("--- /dev/null\n+++ b/{}\n@@ -0,0 +1,{} @@\n{}",
                                        path,
                                        line_count,
                                        lines
                                    ))
                                }
                                _ => None
                            }
                        } else {
                            None
                        };

                        let _ = app_handle.emit(
                            AGENT_EVENT_NAME,
                            AgentEvent::FileTouched {
                                session_id: session.id.0.clone(),
                                file_path: path.to_string(),
                                action: super::events::FileAction::Write,
                                diff: diff.clone(),
                            },
                        );

                        // Persist to database for reload
                        let _ = self
                            .session_store
                            .add_file_touched(&session.id, path, "write", diff.as_deref());
                    }
                }
                "fs_delete_file" => {
                    if let Some(path) = call.arguments.get("path").and_then(|v| v.as_str()) {
                        let _ = app_handle.emit(
                            AGENT_EVENT_NAME,
                            AgentEvent::FileTouched {
                                session_id: session.id.0.clone(),
                                file_path: path.to_string(),
                                action: super::events::FileAction::Delete,
                                diff: None,
                            },
                        );

                        // Persist to database for reload
                        let _ = self
                            .session_store
                            .add_file_touched(&session.id, path, "delete", None);
                    }
                }
                "fs_read_file" => {
                    if let Some(path) = call.arguments.get("path").and_then(|v| v.as_str()) {
                        let _ = app_handle.emit(
                            AGENT_EVENT_NAME,
                            AgentEvent::FileTouched {
                                session_id: session.id.0.clone(),
                                file_path: path.to_string(),
                                action: super::events::FileAction::Read,
                                diff: None,
                            },
                        );

                        // Persist to database for reload
                        let _ = self
                            .session_store
                            .add_file_touched(&session.id, path, "read", None);
                    }
                }
                "agent_emit_status" => {
                    // Extract message from arguments and emit StatusPhrase event
                    if let Some(message) = call.arguments.get("message").and_then(|v| v.as_str()) {
                        let _ = app_handle.emit(
                            AGENT_EVENT_NAME,
                            AgentEvent::StatusPhrase {
                                session_id: session.id.0.clone(),
                                message: message.to_string(),
                                timestamp: Utc::now(),
                            },
                        );

                        // Persist to database for reload
                        let _ = self.session_store.add_status_phrase(&session.id, message);
                    }
                }
                _ => {}
            }
        }

        OrchestratorToolResult {
            call_id: result.call_id,
            name: call.name.clone(),
            output: result.output,
            is_error: result.is_error,
        }
    }

    /// Execute a tool call that requires user approval first.
    async fn execute_with_approval(
        &self,
        call: &ToolCall,
        session: &AgentSession,
        task_id: &str,
        app_handle: &AppHandle,
    ) -> Result<OrchestratorToolResult, AgentRuntimeError> {
        let action_type = infer_action_type(&call.name, &call.arguments);
        let mut pending = PendingAction::new(
            session.id.clone(),
            call.name.clone(),
            action_type,
            format!("Agent wants to call tool: {}", call.name),
            call.arguments.clone(),
        );

        // Generate diff preview for file write operations
        if call.name == "fs_write_file" {
            if let (Some(path), Some(content)) = (
                call.arguments.get("path").and_then(|v| v.as_str()),
                call.arguments.get("content").and_then(|v| v.as_str()),
            ) {
                if let Some(ref workspace) = session.workspace_root {
                    let diff = crate::tools::fs::write_file::generate_diff(workspace, path, content);
                    if let Some(diff) = diff {
                        let lang = std::path::Path::new(path)
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(String::from);
                        pending = pending.with_preview(crate::agents::types::ActionPreview {
                            content_type: "diff".to_string(),
                            content: diff,
                            language: lang,
                        });
                    }
                }
            }
        }

        let action_id = pending.id.clone();

        // Update session state to waiting for input
        self.update_session_state(
            &session.id,
            SessionState::UserInputRequired {
                pending_action_ids: vec![action_id.clone()],
            },
            app_handle,
        )?;

        // Emit approval required event
        let _ = app_handle.emit(
            AGENT_EVENT_NAME,
            AgentEvent::ApprovalRequired {
                session_id: session.id.0.clone(),
                action: pending.clone(),
            },
        );

        // Queue the action
        self.approval_queue.queue(pending).await;

        // Wait for approval decision
        let mut rx = self.approval_queue.subscribe();
        let timeout = tokio::time::sleep(Duration::from_secs(APPROVAL_TIMEOUT_SECS));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                event = rx.recv() => {
                    match event {
                        Ok(ApprovalEvent::ActionApproved { action_id: approved_id })
                            if approved_id == action_id =>
                        {
                            info!("[AgentRuntime] Action {} approved", action_id);
                            return Ok(self.execute_tool(call, session, task_id, app_handle).await);
                        }
                        Ok(ApprovalEvent::ActionDenied { action_id: denied_id })
                            if denied_id == action_id =>
                        {
                            info!("[AgentRuntime] Action {} denied", action_id);
                            return Ok(OrchestratorToolResult {
                                call_id: call.id.clone(),
                                name: call.name.clone(),
                                output: serde_json::json!({
                                    "error": "Action denied by user"
                                }),
                                is_error: true,
                            });
                        }
                        Ok(ApprovalEvent::ActionTimeout { action_id: timeout_id })
                            if timeout_id == action_id =>
                        {
                            warn!("[AgentRuntime] Action {} timed out", action_id);
                            return Ok(OrchestratorToolResult {
                                call_id: call.id.clone(),
                                name: call.name.clone(),
                                output: serde_json::json!({
                                    "error": "Action timed out waiting for approval"
                                }),
                                is_error: true,
                            });
                        }
                        Err(_) => {
                            // Channel closed
                            return Err(AgentRuntimeError::ApprovalTimeout);
                        }
                        _ => continue,
                    }
                }
                _ = &mut timeout => {
                    // Clean up the pending action
                    self.approval_queue.deny(&action_id).await;
                    return Err(AgentRuntimeError::ApprovalTimeout);
                }
            }
        }
    }

    /// Update session state and emit corresponding event.
    pub fn update_session_state(
        &self,
        session_id: &SessionId,
        state: SessionState,
        app_handle: &AppHandle,
    ) -> Result<(), AgentRuntimeError> {
        self.session_store
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
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Derives the system prompt with optional subtask context.
fn derive_system_prompt(
    agent_system_prompt: &str,
    current_subtask: &Option<(SubtaskSummary, usize)>,
) -> String {
    match current_subtask {
        Some((summary, total)) => {
            if summary.status == TaskStatus::InProgress {
                format!(
                    "{}\n\n## Current Task ({}/{}): {}\n\n{}\n\nFocus on completing this task. \
                        When done, call agent_advance_to_next_subtask.",
                    agent_system_prompt,
                    summary.order + 1,
                    total,
                    summary.title,
                    summary.mode.instruction_text()
                )
            } else {
                agent_system_prompt.to_string()
            }
        }
        None => agent_system_prompt.to_string(),
    }
}

/// Infers the action type from tool name and arguments.
fn infer_action_type(tool_name: &str, arguments: &serde_json::Value) -> ActionType {
    let path = arguments
        .get("path")
        .or_else(|| arguments.get("file_path"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    match tool_name {
        name if name.contains("read") => ActionType::ReadFile { path },
        name if name.contains("write") || name.contains("create") => {
            let size = arguments
                .get("content")
                .and_then(|v| v.as_str())
                .map(|s| s.len() as u64)
                .unwrap_or(0);
            ActionType::WriteFile {
                path,
                size_bytes: size,
            }
        }
        name if name.contains("delete") || name.contains("remove") => {
            ActionType::DeleteFile { path }
        }
        name if name.contains("execute") || name.contains("command") || name.contains("shell") => {
            let command = arguments
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            ActionType::ExecuteCommand { command }
        }
        name if name.contains("fetch") || name.contains("http") || name.contains("web") => {
            let url = arguments
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            ActionType::WebRequest { url }
        }
        _ => ActionType::ReadFile { path },
    }
}

/// Generates a diff from two content strings.
fn generate_diff_from_content(path: &str, original: &str, current: &str) -> String {
    use std::path::Path;
    if let Some(diff) = crate::tools::fs::write_file::generate_diff(Path::new(path), original, current) {
        diff
    } else {
        // Fallback: simple diff generation
        let line_count = current.lines().count();
        let lines: String = current
            .lines()
            .map(|line| format!("+{}", line))
            .collect::<Vec<_>>()
            .join("\n");
        format!("--- a/{}\n+++ b/{}\n@@ -0,0 +1,{} @@\n{}",
            path, path, line_count, lines
        )
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_action_type_read() {
        let action = infer_action_type(
            "read_workspace_file",
            &serde_json::json!({"path": "/tmp/test.rs"}),
        );
        match action {
            ActionType::ReadFile { path } => assert_eq!(path, "/tmp/test.rs"),
            _ => panic!("Expected ReadFile"),
        }
    }

    #[test]
    fn test_infer_action_type_write() {
        let action = infer_action_type(
            "write_file",
            &serde_json::json!({"path": "/tmp/out.rs", "content": "hello"}),
        );
        match action {
            ActionType::WriteFile { path, size_bytes } => {
                assert_eq!(path, "/tmp/out.rs");
                assert_eq!(size_bytes, 5);
            }
            _ => panic!("Expected WriteFile"),
        }
    }

    #[test]
    fn test_infer_action_type_execute() {
        let action = infer_action_type(
            "execute_command",
            &serde_json::json!({"command": "cargo build"}),
        );
        match action {
            ActionType::ExecuteCommand { command } => assert_eq!(command, "cargo build"),
            _ => panic!("Expected ExecuteCommand"),
        }
    }

    #[test]
    fn test_infer_action_type_unknown_defaults_to_read() {
        let action = infer_action_type("unknown_tool", &serde_json::json!({}));
        match action {
            ActionType::ReadFile { path } => assert_eq!(path, ""),
            _ => panic!("Expected ReadFile as default"),
        }
    }

    #[test]
    fn test_context_compaction_below_threshold() {
        use crate::llm::Message;

        let mut context = Vec::new();
        for i in 0..10 {
            context.push(Message::user(format!("Message {}", i)));
            context.push(Message::assistant(format!("Response {}", i)));
        }

        let result = maybe_compact_context(&mut context);

        // No compaction should occur
        assert!(result.is_none(), "Compaction should not occur for small contexts");
        assert_eq!(context.len(), 20, "Context should remain unchanged");
    }

    #[test]
    fn test_context_compaction_keeps_first_and_recent_messages() {
        use crate::llm::Message;

        let mut context = Vec::new();

        // First message (should be preserved)
        context.push(Message::user("FIRST MESSAGE - important context".to_string()));

        // Middle messages (should be discarded)
        for i in 0..2500 {
            context.push(Message::user(format!("Middle message {} with filler content", i)));
            context.push(Message::assistant(format!("Middle response {} with filler content", i)));
        }

        // Recent messages (should be preserved - last 15)
        let recent_count = 15;
        for i in 0..recent_count {
            context.push(Message::user(format!("Recent message {}", i)));
            context.push(Message::assistant(format!("Recent response {}", i)));
        }

        let initial_count = context.len();
        let result = maybe_compact_context(&mut context);

        assert!(result.is_some(), "Compaction should occur");
        let compaction = result.unwrap();

        // Should be much smaller
        assert!(compaction.after_count < initial_count);

        // First message should be preserved
        let first_msg = context.first().unwrap();
        assert!(first_msg.as_text().unwrap().contains("FIRST MESSAGE"));

        // Last few messages should include recent ones
        assert!(context.len() <= recent_count + 1, "Should have at most recent messages + first message");
    }

    #[test]
    fn test_loop_detection_skips_agent_tools() {
        // Verify that agent metadata tools are excluded from loop detection
        use super::loop_detection::LOOP_SKIP_TOOLS;

        assert!(LOOP_SKIP_TOOLS.contains(&"agent_advance_to_next_subtask"));
        assert!(LOOP_SKIP_TOOLS.contains(&"agent_emit_status"));
        assert!(LOOP_SKIP_TOOLS.contains(&"agent_mark_subtask_status"));

        // agent_plan_subtasks is NOT in the skip list because it should only be called once
        assert!(!LOOP_SKIP_TOOLS.contains(&"agent_plan_subtasks"));
    }

    #[test]
    fn test_loop_skip_tools_list_is_complete() {
        // Verify the skip list includes all agent_* tools except agent_plan_subtasks
        use super::loop_detection::LOOP_SKIP_TOOLS;

        // These tools can be called multiple times legitimately:
        // - agent_advance_to_next_subtask: once per subtask advancement
        // - agent_emit_status: for progress updates (can be many)
        // - agent_mark_subtask_status: for manual status updates
        assert_eq!(LOOP_SKIP_TOOLS.len(), 3);

        // agent_plan_subtasks should NOT be skipped (has its own protection)
        assert!(!LOOP_SKIP_TOOLS.contains(&"agent_plan_subtasks"));
    }
}
