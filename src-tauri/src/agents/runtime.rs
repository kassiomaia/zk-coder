//! Agent runtime for executing requests with permission-checked tool calls.
//!
//! Wraps the LLM orchestration loop with the agent permission system,
//! approval queue, and event emission for frontend updates.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use log::{error, info, warn};
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::{ChatState, LlmState};
use crate::llm::{
    create_client, ChatRequest, ChatResponse, LlmClient, LlmError, Message, ToolCall,
    ToolResult as LlmToolResult,
};
use crate::storage::ConfigStore;
use crate::tools::context::ToolExecutionContext;
use crate::tools::fs::write_file::generate_diff;
use crate::tools::OrchestratorToolResult;
use crate::tools::ToolRegistry;

use super::approval::{ApprovalEvent, ApprovalQueue};
use super::rollback::RollbackStore;
use super::events::{AgentEvent, AGENT_EVENT_NAME};
use super::permissions::PermissionResolver;
use super::session::{AgentSession, PendingAction, SessionId, SessionState};
use super::store::SessionStore;
use super::task::{
    AgentExecutionResult, AgentRequest, AgentTask, ExecutionStep, StepAction, StepResult,
    TaskStatus, UsageStats,
};
use super::types::{ActionType, PermissionLevel, TrustMode};

/// Approximate context window size for token budgeting (conservative 128k).
const CONTEXT_WINDOW: usize = 128_000;

/// Threshold (60%) at which to compact the context.
const COMPRESSION_THRESHOLD: f64 = 0.60;

/// Number of recent messages to preserve during compaction.
const KEEP_RECENT_MESSAGES: usize = 15;

/// Maximum same-action repetitions before detecting a loop.
const MAX_REPETITIONS: u32 = 3;

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

/// Details about context compaction.
#[derive(Debug, Clone)]
struct CompactionDetails {
    before_count: usize,
    after_count: usize,
    before_tokens: usize,
    after_tokens: usize,
    needs_reinjection: bool,
}

