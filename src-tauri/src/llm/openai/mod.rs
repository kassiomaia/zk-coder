//! OpenAI-compatible API client.
//!
//! This module implements the LLM client for OpenAI and OpenAI-compatible APIs
//! (including Google Gemini via its OpenAI-compatible endpoint).

mod types;
mod client;

#[cfg(test)]
mod integration_tests;

pub use client::{OpenAiClient, GOOGLE_BASE_URL, OPENAI_BASE_URL};
pub use types::*;

/// Converts provider-agnostic tool definitions to OpenAI request tools.
pub fn convert_tools(tools: Vec<crate::llm::ToolDefinition>) -> Option<Vec<types::OpenAiRequestTool>> {
    if tools.is_empty() {
        return None;
    }
    Some(
        tools
            .into_iter()
            .map(|tool| types::OpenAiRequestTool {
                tool_type: "function".to_string(),
                function: types::OpenAiRequestFunction {
                    name: tool.name,
                    description: tool.description,
                    parameters: tool.input_schema,
                },
            })
            .collect(),
    )
}
