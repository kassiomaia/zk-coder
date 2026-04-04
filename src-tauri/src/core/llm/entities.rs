//! Defines the core domain entities for Large Language Model (LLM) communication.
//!
//! These data structures are used to represent requests, responses, and other
//! concepts related to interacting with LLMs.

use serde::{Deserialize, Serialize};

/// A specialized `Result` type for LLM operations.
pub type LlmResult<T> = Result<T, LlmError>;

/// Represents errors that can occur during LLM operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmError {
    /// An error indicating that authentication with the LLM provider failed.
    AuthenticationFailed,
    /// An error indicating that the request was rate-limited.
    RateLimited {
        /// The number of seconds to wait before retrying, if available.
        retry_after_seconds: Option<u64>,
    },
    /// An error indicating that the request was invalid.
    InvalidRequest {
        /// A message describing the reason for the invalid request.
        message: String,
    },
    /// An error indicating a network-related issue.
    Network(String),
    /// An error reported by the LLM provider.
    Provider {
        /// The error code from the provider.
        code: String,
        /// The error message from the provider.
        message: String,
    },
    /// An error indicating that the request timed out.
    Timeout,
    /// An unknown or unexpected error.
    Unknown(String),
}

impl LlmError {
    /// Checks if the error is retryable.
    ///
    /// `RateLimited`, `Network`, and `Timeout` errors are considered retryable.
    #[must_use] 
    pub const fn is_retryable(&self) -> bool {
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

/// The role of a message in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// A system message that sets the behavior of the assistant.
    System,
    /// A message sent by the user.
    User,
    /// A message sent by the assistant.
    Assistant,
    /// A message containing the result of a tool call.
    Tool,
}

impl MessageRole {
    /// Converts the `MessageRole` to a string slice.
    #[must_use] 
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
        }
    }
}

/// A block of content within a multi-modal message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ContentBlock {
    /// A block of text content.
    Text {
        /// The text content.
        text: String,
    },
    /// A block of image content, which can be specified as a base64 string or a URL.
    Image {
        /// The source of the image.
        source: ImageSource,
    },
}

impl ContentBlock {
    /// Creates a new `ContentBlock` with text content.
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text {
            text: content.into(),
        }
    }

    /// Creates a new `ContentBlock` with an image from base64 data.
    pub fn image_base64(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Image {
            source: ImageSource {
                source_type: "base64".to_string(),
                media_type: Some(media_type.into()),
                data: data.into(),
            },
        }
    }

    /// Creates a new `ContentBlock` with an image from a URL.
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

/// The source of image content for a multi-modal message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageSource {
    /// The type of the source, either "base64" or "url".
    #[serde(rename = "type")]
    pub source_type: String,
    /// The media type of the image (e.g., "image/png").
    pub media_type: Option<String>,
    /// The image data as a base64 string or a URL.
    pub data: String,
}

/// The content of a message, which can be simple text or a series of multi-modal blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text content.
    Text(String),
    /// Multi-modal content composed of multiple blocks.
    Blocks(Vec<ContentBlock>),
}

/// A message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// The role of the message sender.
    pub role: MessageRole,
    /// The content of the message.
    pub content: MessageContent,
    /// A list of structured tool calls requested by the assistant.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    /// The result of a tool call, for messages with the `Tool` role.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<ToolResult>,
}

