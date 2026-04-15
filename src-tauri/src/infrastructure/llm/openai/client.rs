//! OpenAI-compatible API client implementation.

use async_stream::stream;
use futures::Stream;
use log::{error, info, warn};
use reqwest::Client;
use serde::Deserialize;
use std::pin::Pin;

use super::types::{
    OpenAiMessage, OpenAiRequest, OpenAiRequestMessageToolCall, OpenAiRequestMessageToolFunction,
    OpenAiResponse, OpenAiStreamResponse,
};
use crate::core::config::{ApiType, Provider};
use crate::infrastructure::llm::{
    ChatRequest, ChatResponse, LlmClient, LlmError, LlmResult, Message, MessageContent,
    MessageRole, StreamChunk, StreamDelta, ToolCall, UsageStats,
};
use crate::infrastructure::utils::truncate_text;

/// Default base URL for `OpenAI` API.
pub const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";

/// Default base URL for Google OpenAI-compatible API.
pub const GOOGLE_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/openai";

/// OpenAI-compatible API client.
pub struct OpenAiClient {
    /// HTTP client
    client: Client,
    /// API key
    api_key: String,
    /// Base URL for API requests
    base_url: String,
    /// Provider type (`OpenAI` or Google)
    provider_type: ApiType,
    /// Provider-specific request compatibility rules.
    compatibility: OpenAiCompatibility,
}

/// Request-shaping compatibility for OpenAI-style providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenAiCompatibility {
    /// Whether the provider accepts `role: "system"` in the chat `messages` array.
    pub supports_system_role_messages: bool,
}

impl OpenAiCompatibility {
    /// Standard OpenAI-compatible behavior.
    pub const fn standard() -> Self {
        Self {
            supports_system_role_messages: true,
        }
    }

    /// Compatibility mode for providers that reject `system` in `messages`.
    pub const fn without_system_role_messages() -> Self {
        Self {
            supports_system_role_messages: false,
        }
    }

    /// Resolves compatibility requirements from provider configuration.
    #[must_use]
    pub fn for_provider(provider: &Provider) -> Self {
        let provider_id = provider.id.to_ascii_lowercase();
        let base_url = provider
            .base_url
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase();

        if provider_id == "minimax" || base_url.contains("minimax.io") {
            return Self::without_system_role_messages();
        }

        Self::standard()
    }
}

