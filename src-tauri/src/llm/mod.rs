//! LLM (Large Language Model) integration module.
//!
//! This module provides a unified interface for communicating with various LLM providers
//! including OpenAI, Anthropic, and Google. It supports both streaming and non-streaming
//! responses, with real-time events emitted to the frontend via Tauri's event system.
//!
//! # Architecture
//!
//! The module is organized around the [`LlmClient`] trait, which defines the common
//! interface for all LLM providers. Each provider implements this trait with their
//! specific API details:
//!
//! - [`openai`] - OpenAI and OpenAI-compatible APIs
//! - [`anthropic`] - Anthropic Claude API (native implementation)
//!
//! # Example
//!
//! ```ignore
//! use coder_lib::llm::{LlmClient, ChatRequest, Message};
//!
//! let client = OpenAiClient::new(api_key, base_url);
//! let request = ChatRequest {
//!     model: "gpt-4".to_string(),
//!     messages: vec![Message::user("Hello!")],
//!     ..Default::default()
//! };
//!
//! let response = client.chat(request).await?;
//! ```

pub mod anthropic;
pub mod error;
pub mod openai;
pub mod streaming;

use futures::future::BoxFuture;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

pub use error::{LlmError, LlmResult};

use crate::nlp::config::ApiType;

// =============================================================================
// Message Types
// =============================================================================

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

impl From<crate::core::chat::entities::MessageRole> for MessageRole {
    fn from(role: crate::core::chat::entities::MessageRole) -> Self {
        match role {
            crate::core::chat::entities::MessageRole::User => Self::User,
            crate::core::chat::entities::MessageRole::Assistant => Self::Assistant,
            crate::core::chat::entities::MessageRole::System => Self::System,
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

impl ContentBlock {
    /// Creates a text content block.
    #[must_use]
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text {
            text: content.into(),
        }
    }

    /// Creates an image content block from base64 data.
    #[must_use]
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
    #[must_use]
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

/// A message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message sender
    pub role: MessageRole,
    /// Content of the message (can be simple text or multi-modal)
    pub content: MessageContent,
    /// Structured tool calls emitted by an assistant message.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    /// Structured tool result payload (for tool role messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<ToolResult>,
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

impl Message {
    /// Creates a new user message with text content.
    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: MessageContent::Text(content.into()),
            tool_calls: Vec::new(),
            tool_result: None,
        }
    }

    /// Creates a new assistant message with text content.
    #[must_use]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: MessageContent::Text(content.into()),
            tool_calls: Vec::new(),
            tool_result: None,
        }
    }

    /// Creates a new system message.
    #[must_use]
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: MessageContent::Text(content.into()),
            tool_calls: Vec::new(),
            tool_result: None,
        }
    }

    /// Creates a message with multi-modal content.
    #[must_use]
    pub fn multimodal(role: MessageRole, blocks: Vec<ContentBlock>) -> Self {
        Self {
            role,
            content: MessageContent::Blocks(blocks),
            tool_calls: Vec::new(),
            tool_result: None,
        }
    }

    /// Creates an assistant message that carries structured tool calls.
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match &self.content {
            MessageContent::Text(text) => Some(text),
            MessageContent::Blocks(_) => None,
        }
    }
}

/// Structured tool result payload for tool-role messages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolResult {
    /// Tool call identifier this result belongs to.
    pub call_id: String,
    /// Tool output JSON payload.
    pub output: serde_json::Value,
    /// Whether the result represents an error.
    #[serde(default)]
    pub is_error: bool,
}

// =============================================================================
// Request/Response Types
// =============================================================================

/// Request to generate a chat completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// The model to use
    pub model: String,
    /// The conversation messages
    pub messages: Vec<Message>,
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
    /// Tool definitions available to the model for this request.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDefinition>,
}

impl ChatRequest {
    /// Creates a new chat request with the given model and messages.
    #[must_use]
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
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
    #[must_use]
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Sets the temperature.
    #[must_use]
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Sets the maximum tokens.
    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Enables streaming mode.
    #[must_use]
    pub fn with_stream(mut self) -> Self {
        self.stream = true;
        self
    }

    /// Sets available tool definitions.
    #[must_use]
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = tools;
        self
    }
}

/// Provider-agnostic tool definition for request payloads.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolDefinition {
    /// Tool name.
    pub name: String,
    /// Short tool description.
    pub description: String,
    /// JSON Schema for tool arguments.
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
    /// Finish reason (e.g., "stop", "length")
    pub finish_reason: Option<String>,
    /// Structured tool calls requested by the model for this turn.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
}

/// Provider-agnostic tool call representation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    /// Unique call identifier emitted by the model/provider.
    pub id: String,
    /// Tool name.
    pub name: String,
    /// JSON arguments for the tool invocation.
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

// =============================================================================
// Streaming Types
// =============================================================================

/// A chunk of a streaming response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Unique ID for this response
    pub id: String,
    /// The model used
    pub model: String,
    /// Delta content (incremental text)
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
}