impl ChatMessage {
    /// Creates a new message with the `User` role and text content.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: MessageContent::Text(content.into()),
            tool_calls: Vec::new(),
            tool_result: None,
        }
    }

    /// Creates a new message with the `Assistant` role and text content.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: MessageContent::Text(content.into()),
            tool_calls: Vec::new(),
            tool_result: None,
        }
    }

    /// Creates a new message with the `System` role.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: MessageContent::Text(content.into()),
            tool_calls: Vec::new(),
            tool_result: None,
        }
    }

    /// Creates a new message with multi-modal content.
    #[must_use] 
    pub const fn multimodal(role: MessageRole, blocks: Vec<ContentBlock>) -> Self {
        Self {
            role,
            content: MessageContent::Blocks(blocks),
            tool_calls: Vec::new(),
            tool_result: None,
        }
    }

    /// Creates a new message with the `Assistant` role and structured tool calls.
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

    /// Creates a new message with the `Tool` role, containing the result of a tool call.
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

    /// Returns the text content of the message if it is a simple text message.
    #[must_use] 
    pub fn as_text(&self) -> Option<&str> {
        match &self.content {
            MessageContent::Text(text) => Some(text),
            MessageContent::Blocks(_) => None,
        }
    }
}

/// The structured result of a tool call.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolResult {
    /// The ID of the tool call this result corresponds to.
    pub call_id: String,
    /// The JSON payload of the tool's output.
    pub output: serde_json::Value,
    /// A flag indicating whether the result represents an error.
    #[serde(default)]
    pub is_error: bool,
}

/// A request to generate a chat completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// The model to be used for the completion.
    pub model: String,
    /// The list of messages in the conversation.
    pub messages: Vec<ChatMessage>,
    /// An optional system prompt to guide the model's behavior.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// The sampling temperature to use, between 0 and 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// The maximum number of tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// The top-p sampling value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// A list of stop sequences to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// A flag indicating whether to stream the response.
    #[serde(default)]
    pub stream: bool,
    /// A list of tool definitions available to the model.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDefinition>,
}

impl ChatRequest {
    /// Creates a new `ChatRequest` with the given model and messages.
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

    /// Sets the system prompt for the request.
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Sets the temperature for the request.
    #[must_use] 
    pub const fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Sets the maximum number of tokens for the request.
    #[must_use] 
    pub const fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Enables streaming mode for the request.
    #[must_use] 
    pub const fn with_stream(mut self) -> Self {
        self.stream = true;
        self
    }

    /// Sets the available tool definitions for the request.
    #[must_use] 
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = tools;
        self
    }
}

/// A provider-agnostic definition of a tool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolDefinition {
    /// The name of the tool.
    pub name: String,
    /// A short description of what the tool does.
    pub description: String,
    /// A JSON Schema object defining the tool's input arguments.
    pub input_schema: serde_json::Value,
}

/// A response from a chat completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// A unique ID for the response.
    pub id: String,
    /// The model that was used to generate the response.
    pub model: String,
    /// The generated content of the response.
    pub content: String,
    /// Token usage statistics for the response.
    pub usage: Option<UsageStats>,
    /// The reason why the model finished generating the response.
    pub finish_reason: Option<String>,
    /// A list of structured tool calls requested by the model.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
}

/// A provider-agnostic representation of a tool call.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCall {
    /// A unique identifier for the tool call.
    pub id: String,
    /// The name of the tool to be called.
    pub name: String,
    /// The JSON arguments for the tool invocation.
    pub arguments: serde_json::Value,
}

/// Token usage statistics for a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    /// The number of input tokens used.
    pub input_tokens: u32,
    /// The number of output tokens generated.
    pub output_tokens: u32,
    /// The total number of tokens used.
    #[serde(default)]
    pub total_tokens: u32,
}

/// A chunk of a streaming response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// A unique ID for the response this chunk belongs to.
    pub id: String,
    /// The model that is generating the response.
    pub model: String,
    /// The delta content of this chunk.
    pub delta: Option<StreamDelta>,
    /// Token usage statistics, typically included in the final chunk.
    pub usage: Option<UsageStats>,
    /// The finish reason, included in the final chunk.
    pub finish_reason: Option<String>,
    /// A flag indicating whether this is the final chunk of the stream.
    pub is_final: bool,
}

/// The delta content in a streaming response chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDelta {
    /// The role of the sender, typically included in the first chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// The incremental text content of this chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// The reasoning content from models that show their thinking process.
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
