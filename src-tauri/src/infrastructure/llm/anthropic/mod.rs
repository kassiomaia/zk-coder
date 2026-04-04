//! Anthropic Claude API client.
//!
//! This module implements the LLM client for Anthropic's Claude API.
//! Unlike `OpenAI`, Anthropic uses a different API structure with:
//! - `x-api-key` header for authentication
//! - `anthropic-version` header for API versioning
//! - System prompt as a top-level field
//! - Content blocks for multi-modal messages
//! - Different streaming event format

mod client;
mod stream;
mod types;

pub use client::{AnthropicClient, ANTHROPIC_BASE_URL, ANTHROPIC_VERSION, DEFAULT_MAX_TOKENS};
pub use stream::*;
pub use types::*;

/// Converts provider-agnostic tool definitions to Anthropic request tools.
#[must_use] 
pub fn convert_tools(
    tools: Vec<crate::core::llm::entities::ToolDefinition>,
) -> Option<Vec<types::AnthropicTool>> {
    if tools.is_empty() {
        return None;
    }
    Some(
        tools
            .into_iter()
            .map(|tool| types::AnthropicTool {
                name: tool.name,
                description: tool.description,
                input_schema: tool.input_schema,
            })
            .collect(),
    )
}
