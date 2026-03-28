//! Mock LLM client for testing without API calls.
//!
//! Provides a predictable, controllable LLM client implementation
//! for testing planning and execution logic.

use crate::core::config::ApiType;
use crate::core::llm::entities::{ChatRequest, ChatResponse, StreamChunk, StreamDelta};
use crate::infrastructure::llm::{LlmClient, LlmError, LlmResult};
use futures::future::BoxFuture;
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;

/// Mock LLM client for testing.
///
/// Allows setting predefined responses for testing without making actual API calls.
#[derive(Clone)]
pub struct MockLlmClient {
    /// Predefined responses to return (in FIFO order)
    responses: Arc<std::sync::Mutex<Vec<String>>>,
    /// Predefined errors to return (in FIFO order)
    errors: Arc<std::sync::Mutex<Vec<LlmError>>>,
    /// Provider type to report
    provider_type: ApiType,
    /// Model name to return in responses
    model_name: String,
}

impl MockLlmClient {
    /// Creates a new mock client with empty response queue.
    pub fn new(provider_type: ApiType, model_name: impl Into<String>) -> Self {
        Self {
            responses: Arc::new(std::sync::Mutex::new(Vec::new())),
            errors: Arc::new(std::sync::Mutex::new(Vec::new())),
            provider_type,
            model_name: model_name.into(),
        }
    }

    /// Creates a mock client with Anthropic provider (default for tests).
    pub fn anthropic(model_name: impl Into<String>) -> Self {
        Self::new(ApiType::Anthropic, model_name)
    }

    /// Creates a mock client with OpenAI provider.
    pub fn openai(model_name: impl Into<String>) -> Self {
        Self::new(ApiType::OpenAI, model_name)
    }

    /// Sets the next response to return.
    ///
    /// If called multiple times, responses are returned in FIFO order.
    pub fn set_response(&self, response: impl Into<String>) {
        self.responses.lock().unwrap().push(response.into());
    }

    /// Sets multiple responses to return in sequence.
    pub fn set_responses(&self, responses: Vec<String>) {
        let mut queue = self.responses.lock().unwrap();
        *queue = responses;
    }

    /// Sets an error to return on the next call.
    pub fn set_error(&self, error: LlmError) {
        self.errors.lock().unwrap().push(error);
    }

    /// Clears all responses and errors.
    pub fn clear(&self) {
        self.responses.lock().unwrap().clear();
        self.errors.lock().unwrap().clear();
    }

    /// Returns the number of pending responses.
    pub fn response_count(&self) -> usize {
        self.responses.lock().unwrap().len()
    }

    /// Returns the number of pending errors.
    pub fn error_count(&self) -> usize {
        self.errors.lock().unwrap().len()
    }

    /// Creates a successful ChatResponse from JSON content.
    pub fn success_response(&self, content: String) -> ChatResponse {
        ChatResponse {
            id: uuid::Uuid::new_v4().to_string(),
            model: self.model_name.clone(),
            content,
            usage: None,
            finish_reason: Some("stop".to_string()),
            tool_calls: Vec::new(),
        }
    }

    /// Checks for pending errors and returns one if available.
    fn take_error(&self) -> Option<LlmError> {
        let mut errors = self.errors.lock().unwrap();
        if !errors.is_empty() {
            Some(errors.remove(0))
        } else {
            None
        }
    }

    /// Takes the next response from the queue.
    ///
    /// Returns a default JSON response if queue is empty.
    fn take_response(&self) -> String {
        let mut responses = self.responses.lock().unwrap();
        if !responses.is_empty() {
            responses.remove(0)
        } else {
            // Default response
            serde_json::json!({
                "title": "Mock Task",
                "description": "Mock task description",
                "is_non_trivial": false,
                "subtasks": []
            })
            .to_string()
        }
    }
}

impl LlmClient for MockLlmClient {
    fn provider_type(&self) -> ApiType {
        self.provider_type
    }

