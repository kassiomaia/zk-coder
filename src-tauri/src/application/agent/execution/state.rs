use crate::core::agent::execution::{ExecutionPhase, ExecutionSnapshot, LlmTracePayload};
use crate::core::agent::task::ExecutionStep;
use crate::core::llm::entities::{ChatMessage as Message, ChatResponse, ToolDefinition};
use crate::infrastructure::agent::runtime::SubtaskSummary;
use chrono::Utc;

pub struct ExecutionContext {
    pub session_id: String,
    pub task_id: String,
    pub model_name: String,
    pub llm_context: Vec<Message>,
    pub steps: Vec<ExecutionStep>,
    pub total_input_tokens: u32,
    pub total_output_tokens: u32,
    pub step_number: u32,
    pub recent_tool_calls: Vec<(String, u64)>,
    pub current_subtask_id: Option<String>,
    pub last_injected_subtask_order: Option<i32>,
    pub current_phase: ExecutionPhase,
    pub context_usage_percent: f64,
}

impl ExecutionContext {
    pub fn snapshot(
        &self,
        phase: ExecutionPhase,
        last_model: Option<String>,
        last_error: Option<String>,
    ) -> ExecutionSnapshot {
        ExecutionSnapshot {
            session_id: self.session_id.clone(),
            task_id: self.task_id.clone(),
            step_number: self.step_number,
            phase,
            current_subtask_id: self.current_subtask_id.clone(),
            context_usage_percent: self.context_usage_percent,
            total_input_tokens: self.total_input_tokens,
            total_output_tokens: self.total_output_tokens,
            last_model,
            last_error,
            updated_at: Utc::now(),
        }
    }
}

#[derive(Clone)]
pub struct ResolvedContext {
    pub current_subtask_summary: Option<(SubtaskSummary, usize)>,
    pub current_subtask_id: Option<String>,
    pub tool_definitions: Vec<ToolDefinition>,
    pub system_prompt: String,
}

pub struct LlmCallResult {
    pub response: ChatResponse,
    pub duration_ms: u64,
    pub request_payload: LlmTracePayload,
    pub response_payload: LlmTracePayload,
}
