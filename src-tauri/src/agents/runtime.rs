//! Agent runtime for executing requests with permission-checked tool calls.
//!
//! Wraps the LLM orchestration loop with the agent permission system,
//! approval queue, and event emission for frontend updates.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use log::{info, warn};
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::{ChatState, LlmState};
use crate::llm::{
    create_client, ChatRequest, ChatResponse, LlmClient, Message, ToolCall,
    ToolResult as LlmToolResult,
};
use crate::storage::ConfigStore;
use crate::tools::context::ToolExecutionContext;
use crate::tools::fs::write_file::generate_diff;
use crate::tools::OrchestratorToolResult;
use crate::tools::ToolRegistry;

use super::approval::{ApprovalEvent, ApprovalQueue};
use super::events::{AgentEvent, AGENT_EVENT_NAME};
use super::permissions::PermissionResolver;
use super::session::{AgentSession, PendingAction, SessionId, SessionState};
use super::store::SessionStore;
use super::task::{
    AgentExecutionResult, AgentRequest, ExecutionStep, StepAction, StepResult, UsageStats,
};
use super::types::{ActionType, PermissionLevel, TrustMode};

/// Maximum orchestration steps before aborting.
const MAX_STEPS: u32 = 6;

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
    #[error("Max steps exceeded ({0})")]
    MaxStepsExceeded(u32),
    #[error("Action denied by user")]
    ActionDenied,
    #[error("Action timed out waiting for approval")]
    ApprovalTimeout,
    #[error("Store error: {0}")]
    Store(String),
    #[error("Config error: {0}")]
    Config(String),
}

/// The agent runtime orchestrates LLM + tool execution with permissions.
pub struct AgentRuntime {
    session_store: Arc<SessionStore>,
    approval_queue: Arc<ApprovalQueue>,
    tool_registry: Arc<ToolRegistry>,
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
        }
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
        let tool_definitions = self.tool_registry.definitions();
        let session_id = session.id.0.clone();
        let permission_resolver = PermissionResolver::new(trust_mode);

        // Update session state to running
        self.update_session_state(
            &session.id,
            SessionState::Running {
                current_step: 0,
                max_steps: MAX_STEPS,
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

        // Create LLM client
        let (client, model_name) = self.create_llm_client(app_handle).await?;

        // Build initial LLM context
        let mut llm_context = self.build_llm_context(session, &request, app_handle).await?;

        let mut steps: Vec<ExecutionStep> = Vec::new();
        let mut total_input_tokens: u32 = 0;
        let mut total_output_tokens: u32 = 0;

        for step_num in 1..=MAX_STEPS {
            info!(
                "[AgentRuntime] Step {}/{} for session {}",
                step_num, MAX_STEPS, session_id
            );

            // Update session state
            self.update_session_state(
                &session.id,
                SessionState::Running {
                    current_step: step_num,
                    max_steps: MAX_STEPS,
                },
                app_handle,
            )?;

            // Build chat request
            let mut chat_request = ChatRequest::new(&model_name, llm_context.clone())
                .with_system(agent_system_prompt)
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

            // If no tool calls, we're done
            if response.tool_calls.is_empty() {
                let result = AgentExecutionResult {
                    content: response.content,
                    steps,
                    usage: Some(UsageStats {
                        input_tokens: total_input_tokens,
                        output_tokens: total_output_tokens,
                        total_tokens: total_input_tokens + total_output_tokens,
                    }),
                };

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
                        result: result.clone(),
                    },
                );

                return Ok(result);
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

        // Max steps exceeded
        self.update_session_state(
            &session.id,
            SessionState::Error {
                message: format!("Max steps exceeded ({MAX_STEPS})"),
                recoverable: false,
            },
            app_handle,
        )?;

        Err(AgentRuntimeError::MaxStepsExceeded(MAX_STEPS))
    }

    /// Process tool calls with permission checks, requesting approval as needed.
    async fn process_tool_calls_with_permissions(
        &self,
        tool_calls: &[ToolCall],
        session: &AgentSession,
        permission_resolver: &PermissionResolver,
        app_handle: &AppHandle,
    ) -> Result<Vec<OrchestratorToolResult>, AgentRuntimeError> {
        let mut results = Vec::with_capacity(tool_calls.len());

        for call in tool_calls {
            let action_type = infer_action_type(&call.name, &call.arguments);
            let permission = permission_resolver.check(&call.name, &action_type);

            // Emit tool call event
            let _ = app_handle.emit(
                AGENT_EVENT_NAME,
                AgentEvent::ToolCall {
                    session_id: session.id.0.clone(),
                    tool: call.name.clone(),
                    arguments: call.arguments.clone(),
                },
            );

            let result = match permission {
                PermissionLevel::AutoRun => {
                    // Execute directly using the tool registry
                    self.execute_tool(call, session, app_handle).await
                }
                PermissionLevel::ApprovalRequired => {
                    // Queue for approval and wait
                    self.execute_with_approval(call, session, app_handle)
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

            // Emit tool result event
            let _ = app_handle.emit(
                AGENT_EVENT_NAME,
                AgentEvent::ToolResult {
                    session_id: session.id.0.clone(),
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
        app_handle: &AppHandle,
    ) -> OrchestratorToolResult {
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
                            return Ok(self.execute_tool(call, session, app_handle).await);
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
        let llm_state = app_handle.state::<LlmState>();
        let config_store = llm_state.config_store.lock().await;

        // Use the "code" operation default model
        let model = crate::commands::config::get_code_operation_model(&config_store)
            .map_err(AgentRuntimeError::Config)?;

        let provider = config_store
            .get_provider(&model.provider_id)
            .map_err(|e| AgentRuntimeError::Config(e.to_string()))?
            .ok_or_else(|| {
                AgentRuntimeError::Config(format!(
                    "Provider '{}' not found",
                    model.provider_id
                ))
            })?;

        let api_key = get_api_key_for_provider(&provider.id, &config_store)?;
        let model_name = model.name.clone();

        drop(config_store);

        let client = create_client(provider.api_type, api_key, provider.base_url);
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
}
