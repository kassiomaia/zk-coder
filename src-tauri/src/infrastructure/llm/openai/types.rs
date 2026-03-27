//! OpenAI API types.

use serde::{Deserialize, Serialize};

/// OpenAI chat completion request.
#[derive(Debug, Serialize)]
pub struct OpenAiRequest {
    pub model: String,
    pub messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<OpenAiRequestTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
}

/// OpenAI tool definition in chat completion requests.
#[derive(Debug, Serialize)]
pub struct OpenAiRequestTool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: OpenAiRequestFunction,
}

/// OpenAI function definition in request tools.
#[derive(Debug, Serialize)]
pub struct OpenAiRequestFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// OpenAI message format.
#[derive(Debug, Serialize)]
pub struct OpenAiMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAiRequestMessageToolCall>>,
}

/// OpenAI assistant tool-call payload in request messages.
#[derive(Debug, Serialize)]
pub struct OpenAiRequestMessageToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: OpenAiRequestMessageToolFunction,
}

/// OpenAI assistant function payload in request messages.
#[derive(Debug, Serialize)]
pub struct OpenAiRequestMessageToolFunction {
    pub name: String,
    pub arguments: String,
}

/// OpenAI chat completion response.
#[derive(Debug, Deserialize)]
pub struct OpenAiResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<OpenAiChoice>,
    #[serde(default)]
    pub usage: Option<OpenAiUsage>,
}

/// OpenAI response choice.
#[derive(Debug, Deserialize)]
pub struct OpenAiChoice {
    #[allow(dead_code)]
    pub index: u32,
    pub message: Option<OpenAiResponseMessage>,
    pub delta: Option<OpenAiDelta>,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

/// OpenAI response message.
#[derive(Debug, Deserialize)]
pub struct OpenAiResponseMessage {
    #[serde(default)]
    #[allow(dead_code)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    /// Reasoning content from models like GLM-5 that show thinking before answering
    #[serde(default, rename = "reasoning_content")]
    pub reasoning_content: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<OpenAiToolCall>>,
}

/// OpenAI tool-call payload in chat completion responses.
#[derive(Debug, Deserialize)]
pub struct OpenAiToolCall {
    pub id: String,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: OpenAiToolFunction,
}

/// OpenAI function payload for a tool call.
#[derive(Debug, Deserialize)]
pub struct OpenAiToolFunction {
    pub name: String,
    pub arguments: String,
}

/// OpenAI streaming delta.
#[derive(Debug, Deserialize)]
pub struct OpenAiDelta {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    /// Reasoning content from models like GLM-5 that show thinking before answering
    #[serde(default, rename = "reasoning_content")]
    pub reasoning_content: Option<String>,
}

/// OpenAI usage statistics.
#[derive(Debug, Deserialize)]
pub struct OpenAiUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    #[serde(default)]
    pub total_tokens: u32,
}

/// OpenAI streaming response.
#[derive(Debug, Deserialize)]
pub struct OpenAiStreamResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<OpenAiChoice>,
    #[serde(default)]
    pub usage: Option<OpenAiUsage>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_response_deserializes_tool_calls() {
        let json = r#"{
            "id": "chatcmpl-tool-1",
            "model": "gpt-4.1",
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
                            "arguments": "{\"query\":\"runner\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 2,
                "total_tokens": 12
            }
        }"#;

        let parsed: OpenAiResponse = serde_json::from_str(json).expect("valid OpenAI response");
        let message = parsed
            .choices
            .first()
            .and_then(|c| c.message.as_ref())
            .expect("message should exist");
        let tool_calls = message
            .tool_calls
            .as_ref()
            .expect("tool calls should exist");
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "call_1");
        assert_eq!(tool_calls[0].function.name, "search_code");
    }

    #[test]
    fn test_openai_response_with_reasoning_content() {
        // Test GLM-5 style response with reasoning_content
        let json = r#"{
            "id": "chatcmpl-glm-1",
            "model": "glm-5",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello!",
                    "reasoning_content": "Let me think about how to respond..."
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 8,
                "completion_tokens": 10,
                "total_tokens": 18
            }
        }"#;

        let parsed: OpenAiResponse = serde_json::from_str(json).expect("valid GLM response");
        let message = parsed
            .choices
            .first()
            .and_then(|c| c.message.as_ref())
            .expect("message should exist");

        assert_eq!(message.content, Some("Hello!".to_string()));
        assert_eq!(
            message.reasoning_content,
            Some("Let me think about how to respond...".to_string())
        );
    }

    #[test]
    fn test_openai_delta_with_reasoning_content() {
        // Test GLM-5 style streaming delta with reasoning_content
        let json = r#"{
            "id": "chatcmpl-glm-2",
            "model": "glm-5",
            "choices": [{
                "index": 0,
                "delta": {
                    "role": "assistant",
                    "reasoning_content": "Thinking..."
                }
            }]
        }"#;

        let parsed: OpenAiStreamResponse = serde_json::from_str(json).expect("valid GLM stream");
        let delta = parsed
            .choices
            .first()
            .and_then(|c| c.delta.as_ref())
            .expect("delta should exist");

        assert_eq!(delta.role, Some("assistant".to_string()));
        assert_eq!(delta.content, None);
        assert_eq!(delta.reasoning_content, Some("Thinking...".to_string()));
    }

    #[test]
    fn test_openai_delta_with_content_and_reasoning() {
        // Test delta with both content and reasoning_content
        let json = r#"{
            "id": "chatcmpl-glm-3",
            "model": "glm-5",
            "choices": [{
                "index": 0,
                "delta": {
                    "content": "Response",
                    "reasoning_content": "Thinking step"
                }
            }]
        }"#;

        let parsed: OpenAiStreamResponse = serde_json::from_str(json).expect("valid GLM stream");
        let delta = parsed
            .choices
            .first()
            .and_then(|c| c.delta.as_ref())
            .expect("delta should exist");

        assert_eq!(delta.content, Some("Response".to_string()));
        assert_eq!(delta.reasoning_content, Some("Thinking step".to_string()));
    }
}
