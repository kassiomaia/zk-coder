//! Integration tests for Anthropic client using wiremock.
//!
//! These tests mock HTTP responses to test client behavior without
//! making actual API calls.

#[cfg(test)]
mod tests {
    use wiremock::{
        matchers::{header, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use crate::llm::anthropic::AnthropicClient;
    use crate::llm::{ChatRequest, LlmClient, Message, StreamChunk};
    use futures::StreamExt;

    // =========================================================================
    // Test Fixtures
    // =========================================================================

    /// Creates a successful Anthropic messages API response.
    fn success_response_template(id: &str, content: &str) -> ResponseTemplate {
        let body = serde_json::json!({
            "id": id,
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": content}],
            "model": "claude-3-5-sonnet-20241022",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 10,
                "output_tokens": 20
            }
        });
        ResponseTemplate::new(200).set_body_raw(body.to_string(), "application/json")
    }

    /// Creates a streaming response with SSE events.
    fn streaming_response_template(chunks: &[&str]) -> ResponseTemplate {
        let mut sse_data = String::new();

        // Message start event
        sse_data.push_str("data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_123\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[],\"model\":\"claude-3-5-sonnet-20241022\",\"stop_reason\":null,\"stop_sequence\":null,\"usage\":{\"input_tokens\":10,\"output_tokens\":0}}}\n\n");

        // Content block start
        sse_data.push_str("data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n");

        // Content deltas for each chunk
        for chunk in chunks {
            sse_data.push_str(&format!(
                "data: {{\"type\":\"content_block_delta\",\"index\":0,\"delta\":{{\"type\":\"text_delta\",\"text\":\"{}\"}}}}\n\n",
                chunk
            ));
        }

        // Content block stop
        sse_data.push_str("data: {\"type\":\"content_block_stop\",\"index\":0}\n\n");

        // Message delta with stop reason
        sse_data.push_str("data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":20}}\n\n");

        // Message stop
        sse_data.push_str("data: {\"type\":\"message_stop\"}\n\n");

        ResponseTemplate::new(200)
            .append_header("content-type", "text/event-stream")
            .set_body_raw(sse_data, "text/event-stream")
    }

    /// Creates an error response template.
    fn error_response_template(status: u16, message: &str) -> ResponseTemplate {
        let body = serde_json::json!({
            "error": {
                "type": "error",
                "message": message
            }
        });
        ResponseTemplate::new(status).set_body_raw(body.to_string(), "application/json")
    }

    /// Creates a basic chat request for testing.
    fn basic_request() -> ChatRequest {
        ChatRequest::new("claude-3-5-sonnet-20241022", vec![Message::user("Hello")])
            .with_max_tokens(100)
    }

    // =========================================================================
    // Non-Streaming Chat Tests
    // =========================================================================

    #[tokio::test]
    async fn test_chat_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .and(header("x-api-key", "test-api-key"))
            .and(header("anthropic-version", "2023-06-01"))
            .respond_with(success_response_template("msg_123", "Hello, world!"))
            .mount(&mock_server)
            .await;

        let client = AnthropicClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let response = client.chat(basic_request()).await.unwrap();

