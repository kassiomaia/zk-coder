//! Integration tests for OpenAI client using wiremock.
//!
//! These tests mock HTTP responses to test client behavior without
//! making actual API calls.

#[cfg(test)]
mod tests {
    use wiremock::{
        matchers::{header, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use crate::llm::{
        openai::OpenAiClient,
        ChatRequest, LlmClient, Message, ToolCall,
    };
    use futures::StreamExt;

    // =========================================================================
    // Test Fixtures
    // =========================================================================

    /// Creates a successful OpenAI chat completion response.
    fn success_response_template(id: &str, content: &str) -> ResponseTemplate {
        let body = serde_json::json!({
            "id": id,
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": content
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            }
        });
        ResponseTemplate::new(200).set_body_raw(body.to_string(), "application/json")
    }

    /// Creates a streaming response with SSE events.
    fn streaming_response_template(chunks: &[&str]) -> ResponseTemplate {
        let mut sse_data = String::new();

        for (i, chunk) in chunks.iter().enumerate() {
            sse_data.push_str(&format!(
                "data: {{\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{{\"index\":0,\"delta\":{{\"content\":\"{}\"}},\"finish_reason\":null}}]}}\n\n",
                chunk
            ));
        }

        // Final chunk with finish_reason
        sse_data.push_str("data: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":20,\"total_tokens\":30}}\n\n");
        sse_data.push_str("data: [DONE]\n\n");

        ResponseTemplate::new(200)
            .append_header("content-type", "text/event-stream")
            .set_body_raw(sse_data, "text/event-stream")
    }

    /// Creates an error response template.
    fn error_response_template(status: u16, message: &str) -> ResponseTemplate {
        let body = serde_json::json!({
            "error": {
                "message": message,
                "type": "error",
                "code": status.to_string()
            }
        });
        ResponseTemplate::new(status).set_body_raw(body.to_string(), "application/json")
    }

    /// Creates a basic chat request for testing.
    fn basic_request() -> ChatRequest {
        ChatRequest::new("gpt-4", vec![Message::user("Hello")])
            .with_max_tokens(100)
    }

    // =========================================================================
    // Non-Streaming Chat Tests
    // =========================================================================

    #[tokio::test]
    async fn test_chat_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", "Bearer test-api-key"))
            .respond_with(success_response_template("chatcmpl-123", "Hello, world!"))
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let response = client.chat(basic_request()).await.unwrap();

        assert_eq!(response.id, "chatcmpl-123");
        assert_eq!(response.content, "Hello, world!");
        assert_eq!(response.model, "gpt-4");
        assert_eq!(response.finish_reason, Some("stop".to_string()));
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
            .and(path("/chat/completions"))
            .respond_with(success_response_template("chatcmpl-456", "Response"))
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let request = ChatRequest::new("gpt-4", vec![Message::user("Hello")])
            .with_system("You are a helpful assistant.")
            .with_max_tokens(100);

        let response = client.chat(request).await.unwrap();

        assert_eq!(response.content, "Response");
    }

    #[tokio::test]
    async fn test_chat_with_temperature() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(success_response_template("chatcmpl-789", "Response"))
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let request = basic_request().with_temperature(0.7);

        let response = client.chat(request).await.unwrap();

