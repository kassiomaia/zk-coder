//! Token budgeting and estimation for the agent runtime context.
//!
//! Owns the per-model context-window table, the adaptive compaction
//! threshold, the per-block budgets used by `ContextPolicyV2`, and the
//! `len / 4` token estimator that feeds compaction decisions.

use crate::core::llm::entities::{
    ChatMessage as Message, ContentBlock, MessageContent, ToolCall, ToolResult,
};
use serde_json::Value as JsonValue;

/// Approximate context window size for token budgeting (conservative 128k).
pub const CONTEXT_WINDOW: usize = 128_000;

/// Threshold (60%) at which to compact the context.
pub const COMPRESSION_THRESHOLD: f64 = 0.60;

/// Number of recent messages to preserve during compaction.
pub const KEEP_RECENT_MESSAGES: usize = 15;
/// Number of recent dialogue messages to preserve in ContextPolicyV2 tail block.
pub const KEEP_RECENT_TAIL_MESSAGES: usize = 12;

/// Hard token budgets for ContextPolicyV2 blocks.
pub const BUDGET_TASK_SUMMARY_TOKENS: usize = 180;
/// Hard token budgets for ContextPolicyV2 blocks.
pub const BUDGET_ACTIVE_SUBTASK_TOKENS: usize = 120;
/// Hard token budgets for ContextPolicyV2 blocks.
pub const BUDGET_FILE_RELEVANCE_TOKENS: usize = 800;
/// Hard token budgets for ContextPolicyV2 blocks.
pub const BUDGET_FAILURE_MEMORY_TOKENS: usize = 140;
/// Hard token budgets for ContextPolicyV2 blocks.
pub const BUDGET_RECENT_DIALOGUE_TOKENS: usize = 1600;
/// Hard token budget for the ContextPolicyV2 memory reference block (M3.2).
/// Sized to carry a compact identifier, a truncated active-plan summary, and
/// knowledge-item counts without inlining full memory contents.
pub const BUDGET_MEMORY_REFERENCE_TOKENS: usize = 180;
/// Hard token budget for the ContextPolicyV2 execution brief block (M4.1).
/// Sized to carry goal / active / recent / next / blocker lines near the
/// attention tail without letting any single field dominate the prompt.
pub const BUDGET_EXECUTION_BRIEF_TOKENS: usize = 240;

/// Flat token overhead per message (role + metadata framing).
pub const PER_MESSAGE_OVERHEAD: usize = 10;
/// Flat token framing cost added for each tool call on a message.
pub const PER_TOOL_CALL_FRAMING: usize = 4;
/// Flat token framing cost added for each tool result on a message.
pub const PER_TOOL_RESULT_FRAMING: usize = 4;
/// Conservative per-image token cost for multi-modal messages.
pub const IMAGE_TOKEN_COST: usize = 1_200;

/// Returns the known context window size for a given model name.
/// Falls back to CONTEXT_WINDOW if the model is unknown.
pub fn model_context_window(model_name: &str) -> usize {
    let name = model_name.to_lowercase();
    if name.contains("claude") {
        200_000
    } else if name.contains("gemini-1.5") {
        1_000_000
    } else if name.contains("gpt-4-32k") {
        32_768
    } else if name.contains("gpt-4") {
        128_000
    } else if name.contains("gpt-3.5-turbo-16k") {
        16_384
    } else if name.contains("gpt-3.5") {
        4_096
    } else {
        CONTEXT_WINDOW // conservative default
    }
}

/// Computes the compression threshold ratio based on the model's context window.
/// Larger context windows compact later (0.70–0.80), smaller ones compact earlier (0.50–0.60).
pub fn adaptive_compression_threshold(model_name: &str) -> f64 {
    let window = model_context_window(model_name);
    match window {
        w if w >= 500_000 => 0.80,
        w if w >= 128_000 => 0.70,
        w if w >= 32_000 => 0.60,
        _ => 0.50,
    }
}

/// Estimates the token count for a message context.
///
/// Counts the payload that the provider will actually serialize: text
/// content (including text inside `Blocks`), images (as a flat cost),
/// tool call arguments, and tool result outputs. A rough `len / 4` rule
/// approximates tokenization; the estimate is used for budgeting only.
pub fn estimate_context_tokens(context: &[Message]) -> usize {
    context.iter().map(estimate_message_tokens).sum()
}

/// Estimates the token cost of a single message, including tool calls and tool results.
pub fn estimate_message_tokens(msg: &Message) -> usize {
    let mut total = PER_MESSAGE_OVERHEAD;
    total = total.saturating_add(estimate_content_tokens(&msg.content));
    for call in &msg.tool_calls {
        total = total.saturating_add(estimate_tool_call_tokens(call));
    }
    if let Some(result) = &msg.tool_result {
        total = total.saturating_add(estimate_tool_result_tokens(result));
    }
    total
}

pub(super) fn estimate_content_tokens(content: &MessageContent) -> usize {
    match content {
        MessageContent::Text(text) => text.len() / 4,
        MessageContent::Blocks(blocks) => blocks
            .iter()
            .map(|block| match block {
                ContentBlock::Text { text } => text.len() / 4,
                ContentBlock::Image { .. } => IMAGE_TOKEN_COST,
            })
            .sum(),
    }
}

pub(super) fn estimate_tool_call_tokens(call: &ToolCall) -> usize {
    call.name.len() / 4 + estimate_json_tokens(&call.arguments) + PER_TOOL_CALL_FRAMING
}

pub(super) fn estimate_tool_result_tokens(result: &ToolResult) -> usize {
    estimate_json_tokens(&result.output) + PER_TOOL_RESULT_FRAMING
}

pub(super) fn estimate_json_tokens(value: &JsonValue) -> usize {
    serde_json::to_string(value)
        .map(|s| s.len() / 4)
        .unwrap_or(0)
}

/// Rough text-only token estimate. Used by retrieval-snippet ranking
/// and by the `ContextPolicyV2` block fillers that need to know how
/// much budget a candidate string consumes before committing it.
pub(super) fn estimate_text_tokens(text: &str) -> usize {
    text.len() / 4 + 4
}
