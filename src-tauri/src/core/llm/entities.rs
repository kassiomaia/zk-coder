//! LLM domain entities.
//!
//! Core data structures for LLM communication.

use serde::{Deserialize, Serialize};

/// Result type for LLM operations.
pub type LlmResult<T> = Result<T, LlmError>;

/// LLM operation errors.
#[derive(Debug, Clone, PartialEq)]
pub enum LlmError {
    /// Authentication failed
    AuthenticationFailed,
    /// Rate limited
    RateLimited { retry_after_seconds: Option<u64> },
    /// Invalid request
    InvalidRequest { message: String },
    /// Network error
    Network(String),
    /// Provider error
    Provider { code: String, message: String },
    /// Timeout
    Timeout,
    /// Unknown error
    Unknown(String),
}

impl LlmError {
    /// Checks if the error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimited { .. } | Self::Network(_) | Self::Timeout
        )
    }
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AuthenticationFailed => write!(f, "Authentication failed"),
            Self::RateLimited {
                retry_after_seconds,
            } => {
                write!(f, "Rate limited")?;
                if let Some(seconds) = retry_after_seconds {
                    write!(f, " (retry after {seconds}s)")?;
                }
                Ok(())
            }
            Self::InvalidRequest { message } => write!(f, "Invalid request: {message}"),
            Self::Network(msg) => write!(f, "Network error: {msg}"),
            Self::Provider { code, message } => write!(f, "Provider error ({code}): {message}"),
            Self::Timeout => write!(f, "Request timed out"),
            Self::Unknown(msg) => write!(f, "Unknown error: {msg}"),
        }
    }
}

impl std::error::Error for LlmError {}

/// Role of a message in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// System message (sets behavior)
    System,
    /// User message
    User,
    /// Assistant/AI message
    Assistant,
    /// Tool result message
    Tool,
}

impl MessageRole {
    /// Converts the role to a string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
        }
    }
}

/// Content block for multi-modal messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ContentBlock {
    /// Text content
    Text {
        /// The text content
        text: String,
    },
    /// Image content (base64 or URL)
    Image {
        /// Image source
        source: ImageSource,
    },
}

impl ContentBlock {
    /// Creates a text content block.
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text {
            text: content.into(),
        }
    }

    /// Creates an image content block from base64 data.
    pub fn image_base64(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Image {
            source: ImageSource {
                source_type: "base64".to_string(),
                media_type: Some(media_type.into()),
                data: data.into(),
            },
        }
    }

    /// Creates an image content block from a URL.
    pub fn image_url(url: impl Into<String>) -> Self {
        Self::Image {
            source: ImageSource {
                source_type: "url".to_string(),
                media_type: None,
                data: url.into(),
            },
        }
    }
}

/// Source for image content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageSource {
    /// Type of source (base64 or url)
    #[serde(rename = "type")]
    pub source_type: String,
    /// Media type (e.g., "image/png")
    pub media_type: Option<String>,
    /// The image data or URL
    pub data: String,
}

/// Content of a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text content
    Text(String),
    /// Multi-modal content with multiple blocks
    Blocks(Vec<ContentBlock>),
}

/// A message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Role of the message sender
    pub role: MessageRole,
    /// Content of the message
    pub content: MessageContent,
    /// Structured tool calls emitted by an assistant message
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    /// Structured tool result payload (for tool role messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<ToolResult>,
}

impl ChatMessage {
    /// Creates a new user message with text content.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: MessageContent::Text(content.into()),
            tool_calls: Vec::new(),
            tool_result: None,
        }
    }

    /// Creates a new assistant message with text content.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: MessageContent::Text(content.into()),
            tool_calls: Vec::new(),
            tool_result: None,
        }
    }

    /// Creates a new system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: MessageContent::Text(content.into()),
            tool_calls: Vec::new(),
            tool_result: None,
        }
    }

    /// Creates a message with multi-modal content.
    pub fn multimodal(role: MessageRole, blocks: Vec<ContentBlock>) -> Self {
        Self {
            role,
            content: MessageContent::Blocks(blocks),
            tool_calls: Vec::new(),
            tool_result: None,
        }
    }

    /// Creates an assistant message that carries structured tool calls.
    pub fn assistant_with_tool_calls(
        content: impl Into<String>,
        tool_calls: Vec<ToolCall>,
    ) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: MessageContent::Text(content.into()),
            tool_calls,
            tool_result: None,
        }
    }

    /// Creates a tool result message for a given tool call ID.
    pub fn tool_result(
        call_id: impl Into<String>,
        output: serde_json::Value,
        is_error: bool,
    ) -> Self {
        Self {
            role: MessageRole::Tool,
            content: MessageContent::Text(String::new()),
            tool_calls: Vec::new(),
            tool_result: Some(ToolResult {
                call_id: call_id.into(),
                output,
                is_error,
            }),
        }
    }

    /// Gets the text content if this is a simple text message.
    pub fn as_text(&self) -> Option<&str> {
        match &self.content {
            MessageContent::Text(text) => Some(text),
            MessageContent::Blocks(_) => None,
        }
    }
}

