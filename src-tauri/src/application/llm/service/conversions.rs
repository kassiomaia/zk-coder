use crate::core::config::ApiType;
use crate::core::llm::entities::{
    ChatMessage, ChatRequest as CoreChatRequest, ChatResponse as CoreChatResponse,
    ContentBlock as CoreContentBlock, ImageSource as CoreImageSource, MessageContent,
    MessageRole as CoreMessageRole, ToolCall, UsageStats as CoreUsageStats,
};
use crate::core::llm::repositories::LlmProvider;
use crate::infrastructure::llm::{
    ChatRequest, ChatResponse, ContentBlock, ImageSource, Message,
    MessageContent as LlmMessageContent, MessageRole as LlmMessageRole,
};

use super::LlmServiceImpl;

impl LlmServiceImpl {
    pub(crate) fn convert_message_role(role: CoreMessageRole) -> LlmMessageRole {
        match role {
            CoreMessageRole::System => LlmMessageRole::System,
            CoreMessageRole::User => LlmMessageRole::User,
            CoreMessageRole::Assistant => LlmMessageRole::Assistant,
            CoreMessageRole::Tool => LlmMessageRole::Tool,
        }
    }

    pub(crate) fn convert_to_core_role(role: LlmMessageRole) -> CoreMessageRole {
        match role {
            LlmMessageRole::System => CoreMessageRole::System,
            LlmMessageRole::User => CoreMessageRole::User,
            LlmMessageRole::Assistant => CoreMessageRole::Assistant,
            LlmMessageRole::Tool => CoreMessageRole::Tool,
        }
    }

    pub(crate) fn convert_content_block(block: CoreContentBlock) -> ContentBlock {
        match block {
            CoreContentBlock::Text { text } => ContentBlock::Text { text },
            CoreContentBlock::Image { source } => ContentBlock::Image {
                source: ImageSource {
                    source_type: source.source_type,
                    media_type: source.media_type,
                    data: source.data,
                },
            },
        }
    }

    pub(crate) fn convert_to_core_content_block(block: ContentBlock) -> CoreContentBlock {
        match block {
            ContentBlock::Text { text } => CoreContentBlock::Text { text },
            ContentBlock::Image { source } => CoreContentBlock::Image {
                source: CoreImageSource {
                    source_type: source.source_type,
                    media_type: source.media_type,
                    data: source.data,
                },
            },
        }
    }

    pub(crate) fn convert_message(msg: ChatMessage) -> Message {
        Message {
            role: Self::convert_message_role(msg.role),
            content: match msg.content {
                MessageContent::Text(text) => LlmMessageContent::Text(text),
                MessageContent::Blocks(blocks) => LlmMessageContent::Blocks(
                    blocks
                        .into_iter()
                        .map(Self::convert_content_block)
                        .collect(),
                ),
            },
            tool_calls: msg
                .tool_calls
                .into_iter()
                .map(|tc| crate::core::llm::entities::ToolCall {
                    id: tc.id,
                    name: tc.name,
                    arguments: tc.arguments,
                })
                .collect(),
            tool_result: msg
                .tool_result
                .map(|tr| crate::core::llm::entities::ToolResult {
                    call_id: tr.call_id,
                    output: tr.output,
                    is_error: tr.is_error,
                }),
        }
    }

    pub(crate) fn convert_to_core_message(msg: Message) -> ChatMessage {
        ChatMessage {
            role: Self::convert_to_core_role(msg.role),
            content: match msg.content {
                LlmMessageContent::Text(text) => MessageContent::Text(text),
                LlmMessageContent::Blocks(blocks) => MessageContent::Blocks(
                    blocks
                        .into_iter()
                        .map(Self::convert_to_core_content_block)
                        .collect(),
                ),
            },
            tool_calls: msg
                .tool_calls
                .into_iter()
                .map(|tc| ToolCall {
                    id: tc.id,
                    name: tc.name,
                    arguments: tc.arguments,
                })
                .collect(),
            tool_result: msg
                .tool_result
                .map(|tr| crate::core::llm::entities::ToolResult {
                    call_id: tr.call_id,
                    output: tr.output,
                    is_error: tr.is_error,
                }),
        }
    }

    pub(crate) fn convert_request(req: CoreChatRequest) -> ChatRequest {
        ChatRequest {
            model: req.model,
            messages: req
                .messages
                .into_iter()
                .map(Self::convert_message)
                .collect(),
            system: req.system,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            top_p: req.top_p,
            stop: req.stop,
            stream: req.stream,
            tools: req
                .tools
                .into_iter()
                .map(|t| crate::core::llm::entities::ToolDefinition {
                    name: t.name,
                    description: t.description,
                    input_schema: t.input_schema,
                })
                .collect(),
        }
    }

    pub(crate) fn convert_to_core_response(resp: ChatResponse) -> CoreChatResponse {
        CoreChatResponse {
            id: resp.id,
            model: resp.model,
            content: resp.content,
            usage: resp.usage.map(|u| CoreUsageStats {
                input_tokens: u.input_tokens,
                output_tokens: u.output_tokens,
                total_tokens: u.total_tokens,
            }),
            finish_reason: resp.finish_reason,
            tool_calls: resp
                .tool_calls
                .into_iter()
                .map(|tc| ToolCall {
                    id: tc.id,
                    name: tc.name,
                    arguments: tc.arguments,
                })
                .collect(),
        }
    }

    pub(crate) fn convert_provider(provider: LlmProvider) -> ApiType {
        match provider {
            LlmProvider::OpenAI => ApiType::OpenAI,
            LlmProvider::Anthropic => ApiType::Anthropic,
            LlmProvider::Google => ApiType::Google,
            LlmProvider::Custom { .. } => ApiType::OpenAI,
        }
    }

    pub(crate) fn get_default_model_for_provider(api_type: ApiType) -> Option<String> {
        match api_type {
            ApiType::OpenAI => Some("gpt-4o".to_string()),
            ApiType::Anthropic => Some("claude-3-5-sonnet-20241022".to_string()),
            ApiType::Google => Some("gemini-2.0-flash-exp".to_string()),
        }
    }
}
