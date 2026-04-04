//! Anthropic streaming types and SSE parsing.

use serde::Deserialize;

use crate::core::llm::entities::{StreamChunk, StreamDelta, UsageStats};
use crate::infrastructure::llm::LlmError;

use super::types::AnthropicUsage;

/// Anthropic streaming event.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)]
pub enum AnthropicStreamEvent {
    /// Message start event - contains the initial message metadata.
    MessageStart {
        /// The message start data.
        message: AnthropicMessageStart,
    },
    /// Content block start event - signals the beginning of a content block.
    ContentBlockStart {
        /// Index of the content block.
        index: u32,
        /// The content block start data.
        content_block: AnthropicContentBlockStart,
    },
    /// Content block delta event - contains incremental text updates.
    ContentBlockDelta {
        /// Index of the content block.
        index: u32,
        /// The content delta data.
        delta: AnthropicContentDelta,
    },
    /// Content block stop event - signals the end of a content block.
    ContentBlockStop {
        /// Index of the content block.
        index: u32,
    },
    /// Message delta event - contains final message metadata and usage.
    MessageDelta {
        /// The message delta data.
        delta: AnthropicMessageDelta,
        /// Usage statistics for the delta.
        usage: Option<AnthropicDeltaUsage>,
    },
    /// Message stop event - signals the end of the streaming response.
    MessageStop,
    /// Error event - contains error information from the API.
    Error {
        /// The error data.
        error: AnthropicError,
    },
}

/// Message start event data.
#[derive(Debug, Deserialize)]
pub struct AnthropicMessageStart {
    /// Unique message identifier.
    pub id: String,
    /// Model used for the request.
    pub model: String,
    /// Role of the message sender.
    #[allow(dead_code)]
    pub role: String,
    /// Content of the message.
    #[allow(dead_code)]
    pub content: Vec<serde_json::Value>,
    /// Usage statistics for the request.
    #[serde(default)]
    pub usage: Option<AnthropicUsage>,
}

/// Content block start event data.
#[derive(Debug, Deserialize)]
pub struct AnthropicContentBlockStart {
    /// Type of the content block.
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub content_type: String,
    /// Text content, if applicable.
    #[allow(dead_code)]
    pub text: Option<String>,
}

/// Content delta event data.
#[derive(Debug, Deserialize)]
pub struct AnthropicContentDelta {
    /// Type of the delta.
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub delta_type: String,
    /// Text content delta.
    pub text: Option<String>,
}

/// Message delta event data.
#[derive(Debug, Deserialize)]
pub struct AnthropicMessageDelta {
    /// Reason why the message stopped.
    pub stop_reason: Option<String>,
}

/// Delta usage statistics.
#[derive(Debug, Deserialize)]
pub struct AnthropicDeltaUsage {
    /// Number of output tokens in this delta.
    pub output_tokens: u32,
}

/// Anthropic error.
#[derive(Debug, Deserialize)]
pub struct AnthropicError {
    /// Type of the error.
    #[serde(rename = "type")]
    pub error_type: String,
    /// Error message.
    pub message: String,
}

/// Parses an SSE line from the Anthropic stream.
#[must_use] 
pub fn parse_sse_line(line: &str) -> Option<Result<AnthropicStreamEvent, LlmError>> {
    let line = line.trim();

    // Skip empty lines and comments
    if line.is_empty() || line.starts_with(':') {
        return None;
    }

    // Parse data line
    let data = line.strip_prefix("data: ")?;

    // Parse JSON
    match serde_json::from_str::<AnthropicStreamEvent>(data) {
        Ok(event) => Some(Ok(event)),
        Err(e) => Some(Err(LlmError::parse(format!(
            "Failed to parse stream event: {e} | Data: {data}"
        )))),
    }
}

/// Streaming state for Anthropic chat stream.
#[derive(Debug, Default)]
pub struct StreamState {
    /// The message ID.
    pub message_id: String,
    /// The model being used.
    pub model: String,
    /// Number of input tokens.
    pub input_tokens: u32,
    /// Number of output tokens.
    pub output_tokens: u32,
    /// The final stop reason.
    pub final_stop_reason: Option<String>,
}

