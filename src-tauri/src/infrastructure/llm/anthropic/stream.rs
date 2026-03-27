//! Anthropic streaming types and SSE parsing.

use serde::Deserialize;
use std::sync::Arc;

use crate::core::llm::entities::{StreamChunk, StreamDelta, UsageStats};
use crate::infrastructure::llm::{LlmError, LlmResult};

use super::types::AnthropicUsage;

/// Anthropic streaming event.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)]
pub enum AnthropicStreamEvent {
    MessageStart {
        message: AnthropicMessageStart,
    },
    ContentBlockStart {
        index: u32,
        content_block: AnthropicContentBlockStart,
    },
    ContentBlockDelta {
        index: u32,
        delta: AnthropicContentDelta,
    },
    ContentBlockStop {
        index: u32,
    },
    MessageDelta {
        delta: AnthropicMessageDelta,
        usage: Option<AnthropicDeltaUsage>,
    },
    MessageStop,
    Error {
        error: AnthropicError,
    },
}

/// Message start event data.
#[derive(Debug, Deserialize)]
pub struct AnthropicMessageStart {
    pub id: String,
    pub model: String,
    #[allow(dead_code)]
    pub role: String,
    #[allow(dead_code)]
    pub content: Vec<serde_json::Value>,
    #[serde(default)]
    pub usage: Option<AnthropicUsage>,
}

/// Content block start event data.
#[derive(Debug, Deserialize)]
pub struct AnthropicContentBlockStart {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub content_type: String,
    #[allow(dead_code)]
    pub text: Option<String>,
}

/// Content delta event data.
#[derive(Debug, Deserialize)]
pub struct AnthropicContentDelta {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub delta_type: String,
    pub text: Option<String>,
}

/// Message delta event data.
#[derive(Debug, Deserialize)]
pub struct AnthropicMessageDelta {
    pub stop_reason: Option<String>,
}

/// Delta usage statistics.
#[derive(Debug, Deserialize)]
pub struct AnthropicDeltaUsage {
    pub output_tokens: u32,
}

/// Anthropic error.
#[derive(Debug, Deserialize)]
pub struct AnthropicError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

/// Parses an SSE line from the Anthropic stream.
pub fn parse_sse_line(line: &str) -> Option<Result<AnthropicStreamEvent, LlmError>> {
    let line = line.trim();

    // Skip empty lines and comments
    if line.is_empty() || line.starts_with(':') {
        return None;
    }

    // Parse data line
    let data = if let Some(d) = line.strip_prefix("data: ") {
        d
    } else {
        return None;
    };

    // Parse JSON
    match serde_json::from_str::<AnthropicStreamEvent>(data) {
        Ok(event) => Some(Ok(event)),
        Err(e) => Some(Err(LlmError::parse(format!(
            "Failed to parse stream event: {} | Data: {}",
            e, data
        )))),
    }
}

/// Streaming state for Anthropic chat stream.
#[derive(Debug, Default)]
pub struct StreamState {
    pub message_id: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
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
