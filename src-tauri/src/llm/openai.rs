//! OpenAI-compatible API client.
//!
//! This module implements the LLM client for OpenAI and OpenAI-compatible APIs
//! (including Google Gemini via its OpenAI-compatible endpoint).

use async_stream::stream;
use futures::future::BoxFuture;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::time::Duration;

use super::{ChatRequest, ChatResponse, LlmClient, LlmError, LlmResult, Message, StreamChunk, StreamDelta, UsageStats};
use crate::nlp::config::ApiType;

// =============================================================================
// OpenAI API Types
// =============================================================================

/// OpenAI chat completion request.
#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    stream: bool,
}

/// OpenAI message format.
#[derive(Debug, Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

/// OpenAI chat completion response.
#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    id: String,
    model: String,
    choices: Vec<OpenAiChoice>,
    #[serde(default)]
    usage: Option<OpenAiUsage>,
}

/// OpenAI response choice.
#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    #[allow(dead_code)]
    index: u32,
    message: Option<OpenAiResponseMessage>,
    delta: Option<OpenAiDelta>,
    #[serde(default)]
    finish_reason: Option<String>,
}

/// OpenAI response message.
#[derive(Debug, Deserialize)]
struct OpenAiResponseMessage {
    #[allow(dead_code)]
    role: String,
    content: String,
}

/// OpenAI streaming delta.
#[derive(Debug, Deserialize)]
struct OpenAiDelta {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    content: Option<String>,
}

/// OpenAI usage statistics.
#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    #[serde(default)]
    total_tokens: u32,
}

/// OpenAI streaming response.
#[derive(Debug, Deserialize)]
struct OpenAiStreamResponse {
    id: String,
    model: String,
    choices: Vec<OpenAiChoice>,
    #[serde(default)]
    usage: Option<OpenAiUsage>,
}

// =============================================================================
// Client Implementation
// =============================================================================

/// Default base URL for OpenAI API.
const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";

/// Default base URL for Google OpenAI-compatible API.
const GOOGLE_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/openai";

/// Request timeout in seconds.
const REQUEST_TIMEOUT_SECS: u64 = 120;

/// OpenAI-compatible API client.
pub struct OpenAiClient {
    /// HTTP client
    client: Client,
    /// API key
    api_key: String,
    /// Base URL for API requests
    base_url: String,
    /// Provider type (OpenAI or Google)
    provider_type: ApiType,
}

impl OpenAiClient {
    /// Creates a new OpenAI client.
    #[must_use]
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key,
            base_url: base_url.unwrap_or_else(|| OPENAI_BASE_URL.to_string()),
            provider_type: ApiType::OpenAI,
        }
    }

    /// Creates a new Google client using OpenAI-compatible API.
    #[must_use]
    pub fn new_google(api_key: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key,
            base_url: GOOGLE_BASE_URL.to_string(),
            provider_type: ApiType::Google,
        }
    }

    /// Gets the authorization header value.
    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    /// Converts a ChatRequest to OpenAI format.
    fn convert_request(&self, request: ChatRequest) -> OpenAiRequest {
        let mut messages = Vec::new();

        // Add system message if provided
        if let Some(system) = &request.system {
            messages.push(OpenAiMessage {
                role: "system".to_string(),
                content: system.clone(),
            });
        }

        // Convert messages
        for msg in request.messages {
            let content = match &msg.content {
                super::MessageContent::Text(text) => text.clone(),
                super::MessageContent::Blocks(blocks) => {
                    // For now, extract text blocks only
                    blocks
                        .iter()
                        .filter_map(|b| match b {
                            super::ContentBlock::Text { text } => Some(text.clone()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            };
            messages.push(OpenAiMessage {
                role: msg.role.as_str().to_string(),
                content,
            });
        }

        OpenAiRequest {
            model: request.model,
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            stop: request.stop,
            stream: request.stream,
        }
    }

    /// Parses a SSE line from the stream.
    fn parse_sse_line(line: &str) -> Option<Result<OpenAiStreamResponse, LlmError>> {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with(':') {
            return None;
        }

        // Parse data line
        if let Some(data) = line.strip_prefix("data: ") {
            // Check for stream end
            if data == "[DONE]" {
                return None;
            }

            // Parse JSON
            match serde_json::from_str::<OpenAiStreamResponse>(data) {
                Ok(response) => Some(Ok(response)),
                Err(e) => Some(Err(LlmError::parse(format!(
                    "Failed to parse stream chunk: {}",
                    e
                )))),
            }
        } else {
            None
        }
    }

    /// Makes a non-streaming chat request.
    async fn chat_internal(&self, request: ChatRequest) -> LlmResult<ChatResponse> {
        let openai_request = self.convert_request(request);

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
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LlmError::api(status.as_u16(), error_text));
        }

        let openai_response: OpenAiResponse = response
            .json()
            .await
            .map_err(|e| LlmError::parse(format!("Failed to parse response: {}", e)))?;

        // Extract content from first choice
        let content = openai_response
            .choices
            .first()
            .and_then(|c| c.message.as_ref())
            .map(|m| m.content.clone())
            .unwrap_or_default();

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
        })
    }
}