        assert_eq!(response.id, "msg_123");
        assert_eq!(response.content, "Hello, world!");
        assert_eq!(response.model, "claude-3-5-sonnet-20241022");
        assert_eq!(response.finish_reason, Some("end_turn".to_string()));
        assert!(response.usage.is_some());
        let usage = response.usage.unwrap();
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 20);
        assert_eq!(usage.total_tokens, 30);
    }

    #[tokio::test]
    async fn test_chat_with_system_prompt() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .and(header("x-api-key", "test-api-key"))
            .respond_with(success_response_template("msg_456", "Response"))
            .mount(&mock_server)
            .await;

        let client = AnthropicClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let request = ChatRequest::new(
            "claude-3-5-sonnet-20241022",
            vec![Message::user("Hello")],
        )
        .with_system("You are a helpful assistant.")
        .with_max_tokens(100);

        let response = client.chat(request).await.unwrap();

        assert_eq!(response.content, "Response");
    }

    #[tokio::test]
    async fn test_chat_with_temperature() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(success_response_template("msg_789", "Response"))
            .mount(&mock_server)
            .await;

        let client = AnthropicClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let request = basic_request().with_temperature(0.7);

        let response = client.chat(request).await.unwrap();

        assert_eq!(response.content, "Response");
    }

    #[tokio::test]
    async fn test_chat_authentication_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(error_response_template(401, "Invalid API key"))
            .mount(&mock_server)
            .await;

        let client = AnthropicClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let result = client.chat(basic_request()).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_auth_error());
        assert!(!err.is_retryable());
    }

    #[tokio::test]
    async fn test_chat_rate_limit_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(error_response_template(429, "Rate limit exceeded"))
            .mount(&mock_server)
            .await;

        let client = AnthropicClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let result = client.chat(basic_request()).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(!err.is_rate_limit()); // API errors don't count unless created via rate_limit()
        assert!(!err.is_retryable()); // 429 is not >= 500
    }

    #[tokio::test]
    async fn test_chat_server_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(error_response_template(503, "Service unavailable"))
            .mount(&mock_server)
            .await;

        let client = AnthropicClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let result = client.chat(basic_request()).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_retryable()); // 5xx errors are retryable
    }

    #[tokio::test]
    async fn test_chat_not_found_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(error_response_template(404, "Model not found"))
            .mount(&mock_server)
            .await;

        let client = AnthropicClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let result = client.chat(basic_request()).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(!err.is_retryable()); // 4xx errors are not retryable
    }

    // =========================================================================
    // Streaming Chat Tests
    // =========================================================================

    #[tokio::test]
    async fn test_chat_stream_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(streaming_response_template(&["Hello", " world", "!"]))
            .mount(&mock_server)
            .await;

        let client = AnthropicClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let request = basic_request().with_stream();
        let mut stream = client.chat_stream(request);

        let mut chunks = Vec::new();
        let mut final_chunk = None;

        while let Some(result) = stream.next().await {
            let chunk = result.expect("Stream chunk should be Ok");
            if chunk.is_final {
                final_chunk = Some(chunk);
            } else {
                chunks.push(chunk);
            }
        }

        // We should have received content chunks
        assert!(!chunks.is_empty());
        assert!(final_chunk.is_some());

        let final_chunk = final_chunk.unwrap();
        assert!(final_chunk.is_final);
        assert!(final_chunk.usage.is_some());
        assert_eq!(final_chunk.finish_reason, Some("end_turn".to_string()));
    }

    #[tokio::test]
    async fn test_chat_stream_content_accumulation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(streaming_response_template(&["Hello", " ", "world", "!"]))
            .mount(&mock_server)
            .await;

        let client = AnthropicClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let request = basic_request().with_stream();
        let mut stream = client.chat_stream(request);

        let mut content = String::new();

        while let Some(result) = stream.next().await {
            let chunk = result.expect("Stream chunk should be Ok");
            if let Some(delta) = chunk.delta {
                if let Some(text) = delta.content {
                    content.push_str(&text);
                }
            }
            if chunk.is_final {
                break;
            }
        }

        assert_eq!(content, "Hello world!");
    }

    #[tokio::test]
    async fn test_chat_stream_with_error() {
        let mock_server = MockServer::start().await;

        // SSE error events need proper newlines to be parsed
        let sse_error = r#"data: {"type":"error","error":{"type":"invalid_request_error","message":"Invalid request"}}

"#;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(
                ResponseTemplate::new(200)
                    .append_header("content-type", "text/event-stream")
                    .set_body_raw(sse_error, "text/event-stream"),
            )
            .mount(&mock_server)
            .await;

        let client = AnthropicClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let request = basic_request().with_stream();
        let mut stream = client.chat_stream(request);

        let result = stream.next().await;
        assert!(result.is_some());

        let chunk_result = result.unwrap();
        assert!(chunk_result.is_err());
    }

    #[tokio::test]
    async fn test_chat_stream_http_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(error_response_template(500, "Internal server error"))
            .mount(&mock_server)
            .await;

        let client = AnthropicClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let request = basic_request().with_stream();
        let mut stream = client.chat_stream(request);

        let result = stream.next().await;
        assert!(result.is_some());

        let chunk_result = result.unwrap();
        assert!(chunk_result.is_err());

        let err = chunk_result.unwrap_err();
        assert!(err.is_retryable()); // 5xx errors are retryable
    }

    // =========================================================================
    // API Key Validation Tests
    // =========================================================================

    #[tokio::test]
    async fn test_validate_api_key_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(success_response_template("msg_valid", "Hi"))
            .mount(&mock_server)
            .await;

        let client = AnthropicClient::new(
            "valid-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let result = client.validate_api_key().await.unwrap();

        assert!(result);
    }

    #[tokio::test]
    async fn test_validate_api_key_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(error_response_template(401, "Invalid API key"))
            .mount(&mock_server)
            .await;

        let client = AnthropicClient::new(
            "invalid-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let result = client.validate_api_key().await.unwrap();

        assert!(!result);
    }

    #[tokio::test]
    async fn test_validate_api_key_network_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(error_response_template(500, "Internal server error"))
            .mount(&mock_server)
            .await;

        let client = AnthropicClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let result = client.validate_api_key().await;

        // Should return the error, not false (since it's not an auth error)
        assert!(result.is_err());
    }

    // =========================================================================
    // Models Fetch Tests
    // =========================================================================

    #[tokio::test]
    async fn test_fetch_models_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/models"))
            .and(header("x-api-key", "test-api-key"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(
                    serde_json::json!({
                        "data": [
                            {"id": "claude-3-5-sonnet-20241022"},
                            {"id": "claude-3-5-haiku-20241022"},
                            {"id": "claude-3-opus-20240229"}
                        ]
                    })
                    .to_string(),
                    "application/json"
                ),
            )
            .mount(&mock_server)
            .await;

        let client = AnthropicClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let models = client.fetch_models().await.unwrap();

        assert_eq!(models.len(), 3);
        assert!(models.contains(&"claude-3-5-sonnet-20241022".to_string()));
        assert!(models.contains(&"claude-3-5-haiku-20241022".to_string()));
        assert!(models.contains(&"claude-3-opus-20240229".to_string()));
    }

    #[tokio::test]
    async fn test_fetch_models_failure_returns_defaults() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(
                ResponseTemplate::new(404).set_body_raw(
                    serde_json::json!({
                        "error": {"type": "not_found_error", "message": "Not found"}
                    })
                    .to_string(),
                    "application/json"
                )
            )
            .mount(&mock_server)
            .await;

        let client = AnthropicClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let models = client.fetch_models().await.unwrap();

        // Should return default models when API returns 404
        assert!(!models.is_empty());
        assert!(models.contains(&"claude-3-5-sonnet-20241022".to_string()));
    }

    #[tokio::test]
    async fn test_fetch_models_network_error_returns_defaults() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(
                ResponseTemplate::new(500).set_body_raw(
                    serde_json::json!({
                        "error": {"type": "internal_error", "message": "Internal error"}
                    })
                    .to_string(),
                    "application/json"
                )
            )
            .mount(&mock_server)
            .await;

        let client = AnthropicClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let models = client.fetch_models().await.unwrap();

        // Should return default models on network error
        assert!(!models.is_empty());
    }

    // =========================================================================
    // Tool Call Tests
    // =========================================================================

    #[tokio::test]
    async fn test_chat_with_tool_calls_response() {
        let mock_server = MockServer::start().await;

        let tool_use_response = ResponseTemplate::new(200).set_body_raw(
            serde_json::json!({
                "id": "msg_tool",
                "type": "message",
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "I'll help you search."},
                    {
                        "type": "tool_use",
                        "id": "toolu_1",
                        "name": "search_code",
                        "input": {"query": "test"}
                    }
                ],
                "model": "claude-3-5-sonnet-20241022",
                "stop_reason": "tool_use",
                "usage": {"input_tokens": 15, "output_tokens": 25}
            })
            .to_string(),
            "application/json"
        );

        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(tool_use_response)
            .mount(&mock_server)
            .await;

        let client = AnthropicClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let request = ChatRequest::new(
            "claude-3-5-sonnet-20241022",
            vec![Message::user("Search for test")],
        )
        .with_tools(vec![crate::llm::ToolDefinition {
            name: "search_code".to_string(),
            description: "Search code".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"}
                }
            }),
        }])
        .with_max_tokens(100);

        let response = client.chat(request).await.unwrap();

        assert_eq!(response.content, "I'll help you search.");
        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(response.tool_calls[0].id, "toolu_1");
        assert_eq!(response.tool_calls[0].name, "search_code");
        assert_eq!(response.tool_calls[0].arguments, serde_json::json!({"query": "test"}));
        assert_eq!(response.finish_reason, Some("tool_use".to_string()));
    }
}