/// Summary of subtask data needed for prompt derivation and tracking.
/// Extracted to avoid magic tuple indices and improve code clarity.
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
        let (client, model_name) = self.create_llm_client(app_handle).await?;

        // Build initial LLM context
        let mut llm_context = self.build_llm_context(session, &request, app_handle).await?;

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
        // The agent is responsible for creating its own plan via agent_plan_subtasks
        // as its first action for non-trivial tasks.
        loop {
            step_num += 1;
            info!("[AgentRuntime] Step {} for session {}", step_num, session_id);

            // Calculate context usage percentage
            let estimated_tokens = Self::estimate_context_tokens(&llm_context);
            let context_usage = (estimated_tokens as f64 / CONTEXT_WINDOW as f64 * 100.0).min(100.0);

            // Auto-compact context if it's getting too large (60% of context window)
            if let Some(compaction) = Self::maybe_compact_context(&mut llm_context) {
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
            let system_prompt = match &current_subtask_summary {
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
            };
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
            let response = self.chat_with_retry(client.as_ref(), &chat_request).await?;
            let duration_ms = start.elapsed().as_millis() as u64;

            // Track usage
            if let Some(usage) = &response.usage {
                total_input_tokens += usage.input_tokens;
                total_output_tokens += usage.output_tokens;
            }

            // Record LLM step
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
                    duration_ms,
                }),
                timestamp: Utc::now(),
            };

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
                // Use streaming for the final response to show real-time progress
                let stream_response = self
                    .chat_streaming_final(
                        client.as_ref(),
                        &chat_request,
                        &session_id,
                        &session.conversation_id,
                        &session.thread_id,
                        app_handle,
                    )
                    .await?;

                // Update usage with streaming response
                if let Some(usage) = &stream_response.usage {
                    total_input_tokens += usage.input_tokens;
                    total_output_tokens += usage.output_tokens;
                }

                let result = AgentExecutionResult {
                    content: stream_response.content,
                    steps,
                    usage: Some(UsageStats {
                        input_tokens: total_input_tokens,
                        output_tokens: total_output_tokens,
                        total_tokens: total_input_tokens + total_output_tokens,
                    }),
                };

                // Update task to completed
                if let Err(e) = self.session_store.update_task_status(&task_id, &TaskStatus::Completed) {
                    warn!("[AgentRuntime] Failed to update task status: {e}");
                }
                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::TaskUpdated {
                        session_id: session_id.clone(),
                        task_id: task_id.clone(),
                        status: TaskStatus::Completed,
                    },
                );

                // Update session to finished
                self.update_session_state(
                    &session.id,
                    SessionState::Finished {
                        completed_at: Utc::now(),
                    },
                    app_handle,
                )?;

                let _ = app_handle.emit(
                    AGENT_EVENT_NAME,
                    AgentEvent::ExecutionComplete {
                        session_id: session_id.clone(),
                        conversation_id: session.conversation_id.clone(),
                        thread_id: session.thread_id.clone(),
                        result: result.clone(),
                    },
                );

                return Ok(result);
            }

            // Check for repetitive action loops before executing tool calls
            for tool_call in &response.tool_calls {
                // Create a hash of the tool call arguments for comparison
                let args_hash = {
                    use std::hash::{Hash, Hasher};
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    tool_call.arguments.hash(&mut hasher);
                    hasher.finish()
                };

                let tool_key = (tool_call.name.clone(), args_hash);

                // Count how many times this exact tool call has appeared recently
                let repeat_count = recent_tool_calls.iter().filter(|x| **x == tool_key).count();

                if repeat_count >= MAX_REPETITIONS as usize {
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
            for tool_call in &response.tool_calls {
                let args_hash = {
                    use std::hash::{Hash, Hasher};
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    tool_call.arguments.hash(&mut hasher);
                    hasher.finish()
                };
                recent_tool_calls.push((tool_call.name.clone(), args_hash));
            }

            // Keep only recent tool calls (last 10) for loop detection
            if recent_tool_calls.len() > 10 {
                recent_tool_calls.drain(0..recent_tool_calls.len() - 10);
            }

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
        task_id: &str,
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

        let result: LlmToolResult = self
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
                            let diff = match (original_content, current_content) {
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
                            };

                            diff
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
                        let action_str = "write";
                        let _ = self
                            .session_store
                            .add_file_touched(&session.id, path, action_str, diff.as_deref());
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
                        use chrono::Utc;
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
                "agent_plan_subtasks" => {
                    // Create subtasks from the planned list
                    if let Some(subtasks_arr) = call.arguments.get("subtasks").and_then(|v| v.as_array()) {
                        use super::task::{SubTask, SubTaskMode, TaskStatus};
                        for (i, item) in subtasks_arr.iter().enumerate() {
                            let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled");
                            let description = item.get("description").and_then(|v| v.as_str());

                            // Parse mode from request, default to FullAccess
                            let mode_str = item.get("mode").and_then(|v| v.as_str()).unwrap_or("full_access");
                            let mode = match mode_str {
                                "read" => SubTaskMode::Read,
                                "write" => SubTaskMode::Write,
                                "execute" => SubTaskMode::Execute,
                                _ => SubTaskMode::FullAccess,
                            };

                            let mut subtask = SubTask::new(task_id.to_string(), title.to_string(), mode, i as i32);
                            if let Some(desc) = description {
                                subtask = subtask.with_description(desc.to_string());
                            }

                            // Mark the first subtask as in_progress automatically
                            let is_first_subtask = i == 0;
                            if is_first_subtask {
                                subtask.status = TaskStatus::InProgress;
                            }

                            // Persist to database
                            let _ = self.session_store.create_subtask(&subtask);

                            // For first subtask, also update status to ensure started_at is set
                            // (create_subtask only inserts, doesn't set started_at for InProgress)
                            if is_first_subtask {
                                let _ = self.session_store.update_subtask_status(&subtask.id, &TaskStatus::InProgress);
                            }

                            // Emit event for frontend
                            let _ = app_handle.emit(
                                AGENT_EVENT_NAME,
                                AgentEvent::SubtaskCreated {
                                    session_id: session.id.0.clone(),
                                    subtask,
                                },
                            );
                        }
                    }
                }
                "agent_mark_subtask_status" => {
                    // Update subtask status during execution for real-time progress tracking
                    if let (Some(subtask_index), Some(status_str)) = (
                        call.arguments.get("index").and_then(|v| v.as_i64()),
                        call.arguments.get("status").and_then(|v| v.as_str())
                    ) {
                        use super::task::TaskStatus;

                        // Parse status string to TaskStatus enum
                        let task_status: Option<TaskStatus> = match status_str {
                            "pending" => Some(TaskStatus::Pending),
                            "in_progress" => Some(TaskStatus::InProgress),
                            "completed" => Some(TaskStatus::Completed),
                            "failed" => Some(TaskStatus::Failed),
                            _ => {
                                warn!("Invalid subtask status: {status_str}");
                                None
                            }
                        };

                        // Only proceed if status is valid
                        if let Some(task_status) = task_status {
                            // Find the subtask by order index
                            match self.session_store.get_subtasks(task_id) {
                                Ok(mut subtasks) => {
                                    if let Some(subtask) = subtasks.iter_mut().find(|st| st.order == subtask_index as i32) {
                                        // Update status and timestamps
                                        match task_status {
                                            TaskStatus::Completed => subtask.complete(),
                                            TaskStatus::Failed => subtask.fail(),
                                            _ => subtask.status = task_status.clone(),
                                        };

                                        // Persist to database
                                        if let Err(e) = self.session_store.update_subtask_status(&subtask.id, &subtask.status) {
                                            error!("Failed to update subtask status: {e}");
                                        }

                                        // Emit event for real-time UI update
                                        let _ = app_handle.emit(
                                            AGENT_EVENT_NAME,
                                            AgentEvent::SubtaskUpdated {
                                                session_id: session.id.0.clone(),
                                                subtask: subtask.clone(),
                                            },
                                        );

                                        info!("Updated subtask {} to status: {:?}", subtask_index, task_status);
                                    } else {
                                        warn!("Subtask with index {subtask_index} not found");
                                    }
                                }
                                Err(e) => error!("Failed to get subtasks: {e}"),
                            }
                        }
                    }
                }
                "agent_advance_to_next_subtask" => {
                    // Convenience tool to advance from current subtask to next one
                    // Marks current in_progress subtask as completed, next as in_progress
                    match self.session_store.get_subtasks(task_id) {
                        Ok(mut subtasks) => {
                            // Find current in_progress subtask
                            let current_idx = subtasks.iter().position(|st| st.status == TaskStatus::InProgress);

                            if let Some(idx) = current_idx {
                                // Mark current as completed
                                subtasks[idx].complete();
                                let _ = self.session_store.update_subtask_status(&subtasks[idx].id, &subtasks[idx].status);
                                let _ = app_handle.emit(
                                    AGENT_EVENT_NAME,
                                    AgentEvent::SubtaskUpdated {
                                        session_id: session.id.0.clone(),
                                        subtask: subtasks[idx].clone(),
                                    },
                                );

                                info!("Marked subtask {} as completed", idx);

                                // Mark next as in_progress
                                if idx + 1 < subtasks.len() {
                                    subtasks[idx + 1].status = TaskStatus::InProgress;
                                    let _ = self.session_store.update_subtask_status(&subtasks[idx + 1].id, &subtasks[idx + 1].status);
                                    let _ = app_handle.emit(
                                        AGENT_EVENT_NAME,
                                        AgentEvent::SubtaskUpdated {
                                            session_id: session.id.0.clone(),
                                            subtask: subtasks[idx + 1].clone(),
                                        },
                                    );

                                    info!("Marked subtask {} as in_progress", idx + 1);
                                } else {
                                    info!("No more subtasks to advance to");
                                }
                            } else {
                                warn!("No in_progress subtask found to advance from");
                            }
                        }
                        Err(e) => error!("Failed to get subtasks: {e}"),
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
                    if let Some(diff) = generate_diff(workspace, path, content) {
                        let lang = std::path::Path::new(path)
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(String::from);
                        pending = pending.with_preview(super::types::ActionPreview {
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

    /// Generate an execution plan for the given request.
    /// Update session state and emit event.
    fn update_session_state(
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

    /// Create an LLM client from app config.
    async fn create_llm_client(
        &self,
        app_handle: &AppHandle,
    ) -> Result<(Box<dyn LlmClient>, String), AgentRuntimeError> {
        info!("[AgentRuntime] Creating LLM client for agent execution");
        let llm_state = app_handle.state::<LlmState>();
        let config_store = llm_state.config_store.lock().await;

        // Use the "code" operation default model
        let model = crate::commands::config::get_code_operation_model(&config_store)
            .map_err(|e| {
                error!("[AgentRuntime] Failed to get code operation model: {}", e);
                AgentRuntimeError::Config(e.to_string())
            })?;

        info!("[AgentRuntime] Using model: {} ({})", model.name, model.provider_id);

        let provider = config_store
            .get_provider(&model.provider_id)
            .map_err(|e| AgentRuntimeError::Config(e.to_string()))?
            .ok_or_else(|| {
                error!("[AgentRuntime] Provider '{}' not found", model.provider_id);
                AgentRuntimeError::Config(format!(
                    "Provider '{}' not found",
                    model.provider_id
                ))
            })?;

        info!("[AgentRuntime] Provider found: {}", provider.id);

        let api_key = get_api_key_for_provider(&provider.id, &config_store)?;
        let model_name = model.name.clone();

        drop(config_store);

        info!("[AgentRuntime] Creating client with api_type: {:?}", provider.api_type);
        let client = create_client(provider.api_type, api_key, provider.base_url);
        info!("[AgentRuntime] LLM client created successfully");
        Ok((client, model_name))
    }

    /// Build LLM message context from conversation history.
    async fn build_llm_context(
        &self,
        session: &AgentSession,
        request: &AgentRequest,
        app_handle: &AppHandle,
    ) -> Result<Vec<Message>, AgentRuntimeError> {
        let chat_state = app_handle.state::<ChatState>();
        let chat_store = chat_state.store.lock().await;

        let history = chat_store
            .load_thread_messages(&session.thread_id)
            .unwrap_or_default();

        drop(chat_store);

        let mut context: Vec<Message> = history
            .iter()
            .map(|msg| match msg.role {
                crate::nlp::chat::MessageRole::User => {
                    Message::user(msg.content.clone())
                }
                crate::nlp::chat::MessageRole::Assistant => {
                    Message::assistant(msg.content.clone())
                }
                crate::nlp::chat::MessageRole::System => {
                    Message::system(msg.content.clone())
                }
            })
            .collect();

        // Add the current request
        context.push(Message::user(request.content.clone()));

        Ok(context)
    }

    /// Execute a chat request with retry on transient errors.
    async fn chat_with_retry(
        &self,
        client: &dyn LlmClient,
        request: &ChatRequest,
    ) -> Result<ChatResponse, AgentRuntimeError> {
        let mut attempt = 0usize;
        loop {
            match client.chat(request.clone()).await {
                Ok(response) => return Ok(response),
                Err(err) => {
                    let can_retry = err.is_retryable() && attempt < LLM_MAX_RETRIES;
                    if !can_retry {
                        return Err(AgentRuntimeError::Llm(err.to_string()));
                    }

                    let delay_ms = LLM_RETRY_BASE_MS * (1u64 << attempt);
                    warn!(
                        "[AgentRuntime] Transient LLM error (attempt {}/{}), retrying in {}ms: {}",
                        attempt + 1,
                        LLM_MAX_RETRIES + 1,
                        delay_ms,
                        err
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    attempt += 1;
                }
            }
        }
    }

    /// Execute a streaming chat request for the final agent response.
    /// Emits StreamingContent events as chunks arrive and returns the complete response.
    ///
    /// Uses non-blocking event emission via tokio spawn to prevent the backend
    /// from freezing if the frontend is slow to process events.
    async fn chat_streaming_final(
        &self,
        client: &dyn LlmClient,
        request: &ChatRequest,
        session_id: &str,
        conversation_id: &str,
        thread_id: &str,
        app_handle: &AppHandle,
    ) -> Result<ChatResponse, AgentRuntimeError> {
        use futures::StreamExt;

        // Enable streaming for the request
        let mut stream_request = request.clone();
        stream_request.stream = true;

        let mut attempt = 0usize;
        loop {
            let mut stream = client.chat_stream(stream_request.clone());

            let mut full_content = String::new();
            let mut final_usage = None;
            let mut final_finish_reason = None;
            let mut response_id = String::new();
            let mut response_model = String::new();

            let mut succeeded = false;
            let mut last_error = None;

            // Add a timeout to prevent infinite hanging (5 minutes for long LLM responses)
            let stream_timeout = tokio::time::sleep(Duration::from_secs(300));
            tokio::pin!(stream_timeout);

            loop {
                tokio::select! {
                    chunk_result = StreamExt::next(&mut stream) => {
                        match chunk_result {
                            Some(Ok(chunk)) => {
                                response_id = chunk.id;
                                response_model = chunk.model;

                                if let Some(ref delta) = chunk.delta {
                                    if let Some(ref content) = delta.content {
                                        // Clone data for non-blocking emit
                                        let session_id = session_id.to_string();
                                        let conversation_id = conversation_id.to_string();
                                        let thread_id = thread_id.to_string();
                                        let chunk_content = content.clone();
                                        let app_handle = app_handle.clone();

                                        // Spawn a task to emit the event non-blockingly
                                        tokio::spawn(async move {
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
                                        });

                                        full_content.push_str(content);
                                    }
                                }

                                if chunk.is_final {
                                    final_usage = chunk.usage;
                                    final_finish_reason = chunk.finish_reason;
                                }
                            }
                            Some(Err(err)) => {
                                last_error = Some(err);
                                break;
                            }
                            None => {
                                // Stream ended naturally
                                break;
                            }
                        }
                    }
                    _ = &mut stream_timeout => {
                        error!("[AgentRuntime] Streaming timeout after 300s");
                        last_error = Some(LlmError::timeout(300));
                        break;
                    }
                }

                // Break on error
                if last_error.is_some() {
                    break;
                }
            }

            // Check if we completed successfully (empty content is valid for streaming)
            if last_error.is_none() {
                // Emit final chunk non-blockingly
                let session_id = session_id.to_string();
                let conversation_id = conversation_id.to_string();
                let thread_id = thread_id.to_string();
                let app_handle = app_handle.clone();

                tokio::spawn(async move {
                    let _ = app_handle.emit(
                        AGENT_EVENT_NAME,
                        AgentEvent::StreamingContent {
                            session_id,
                            conversation_id,
                            thread_id,
                            chunk: String::new(),
                            is_final: true,
                        },
                    );
                });

                return Ok(ChatResponse {
                    id: response_id,
                    model: response_model,
                    content: full_content,
                    usage: final_usage,
                    finish_reason: final_finish_reason,
                    tool_calls: Vec::new(),
                });
            }

            // Handle retry
            let err = last_error.unwrap();
            let can_retry = err.is_retryable() && attempt < LLM_MAX_RETRIES;
            if !can_retry {
                return Err(AgentRuntimeError::Llm(err.to_string()));
            }

            let delay_ms = LLM_RETRY_BASE_MS * (1u64 << attempt);
            warn!(
                "[AgentRuntime] Transient LLM streaming error (attempt {}/{}), retrying in {}ms: {}",
                attempt + 1,
                LLM_MAX_RETRIES + 1,
                delay_ms,
                err
            );
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            attempt += 1;
        }
    }

    /// Estimate token count for the LLM context.
    /// This is a rough estimate: ~4 chars per token for English text.
    fn estimate_context_tokens(context: &[Message]) -> usize {
        let total_chars: usize = context
            .iter()
            .map(|m| {
                let content_len = m.as_text().map(|s| s.chars().count()).unwrap_or(0);
                // Add overhead for role, tool calls, tool results, etc.
                content_len + 100
            })
            .sum();
        total_chars / 4 // Rough estimate: ~4 chars per token
    }

    /// Compact the context if it exceeds the compression threshold.
    /// Keeps the most recent messages and discards older ones.
    /// Returns compaction details if compaction occurred.
    fn maybe_compact_context(context: &mut Vec<Message>) -> Option<CompactionDetails> {
        let estimated = Self::estimate_context_tokens(context);
        let threshold = (CONTEXT_WINDOW as f64 * COMPRESSION_THRESHOLD) as usize;

        if estimated > threshold {
            let before_count = context.len();
            let before_tokens = estimated;

            // Check if we have subtask context before compaction
            let has_subtask_context_before = context.iter().any(|m| {
                m.role == crate::llm::MessageRole::System && m.as_text().unwrap_or_default().contains("## Current Task")
            });

            info!(
                "[AgentRuntime] Context at {} tokens (threshold {}), compacting from {} messages... (has subtask context: {})",
                estimated, threshold, before_count, has_subtask_context_before
            );

            // Always keep the first message (usually user request) and most recent messages
            if context.len() > KEEP_RECENT_MESSAGES + 1 {
                // Keep first message + last N messages
                let first = context.first().cloned();
                let recent: Vec<Message> = context
                    .iter()
                    .rev()
                    .take(KEEP_RECENT_MESSAGES)
                    .cloned()
                    .collect();

                *context = recent.into_iter().rev().collect();

                // Prepend the first message if it exists and isn't already included
                if let Some(first_msg) = first {
                    let first_text = first_msg.as_text();
                    let already_has_first = context
                        .iter()
                        .any(|m| m.as_text() == first_text);
                    if !already_has_first {
                        context.insert(0, first_msg);
                    }
                }

                let after_tokens = Self::estimate_context_tokens(context);

                // Check if subtask context was removed
                let has_subtask_context_after = context.iter().any(|m| {
                    m.role == crate::llm::MessageRole::System && m.as_text().unwrap_or_default().contains("## Current Task")
                });

                let needs_reinjection = has_subtask_context_before && !has_subtask_context_after;

                if needs_reinjection {
                    info!(
                        "[AgentRuntime] Compaction removed subtask context - will re-inject"
                    );
                }

                info!(
                    "[AgentRuntime] Compacted context to {} messages, {} tokens",
                    context.len(),
                    after_tokens
                );

                return Some(CompactionDetails {
                    before_count,
                    after_count: context.len(),
                    before_tokens,
                    after_tokens,
                    needs_reinjection,
                });
            }
        }
        None
    }
}

/// Infer the action type from a tool name and arguments.
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

/// Get API key for a provider from the config store.
fn get_api_key_for_provider(
    provider_id: &str,
    config_store: &ConfigStore,
) -> Result<String, AgentRuntimeError> {
    config_store
        .retrieve_api_key(provider_id)
        .map_err(|e| AgentRuntimeError::Config(e.to_string()))
}

/// Generate a unified diff from original and new content strings.
fn generate_diff_from_content(path: &str, original: &str, new: &str) -> String {
    let diff = similar::TextDiff::from_lines(original, new);

    let hunks: Vec<String> = diff
        .unified_diff()
        .context_radius(3)
        .iter_hunks()
        .map(|h| h.to_string())
        .collect();

    if hunks.is_empty() {
        // No changes - return empty diff
        return format!("--- a/{}\n+++ b/{}\n", path, path);
    }

    let mut output = format!("--- a/{}\n+++ b/{}\n", path, path);
    for hunk in hunks {
        output.push_str(&hunk);
    }
    output
}

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

    // ------------------------------------------------------------------------
    // Context Compaction Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_context_compaction_preserves_subtask_context() {
        // Test that context compaction detects and preserves subtask context markers
        use crate::llm::{Message, MessageRole};

        // Create a context large enough to trigger compaction (> 60% of 128k tokens)
        let mut context = Vec::new();

        // Add initial user message (first message is always preserved)
        context.push(Message::user("Initial request to analyze the codebase".to_string()));

        // Add subtask context as a system message
        context.push(Message::system(
            "## Current Task (1/3): Explore codebase structure\n\nREAD-ONLY: You may only read files.".to_string()
        ));

        // Add many messages to exceed threshold (need > 60% of 128k tokens = ~76800 tokens)
        // At ~150 chars per message (content + overhead), need ~512 messages to trigger compaction
        for i in 0..2500 {
            context.push(Message::user(format!("Message {} with some content to fill space and ensure we exceed the compression threshold significantly", i)));
            context.push(Message::assistant(format!("Response {} for message with more content to increase token count", i)));
        }

        // Get initial count
        let initial_count = context.len();
        let has_subtask_before = context.iter().any(|m| {
            m.role == MessageRole::System && m.as_text().unwrap_or_default().contains("## Current Task")
        });

        assert!(has_subtask_before, "Should have subtask context before compaction");

        // Run compaction
        let result = AgentRuntime::maybe_compact_context(&mut context);

        // Verify compaction occurred
        assert!(result.is_some(), "Compaction should have occurred");
        let compaction = result.unwrap();
        assert!(compaction.after_count < initial_count, "Context should be smaller after compaction");
        assert!(compaction.before_tokens > compaction.after_tokens, "Token count should decrease");

        // Check if subtask context was preserved
        let has_subtask_after = context.iter().any(|m| {
            m.role == MessageRole::System && m.as_text().unwrap_or_default().contains("## Current Task")
        });

        // The key assertion: if subtask context was removed, needs_reinjection should be true
        if !has_subtask_after {
            assert!(compaction.needs_reinjection, "needs_reinjection should be true when subtask context was removed");
        }

        // Verify the first message was preserved
        assert!(context.first().unwrap().as_text().unwrap().contains("Initial request"));
    }

    #[test]
    fn test_context_compaction_without_subtask_context() {
        // Test compaction when there's no subtask context (should not set needs_reinjection)
        use crate::llm::Message;

        let mut context = Vec::new();

        // Add initial user message
        context.push(Message::user("Simple request".to_string()));

        // Add many messages to exceed threshold (no subtask context this time)
        for i in 0..2500 {
            context.push(Message::user(format!("Message {} with some content to fill space and ensure we exceed the compression threshold", i)));
            context.push(Message::assistant(format!("Response {} for message", i)));
        }

        // Run compaction
        let result = AgentRuntime::maybe_compact_context(&mut context);

        // Verify compaction occurred
        assert!(result.is_some(), "Compaction should have occurred");
        let compaction = result.unwrap();

        // Since there was no subtask context to begin with, needs_reinjection should be false
        assert!(!compaction.needs_reinjection, "needs_reinjection should be false when there was no subtask context");
    }

    #[test]
    fn test_context_compaction_below_threshold() {
        // Test that compaction doesn't occur when context is small
        use crate::llm::Message;

        let mut context = Vec::new();
        for i in 0..10 {
            context.push(Message::user(format!("Message {}", i)));
            context.push(Message::assistant(format!("Response {}", i)));
        }

        let result = AgentRuntime::maybe_compact_context(&mut context);

        // No compaction should occur
        assert!(result.is_none(), "Compaction should not occur for small contexts");
        assert_eq!(context.len(), 20, "Context should remain unchanged");
    }

    #[test]
    fn test_context_compaction_keeps_first_and_recent_messages() {
        // Test that compaction preserves the first message and most recent messages
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
        let result = AgentRuntime::maybe_compact_context(&mut context);

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
}
