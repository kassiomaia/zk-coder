//! Anthropic API types.

use serde::{Deserialize, Serialize};

/// Anthropic messages API request.
#[derive(Debug, Serialize)]
pub struct AnthropicRequest {
    /// Model identifier to use for the request.
    pub model: String,
    /// List of messages in the conversation.
    pub messages: Vec<AnthropicMessage>,
    /// Maximum number of tokens to generate.
    pub max_tokens: u32,
    /// Optional system prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Sampling temperature (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Nucleus sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Custom stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Whether to stream the response.
    pub stream: bool,
    /// Available tools for the model to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<AnthropicTool>>,
    /// Tool selection strategy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<AnthropicToolChoice>,
}

/// Anthropic tool selection strategy.
#[derive(Debug, Serialize)]
pub struct AnthropicToolChoice {
    /// Type of tool choice (e.g., "auto", "any", "tool").
    #[serde(rename = "type")]
    pub choice_type: String,
}

/// Anthropic message format.
#[derive(Debug, Serialize)]
pub struct AnthropicMessage {
    /// Role of the message sender (e.g., "user", "assistant").
    pub role: String,
    /// Content of the message.
    pub content: AnthropicContent,
}

/// Anthropic content (can be string or array of blocks).
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum AnthropicContent {
    /// Simple text content
    Text(String),
    /// Multi-modal content blocks
    Blocks(Vec<AnthropicContentBlock>),
}

/// Anthropic content block.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnthropicContentBlock {
    /// Text content
    Text {
        /// Text content.
        text: String,
    },
    /// Image content
    Image {
        /// Source of the image.
        source: AnthropicImageSource,
    },
    /// Assistant tool call block.
    ToolUse {
        /// Unique identifier for the tool use.
        id: String,
        /// Name of the tool being called.
        name: String,
        /// Input arguments for the tool.
        input: serde_json::Value,
    },
    /// User tool result block.
    ToolResult {
        /// ID of the tool use this result is for.
        tool_use_id: String,
        /// Result content.
        content: String,
        /// Whether this is an error result.
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        is_error: bool,
    },
}

/// Anthropic image source.
#[derive(Debug, Serialize)]
pub struct AnthropicImageSource {
    /// Type of the source (e.g., "base64").
    #[serde(rename = "type")]
    pub source_type: String,
    /// Media type of the image (e.g., "image/jpeg").
    pub media_type: String,
    /// Base64-encoded image data.
    pub data: String,
}

/// Anthropic tool definition in messages API requests.
#[derive(Debug, Serialize)]
pub struct AnthropicTool {
    /// Name of the tool.
    pub name: String,
    /// Description of what the tool does.
    pub description: String,
    /// JSON schema for the tool's input parameters.
    pub input_schema: serde_json::Value,
}

/// Anthropic messages API response.
#[derive(Debug, Deserialize)]
pub struct AnthropicResponse {
    /// Unique response identifier.
    pub id: String,
    /// Model used for the request.
    pub model: String,
    /// Role of the message sender.
    #[allow(dead_code)]
    pub role: String,
    /// Content of the response.
    pub content: Vec<AnthropicResponseContent>,
    /// Usage statistics.
    #[serde(default)]
    pub usage: Option<AnthropicUsage>,
    /// Reason why the response stopped.
    #[serde(default)]
    pub stop_reason: Option<String>,
}

/// Anthropic response content block.
#[derive(Debug, Deserialize)]
pub struct AnthropicResponseContent {
    /// Type of the content block.
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub content_type: String,
    /// Text content, if present.
    #[serde(default)]
    pub text: Option<String>,
    /// ID for tool use blocks.
    #[serde(default)]
    pub id: Option<String>,
    /// Name for tool call blocks.
    #[serde(default)]
    pub name: Option<String>,
    /// Input for tool use blocks.
    #[serde(default)]
    pub input: Option<serde_json::Value>,
}

/// Anthropic usage statistics.
#[derive(Debug, Deserialize)]
pub struct AnthropicUsage {
    /// Number of tokens in the input.
    pub input_tokens: u32,
    /// Number of tokens in the output.
    pub output_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_response_deserializes_tool_use_block() {
        let json = r#"{
            "id": "msg_1",
            "model": "claude-3-5-sonnet-20241022",
            "role": "assistant",
            "content": [
                {"type":"text","text":"Let me call a tool."},
                {"type":"tool_use","id":"toolu_1","name":"search_code","input":{"query":"runner"}}
            ],
            "usage": {"input_tokens": 10, "output_tokens": 3},
            "stop_reason": "tool_use"
        }"#;

        let parsed: AnthropicResponse =
            serde_json::from_str(json).expect("valid Anthropic response");
        assert_eq!(parsed.content.len(), 2);
        assert_eq!(parsed.content[1].content_type, "tool_use");
        assert_eq!(parsed.content[1].id.as_deref(), Some("toolu_1"));
        assert_eq!(parsed.content[1].name.as_deref(), Some("search_code"));
    }
}