impl StreamState {
    /// Handle a stream event and return a chunk if applicable.
    pub fn handle_event(
        &mut self,
        event: AnthropicStreamEvent,
    ) -> Option<Result<StreamChunk, LlmError>> {
        match event {
            AnthropicStreamEvent::MessageStart { message } => {
                self.message_id = message.id;
                self.model = message.model;
                if let Some(usage) = message.usage {
                    self.input_tokens = usage.input_tokens;
                }
                None
            }
            AnthropicStreamEvent::ContentBlockStart { content_block, .. } => {
                log::info!(
                    "anthropic::chat_stream: content block started, type={}",
                    content_block.content_type
                );
                None
            }
            AnthropicStreamEvent::ContentBlockDelta { delta, .. } => {
                if let Some(text) = delta.text {
                    log::info!(
                        "anthropic::chat_stream: yielding delta with {} chars",
                        text.len()
                    );
                    Some(Ok(StreamChunk {
                        id: self.message_id.clone(),
                        model: self.model.clone(),
                        delta: Some(StreamDelta {
                            role: None,
                            content: Some(text),
                            reasoning_content: None,
                        }),
                        usage: None,
                        finish_reason: None,
                        is_final: false,
                    }))
                } else {
                    None
                }
            }
            AnthropicStreamEvent::ContentBlockStop { .. } => {
                log::info!("anthropic::chat_stream: content block stopped");
                None
            }
            AnthropicStreamEvent::MessageDelta { delta, usage } => {
                self.final_stop_reason = delta.stop_reason;
                if let Some(u) = usage {
                    self.output_tokens = u.output_tokens;
                }
                log::info!(
                    "anthropic::chat_stream: message delta received, stop_reason={:?}",
                    self.final_stop_reason
                );
                None
            }
            AnthropicStreamEvent::MessageStop => {
                log::info!("anthropic::chat_stream: message stopped, sending final chunk");
                // Send final chunk with usage
                Some(Ok(StreamChunk {
                    id: self.message_id.clone(),
                    model: self.model.clone(),
                    delta: None,
                    usage: Some(UsageStats {
                        input_tokens: self.input_tokens,
                        output_tokens: self.output_tokens,
                        total_tokens: self.input_tokens + self.output_tokens,
                    }),
                    finish_reason: self.final_stop_reason.clone(),
                    is_final: true,
                }))
            }
            AnthropicStreamEvent::Error { error } => {
                log::error!(
                    "anthropic::chat_stream: API error: {} - {}",
                    error.error_type,
                    error.message
                );
                Some(Err(LlmError::api(400, error.message)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_line_empty() {
        assert!(parse_sse_line("").is_none());
        assert!(parse_sse_line(": comment").is_none());
        assert!(parse_sse_line("not data").is_none());
    }

    #[test]
    fn test_parse_sse_line_message_start() {
        let line = r#"data: {"type":"message_start","message":{"id":"msg_123","model":"claude-3-opus","role":"assistant","content":[]}}"#;
        let result = parse_sse_line(line);

        assert!(result.is_some());
        let parsed = result.unwrap();
        assert!(parsed.is_ok());

        let event = parsed.unwrap();
        match event {
            AnthropicStreamEvent::MessageStart { message } => {
                assert_eq!(message.id, "msg_123");
                assert_eq!(message.model, "claude-3-opus");
            }
            _ => panic!("Expected MessageStart event"),
        }
    }

    #[test]
    fn test_parse_sse_line_content_delta() {
        let line = r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
        let result = parse_sse_line(line);

        assert!(result.is_some());
        let parsed = result.unwrap();
        assert!(parsed.is_ok());

        let event = parsed.unwrap();
        match event {
            AnthropicStreamEvent::ContentBlockDelta { delta, .. } => {
                assert_eq!(delta.text, Some("Hello".to_string()));
            }
            _ => panic!("Expected ContentBlockDelta event"),
        }
    }

    #[test]
    fn test_stream_state_message_start() {
        let mut state = StreamState::default();
        let event = AnthropicStreamEvent::MessageStart {
            message: AnthropicMessageStart {
                id: "msg_123".to_string(),
                model: "claude-3".to_string(),
                role: "assistant".to_string(),
                content: vec![],
                usage: Some(AnthropicUsage {
                    input_tokens: 100,
                    output_tokens: 0,
                }),
            },
        };

        let chunk = state.handle_event(event);
        assert!(chunk.is_none());
        assert_eq!(state.message_id, "msg_123");
        assert_eq!(state.model, "claude-3");
        assert_eq!(state.input_tokens, 100);
    }

    #[test]
    fn test_stream_state_content_delta() {
        let mut state = StreamState {
            message_id: "msg_123".to_string(),
            model: "claude-3".to_string(),
            ..Default::default()
        };

        let event = AnthropicStreamEvent::ContentBlockDelta {
            index: 0,
            delta: AnthropicContentDelta {
                delta_type: "text_delta".to_string(),
                text: Some("Hello".to_string()),
            },
        };

        let chunk = state.handle_event(event);
        assert!(chunk.is_some());
        let chunk = chunk.unwrap().unwrap();
        assert_eq!(chunk.id, "msg_123");
        assert_eq!(
            chunk.delta.as_ref().unwrap().content.as_ref().unwrap(),
            "Hello"
        );
    }

    #[test]
    fn test_stream_state_message_stop() {
        let mut state = StreamState {
            message_id: "msg_123".to_string(),
            model: "claude-3".to_string(),
            input_tokens: 100,
            output_tokens: 50,
            ..Default::default()
        };

        let event = AnthropicStreamEvent::MessageStop;
        let chunk = state.handle_event(event);

        assert!(chunk.is_some());
        let chunk = chunk.unwrap().unwrap();
        assert!(chunk.is_final);
        assert_eq!(chunk.usage.unwrap().total_tokens, 150);
    }
}
