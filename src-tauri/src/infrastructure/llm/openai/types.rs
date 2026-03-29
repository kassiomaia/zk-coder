//! OpenAI API types.

use serde::{Deserialize, Serialize};

/// OpenAI chat completion request.
#[derive(Debug, Serialize)]
pub struct OpenAiRequest {
    /// Model identifier to use for the request.
    pub model: String,
    /// List of messages in the conversation.
    pub messages: Vec<OpenAiMessage>,
    /// Sampling temperature (0.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Maximum number of tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Maximum number of tokens for completion only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    /// Nucleus sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Custom stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// Whether to stream the response.
    pub stream: bool,
    /// Available tools for the model to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<OpenAiRequestTool>>,
    /// Tool selection strategy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
}

/// OpenAI tool definition in chat completion requests.
#[derive(Debug, Serialize)]
pub struct OpenAiRequestTool {
    /// Type of the tool (currently only "function").
    #[serde(rename = "type")]
    pub tool_type: String,
    /// Function definition.
    pub function: OpenAiRequestFunction,
}

/// OpenAI function definition in request tools.
#[derive(Debug, Serialize)]
pub struct OpenAiRequestFunction {
    /// Name of the function.
    pub name: String,
    /// Description of what the function does.
    pub description: String,
    /// JSON schema for the function's parameters.
    pub parameters: serde_json::Value,
}

/// OpenAI message format.
#[derive(Debug, Serialize)]
pub struct OpenAiMessage {
    /// Role of the message sender (e.g., "system", "user", "assistant").
    pub role: String,
    /// Content of the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// ID of the tool call this message is responding to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Tool calls made by the assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAiRequestMessageToolCall>>,
}

/// OpenAI assistant tool-call payload in request messages.
#[derive(Debug, Serialize)]
pub struct OpenAiRequestMessageToolCall {
    /// Unique identifier for the tool call.
    pub id: String,
    /// Type of the tool call (e.g., "function").
    #[serde(rename = "type")]
    pub tool_type: String,
    /// Function to call.
    pub function: OpenAiRequestMessageToolFunction,
}

/// OpenAI assistant function payload in request messages.
#[derive(Debug, Serialize)]
pub struct OpenAiRequestMessageToolFunction {
    /// Name of the function to call.
    pub name: String,
    /// JSON-encoded arguments for the function.
    pub arguments: String,
}

/// OpenAI chat completion response.
#[derive(Debug, Deserialize)]
pub struct OpenAiResponse {
    /// Unique response identifier.
    pub id: String,
    /// Model used for the request.
    pub model: String,
    /// List of completion choices.
    pub choices: Vec<OpenAiChoice>,
    /// Usage statistics.
    #[serde(default)]
    pub usage: Option<OpenAiUsage>,
}

/// OpenAI response choice.
#[derive(Debug, Deserialize)]
pub struct OpenAiChoice {
    /// Index of this choice.
    #[allow(dead_code)]
    pub index: u32,
    /// The message content (for non-streaming responses).
    pub message: Option<OpenAiResponseMessage>,
    /// The delta content (for streaming responses).
    pub delta: Option<OpenAiDelta>,
    /// Reason why the completion stopped.
    #[serde(default)]
    pub finish_reason: Option<String>,
}

/// OpenAI response message.
#[derive(Debug, Deserialize)]
pub struct OpenAiResponseMessage {
    /// Role of the message sender.
    #[serde(default)]
    #[allow(dead_code)]
    pub role: Option<String>,
    /// Content of the message.
    #[serde(default)]
    pub content: Option<String>,
    /// Reasoning content from models like GLM-5 that show thinking before answering.
    #[serde(default, rename = "reasoning_content")]
    pub reasoning_content: Option<String>,
    /// Tool calls made by the assistant.
    #[serde(default)]
    pub tool_calls: Option<Vec<OpenAiToolCall>>,
}

/// OpenAI tool-call payload in chat completion responses.
#[derive(Debug, Deserialize)]
pub struct OpenAiToolCall {
    /// Unique identifier for the tool call.
    pub id: String,
    /// Type of the tool call.
    #[allow(dead_code)]
    #[serde(rename = "type")]
    pub tool_type: String,
    /// Function to call.
    pub function: OpenAiToolFunction,
}

/// OpenAI function payload for a tool call.
#[derive(Debug, Deserialize)]
pub struct OpenAiToolFunction {
    /// Name of the function to call.
    pub name: String,
    /// JSON-encoded arguments for the function.
    pub arguments: String,
}

/// OpenAI streaming delta.
#[derive(Debug, Deserialize)]
pub struct OpenAiDelta {
    /// Role of the message sender.
    #[serde(default)]
    pub role: Option<String>,
    /// Content delta.
    #[serde(default)]
    pub content: Option<String>,
    /// Reasoning content from models like GLM-5 that show thinking before answering.
    #[serde(default, rename = "reasoning_content")]
    pub reasoning_content: Option<String>,
}

/// OpenAI usage statistics.
#[derive(Debug, Deserialize)]
pub struct OpenAiUsage {
    /// Number of tokens in the prompt.
    pub prompt_tokens: u32,
    /// Number of tokens in the completion.
    pub completion_tokens: u32,
    /// Total number of tokens.
    #[serde(default)]
    pub total_tokens: u32,
}

/// OpenAI streaming response.
#[derive(Debug, Deserialize)]
pub struct OpenAiStreamResponse {
    /// Unique response identifier.
    pub id: String,
    /// Model used for the request.
    pub model: String,
    /// List of completion choices.
    pub choices: Vec<OpenAiChoice>,
    /// Usage statistics.
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