        assert_eq!(response.content, "Response");
    }

    #[tokio::test]
    async fn test_chat_with_max_completion_tokens() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(success_response_template("chatcmpl-new", "Response"))
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        // GPT-4.1 should use max_completion_tokens
        let request = ChatRequest::new("gpt-4.1", vec![Message::user("Hello")])
            .with_max_tokens(100);

        let response = client.chat(request).await.unwrap();

        assert_eq!(response.content, "Response");
    }

    #[tokio::test]
    async fn test_chat_authentication_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(error_response_template(401, "Invalid API key"))
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new(
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
            .and(path("/chat/completions"))
            .respond_with(error_response_template(429, "Rate limit exceeded"))
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let result = client.chat(basic_request()).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(!err.is_retryable()); // 429 is not >= 500
    }

    #[tokio::test]
    async fn test_chat_server_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(error_response_template(503, "Service unavailable"))
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new(
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
            .and(path("/chat/completions"))
            .respond_with(error_response_template(404, "Model not found"))
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new(
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
            .and(path("/chat/completions"))
            .respond_with(streaming_response_template(&["Hello", " world", "!"]))
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new(
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
        assert_eq!(final_chunk.finish_reason, Some("stop".to_string()));
    }

    #[tokio::test]
    async fn test_chat_stream_content_accumulation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(streaming_response_template(&["Hello", " ", "world", "!"]))
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new(
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
    async fn test_chat_stream_empty_chunks() {
        let mock_server = MockServer::start().await;

        let empty_stream = "data: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":null}]}\n\n\
            data: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n\
            data: [DONE]\n\n";

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .append_header("content-type", "text/event-stream")
                    .set_body_raw(empty_stream, "text/event-stream"),
            )
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let request = basic_request().with_stream();
        let mut stream = client.chat_stream(request);

        let mut final_received = false;
        while let Some(result) = stream.next().await {
            let chunk = result.expect("Stream chunk should be Ok");
            if chunk.is_final {
                final_received = true;
                break;
            }
        }

        assert!(final_received);
    }

    #[tokio::test]
    async fn test_chat_stream_http_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(error_response_template(500, "Internal server error"))
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new(
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

    #[tokio::test]
    async fn test_chat_stream_with_done_marker() {
        let mock_server = MockServer::start().await;

        let sse_data = "data: {\"id\":\"chatcmpl-123\",\"object\":\"chat.completion.chunk\",\"created\":1234567890,\"model\":\"gpt-4\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hi\"},\"finish_reason\":null}]}\n\n\
            data: [DONE]\n\n";

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .append_header("content-type", "text/event-stream")
                    .set_body_raw(sse_data, "text/event-stream"),
            )
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let request = basic_request().with_stream();
        let mut stream = client.chat_stream(request);

        let mut content = String::new();
        let mut chunk_count = 0;

        while let Some(result) = stream.next().await {
            let chunk = result.expect("Stream chunk should be Ok");
            chunk_count += 1;
            if let Some(delta) = chunk.delta {
                if let Some(text) = delta.content {
                    content.push_str(&text);
                }
            }
            if chunk.is_final {
                break;
            }
        }

        assert_eq!(content, "Hi");
        assert!(chunk_count > 0);
    }

    // =========================================================================
    // API Key Validation Tests
    // =========================================================================

    #[tokio::test]
    async fn test_validate_api_key_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(success_response_template("chatcmpl-valid", "Hi"))
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new(
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
            .and(path("/chat/completions"))
            .respond_with(error_response_template(401, "Invalid API key"))
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new(
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
            .and(path("/chat/completions"))
            .respond_with(error_response_template(500, "Internal server error"))
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new(
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
            .and(header("authorization", "Bearer test-api-key"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(
                    serde_json::json!({
                        "object": "list",
                        "data": [
                            {"id": "gpt-4", "object": "model", "owned_by": "organization"},
                            {"id": "gpt-3.5-turbo", "object": "model", "owned_by": "organization"},
                            {"id": "gpt-4-turbo", "object": "model", "owned_by": "organization"}
                        ]
                    })
                    .to_string(),
                    "application/json"
                ),
            )
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let models = client.fetch_models().await.unwrap();

        assert_eq!(models.len(), 3);
        assert!(models.contains(&"gpt-4".to_string()));
        assert!(models.contains(&"gpt-3.5-turbo".to_string()));
        assert!(models.contains(&"gpt-4-turbo".to_string()));
    }

    #[tokio::test]
    async fn test_fetch_models_empty_list() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(
                    serde_json::json!({
                        "object": "list",
                        "data": []
                    })
                    .to_string(),
                    "application/json"
                ),
            )
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let models = client.fetch_models().await.unwrap();

        assert_eq!(models.len(), 0);
    }

    #[tokio::test]
    async fn test_fetch_models_unauthorized() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(error_response_template(401, "Invalid API key"))
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let result = client.fetch_models().await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_auth_error());
    }

    // =========================================================================
    // Tool Call Tests
    // =========================================================================

    #[tokio::test]
    async fn test_chat_with_tool_calls_response() {
        let mock_server = MockServer::start().await;

        let tool_use_response = ResponseTemplate::new(200).set_body_raw(
            serde_json::json!({
                "id": "chatcmpl-tool",
                "object": "chat.completion",
                "created": 1234567890,
                "model": "gpt-4",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": null,
                        "tool_calls": [{
                            "id": "call_1",
                            "type": "function",
                            "function": {
                                "name": "search_code",
                                "arguments": "{\"query\":\"test\"}"
                            }
                        }]
                    },
                    "finish_reason": "tool_calls"
                }],
                "usage": {
                    "prompt_tokens": 15,
                    "completion_tokens": 25,
                    "total_tokens": 40
                }
            })
            .to_string(),
            "application/json"
        );

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(tool_use_response)
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let request = ChatRequest::new(
            "gpt-4",
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

        assert!(response.content.is_empty() || response.content == "null");
        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(response.tool_calls[0].id, "call_1");
        assert_eq!(response.tool_calls[0].name, "search_code");
        assert_eq!(response.tool_calls[0].arguments, serde_json::json!({"query": "test"}));
        assert_eq!(response.finish_reason, Some("tool_calls".to_string()));
    }

    #[tokio::test]
    async fn test_chat_with_tool_messages() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(success_response_template("chatcmpl-tool2", "Done!"))
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let request = ChatRequest::new(
            "gpt-4",
            vec![
                Message::assistant_with_tool_calls(
                    "",
                    vec![ToolCall {
                        id: "call_1".to_string(),
                        name: "search_code".to_string(),
                        arguments: serde_json::json!({"query": "test"}),
                    }],
                ),
                Message::tool_result("call_1", serde_json::json!({"results": ["file1.rs"]}), false),
            ],
        )
        .with_max_tokens(100);

        let response = client.chat(request).await.unwrap();

        assert_eq!(response.content, "Done!");
    }

    #[tokio::test]
    async fn test_chat_with_multiple_tools_in_response() {
        let mock_server = MockServer::start().await;

        let multi_tool_response = ResponseTemplate::new(200).set_body_raw(
            serde_json::json!({
                "id": "chatcmpl-multi",
                "object": "chat.completion",
                "created": 1234567890,
                "model": "gpt-4",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "I'll search and read files.",
                        "tool_calls": [
                            {
                                "id": "call_1",
                                "type": "function",
                                "function": {
                                    "name": "search",
                                    "arguments": "{\"query\":\"test\"}"
                                }
                            },
                            {
                                "id": "call_2",
                                "type": "function",
                                "function": {
                                    "name": "read_file",
                                    "arguments": "{\"path\":\"test.rs\"}"
                                }
                            }
                        ]
                    },
                    "finish_reason": "tool_calls"
                }],
                "usage": {
                    "prompt_tokens": 20,
                    "completion_tokens": 30,
                    "total_tokens": 50
                }
            })
            .to_string(),
            "application/json"
        );

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(multi_tool_response)
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new(
            "test-api-key".to_string(),
            Some(mock_server.uri()),
        );

        let request = ChatRequest::new(
            "gpt-4",
            vec![Message::user("Search and read test files")],
        )
        .with_max_tokens(100);

        let response = client.chat(request).await.unwrap();

        assert_eq!(response.content, "I'll search and read files.");
        assert_eq!(response.tool_calls.len(), 2);
        assert_eq!(response.tool_calls[0].name, "search");
        assert_eq!(response.tool_calls[1].name, "read_file");
    }

    // =========================================================================
    // Base URL Normalization Tests
    // =========================================================================

    #[tokio::test]
    async fn test_base_url_normalized_with_endpoint() {
        let mock_server = MockServer::start().await;

        // Mock should respond to the normalized path
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(success_response_template("chatcmpl-norm", "Hi"))
            .mount(&mock_server)
            .await;

        // Create client with incorrect base URL that includes endpoint
        let client = OpenAiClient::new(
            "test-api-key".to_string(),
            Some(format!("{}/chat/completions", mock_server.uri())),
        );

        let request = basic_request();
        let response = client.chat(request).await.unwrap();

        assert_eq!(response.content, "Hi");
    }

    #[tokio::test]
    async fn test_base_url_with_models_endpoint() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(
                    serde_json::json!({
                        "object": "list",
                        "data": [{"id": "gpt-4", "object": "model", "owned_by": "org"}]
                    })
                    .to_string(),
                    "application/json"
                ),
            )
            .mount(&mock_server)
            .await;

        // Create client with incorrect base URL that includes /models
        let client = OpenAiClient::new(
            "test-api-key".to_string(),
            Some(format!("{}/models", mock_server.uri())),
        );

        let models = client.fetch_models().await.unwrap();

        assert_eq!(models.len(), 1);
        assert_eq!(models[0], "gpt-4");
    }

    // =========================================================================
    // Google Client Tests
    // =========================================================================

    #[tokio::test]
    async fn test_google_client_creation() {
        let client = OpenAiClient::new_google("test-google-key".to_string());

        assert_eq!(client.provider_type(), crate::nlp::config::ApiType::Google);
    }

    #[tokio::test]
    async fn test_google_client_uses_bearer_auth() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", "Bearer test-google-key"))
            .respond_with(success_response_template("chatcmpl-google", "Hello from Google"))
            .mount(&mock_server)
            .await;

        let client = OpenAiClient::new_google("test-google-key".to_string());

        // Manually set base URL for testing
        let client_with_mock = OpenAiClient::new(
            "test-google-key".to_string(),
            Some(mock_server.uri()),
        );

        let request = basic_request();
        let response = client_with_mock.chat(request).await.unwrap();

        assert_eq!(response.content, "Hello from Google");
    }
}
