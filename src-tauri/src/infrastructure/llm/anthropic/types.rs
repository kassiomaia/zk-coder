//! Anthropic API types.

use serde::{Deserialize, Serialize};

/// Anthropic messages API request.
#[derive(Debug, Serialize)]
pub struct AnthropicRequest {
    pub model: String,
    pub messages: Vec<AnthropicMessage>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<AnthropicTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<AnthropicToolChoice>,
}

/// Anthropic tool selection strategy.
#[derive(Debug, Serialize)]
pub struct AnthropicToolChoice {
    #[serde(rename = "type")]
    pub choice_type: String,
}

/// Anthropic message format.
#[derive(Debug, Serialize)]
pub struct AnthropicMessage {
    pub role: String,
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
    Text { text: String },
    /// Image content
    Image { source: AnthropicImageSource },
    /// Assistant tool call block.
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// User tool result block.
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        is_error: bool,
    },
}

/// Anthropic image source.
#[derive(Debug, Serialize)]
pub struct AnthropicImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: String,
    pub data: String,
}

/// Anthropic tool definition in messages API requests.
#[derive(Debug, Serialize)]
pub struct AnthropicTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Anthropic messages API response.
#[derive(Debug, Deserialize)]
pub struct AnthropicResponse {
    pub id: String,
    pub model: String,
    #[allow(dead_code)]
    pub role: String,
    pub content: Vec<AnthropicResponseContent>,
    #[serde(default)]
    pub usage: Option<AnthropicUsage>,
    #[serde(default)]
    pub stop_reason: Option<String>,
}

/// Anthropic response content block.
#[derive(Debug, Deserialize)]
pub struct AnthropicResponseContent {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub content_type: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub input: Option<serde_json::Value>,
}

/// Anthropic usage statistics.
#[derive(Debug, Deserialize)]
pub struct AnthropicUsage {
    pub input_tokens: u32,
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
