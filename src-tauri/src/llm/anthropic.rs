//! Anthropic Claude API client.
//!
//! This module implements the LLM client for Anthropic's Claude API.
//! Unlike OpenAI, Anthropic uses a different API structure with:
//! - `x-api-key` header for authentication
//! - `anthropic-version` header for API versioning
//! - System prompt as a top-level field
//! - Content blocks for multi-modal messages
//! - Different streaming event format

use async_stream::stream;
use futures::future::BoxFuture;
use futures::Stream;
use log::{error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

use super::{
    ChatRequest, ChatResponse, LlmClient, LlmError, LlmResult, Message, StreamChunk, StreamDelta,
    UsageStats,
};
use crate::nlp::config::ApiType;

// =============================================================================
// Anthropic API Types
// =============================================================================

/// Anthropic messages API request.
#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
    stream: bool,
}

/// Anthropic message format.
#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: AnthropicContent,
}

/// Anthropic content (can be string or array of blocks).
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum AnthropicContent {
    /// Simple text content
    Text(String),
    /// Multi-modal content blocks
    Blocks(Vec<AnthropicContentBlock>),
}

/// Anthropic content block.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicContentBlock {
    /// Text content
    Text { text: String },
    /// Image content
    Image { source: AnthropicImageSource },
}

/// Anthropic image source.
#[derive(Debug, Serialize)]
struct AnthropicImageSource {
    #[serde(rename = "type")]
    source_type: String,
    media_type: String,
    data: String,
}

/// Anthropic messages API response.
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    model: String,
    #[allow(dead_code)]
    role: String,
    content: Vec<AnthropicResponseContent>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
    #[serde(default)]
    stop_reason: Option<String>,
}

/// Anthropic response content block.
#[derive(Debug, Deserialize)]
struct AnthropicResponseContent {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    content_type: String,
    text: Option<String>,
}

/// Anthropic usage statistics.
#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

// =============================================================================
// Streaming Types
// =============================================================================

/// Anthropic streaming event.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)]
enum AnthropicStreamEvent {
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
struct AnthropicMessageStart {
    id: String,
    model: String,
    #[allow(dead_code)]
    role: String,
    #[allow(dead_code)]
    content: Vec<serde_json::Value>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

/// Content block start event data.
#[derive(Debug, Deserialize)]
struct AnthropicContentBlockStart {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    content_type: String,
    #[allow(dead_code)]
    text: Option<String>,
}

/// Content delta event data.
#[derive(Debug, Deserialize)]
struct AnthropicContentDelta {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    delta_type: String,
    text: Option<String>,
}

/// Message delta event data.
#[derive(Debug, Deserialize)]
struct AnthropicMessageDelta {
    stop_reason: Option<String>,
}

/// Delta usage statistics.
#[derive(Debug, Deserialize)]
struct AnthropicDeltaUsage {
    output_tokens: u32,
}

/// Anthropic error.
#[derive(Debug, Deserialize)]
struct AnthropicError {
    #[serde(rename = "type")]
    error_type: String,
    message: String,
}

// =============================================================================
// Client Implementation
// =============================================================================

/// Default base URL for Anthropic API.
const ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com/v1";

/// Anthropic API version header value.
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Default maximum tokens for requests.
const DEFAULT_MAX_TOKENS: u32 = 4096;

/// Anthropic Claude API client.
pub struct AnthropicClient {
    /// HTTP client
    client: Client,
    /// API key
    api_key: String,
    /// Base URL for API requests
    base_url: String,
}

impl AnthropicClient {
    /// Creates a new Anthropic client.
    #[must_use]
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        let client = Client::builder()
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key,
            base_url: base_url.unwrap_or_else(|| ANTHROPIC_BASE_URL.to_string()),
        }
    }