// =============================================================================
// Event Types (for Tauri events)
// =============================================================================

/// Event emitted during LLM streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LlmEvent {
    /// Streaming started
    Started {
        /// Request ID
        request_id: String,
        /// Model being used
        model: String,
    },
    /// Content chunk received
    Chunk {
        /// Request ID
        request_id: String,
        /// Delta content
        delta: String,
    },
    /// Streaming completed
    Complete {
        /// Request ID
        request_id: String,
        /// Full response content
        content: String,
        /// Usage statistics
        usage: Option<UsageStats>,
        /// Finish reason
        finish_reason: Option<String>,
    },
    /// Error occurred
    Error {
        /// Request ID
        request_id: String,
        /// Error message
        error: String,
        /// Whether the error is retryable
        retryable: bool,
    },
}

// =============================================================================
// Client Trait
// =============================================================================

/// Trait for LLM API clients.
///
/// This trait defines the common interface for all LLM providers.
/// Implementations handle the specific API details for each provider.
pub trait LlmClient: Send + Sync {
    /// Gets the provider type for this client.
    fn provider_type(&self) -> ApiType;

    /// Sends a chat completion request and returns the response.
    fn chat<'a>(&'a self, request: ChatRequest) -> BoxFuture<'a, LlmResult<ChatResponse>>;

    /// Sends a streaming chat completion request.
    /// Returns a stream of chunks.
    fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Pin<Box<dyn Stream<Item = LlmResult<StreamChunk>> + Send>>;

    /// Validates the API key by making a test request.
    fn validate_api_key(&self) -> BoxFuture<'_, LlmResult<bool>>;

    /// Fetches available models from the provider.
    fn fetch_models(&self) -> BoxFuture<'_, LlmResult<Vec<String>>>;
}

// =============================================================================
// Client Factory
// =============================================================================

/// Creates an LLM client for the given provider type.
#[must_use]
pub fn create_client(
    api_type: ApiType,
    api_key: String,
    base_url: Option<String>,
) -> Box<dyn LlmClient> {
    match api_type {
        ApiType::OpenAI => Box::new(openai::OpenAiClient::new(api_key, base_url)),
        ApiType::Google => Box::new(openai::OpenAiClient::new_google(api_key)),
        ApiType::Anthropic => Box::new(anthropic::AnthropicClient::new(api_key, base_url)),
    }
}

// =============================================================================
// Streaming Support
// =============================================================================

// Re-export streaming types and functions
pub use streaming::{stream_to_events, BatchingConfig, ChunkBatcher};

// =============================================================================
// Tests
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello!");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.as_text(), Some("Hello!"));
        assert!(msg.tool_calls.is_empty());
        assert!(msg.tool_result.is_none());
    }

    #[test]
    fn test_message_system() {
        let msg = Message::system("You are a helpful assistant.");
        assert_eq!(msg.role, MessageRole::System);
        assert_eq!(msg.as_text(), Some("You are a helpful assistant."));
        assert!(msg.tool_calls.is_empty());
        assert!(msg.tool_result.is_none());
    }

    #[test]
    fn test_message_tool_result_creation() {
        let msg = Message::tool_result("call_1", serde_json::json!({"ok": true}), false);
        assert_eq!(msg.role, MessageRole::Tool);
        assert!(msg.tool_calls.is_empty());
        let result = msg.tool_result.expect("tool result should exist");
        assert_eq!(result.call_id, "call_1");
        assert_eq!(result.output, serde_json::json!({"ok": true}));
        assert!(!result.is_error);
    }

    #[test]
    fn test_chat_request_builder() {
        let request = ChatRequest::new("gpt-4", vec![Message::user("Hi")])
            .with_temperature(0.7)
            .with_max_tokens(100)
            .with_stream();

        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(100));
        assert!(request.stream);
    }

    #[test]
    fn test_chat_request_with_tools() {
        let request = ChatRequest::new("gpt-4.1", vec![Message::user("Hi")]).with_tools(vec![
            ToolDefinition {
                name: "read_workspace_file".to_string(),
                description: "Read file content".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"}
                    },
                    "required": ["path"]
                }),
            },
        ]);

        assert_eq!(request.tools.len(), 1);
        assert_eq!(request.tools[0].name, "read_workspace_file");
    }

    #[test]
    fn test_content_block_text() {
        let block = ContentBlock::text("Hello");
        match block {
            ContentBlock::Text { text } => assert_eq!(text, "Hello"),
            _ => panic!("Expected text block"),
        }
    }

    #[test]
    fn test_llm_event_serialization() {
        let event = LlmEvent::Chunk {
            request_id: "test-123".to_string(),
            delta: "Hello".to_string(),
        };
        let json = serde_json::to_string(&event).expect("Failed to serialize");
        assert!(json.contains("chunk"));
        assert!(json.contains("test-123"));
    }
}