impl LlmClient for OpenAiClient {
    fn provider_type(&self) -> ApiType {
        self.provider_type
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
        let auth_header = self.auth_header();
        let openai_request = self.convert_request(request);

        Box::pin(stream! {
            let response = client
                .post(format!("{}/chat/completions", base_url))
                .header("Authorization", auth_header)
                .header("Content-Type", "application/json")
                .json(&openai_request)
                .send()
                .await;

            let response = match response {
                Ok(r) => r,
                Err(e) => {
                    yield Err(LlmError::from(e));
                    return;
                }
            };

            let status = response.status();
            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                yield Err(LlmError::api(status.as_u16(), error_text));
                return;
            }

            // Stream the response
            #[allow(clippy::expect_used)]
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_result) = futures::StreamExt::next(&mut stream).await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        yield Err(LlmError::streaming(e.to_string()));
                        continue;
                    }
                };

                // Convert bytes to string and append to buffer
                match String::from_utf8(chunk.to_vec()) {
                    Ok(text) => buffer.push_str(&text),
                    Err(_) => continue,
                }

                // Process complete lines
                while let Some(newline_pos) = buffer.find('\n') {
                    let line = buffer[..newline_pos].to_string();
                    buffer = buffer[newline_pos + 1..].to_string();

                    if let Some(result) = Self::parse_sse_line(&line) {
                        match result {
                            Ok(openai_response) => {
                                let id = openai_response.id.clone();
                                let model = openai_response.model.clone();

                                // Extract delta from first choice
                                if let Some(choice) = openai_response.choices.first() {
                                    let delta = choice.delta.as_ref().map(|d| StreamDelta {
                                        role: d.role.clone(),
                                        content: d.content.clone(),
                                    });

                                    let finish_reason = choice.finish_reason.clone();
                                    let is_final = finish_reason.is_some();

                                    let usage = openai_response.usage.as_ref().map(|u| UsageStats {
                                        input_tokens: u.prompt_tokens,
                                        output_tokens: u.completion_tokens,
                                        total_tokens: u.total_tokens,
                                    });

                                    yield Ok(StreamChunk {
                                        id,
                                        model,
                                        delta,
                                        usage,
                                        finish_reason,
                                        is_final,
                                    });
                                }
                            }
                            Err(e) => {
                                yield Err(e);
                            }
                        }
                    }
                }
            }
        })
    }

    fn validate_api_key(&self) -> BoxFuture<'_, LlmResult<bool>> {
        Box::pin(async {
            // Make a minimal request to validate the API key
            let request = ChatRequest::new("gpt-3.5-turbo", vec![Message::user("test")])
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
            let response = self
                .client
                .get(format!("{}/models", self.base_url))
                .header("Authorization", self.auth_header())
                .send()
                .await?;

            let status = response.status();
            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
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
                .map_err(|e| LlmError::parse(format!("Failed to parse models response: {}", e)))?;

            Ok(models_response.data.into_iter().map(|m| m.id).collect())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = OpenAiClient::new("test-key".to_string(), None);
        assert_eq!(client.provider_type(), ApiType::OpenAI);
        assert_eq!(client.base_url, OPENAI_BASE_URL);
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
        let request = ChatRequest::new("gpt-4", vec![
            Message::system("Be helpful"),
            Message::user("Hello"),
        ]).with_system("System prompt");

        let openai_request = client.convert_request(request);

        assert_eq!(openai_request.model, "gpt-4");
        // System prompt + system message + user message
        assert_eq!(openai_request.messages.len(), 3);
        assert_eq!(openai_request.messages[0].role, "system");
        assert_eq!(openai_request.messages[1].role, "system");
        assert_eq!(openai_request.messages[2].role, "user");
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
}
