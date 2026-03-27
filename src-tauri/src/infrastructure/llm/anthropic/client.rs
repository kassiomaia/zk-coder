//! Anthropic Claude API client implementation.

use async_stream::stream;
use futures::Stream;
use log::{error, info, warn};
use reqwest::Client;
use serde::Deserialize;
use std::pin::Pin;

use super::stream::{parse_sse_line, AnthropicStreamEvent, StreamState};
use super::types::*;
use crate::core::config::ApiType;
use crate::infrastructure::llm::{
    ChatRequest, ChatResponse, LlmClient, LlmError, LlmResult, Message, MessageContent,
    MessageRole, StreamChunk, ToolCall, ToolDefinition, UsageStats,
};

/// Default base URL for Anthropic API.
pub const ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com/v1";

/// Anthropic API version header value.
pub const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Default maximum tokens for requests.
pub const DEFAULT_MAX_TOKENS: u32 = 4096;

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

    /// Returns default models for when API fetch is not available.
    /// Returns GLM models for z.ai provider, Claude models for native Anthropic.
    pub fn get_default_models(&self) -> Vec<String> {
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

    /// Converts a ChatRequest to Anthropic format.
    fn convert_request(&self, request: ChatRequest) -> AnthropicRequest {
        self.convert_request_with_tool_choice(request, true)
    }

    fn convert_request_with_tool_choice(
        &self,
        request: ChatRequest,
        include_tool_choice: bool,
    ) -> AnthropicRequest {
        let mut messages = Vec::new();
        let mut system_prompt = request.system.clone();

        // Convert messages - Anthropic doesn't use system role in messages
        for msg in request.messages {
            match msg.role {
                MessageRole::System => {
                    // System messages go to the top-level system field
                    if system_prompt.is_none() {
                        system_prompt = msg.as_text().map(|s| s.to_string());
                    }
                }
                MessageRole::Tool => {
                    let Some(tool_result) = &msg.tool_result else {
                        continue;
                    };
                    let output = serde_json::to_string(&tool_result.output)
                        .unwrap_or_else(|_| tool_result.output.to_string());
                    messages.push(AnthropicMessage {
                        role: "user".to_string(),
                        content: AnthropicContent::Blocks(vec![
                            AnthropicContentBlock::ToolResult {
                                tool_use_id: tool_result.call_id.clone(),
                                content: output,
                                is_error: tool_result.is_error,
                            },
                        ]),
                    });
                }
                MessageRole::User | MessageRole::Assistant => {
                    let text_or_blocks = match &msg.content {
                        MessageContent::Text(text) => AnthropicContent::Text(text.clone()),
                        MessageContent::Blocks(blocks) => {
                            let anthropic_blocks: Vec<AnthropicContentBlock> = blocks
                                .iter()
                                .filter_map(|b| match b {
                                    crate::core::llm::entities::ContentBlock::Text { text } => {
                                        Some(AnthropicContentBlock::Text { text: text.clone() })
                                    }
                                    crate::core::llm::entities::ContentBlock::Image { source } => {
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
                                            None
                                        }
                                    }
                                })
                                .collect();
                            AnthropicContent::Blocks(anthropic_blocks)
                        }
                    };

                    if msg.role == MessageRole::Assistant && !msg.tool_calls.is_empty() {
                        let mut blocks = match text_or_blocks {
                            AnthropicContent::Text(text) => {
                                if text.is_empty() {
                                    Vec::new()
                                } else {
                                    vec![AnthropicContentBlock::Text { text }]
                                }
                            }
                            AnthropicContent::Blocks(existing) => existing,
                        };
                        blocks.extend(msg.tool_calls.iter().map(|call| {
                            AnthropicContentBlock::ToolUse {
                                id: call.id.clone(),
                                name: call.name.clone(),
                                input: call.arguments.clone(),
                            }
                        }));

                        messages.push(AnthropicMessage {
                            role: "assistant".to_string(),
                            content: AnthropicContent::Blocks(blocks),
                        });
                    } else {
                        messages.push(AnthropicMessage {
                            role: msg.role.as_str().to_string(),
                            content: text_or_blocks,
                        });
                    }
                }
            }
        }

        let tools = super::convert_tools(request.tools);
        let tool_choice = if include_tool_choice {
            tools.as_ref().map(|_| AnthropicToolChoice {
                choice_type: "auto".to_string(),
            })
        } else {
            None
        };

        AnthropicRequest {
            model: request.model,
            messages,
            max_tokens: request.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
            system: system_prompt,
            temperature: request.temperature,
            top_p: request.top_p,
            stop_sequences: request.stop,
            stream: request.stream,
            tools,
            tool_choice,
        }
    }

    /// Makes a non-streaming chat request.
    pub async fn chat_internal(&self, request: ChatRequest) -> LlmResult<ChatResponse> {
        let mut include_tool_choice = true;
        let can_retry_without_tool_choice = !request.tools.is_empty();
        let anthropic_response: AnthropicResponse = loop {
            let anthropic_request =
                self.convert_request_with_tool_choice(request.clone(), include_tool_choice);

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

                if include_tool_choice && can_retry_without_tool_choice && status.is_server_error()
                {
                    warn!(
                        "anthropic::chat_internal: tool-enabled request failed with status {}; retrying once without tool_choice",
                        status.as_u16()
                    );
                    include_tool_choice = false;
                    continue;
                }

                return Err(LlmError::api(status.as_u16(), error_text));
            }

            break response
                .json()
                .await
                .map_err(|e| LlmError::parse(format!("Failed to parse response: {}", e)))?;
        };

        let AnthropicResponse {
            id,
            model,
            content: response_blocks,
            usage,
            stop_reason,
            ..
        } = anthropic_response;

        // Extract text content and structured tool calls from response blocks.
        let mut text_parts = Vec::new();
        let mut tool_calls = Vec::new();
        for block in response_blocks {
            match block.content_type.as_str() {
                "text" => {
                    if let Some(text) = block.text {
                        text_parts.push(text);
                    }
                }
                "tool_use" => {
                    if let Some(name) = block.name {
                        tool_calls.push(ToolCall {
                            id: block.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                            name,
                            arguments: block.input.unwrap_or_else(|| serde_json::json!({})),
                        });
                    }
                }
                _ => {}
            }
        }
        let content = text_parts.join("");

        let usage = usage.map(|u| UsageStats {
            input_tokens: u.input_tokens,
            output_tokens: u.output_tokens,
            total_tokens: u.input_tokens + u.output_tokens,
        });

        Ok(ChatResponse {
            id,
            model,
            content,
            usage,
            finish_reason: stop_reason,
            tool_calls,
        })
    }

    /// Makes a streaming chat request.
    pub fn chat_stream_internal(
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
                let error_text =
                    response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                error!(
                    "anthropic::chat_stream: API error status={}, body={}",
                    status, error_text
                );
                yield Err(LlmError::api(status.as_u16(), error_text));
                return;
            }

            info!("anthropic::chat_stream: starting to read stream");

            // Stream the response
            #[allow(clippy::expect_used)]
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();
            let mut state = StreamState::default();
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
                            info!(
                                "anthropic::chat_stream: received chunk #{}, length={} bytes",
                                chunk_count,
                                text.len()
                            );
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
                        info!(
                            "anthropic::chat_stream: processing line #{}, content={:?}",
                            line_count,
                            &line[..line.len().min(150)]
                        );
                    }

                    if let Some(result) = parse_sse_line(&line) {
                        match result {
                            Ok(event) => {
                                if let Some(chunk) = state.handle_event(event) {
                                    yield chunk;
                                }
                            }
                            Err(e) => {
                                error!(
                                    "anthropic::chat_stream: failed to parse SSE line {}: {}",
                                    line_count, e
                                );
                                yield Err(e);
                            }
                        }
                    }
                }
            }

            info!(
                "anthropic::chat_stream: stream completed after {} chunks and {} lines",
                chunk_count, line_count
            );
        })
    }

    /// Validates the API key by making a minimal request.
    pub async fn validate_api_key(&self) -> LlmResult<bool> {
        let request = ChatRequest::new("claude-3-haiku-20240307", vec![Message::user("test")])
            .with_max_tokens(1);

        match self.chat_internal(request).await {
            Ok(_) => Ok(true),
            Err(e) if e.is_auth_error() => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Fetches available models from the API.
    pub async fn fetch_models(&self) -> LlmResult<Vec<String>> {
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
    }
}

impl LlmClient for AnthropicClient {
    fn provider_type(&self) -> ApiType {
        ApiType::Anthropic
    }

    fn chat<'a>(
        &'a self,
        request: ChatRequest,
    ) -> futures::future::BoxFuture<'a, LlmResult<ChatResponse>> {
        Box::pin(async move { self.chat_internal(request).await })
    }

    fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Pin<Box<dyn Stream<Item = LlmResult<StreamChunk>> + Send>> {
        self.chat_stream_internal(request)
    }

    fn validate_api_key(&self) -> futures::future::BoxFuture<'_, LlmResult<bool>> {
        Box::pin(self.validate_api_key())
    }

    fn fetch_models(&self) -> futures::future::BoxFuture<'_, LlmResult<Vec<String>>> {
        Box::pin(self.fetch_models())
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
        assert!(anthropic_request.tools.is_none());
        assert!(anthropic_request.tool_choice.is_none());
    }

    #[test]
    fn test_convert_request_with_tools() {
        let client = AnthropicClient::new("test-key".to_string(), None);
        let request = ChatRequest::new("claude-3-5-sonnet-20241022", vec![Message::user("Hello")])
            .with_tools(vec![ToolDefinition {
                name: "list_workspace_files".to_string(),
                description: "List files in workspace".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"}
                    }
                }),
            }]);

        let anthropic_request = client.convert_request(request);
        let tools = anthropic_request.tools.expect("tools should be present");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "list_workspace_files");
        assert_eq!(
            anthropic_request
                .tool_choice
                .as_ref()
                .map(|choice| choice.choice_type.as_str()),
            Some("auto")
        );
    }

    #[test]
    fn test_convert_request_with_tools_without_tool_choice() {
        let client = AnthropicClient::new("test-key".to_string(), None);
        let request = ChatRequest::new("claude-3-5-sonnet-20241022", vec![Message::user("Hello")])
            .with_tools(vec![ToolDefinition {
                name: "list_workspace_files".to_string(),
                description: "List files in workspace".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"}
                    }
                }),
            }]);

        let anthropic_request = client.convert_request_with_tool_choice(request, false);
        assert!(anthropic_request.tools.is_some());
        assert!(anthropic_request.tool_choice.is_none());
    }

    #[test]
    fn test_convert_request_with_tool_messages() {
        let client = AnthropicClient::new("test-key".to_string(), None);
        let request = ChatRequest::new(
            "claude-3-5-sonnet-20241022",
            vec![
                Message::assistant_with_tool_calls(
                    "",
                    vec![ToolCall {
                        id: "toolu_1".to_string(),
                        name: "search_code".to_string(),
                        arguments: serde_json::json!({"query": "runner"}),
                    }],
                ),
                Message::tool_result("toolu_1", serde_json::json!({"ok": true}), false),
            ],
        );

        let anthropic_request = client.convert_request(request);
        assert_eq!(anthropic_request.messages.len(), 2);
        assert_eq!(anthropic_request.messages[0].role, "assistant");
        assert_eq!(anthropic_request.messages[1].role, "user");
        match &anthropic_request.messages[0].content {
            AnthropicContent::Blocks(blocks) => {
                assert!(matches!(blocks[0], AnthropicContentBlock::ToolUse { .. }));
            }
            _ => panic!("assistant tool call should be sent as blocks"),
        }
        match &anthropic_request.messages[1].content {
            AnthropicContent::Blocks(blocks) => {
                assert!(matches!(
                    blocks[0],
                    AnthropicContentBlock::ToolResult { .. }
                ));
            }
            _ => panic!("tool result should be sent as blocks"),
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