impl OpenAiClient {
    /// Creates a new `OpenAI` client.
    #[must_use]
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self::new_with_compatibility(api_key, base_url, OpenAiCompatibility::standard())
    }

    /// Creates a new `OpenAI` client using provider-specific compatibility rules.
    #[must_use]
    pub fn new_for_provider(api_key: String, provider: &Provider) -> Self {
        Self::new_with_compatibility(
            api_key,
            provider.base_url.clone(),
            OpenAiCompatibility::for_provider(provider),
        )
    }

    fn new_with_compatibility(
        api_key: String,
        base_url: Option<String>,
        compatibility: OpenAiCompatibility,
    ) -> Self {
        let client = Client::builder()
            .build()
            .expect("Failed to create HTTP client");

        // Normalize base URL: strip any endpoint paths that might have been incorrectly included
        let normalized_base_url =
            Self::normalize_base_url(base_url.unwrap_or_else(|| OPENAI_BASE_URL.to_string()));

        Self {
            client,
            api_key,
            base_url: normalized_base_url,
            provider_type: ApiType::OpenAI,
            compatibility,
        }
    }

    /// Normalizes a base URL by removing any trailing endpoint paths.
    ///
    /// This handles cases where the base URL was incorrectly stored with
    /// endpoint paths like `/chat/completions` or `/models` already included.
    fn normalize_base_url(url: String) -> String {
        let url = url.trim_end_matches('/');

        // Remove common endpoint paths that shouldn't be in the base URL
        let endpoints_to_strip = [
            "/chat/completions",
            "/completions",
            "/models",
            "/v1/chat/completions",
            "/v1/completions",
            "/v1/models",
        ];

        for endpoint in endpoints_to_strip {
            if let Some(normalized) = url.strip_suffix(endpoint) {
                info!("Normalized base URL from '{url}' to '{normalized}'");
                return normalized.to_string();
            }
        }

        url.to_string()
    }

    /// Creates a new Google client using OpenAI-compatible API.
    #[must_use]
    pub fn new_google(api_key: String) -> Self {
        let client = Client::builder()
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key,
            base_url: GOOGLE_BASE_URL.to_string(),
            provider_type: ApiType::Google,
            compatibility: OpenAiCompatibility::standard(),
        }
    }

    /// Gets the authorization header value.
    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    /// Converts a `ChatRequest` to `OpenAI` format.
    fn convert_request(&self, request: ChatRequest) -> OpenAiRequest {
        self.convert_request_with_tool_choice(request, true)
    }

    fn convert_request_with_tool_choice(
        &self,
        request: ChatRequest,
        include_tool_choice: bool,
    ) -> OpenAiRequest {
        let ChatRequest {
            model,
            messages: request_messages,
            system,
            temperature,
            max_tokens: request_max_tokens,
            top_p,
            stop,
            stream,
            tools: request_tools,
        } = request;
        let mut messages = Vec::new();
        let mut folded_system_segments = Vec::new();

        if self.compatibility.supports_system_role_messages {
            if let Some(system) = &system {
                messages.push(OpenAiMessage {
                    role: "system".to_string(),
                    content: Some(system.clone()),
                    tool_call_id: None,
                    tool_calls: None,
                });
            }
        } else if let Some(system) = system {
            folded_system_segments.push(system);
        }

        // Convert messages
        for msg in request_messages {
            if msg.role == MessageRole::System {
                if self.compatibility.supports_system_role_messages {
                    let content = match &msg.content {
                        MessageContent::Text(text) => Some(text.clone()),
                        MessageContent::Blocks(blocks) => Some(
                            blocks
                                .iter()
                                .filter_map(|b| match b {
                                    crate::core::llm::entities::ContentBlock::Text { text } => {
                                        Some(text.clone())
                                    }
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                                .join("\n"),
                        ),
                    };

                    messages.push(OpenAiMessage {
                        role: "system".to_string(),
                        content,
                        tool_call_id: None,
                        tool_calls: None,
                    });
                } else if let Some(text) = msg.as_text() {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        folded_system_segments.push(trimmed.to_string());
                    }
                }
                continue;
            }

            if msg.role == MessageRole::Tool {
                if let Some(tool_result) = &msg.tool_result {
                    let output = serde_json::to_string(&tool_result.output)
                        .unwrap_or_else(|_| tool_result.output.to_string());
                    messages.push(OpenAiMessage {
                        role: "tool".to_string(),
                        content: Some(output),
                        tool_call_id: Some(tool_result.call_id.clone()),
                        tool_calls: None,
                    });
                }
                continue;
            }

            let content = match &msg.content {
                MessageContent::Text(text) => text.clone(),
                MessageContent::Blocks(blocks) => {
                    // For now, extract text blocks only
                    blocks
                        .iter()
                        .filter_map(|b| match b {
                            crate::core::llm::entities::ContentBlock::Text { text } => {
                                Some(text.clone())
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            };
            let request_tool_calls =
                if msg.role == MessageRole::Assistant && !msg.tool_calls.is_empty() {
                    Some(
                        msg.tool_calls
                            .iter()
                            .map(|call| OpenAiRequestMessageToolCall {
                                id: call.id.clone(),
                                tool_type: "function".to_string(),
                                function: OpenAiRequestMessageToolFunction {
                                    name: call.name.clone(),
                                    arguments: serde_json::to_string(&call.arguments)
                                        .unwrap_or_else(|_| call.arguments.to_string()),
                                },
                            })
                            .collect(),
                    )
                } else {
                    None
                };
            let content = if content.is_empty() && request_tool_calls.is_some() {
                None
            } else {
                Some(content)
            };
            messages.push(OpenAiMessage {
                role: msg.role.as_str().to_string(),
                content,
                tool_call_id: None,
                tool_calls: request_tool_calls,
            });
        }

        if !self.compatibility.supports_system_role_messages {
            self.fold_system_segments_into_messages(&mut messages, folded_system_segments);
        }

        // Determine which token parameter to use based on model
        // Newer models (GPT-5, o1, o3 series) require max_completion_tokens
        let model_lower = model.to_lowercase();
        let uses_max_completion_tokens = model_lower.starts_with("gpt-5")
            || model_lower.starts_with("o1")
            || model_lower.starts_with("o3")
            || model_lower.starts_with("gpt-4.1");

        let (max_tokens, max_completion_tokens) = if uses_max_completion_tokens {
            (None, request_max_tokens)
        } else {
            (request_max_tokens, None)
        };

        let tools = super::convert_tools(request_tools);
        let tool_choice = if include_tool_choice {
            tools.as_ref().map(|_| "auto".to_string())
        } else {
            None
        };

        OpenAiRequest {
            model,
            messages,
            temperature,
            max_tokens,
            max_completion_tokens,
            top_p,
            stop,
            stream,
            tools,
            tool_choice,
        }
    }

    fn fold_system_segments_into_messages(
        &self,
        messages: &mut Vec<OpenAiMessage>,
        system_segments: Vec<String>,
    ) {
        let system_prompt = system_segments
            .into_iter()
            .map(|segment| segment.trim().to_string())
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");

        if system_prompt.is_empty() {
            return;
        }

        let folded_content = format!("<system>\n{system_prompt}\n</system>");

        if let Some(first_user_message) = messages.iter_mut().find(|message| message.role == "user")
        {
            let existing_content = first_user_message.content.take().unwrap_or_default();
            first_user_message.content = Some(if existing_content.is_empty() {
                folded_content
            } else {
                format!("{folded_content}\n\n{existing_content}")
            });
            return;
        }

        messages.insert(
            0,
            OpenAiMessage {
                role: "user".to_string(),
                content: Some(folded_content),
                tool_call_id: None,
                tool_calls: None,
            },
        );
    }

    /// Parses a SSE line from the stream.
    #[must_use]
    pub fn parse_sse_line(line: &str) -> Option<Result<OpenAiStreamResponse, LlmError>> {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with(':') {
            return None;
        }

        // Parse data line - handle both "data: " and plain JSON (some APIs omit the prefix)
        let data = if let Some(d) = line.strip_prefix("data: ") {
            d
        } else {
            line
        };

        // Check for stream end
        if data == "[DONE]" {
            return None;
        }

        // Parse JSON
        match serde_json::from_str::<OpenAiStreamResponse>(data) {
            Ok(response) => Some(Ok(response)),
            Err(e) => {
                // Try parsing without the "data: " prefix if it was already stripped
                if line.starts_with("data: ") {
                    match serde_json::from_str::<OpenAiStreamResponse>(
                        line.strip_prefix("data: ").unwrap(),
                    ) {
                        Ok(response) => Some(Ok(response)),
                        Err(_) => Some(Err(LlmError::parse(format!(
                            "Failed to parse stream chunk: {e} | Line: {line}"
                        )))),
                    }
                } else {
                    Some(Err(LlmError::parse(format!(
                        "Failed to parse stream chunk: {e} | Line: {line}"
                    ))))
                }
            }
        }
    }

    /// Makes a non-streaming chat request.
    pub async fn chat_internal(&self, request: ChatRequest) -> LlmResult<ChatResponse> {
        let mut include_tool_choice = true;
        let can_retry_without_tool_choice = !request.tools.is_empty();
        let openai_response: OpenAiResponse = loop {
            let openai_request =
                self.convert_request_with_tool_choice(request.clone(), include_tool_choice);

            let response = self
                .client
                .post(format!("{}/chat/completions", self.base_url))
                .header("Authorization", self.auth_header())
                .header("Content-Type", "application/json")
                .json(&openai_request)
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
                        "openai::chat_internal: tool-enabled request failed with status {}; retrying once without tool_choice",
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
                .map_err(|e| LlmError::parse(format!("Failed to parse response: {e}")))?;
        };

        // Extract content and structured tool calls from first choice.
        // For models like GLM-5, fall back to reasoning_content if content is empty.
        let message = openai_response
            .choices
            .first()
            .and_then(|c| c.message.as_ref())
            .ok_or_else(|| LlmError::parse("No message in response"))?;

        // Use reasoning_content as fallback for models that show thinking before answering
        let content = message
            .content
            .clone()
            .or(message.reasoning_content.clone())
            .unwrap_or_default();

        // Parse tool calls with proper error handling
        let tool_calls = if let Some(calls) = &message.tool_calls {
            calls
                .iter()
                .map(|call| -> Result<ToolCall, LlmError> {
                    let arguments =
                        match serde_json::from_str::<serde_json::Value>(&call.function.arguments) {
                            Ok(value) => value,
                            Err(e) => {
                                let truncated = truncate_text(&call.function.arguments, 500);
                                log::error!(
                                    "Failed to parse tool arguments for '{}': {}. Arguments: {}",
                                    call.function.name,
                                    e,
                                    truncated
                                );
                                return Err(LlmError::parse(format!(
                                    "Provider returned malformed tool arguments for '{}': {}. Arguments JSON could not be parsed.",
                                    call.function.name, e
                                )));
                            }
                        };
                    Ok(ToolCall {
                        id: call.id.clone(),
                        name: call.function.name.clone(),
                        arguments,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };

        let finish_reason = openai_response
            .choices
            .first()
            .and_then(|c| c.finish_reason.clone());

        let usage = openai_response.usage.map(|u| UsageStats {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        Ok(ChatResponse {
            id: openai_response.id,
            model: openai_response.model,
            content,
            usage,
            finish_reason,
            tool_calls,
        })
    }

    /// Makes a streaming chat request.
    #[must_use]
    pub fn chat_stream_internal(
        &self,
        request: ChatRequest,
    ) -> Pin<Box<dyn Stream<Item = LlmResult<StreamChunk>> + Send>> {
        let client = self.client.clone();
        let base_url = self.base_url.clone();
        let auth_header = self.auth_header();
        let openai_request = self.convert_request(request);
        let url = format!("{base_url}/chat/completions");

        info!("openai::chat_stream: starting request to {url}");

        Box::pin(stream! {
            info!("openai::chat_stream: sending POST request");
            let response = client
                .post(&url)
                .header("Authorization", &auth_header)
                .header("Content-Type", "application/json")
                .json(&openai_request)
                .send()
                .await;

            let response = match response {
                Ok(r) => {
                    info!("openai::chat_stream: got response, status={}", r.status());
                    r
                }
                Err(e) => {
                    error!("openai::chat_stream: request failed: {e}");
                    yield Err(LlmError::from(e));
                    return;
                }
            };

            let status = response.status();
            if !status.is_success() {
                let error_text =
                    response.text().await.unwrap_or_else(|_| "Unknown error".to_string());

                // Special handling for 404 on z.ai endpoint
                if status.as_u16() == 404 && url.contains("api.z.ai") {
                    error!("openai::chat_stream: GLM provider endpoint not found. The z.ai endpoint {url} does not exist. Please check your GLM provider configuration.");
                    yield Err(LlmError::api(status.as_u16(),
                        "GLM provider endpoint not found. Please check your provider configuration. The z.ai API endpoint structure may have changed.".to_string()));
                } else {
                    error!("openai::chat_stream: API error status={status}, body={error_text}");
                    yield Err(LlmError::api(status.as_u16(), error_text));
                }
                return;
            }

            info!("openai::chat_stream: starting to read stream");

            // Stream the response
            #[allow(clippy::expect_used)]
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();
            let mut chunk_count = 0;
            let mut line_count = 0;
            let mut has_content = false;

            while let Some(chunk_result) = futures::StreamExt::next(&mut stream).await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        error!("openai::chat_stream: error reading from stream: {e}");
                        yield Err(LlmError::streaming(e.to_string()));
                        continue;
                    }
                };

                chunk_count += 1;

                // Convert bytes to string and append to buffer
                match String::from_utf8(chunk.to_vec()) {
                    Ok(text) => {
                        if chunk_count <= 5 {
                            // Log first few chunks for debugging
                            info!(
                                "openai::chat_stream: received chunk #{}, length={} bytes",
                                chunk_count,
                                text.len()
                            );
                            if !text.is_empty() {
                                info!(
                                    "openai::chat_stream: chunk content preview: {:?}",
                                    crate::infrastructure::utils::truncate_text(&text, 200)
                                );
                            }
                        }
                        buffer.push_str(&text);
                    }
                    Err(e) => {
                        error!("openai::chat_stream: failed to convert chunk to utf8: {e}");
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
                            "openai::chat_stream: processing line #{}, content={:?}",
                            line_count,
                            crate::infrastructure::utils::truncate_text(&line, 150)
                        );
                    }

                    if let Some(result) = Self::parse_sse_line(&line) {
                        match result {
                            Ok(openai_response) => {
                                has_content = true;
                                // Only log every 50 chunks to reduce log spam
                                if line_count % 50 == 0 {
                                    info!(
                                        "openai::chat_stream: parsed SSE response #{}, id={}, choices={}",
                                        line_count,
                                        openai_response.id,
                                        openai_response.choices.len()
                                    );
                                }

                                // Extract delta from first choice
                                if let Some(choice) = openai_response.choices.first() {
                                    let delta = choice.delta.as_ref().map(|d| {
                                        // For models like GLM-5, fall back to reasoning_content if content is not present
                                        let content = d.content.clone()
                                            .or(d.reasoning_content.clone());
                                        let reasoning = d.reasoning_content.clone();
                                        StreamDelta {
                                            role: d.role.clone(),
                                            content,
                                            reasoning_content: reasoning,
                                        }
                                    });

                                    let finish_reason = choice.finish_reason.clone();
                                    let is_final = finish_reason.is_some();

                                    let usage = openai_response.usage.as_ref().map(|u| UsageStats {
                                        input_tokens: u.prompt_tokens,
                                        output_tokens: u.completion_tokens,
                                        total_tokens: u.total_tokens,
                                    });

                                    if line_count <= 10 {
                                        info!(
                                            "openai::chat_stream: yielding chunk with delta content={:?}",
                                            delta.as_ref().and_then(|d| d.content.clone())
                                        );
                                    }

                                    yield Ok(StreamChunk {
                                        id: openai_response.id,
                                        model: openai_response.model,
                                        delta,
                                        usage,
                                        finish_reason,
                                        is_final,
                                    });
                                }
                            }
                            Err(e) => {
                                error!(
                                    "openai::chat_stream: failed to parse SSE line {line_count}: {e}"
                                );
                                yield Err(e);
                            }
                        }
                    } else if !line.trim().is_empty()
                        && line_count <= 10 {
                            info!(
                                "openai::chat_stream: skipping unparsable line (not SSE format): {:?}",
                                crate::infrastructure::utils::truncate_text(&line, 100)
                            );
                        }
                }
            }

            // Final state logging
            info!(
                "openai::chat_stream: stream completed after {} chunks and {} lines",
                chunk_count, line_count
            );
            info!("openai::chat_stream: final buffer size: {} bytes", buffer.len());
            if !has_content {
                error!("openai::chat_stream: no content received in stream - this indicates a server or protocol issue");
            }

            // Check if there's any remaining data in buffer that wasn't processed
            if !buffer.is_empty() {
                error!(
                    "openai::chat_stream: {} bytes remaining in buffer that weren't processed as complete lines",
                    buffer.len()
                );
                if buffer.len() < 200 {
                    info!("openai::chat_stream: remaining buffer content: {buffer:?}");
                }
            }
        })
    }

    /// Validates the API key by making a minimal request.
    pub async fn validate_api_key(&self) -> LlmResult<bool> {
        let request =
            ChatRequest::new("gpt-3.5-turbo", vec![Message::user("test")]).with_max_tokens(1);

        match self.chat_internal(request).await {
            Ok(_) => Ok(true),
            Err(e) if e.is_auth_error() => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Fetches available models from the API.
    pub async fn fetch_models(&self) -> LlmResult<Vec<String>> {
        let response = self
            .client
            .get(format!("{}/models", self.base_url))
            .header("Authorization", self.auth_header())
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

        #[derive(Debug, Deserialize)]
        struct ModelsResponse {
            data: Vec<ModelInfo>,
        }

        #[derive(Debug, Deserialize)]
        struct ModelInfo {
            id: String,
        }

        let models_response: ModelsResponse = response
            .json()
            .await
            .map_err(|e| LlmError::parse(format!("Failed to parse models response: {e}")))?;

        Ok(models_response.data.into_iter().map(|m| m.id).collect())
    }
}

impl LlmClient for OpenAiClient {
    fn provider_type(&self) -> ApiType {
        self.provider_type
    }

    fn chat(
        &self,
        request: ChatRequest,
    ) -> futures::future::BoxFuture<'_, LlmResult<ChatResponse>> {
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
    use crate::core::config::{ApiType, Provider};
    use crate::infrastructure::llm::ToolDefinition;

    #[test]
    fn test_client_creation() {
        let client = OpenAiClient::new("test-key".to_string(), None);
        assert_eq!(client.provider_type(), ApiType::OpenAI);
        assert_eq!(client.base_url, OPENAI_BASE_URL);
        assert!(client.compatibility.supports_system_role_messages);
    }

    #[test]
    fn test_google_client_creation() {
        let client = OpenAiClient::new_google("test-key".to_string());
        assert_eq!(client.provider_type(), ApiType::Google);
        assert_eq!(client.base_url, GOOGLE_BASE_URL);
    }

    #[test]
    fn test_convert_request() {
        let client = OpenAiClient::new("test-key".to_string(), None);
        let request = ChatRequest::new(
            "gpt-4",
            vec![Message::system("Be helpful"), Message::user("Hello")],
        )
        .with_system("System prompt");

        let openai_request = client.convert_request(request);

        assert_eq!(openai_request.model, "gpt-4");
        // System prompt + system message + user message
        assert_eq!(openai_request.messages.len(), 3);
        assert_eq!(openai_request.messages[0].role, "system");
        assert_eq!(openai_request.messages[1].role, "system");
        assert_eq!(openai_request.messages[2].role, "user");
        assert!(openai_request.tools.is_none());
        assert!(openai_request.tool_choice.is_none());
    }

    #[test]
    fn test_client_creation_for_minimax_provider_disables_system_role_messages() {
        let provider = Provider::custom_with_id(
            "minimax",
            "MiniMax",
            "MM",
            ApiType::OpenAI,
            Some("https://api.minimax.io/v1".to_string()),
        );

        let client = OpenAiClient::new_for_provider("test-key".to_string(), &provider);

        assert_eq!(client.provider_type(), ApiType::OpenAI);
        assert_eq!(client.base_url, "https://api.minimax.io/v1");
        assert!(!client.compatibility.supports_system_role_messages);
    }

    #[test]
    fn test_convert_request_folds_system_instructions_for_incompatible_provider() {
        let provider = Provider::custom_with_id(
            "minimax",
            "MiniMax",
            "MM",
            ApiType::OpenAI,
            Some("https://api.minimax.io/v1".to_string()),
        );
        let client = OpenAiClient::new_for_provider("test-key".to_string(), &provider);
        let request = ChatRequest::new(
            "MiniMax-M2.5",
            vec![
                Message::system("Follow repository rules"),
                Message::user("Review this code"),
            ],
        )
        .with_system("You are a code assistant");

        let openai_request = client.convert_request(request);

        assert_eq!(openai_request.messages.len(), 1);
        assert_eq!(openai_request.messages[0].role, "user");
        let content = openai_request.messages[0]
            .content
            .as_deref()
            .expect("content should be present");
        assert!(content.contains("You are a code assistant"));
        assert!(content.contains("Follow repository rules"));
        assert!(content.contains("Review this code"));
    }

    #[test]
    fn test_convert_request_inserts_synthetic_user_message_when_needed() {
        let provider = Provider::custom_with_id(
            "minimax",
            "MiniMax",
            "MM",
            ApiType::OpenAI,
            Some("https://api.minimax.io/v1".to_string()),
        );
        let client = OpenAiClient::new_for_provider("test-key".to_string(), &provider);
        let request = ChatRequest::new(
            "MiniMax-M2.5",
            vec![Message::assistant_with_tool_calls(
                "",
                vec![ToolCall {
                    id: "call_1".to_string(),
                    name: "search_code".to_string(),
                    arguments: serde_json::json!({"query": "runner"}),
                }],
            )],
        )
        .with_system("You are a code assistant");

        let openai_request = client.convert_request(request);

        assert_eq!(openai_request.messages.len(), 2);
        assert_eq!(openai_request.messages[0].role, "user");
        assert_eq!(openai_request.messages[1].role, "assistant");
        assert!(openai_request.messages[0]
            .content
            .as_deref()
            .expect("synthetic user content should be present")
            .contains("You are a code assistant"));
    }

    #[test]
    fn test_convert_request_with_tools() {
        let client = OpenAiClient::new("test-key".to_string(), None);
        let request = ChatRequest::new("gpt-4.1", vec![Message::user("Hello")]).with_tools(vec![
            ToolDefinition {
                name: "read_workspace_file".to_string(),
                description: "Read a UTF-8 file from the workspace".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"}
                    },
                    "required": ["path"]
                }),
            },
        ]);

        let openai_request = client.convert_request(request);
        let tools = openai_request.tools.expect("tools should be present");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].tool_type, "function");
        assert_eq!(tools[0].function.name, "read_workspace_file");
        assert_eq!(openai_request.tool_choice.as_deref(), Some("auto"));
    }

    #[test]
    fn test_convert_request_with_tools_without_tool_choice() {
        let client = OpenAiClient::new("test-key".to_string(), None);
        let request = ChatRequest::new("gpt-4.1", vec![Message::user("Hello")]).with_tools(vec![
            ToolDefinition {
                name: "read_workspace_file".to_string(),
                description: "Read a UTF-8 file from the workspace".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"}
                    },
                    "required": ["path"]
                }),
            },
        ]);

        let openai_request = client.convert_request_with_tool_choice(request, false);
        assert!(openai_request.tools.is_some());
        assert!(openai_request.tool_choice.is_none());
    }

    #[test]
    fn test_convert_request_with_tool_messages() {
        let client = OpenAiClient::new("test-key".to_string(), None);
        let request = ChatRequest::new(
            "gpt-4.1",
            vec![
                Message::assistant_with_tool_calls(
                    "",
                    vec![ToolCall {
                        id: "call_1".to_string(),
                        name: "search_code".to_string(),
                        arguments: serde_json::json!({"query": "runner"}),
                    }],
                ),
                Message::tool_result("call_1", serde_json::json!({"ok": true}), false),
            ],
        );

        let openai_request = client.convert_request(request);
        assert_eq!(openai_request.messages.len(), 2);
        assert_eq!(openai_request.messages[0].role, "assistant");
        assert!(openai_request.messages[0].tool_calls.is_some());
        assert_eq!(openai_request.messages[1].role, "tool");
        assert_eq!(
            openai_request.messages[1].tool_call_id.as_deref(),
            Some("call_1")
        );
    }

    #[test]
    fn test_parse_sse_line() {
        let line = r#"data: {"id":"chatcmpl-123","model":"gpt-4","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#;
        let result = OpenAiClient::parse_sse_line(line);

        assert!(result.is_some());
        let parsed = result.unwrap();
        assert!(parsed.is_ok());

        let response = parsed.unwrap();
        assert_eq!(response.id, "chatcmpl-123");
        assert_eq!(response.model, "gpt-4");
        assert_eq!(response.choices.len(), 1);
    }

    #[test]
    fn test_parse_sse_line_done() {
        let line = "data: [DONE]";
        let result = OpenAiClient::parse_sse_line(line);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_sse_line_empty() {
        assert!(OpenAiClient::parse_sse_line("").is_none());
        assert!(OpenAiClient::parse_sse_line(": comment").is_none());
    }

    #[test]
    fn test_normalize_base_url() {
        // Test correct base URL (no change)
        assert_eq!(
            OpenAiClient::normalize_base_url("https://api.openai.com/v1".to_string()),
            "https://api.openai.com/v1"
        );

        // Test base URL with trailing slash
        assert_eq!(
            OpenAiClient::normalize_base_url("https://api.openai.com/v1/".to_string()),
            "https://api.openai.com/v1"
        );

        // Test base URL with /chat/completions (the bug case)
        assert_eq!(
            OpenAiClient::normalize_base_url(
                "https://api.openai.com/v1/chat/completions".to_string()
            ),
            "https://api.openai.com/v1"
        );

        // Test base URL with /models
        assert_eq!(
            OpenAiClient::normalize_base_url("https://api.openai.com/v1/models".to_string()),
            "https://api.openai.com/v1"
        );

        // Test base URL with /completions
        assert_eq!(
            OpenAiClient::normalize_base_url("https://api.example.com/v1/completions".to_string()),
            "https://api.example.com/v1"
        );

        // Test custom provider URL
        assert_eq!(
            OpenAiClient::normalize_base_url(
                "https://custom.api.com/api/v1/chat/completions".to_string()
            ),
            "https://custom.api.com/api/v1"
        );
    }

    #[test]
    fn test_client_creation_with_normalized_url() {
        // Test that client normalizes incorrect base URL
        let client = OpenAiClient::new(
            "test-key".to_string(),
            Some("https://api.openai.com/v1/chat/completions".to_string()),
        );
        assert_eq!(client.base_url, "https://api.openai.com/v1");
    }

    #[test]
    fn test_truncate_text_handles_multibyte_boundary() {
        let text = format!("{}{}{}", "a".repeat(499), "─", "b");

        let truncated = crate::infrastructure::utils::truncate_text(&text, 500);

        assert!(truncated.ends_with("..."));
        assert!(truncated.starts_with(&"a".repeat(499)));
    }
}
