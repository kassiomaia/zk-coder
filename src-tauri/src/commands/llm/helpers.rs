use crate::application::llm::get_code_operation_model;
use crate::core::chat::entities::{ChatMessage, MessageFeedback, MessageRole};
use crate::core::config::ConfigRepository;
use crate::core::llm::entities::{
    ChatMessage as LlmMessage, MessageContent, MessageRole as CoreMessageRole,
};

/// System prompt explaining the feedback system.
pub const FEEDBACK_SYSTEM_PROMPT: &str = r#"The user may provide feedback (like/dislike) on earlier assistant responses. A feedback summary section will be provided, including stable IDs for reference. Use that feedback to improve your response quality and avoid repeating disliked patterns."#;

/// Maximum number of feedback items to include in context.
pub const FEEDBACK_LIMIT: usize = 5;

/// Resolves the model ID to use for a chat request.
pub async fn resolve_model_id(
    model_id: Option<String>,
    config_repository: &dyn ConfigRepository,
) -> Result<String, String> {
    if let Some(id) = model_id {
        return Ok(id);
    }

    let model = get_code_operation_model(config_repository)?;
    Ok(model.id)
}

/// Converts chat messages from storage to LLM format.
pub fn convert_messages(messages: &[ChatMessage]) -> Vec<LlmMessage> {
    messages
        .iter()
        .map(|msg| LlmMessage {
            role: match msg.role {
                MessageRole::User => CoreMessageRole::User,
                MessageRole::Assistant => CoreMessageRole::Assistant,
                MessageRole::System => CoreMessageRole::System,
            },
            content: MessageContent::Text(msg.content.clone()),
            tool_calls: Vec::new(),
            tool_result: None,
        })
        .collect()
}

/// Converts storage ChatMessage to core ChatMessage.
pub fn convert_to_core_message(msg: ChatMessage) -> crate::core::llm::entities::ChatMessage {
    crate::core::llm::entities::ChatMessage {
        role: match msg.role {
            MessageRole::User => CoreMessageRole::User,
            MessageRole::Assistant => CoreMessageRole::Assistant,
            MessageRole::System => CoreMessageRole::System,
        },
        content: MessageContent::Text(msg.content),
        tool_calls: Vec::new(),
        tool_result: None,
    }
}

/// Builds a feedback summary string referencing message IDs.
pub fn build_feedback_summary(
    feedback_messages: &[(ChatMessage, MessageFeedback)],
) -> Option<String> {
    if feedback_messages.is_empty() {
        return None;
    }

    let feedback_lines: Vec<String> = feedback_messages
        .iter()
        .map(|(msg, feedback)| {
            let short_id = &msg.id[..8.min(msg.id.len())];
            let action = match feedback {
                MessageFeedback::Like => "LIKED",
                MessageFeedback::Dislike => "DISLIKED",
            };
            format!("- [ID:{}] {} by user", short_id, action)
        })
        .collect();

    Some(format!(
        "Feedback on previous messages:\n{}",
        feedback_lines.join("\n")
    ))
}

/// Builds the combined system prompt including feedback context.
pub fn build_system_prompt_with_feedback(
    original_prompt: &str,
    feedback_summary: Option<&str>,
) -> String {
    match feedback_summary {
        Some(summary) => {
            if original_prompt.is_empty() {
                format!(
                    "{}\n\n{}\n\n{}",
                    FEEDBACK_SYSTEM_PROMPT, summary, "Now continue the conversation below:"
                )
            } else {
                format!(
                    "{}\n\n{}\n\n{}\n\n{}",
                    original_prompt,
                    FEEDBACK_SYSTEM_PROMPT,
                    summary,
                    "Now continue the conversation below:"
                )
            }
        }
        None => original_prompt.to_string(),
    }
}