    fn chat<'a>(&'a self, _request: ChatRequest) -> BoxFuture<'a, LlmResult<ChatResponse>> {
        Box::pin(async move {
            // Check for pending error first
            if let Some(err) = self.take_error() {
                return Err(err);
            }

            // Get next response
            let content = self.take_response();
            Ok(self.success_response(content))
        })
    }

    fn chat_stream(
        &self,
        _request: ChatRequest,
    ) -> Pin<Box<dyn Stream<Item = LlmResult<StreamChunk>> + Send>> {
        use futures::stream;

        // Check for pending error
        if let Some(err) = self.take_error() {
            return Box::pin(stream::once(async move { Err(err) }));
        }

        // Return a single chunk with the next response
        let content = self.take_response();
        let delta = StreamDelta {
            role: None,
            content: Some(content),
            reasoning_content: None,
        };
        let chunk = StreamChunk {
            id: uuid::Uuid::new_v4().to_string(),
            model: self.model_name.clone(),
            delta: Some(delta),
            usage: None,
            finish_reason: None,
            is_final: true,
        };
        Box::pin(stream::once(async move { Ok(chunk) }))
    }

    fn validate_api_key(&self) -> BoxFuture<'_, LlmResult<bool>> {
        Box::pin(async move { Ok(true) })
    }

    fn fetch_models(&self) -> BoxFuture<'_, LlmResult<Vec<String>>> {
        Box::pin(async move {
            Ok(vec![
                format!("{}-mock-1", self.model_name),
                format!("{}-mock-2", self.model_name),
            ])
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_client_response_queue() {
        let client = MockLlmClient::anthropic("test-model");

        client.set_response("First response".to_string());
        client.set_response("Second response".to_string());

        assert_eq!(client.response_count(), 2);

        let rt = tokio::runtime::Runtime::new().unwrap();
        let response1 = rt.block_on(client.chat(ChatRequest::new("test-model", vec![])));
        assert!(response1.is_ok());
        assert_eq!(response1.unwrap().content, "First response");
        assert_eq!(client.response_count(), 1);

        let response2 = rt.block_on(client.chat(ChatRequest::new("test-model", vec![])));
        assert!(response2.is_ok());
        assert_eq!(response2.unwrap().content, "Second response");
        assert_eq!(client.response_count(), 0);
    }

    #[test]
    fn test_mock_client_error_queue() {
        let client = MockLlmClient::anthropic("test-model");

        client.set_error(LlmError::rate_limit(Some(60)));
        client.set_response("After error".to_string());

        let rt = tokio::runtime::Runtime::new().unwrap();

        let response1 = rt.block_on(client.chat(ChatRequest::new("test-model", vec![])));
        assert!(response1.is_err());
        assert!(matches!(
            response1.unwrap_err(),
            LlmError::RateLimit { .. }
        ));

        let response2 = rt.block_on(client.chat(ChatRequest::new("test-model", vec![])));
        assert!(response2.is_ok());
        assert_eq!(response2.unwrap().content, "After error");
    }

    #[test]
    fn test_mock_client_default_response() {
        let client = MockLlmClient::anthropic("test-model");

        let rt = tokio::runtime::Runtime::new().unwrap();
        let response = rt.block_on(client.chat(ChatRequest::new("test-model", vec![])));

        assert!(response.is_ok());
        let resp = response.unwrap();
        assert_eq!(resp.model, "test-model");
        assert!(resp.content.contains("Mock Task"));
    }

    #[test]
    fn test_mock_client_clear() {
        let client = MockLlmClient::anthropic("test-model");

        client.set_response("Test".to_string());
        client.set_error(LlmError::api(400, "test"));

        assert_eq!(client.response_count(), 1);
        assert_eq!(client.error_count(), 1);

        client.clear();

        assert_eq!(client.response_count(), 0);
        assert_eq!(client.error_count(), 0);
    }

    #[test]
    fn test_mock_client_provider_type() {
        let anthropic = MockLlmClient::anthropic("claude");
        assert_eq!(anthropic.provider_type(), ApiType::Anthropic);

        let openai = MockLlmClient::openai("gpt");
        assert_eq!(openai.provider_type(), ApiType::OpenAI);
    }

    #[test]
    fn test_mock_client_validate_and_fetch() {
        let client = MockLlmClient::anthropic("test-model");

        let rt = tokio::runtime::Runtime::new().unwrap();

        let valid = rt.block_on(client.validate_api_key());
        assert!(valid.is_ok());
        assert!(valid.unwrap());

        let models = rt.block_on(client.fetch_models());
        assert!(models.is_ok());
        let model_list = models.unwrap();
        assert_eq!(model_list, vec!["test-model-mock-1", "test-model-mock-2"]);
    }
}
