//! LLM-based message summarization for context compaction.
//!
//! When context is being compacted and messages are dropped, this module
//! uses an LLM call to produce a summary of the dropped messages, preserving
//! important decisions, errors, and state information.

use crate::core::llm::entities::ChatMessage as Message;
use crate::infrastructure::llm::LlmClient;
use crate::core::llm::entities::ChatRequest;
use log::warn;

const SUMMARIZER_MAX_INPUT_CHARS: usize = 60_000;
const SUMMARIZER_MAX_OUTPUT_TOKENS: u32 = 1_200;

const SUMMARIZATION_SYSTEM_PROMPT: &str = r#"You are a context compression assistant. Your job is to produce a dense, structured summary of agent execution history. Focus ONLY on information that is still relevant and actionable:

**Include:**
- Key decisions made and the reasoning behind them
- Files that were created, edited, or deleted (include full paths)
- Errors encountered and how they were resolved (or if unresolved)
- The current state of the task and what step the agent was on
- Important tool results that changed the course of action
- Any user clarifications or constraints established during execution

**Exclude:**
- Verbose tool outputs (just note what the tool did)
- Step-by-step reasoning that led nowhere
- Progress updates that are now superseded
- Intermediate debugging or temporary exploration
- Explanations of why the agent is trying things

**Output format:**
Provide your summary in plain text with clear sections. Be concise but complete. Maximum 1200 tokens."#;

/// Summarizes messages that are about to be dropped from context.
///
/// This function uses an LLM call to produce a compact summary of the messages
/// that will be dropped during compaction. The summary captures key decisions,
/// errors, file changes, and task progress that should not be lost.
///
/// # Arguments
///
/// * `client` - The LLM client to use for the summarization call
/// * `model_name` - The model name (passed through for consistency)
/// * `dropped_messages` - The messages being dropped from context
///
/// # Returns
///
/// `Some(Message)` with the summary on success, `None` if the call fails.
/// Failures are logged at warn level but do not interrupt the execution.
pub async fn summarize_dropped_messages(
    client: &dyn LlmClient,
    model_name: &str,
    dropped_messages: &[Message],
) -> Option<Message> {
    if dropped_messages.is_empty() {
        return None;
    }

    let formatted_input = format_messages_for_summarization(dropped_messages);
    if formatted_input.is_empty() {
        return None;
    }

    let request = ChatRequest {
        model: model_name.to_string(),
        messages: vec![
            Message::system(SUMMARIZATION_SYSTEM_PROMPT.to_string()),
            Message::user(format!(
                "Summarize the following execution history:\n\n{}",
                formatted_input
            )),
        ],
        system: None,
        temperature: Some(0.0),
        max_tokens: Some(SUMMARIZER_MAX_OUTPUT_TOKENS),
        top_p: None,
        stop: None,
        stream: false,
        tools: Vec::new(),
    };

    match client.chat(request).await {
        Ok(response) => {
            let summary_text = response
                .content
                .chars()
                .take(4_800) // ~1200 tokens max
                .collect::<String>();

            if summary_text.is_empty() {
                warn!("Summarization call returned empty content");
                return None;
            }

            Some(Message::system(format!(
                "LLM context summary:\n{}",
                summary_text
            )))
        }
        Err(e) => {
            warn!("Failed to summarize dropped messages: {}", e);
            None
        }
    }
}

/// Formats dropped messages into a readable input for the summarizer LLM.
///
/// Converts messages to `[ROLE] content` format, truncating the input to
/// prevent excessive tokens being sent to the summarizer.
fn format_messages_for_summarization(messages: &[Message]) -> String {
    let mut lines = Vec::new();

    for msg in messages {
        let role = match msg.role {
            crate::core::llm::entities::MessageRole::User => "USER",
            crate::core::llm::entities::MessageRole::Assistant => "ASSISTANT",
            crate::core::llm::entities::MessageRole::System => "SYSTEM",
            crate::core::llm::entities::MessageRole::Tool => "TOOL",
        };

        let content = if let Some(text) = msg.as_text() {
            text.to_string()
        } else if !msg.tool_calls.is_empty() {
            let tool_names = msg
                .tool_calls
                .iter()
                .map(|tc| tc.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!("[Tool calls: {}]", tool_names)
        } else if let Some(result) = &msg.tool_result {
            let status = if result.is_error { "ERROR" } else { "SUCCESS" };
            format!("[Tool result: {}]", status)
        } else {
            continue;
        };

        lines.push(format!("[{}] {}", role, content));
    }

    let combined = lines.join("\n");

    // Truncate to max input size
    if combined.len() > SUMMARIZER_MAX_INPUT_CHARS {
        let truncated = combined
            .chars()
            .take(SUMMARIZER_MAX_INPUT_CHARS)
            .collect::<String>();
        format!("{}\n[... truncated ...]", truncated)
    } else {
        combined
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_format_messages_for_summarization_empty() {
        let messages: Vec<Message> = vec![];
        let output = format_messages_for_summarization(&messages);
        assert_eq!(output, "");
    }

    #[test]
    fn test_format_messages_for_summarization_single() {
        let messages = vec![Message::user("Hello, world!")];
        let output = format_messages_for_summarization(&messages);
        assert!(output.contains("USER"));
        assert!(output.contains("Hello, world!"));
    }

    #[test]
    fn test_format_messages_for_summarization_with_tool_calls() {
        let messages = vec![Message::assistant_with_tool_calls(
            "I'll read the file.".to_string(),
            vec![crate::core::llm::entities::ToolCall {
                id: "call_1".to_string(),
                name: "fs_read_file".to_string(),
                arguments: json!({"path": "/test.txt"}),
            }],
        )];
        let output = format_messages_for_summarization(&messages);
        assert!(output.contains("ASSISTANT"));
        // The message has both text and tool calls, so text takes precedence
        assert!(output.contains("I'll read the file."));
    }

    #[test]
    fn test_format_messages_for_summarization_truncation() {
        let long_content = "A".repeat(70_000);
        let messages = vec![Message::user(&long_content)];
        let output = format_messages_for_summarization(&messages);
        assert!(output.contains("truncated"));
        assert!(output.len() < long_content.len());
    }
}