/// Structured tool result payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolResult {
    /// Tool call identifier this result belongs to
    pub call_id: String,
    /// Tool output JSON payload
    pub output: serde_json::Value,
    /// Whether the result represents an error
    #[serde(default)]
    pub is_error: bool,
}

/// Request to generate a chat completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// The model to use
    pub model: String,
    /// The conversation messages
    pub messages: Vec<ChatMessage>,
    /// System prompt (optional, prepended to messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Sampling temperature (0-2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Top-p sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// Whether to stream the response
    #[serde(default)]
    pub stream: bool,
    /// Tool definitions available to the model
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDefinition>,
}

impl ChatRequest {
    /// Creates a new chat request.
    pub fn new(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self {
            model: model.into(),
            messages,
            system: None,
            temperature: None,
            max_tokens: None,
            top_p: None,
            stop: None,
            stream: false,
            tools: Vec::new(),
        }
    }

    /// Sets the system prompt.
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Sets the temperature.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Sets the maximum tokens.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Enables streaming mode.
    pub fn with_stream(mut self) -> Self {
        self.stream = true;
        self
    }

    /// Sets available tool definitions.
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = tools;
        self
    }
}

/// Provider-agnostic tool definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolDefinition {
    /// Tool name
    pub name: String,
    /// Short tool description
    pub description: String,
    /// JSON Schema for tool arguments
    pub input_schema: serde_json::Value,
}

/// Response from a chat completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Unique ID for this response
    pub id: String,
    /// The model used
    pub model: String,
    /// The generated content
    pub content: String,
    /// Usage statistics
    pub usage: Option<UsageStats>,
    /// Finish reason
    pub finish_reason: Option<String>,
    /// Structured tool calls requested by the model
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
}

/// Provider-agnostic tool call representation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    /// Unique call identifier
    pub id: String,
    /// Tool name
    pub name: String,
    /// JSON arguments for the tool invocation
    pub arguments: serde_json::Value,
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    /// Input tokens used
    pub input_tokens: u32,
    /// Output tokens generated
    pub output_tokens: u32,
    /// Total tokens
    #[serde(default)]
    pub total_tokens: u32,
}

/// A chunk of a streaming response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Unique ID for this response
    pub id: String,
    /// The model used
    pub model: String,
    /// Delta content
    pub delta: Option<StreamDelta>,
    /// Usage statistics (usually only in final chunk)
    pub usage: Option<UsageStats>,
    /// Finish reason (only in final chunk)
    pub finish_reason: Option<String>,
    /// Whether this is the final chunk
    pub is_final: bool,
}

/// Delta content in a streaming response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDelta {
    /// The role (usually only in first chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// The incremental text content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Reasoning content from models like GLM-5 that show thinking before answering
    #[serde(skip_serializing_if = "Option::is_none", rename = "reasoning_content")]
    pub reasoning_content: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = ChatMessage::user("Hello!");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.as_text(), Some("Hello!"));
    }

    #[test]
    fn test_chat_request_builder() {
        let request = ChatRequest::new("gpt-4", vec![ChatMessage::user("Hi")]);
        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.messages.len(), 1);
    }

    #[test]
    fn test_llm_error_retryable() {
        assert!(LlmError::RateLimited {
            retry_after_seconds: None
        }
        .is_retryable());
        assert!(LlmError::Network("timeout".to_string()).is_retryable());
        assert!(!LlmError::AuthenticationFailed.is_retryable());
    }

    #[test]
    fn test_stream_delta_with_reasoning_content() {
        let json = r#"{"role":"assistant","reasoning_content":"Thinking..."}"#;
        let delta: StreamDelta = serde_json::from_str(json).expect("valid delta");
        assert_eq!(delta.role, Some("assistant".to_string()));
        assert_eq!(delta.content, None);
        assert_eq!(delta.reasoning_content, Some("Thinking...".to_string()));
    }

    #[test]
    fn test_stream_delta_with_content_and_reasoning() {
        let json = r#"{"content":"Response","reasoning_content":"Thought"}"#;
        let delta: StreamDelta = serde_json::from_str(json).expect("valid delta");
        assert_eq!(delta.content, Some("Response".to_string()));
        assert_eq!(delta.reasoning_content, Some("Thought".to_string()));
    }
}