    /// Converts a ChatRequest to Anthropic format.
    fn convert_request(&self, request: ChatRequest) -> AnthropicRequest {
        let mut messages = Vec::new();
        let mut system_prompt = request.system.clone();

        // Convert messages - Anthropic doesn't use system role in messages
        for msg in request.messages {
            match msg.role {
                super::MessageRole::System => {
                    // System messages go to the top-level system field
                    if system_prompt.is_none() {
                        system_prompt = msg.as_text().map(|s| s.to_string());
                    }
                }
                super::MessageRole::User | super::MessageRole::Assistant => {
                    let content = match &msg.content {
                        super::MessageContent::Text(text) => AnthropicContent::Text(text.clone()),
                        super::MessageContent::Blocks(blocks) => {
                            let anthropic_blocks: Vec<AnthropicContentBlock> = blocks
                                .iter()
                                .filter_map(|b| match b {
                                    super::ContentBlock::Text { text } => {
                                        Some(AnthropicContentBlock::Text { text: text.clone() })
                                    }
                                    super::ContentBlock::Image { source } => {
                                        // Convert image source to Anthropic format
                                        if source.source_type == "base64" {
                                            Some(AnthropicContentBlock::Image {
                                                source: AnthropicImageSource {
                                                    source_type: "base64".to_string(),
                                                    media_type: source
                                                        .media_type
                                                        .clone()
                                                        .unwrap_or_else(|| "image/png".to_string()),
                                                    data: source.data.clone(),
                                                },
                                            })
                                        } else {
                                            // Anthropic doesn't support URL images directly
                                            // Would need to download and convert to base64
                                            None
                                        }
                                    }
                                })
                                .collect();
                            AnthropicContent::Blocks(anthropic_blocks)
                        }
                    };

                    messages.push(AnthropicMessage {
                        role: msg.role.as_str().to_string(),
                        content,
                    });
                }
            }
        }

        AnthropicRequest {
            model: request.model,
            messages,
            max_tokens: request.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
            system: system_prompt,
            temperature: request.temperature,
            top_p: request.top_p,
            stop_sequences: request.stop,
            stream: request.stream,
        }
    }

    /// Parses an SSE line from the Anthropic stream.
    fn parse_sse_line(line: &str) -> Option<Result<AnthropicStreamEvent, LlmError>> {
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

    /// Returns default models for when API fetch is not available.
    /// Returns GLM models for z.ai provider, Claude models for native Anthropic.
    fn get_default_models(&self) -> Vec<String> {
        // Check if this is a z.ai endpoint (GLM models)
        if self.base_url.contains("z.ai") {
            info!("anthropic::get_default_models: returning GLM models for z.ai provider");
            vec![
                "GLM-5".to_string(),
                "GLM-5-Code".to_string(),
                "GLM-4.7".to_string(),
                "GLM-4.7-FlashX".to_string(),
                "GLM-4.7-Flash".to_string(),
                "GLM-4.6".to_string(),
                "GLM-4.5".to_string(),
                "GLM-4.5-X".to_string(),
                "GLM-4.5-Air".to_string(),
                "GLM-4.5-AirX".to_string(),
                "GLM-4.5-Flash".to_string(),
                "GLM-4-32B-0414-128K".to_string(),
            ]
        } else {
            // Native Anthropic Claude models
            info!("anthropic::get_default_models: returning Claude models for Anthropic provider");
            vec![
                "claude-3-5-sonnet-20241022".to_string(),
                "claude-3-5-haiku-20241022".to_string(),
                "claude-3-opus-20240229".to_string(),
                "claude-3-sonnet-20240229".to_string(),
                "claude-3-haiku-20240307".to_string(),
            ]
        }
    }

    /// Makes a non-streaming chat request.
    async fn chat_internal(&self, request: ChatRequest) -> LlmResult<ChatResponse> {
        let anthropic_request = self.convert_request(request);

        let response = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("Content-Type", "application/json")
            .json(&anthropic_request)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LlmError::api(status.as_u16(), error_text));
        }

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| LlmError::parse(format!("Failed to parse response: {}", e)))?;

        // Extract text content from response
        let content = anthropic_response
            .content
            .iter()
            .filter_map(|c| c.text.clone())
            .collect::<Vec<_>>()
            .join("");

        let usage = anthropic_response.usage.map(|u| UsageStats {
            input_tokens: u.input_tokens,
            output_tokens: u.output_tokens,
            total_tokens: u.input_tokens + u.output_tokens,
        });

        Ok(ChatResponse {
            id: anthropic_response.id,
            model: anthropic_response.model,
            content,
            usage,
            finish_reason: anthropic_response.stop_reason,
        })
    }
}

impl LlmClient for AnthropicClient {
    fn provider_type(&self) -> ApiType {
        ApiType::Anthropic
    }

    fn chat<'a>(&'a self, request: ChatRequest) -> BoxFuture<'a, LlmResult<ChatResponse>> {
        Box::pin(async move { self.chat_internal(request).await })
    }

    fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Pin<Box<dyn Stream<Item = LlmResult<StreamChunk>> + Send>> {
        let client = self.client.clone();
        let base_url = self.base_url.clone();
        let api_key = self.api_key.clone();
        let anthropic_request = self.convert_request(request);
        let url = format!("{}/messages", base_url);

        info!("anthropic::chat_stream: starting request to {}", url);

        Box::pin(stream! {
            info!("anthropic::chat_stream: sending POST request");
            let response = client
                .post(&url)
                .header("x-api-key", &api_key)
                .header("anthropic-version", ANTHROPIC_VERSION)
                .header("Content-Type", "application/json")
                .json(&anthropic_request)
                .send()
                .await;

            let response = match response {
                Ok(r) => {
                    info!("anthropic::chat_stream: got response, status={}", r.status());
                    r
                }
                Err(e) => {
                    error!("anthropic::chat_stream: request failed: {}", e);
                    yield Err(LlmError::from(e));
                    return;
                }
            };

            let status = response.status();
            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                error!("anthropic::chat_stream: API error status={}, body={}", status, error_text);
                yield Err(LlmError::api(status.as_u16(), error_text));
                return;
            }

            info!("anthropic::chat_stream: starting to read stream");

            // Stream the response
            #[allow(clippy::expect_used)]
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();
            let mut message_id = String::new();
            let mut model = String::new();
            let mut input_tokens = 0u32;
            let mut output_tokens = 0u32;
            let mut final_stop_reason = None;
            let mut chunk_count = 0;
            let mut line_count = 0;

            while let Some(chunk_result) = futures::StreamExt::next(&mut stream).await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        error!("anthropic::chat_stream: error reading from stream: {}", e);
                        yield Err(LlmError::streaming(e.to_string()));
                        continue;
                    }
                };

                chunk_count += 1;

                // Convert bytes to string and append to buffer
                match String::from_utf8(chunk.to_vec()) {
                    Ok(text) => {
                        if chunk_count <= 5 {
                            info!("anthropic::chat_stream: received chunk #{}, length={} bytes", chunk_count, text.len());
                        }
                        buffer.push_str(&text);
                    }
                    Err(e) => {
                        error!("anthropic::chat_stream: failed to convert chunk to utf8: {}", e);
                        continue;
                    }
                }

                // Process complete lines
                while let Some(newline_pos) = buffer.find('\n') {
                    line_count += 1;
                    let line = buffer[..newline_pos].to_string();
                    buffer = buffer[newline_pos + 1..].to_string();

                    if line_count <= 10 {
                        info!("anthropic::chat_stream: processing line #{}, content={:?}", line_count, &line[..line.len().min(150)]);
                    }

                    if let Some(result) = Self::parse_sse_line(&line) {
                        match result {
                            Ok(event) => {
                                match event {
                                    AnthropicStreamEvent::MessageStart { message } => {
                                        message_id = message.id;
                                        model = message.model;
                                        if let Some(usage) = message.usage {
                                            input_tokens = usage.input_tokens;
                                        }
                                        info!("anthropic::chat_stream: message started, id={}, model={}", message_id, model);
                                    }
                                    AnthropicStreamEvent::ContentBlockStart { content_block, .. } => {
                                        info!("anthropic::chat_stream: content block started, type={}", content_block.content_type);
                                    }
                                    AnthropicStreamEvent::ContentBlockDelta { delta, .. } => {
                                        if let Some(text) = delta.text {
                                            info!("anthropic::chat_stream: yielding delta with {} chars", text.len());
                                            yield Ok(StreamChunk {
                                                id: message_id.clone(),
                                                model: model.clone(),
                                                delta: Some(StreamDelta {
                                                    role: None,
                                                    content: Some(text),
                                                }),
                                                usage: None,
                                                finish_reason: None,
                                                is_final: false,
                                            });
                                        }
                                    }
                                    AnthropicStreamEvent::ContentBlockStop { .. } => {
                                        info!("anthropic::chat_stream: content block stopped");
                                    }
                                    AnthropicStreamEvent::MessageDelta { delta, usage } => {
                                        final_stop_reason = delta.stop_reason;
                                        if let Some(u) = usage {
                                            output_tokens = u.output_tokens;
                                        }
                                        info!("anthropic::chat_stream: message delta received, stop_reason={:?}", final_stop_reason);
                                    }
                                    AnthropicStreamEvent::MessageStop => {
                                        info!("anthropic::chat_stream: message stopped, sending final chunk");
                                        // Send final chunk with usage
                                        yield Ok(StreamChunk {
                                            id: message_id.clone(),
                                            model: model.clone(),
                                            delta: None,
                                            usage: Some(UsageStats {
                                                input_tokens,
                                                output_tokens,
                                                total_tokens: input_tokens + output_tokens,
                                            }),
                                            finish_reason: final_stop_reason.clone(),
                                            is_final: true,
                                        });
                                    }
                                    AnthropicStreamEvent::Error { error } => {
                                        error!("anthropic::chat_stream: API error: {} - {}", error.error_type, error.message);
                                        yield Err(LlmError::api(400, error.message));
                                        return;
                                    }
                                }
                            }
                            Err(e) => {
                                error!("anthropic::chat_stream: failed to parse SSE line {}: {}", line_count, e);
                                yield Err(e);
                            }
                        }
                    }
                }
            }

            info!("anthropic::chat_stream: stream completed after {} chunks and {} lines", chunk_count, line_count);
        })
    }

    fn validate_api_key(&self) -> BoxFuture<'_, LlmResult<bool>> {
        Box::pin(async {
            // Make a minimal request to validate the API key
            let request = ChatRequest::new("claude-3-haiku-20240307", vec![Message::user("test")])
                .with_max_tokens(1);

            match self.chat_internal(request).await {
                Ok(_) => Ok(true),
                Err(e) if e.is_auth_error() => Ok(false),
                Err(e) => Err(e),
            }
        })
    }

    fn fetch_models(&self) -> BoxFuture<'_, LlmResult<Vec<String>>> {
        Box::pin(async {
            info!(
                "anthropic::fetch_models: starting request to {}/models",
                self.base_url
            );

            // Response types for OpenAI-compatible models endpoint
            #[derive(Debug, Deserialize)]
            struct ModelsResponse {
                data: Vec<ModelInfo>,
            }

            #[derive(Debug, Deserialize)]
            struct ModelInfo {
                id: String,
            }

            // Try to fetch models from the API (OpenAI-compatible endpoint)
            let response = self
                .client
                .get(format!("{}/models", self.base_url))
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", ANTHROPIC_VERSION)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    info!("anthropic::fetch_models: got response, status={}", status);

                    if status.is_success() {
                        match resp.json::<ModelsResponse>().await {
                            Ok(models_response) => {
                                let models: Vec<String> =
                                    models_response.data.into_iter().map(|m| m.id).collect();
                                info!(
                                    "anthropic::fetch_models: successfully fetched {} models: {:?}",
                                    models.len(),
                                    models
                                );
                                Ok(models)
                            }
                            Err(e) => {
                                error!(
                                    "anthropic::fetch_models: failed to parse models response: {}",
                                    e
                                );
                                // Return default models if parsing fails
                                let defaults = self.get_default_models();
                                info!(
                                    "anthropic::fetch_models: returning {} default models",
                                    defaults.len()
                                );
                                Ok(defaults)
                            }
                        }
                    } else {
                        info!("anthropic::fetch_models: API returned non-success status {}, returning default models", status);
                        // For native Anthropic API, return known models since they don't have a models endpoint
                        let defaults = self.get_default_models();
                        info!(
                            "anthropic::fetch_models: returning {} default models",
                            defaults.len()
                        );
                        Ok(defaults)
                    }
                }
                Err(e) => {
                    error!("anthropic::fetch_models: request failed: {}", e);
                    // Return default models if request fails
                    let defaults = self.get_default_models();
                    info!(
                        "anthropic::fetch_models: returning {} default models due to error",
                        defaults.len()
                    );
                    Ok(defaults)
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = AnthropicClient::new("test-key".to_string(), None);
        assert_eq!(client.provider_type(), ApiType::Anthropic);
        assert_eq!(client.base_url, ANTHROPIC_BASE_URL);
    }

    #[test]
    fn test_client_with_custom_url() {
        let client = AnthropicClient::new(
            "test-key".to_string(),
            Some("https://custom.api.com".to_string()),
        );
        assert_eq!(client.base_url, "https://custom.api.com");
    }

    #[test]
    fn test_convert_request() {
        let client = AnthropicClient::new("test-key".to_string(), None);
        let request = ChatRequest::new(
            "claude-3-opus-20240229",
            vec![Message::user("Hello"), Message::assistant("Hi there!")],
        )
        .with_system("Be helpful");

        let anthropic_request = client.convert_request(request);

        assert_eq!(anthropic_request.model, "claude-3-opus-20240229");
        assert_eq!(anthropic_request.system, Some("Be helpful".to_string()));
        assert_eq!(anthropic_request.messages.len(), 2);
        assert_eq!(anthropic_request.messages[0].role, "user");
        assert_eq!(anthropic_request.messages[1].role, "assistant");
    }

    #[test]
    fn test_parse_sse_line_empty() {
        assert!(AnthropicClient::parse_sse_line("").is_none());
        assert!(AnthropicClient::parse_sse_line(": comment").is_none());
        assert!(AnthropicClient::parse_sse_line("not data").is_none());
    }

    #[test]
    fn test_parse_sse_line_message_start() {
        let line = r#"data: {"type":"message_start","message":{"id":"msg_123","model":"claude-3-opus","role":"assistant","content":[]}}"#;
        let result = AnthropicClient::parse_sse_line(line);

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
        let result = AnthropicClient::parse_sse_line(line);

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
    fn test_fetch_models() {
        let client = AnthropicClient::new("test-key".to_string(), None);
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        // Since we're using a test key, the API call will fail and return default models
        let models = rt
            .block_on(client.fetch_models())
            .expect("Failed to fetch models");

        // Should return default models when API fails
        assert!(!models.is_empty());
        assert!(models.contains(&"claude-3-opus-20240229".to_string()));
        assert!(models.contains(&"claude-3-5-sonnet-20241022".to_string()));
    }

    #[test]
    fn test_get_default_models_claude() {
        let client = AnthropicClient::new("test-key".to_string(), None);
        let models = client.get_default_models();

        // Should return Claude models for native Anthropic
        assert!(!models.is_empty());
        assert!(models.contains(&"claude-3-opus-20240229".to_string()));
        assert!(models.contains(&"claude-3-5-sonnet-20241022".to_string()));
        assert!(models.contains(&"claude-3-5-haiku-20241022".to_string()));
    }

    #[test]
    fn test_get_default_models_glm() {
        let client = AnthropicClient::new(
            "test-key".to_string(),
            Some("https://api.z.ai/v1".to_string()),
        );
        let models = client.get_default_models();

        // Should return GLM models for z.ai provider
        assert!(!models.is_empty());
        assert!(models.contains(&"GLM-5".to_string()));
        assert!(models.contains(&"GLM-5-Code".to_string()));
        assert!(models.contains(&"GLM-4.7".to_string()));
        assert!(models.contains(&"GLM-4.7-Flash".to_string()));
        assert!(models.contains(&"GLM-4.5-Flash".to_string()));
    }
}
