//! Tests for the token estimator and budget constants.

use serde_json::json;

use crate::core::llm::entities::{ChatMessage as Message, ContentBlock, MessageRole, ToolCall};

use super::super::{
    estimate_context_tokens, estimate_message_tokens, COMPRESSION_THRESHOLD, CONTEXT_WINDOW,
    IMAGE_TOKEN_COST, KEEP_RECENT_MESSAGES, PER_MESSAGE_OVERHEAD,
};
use super::common::large_json_string;

#[test]
fn test_constants() {
    assert_eq!(CONTEXT_WINDOW, 128_000);
    assert_eq!(COMPRESSION_THRESHOLD, 0.60);
    assert_eq!(KEEP_RECENT_MESSAGES, 15);
}

#[test]
fn test_estimate_context_tokens_empty() {
    let context: Vec<Message> = vec![];
    assert_eq!(estimate_context_tokens(&context), 0);
}

#[test]
fn test_estimate_context_tokens_single_message() {
    let context = vec![Message::user("Hello, world!")];
    let estimate = estimate_context_tokens(&context);
    assert!(estimate > 0);
    // ~13 chars / 4 = ~3, +10 overhead = ~13
    assert!(estimate < 20); // Should be reasonable
}

#[test]
fn test_estimate_context_tokens_multiple_messages() {
    let context = vec![
        Message::system("You are a helpful assistant."),
        Message::user("Hello!"),
        Message::assistant("Hi there!"),
    ];
    let estimate = estimate_context_tokens(&context);
    assert!(estimate > 0);
    // Should be roughly sum of all content / 4 + overhead
    assert!(estimate < 100); // Should be reasonable for short messages
}

#[test]
fn test_estimate_context_tokens_with_long_content() {
    let long_content = "A".repeat(1000);
    let context = vec![Message::user(&long_content)];
    let estimate = estimate_context_tokens(&context);
    // ~1000 chars / 4 = ~250, +10 overhead = ~260
    assert!(estimate > 200);
    assert!(estimate < 300);
}

#[test]
fn test_estimate_context_tokens_with_tool_calls() {
    let context = vec![Message::assistant_with_tool_calls(
        "I'll read the file.".to_string(),
        vec![ToolCall {
            id: "call_1".to_string(),
            name: "fs_read_file".to_string(),
            arguments: json!({"path": "/test.txt"}),
        }],
    )];

    let estimate = estimate_context_tokens(&context);
    assert!(estimate > 0);
}

#[test]
fn test_estimate_counts_tool_call_arguments() {
    let big_payload = large_json_string(4_000);
    let msg_with = Message::assistant_with_tool_calls(
        "text".to_string(),
        vec![ToolCall {
            id: "call_1".into(),
            name: "fs_write".into(),
            arguments: json!({ "contents": big_payload }),
        }],
    );
    let msg_without = Message::assistant("text");

    let with_tokens = estimate_message_tokens(&msg_with);
    let without_tokens = estimate_message_tokens(&msg_without);

    // The 4KB JSON payload should contribute ~1000 tokens under len/4.
    assert!(
        with_tokens >= without_tokens + 900,
        "tool-call JSON should be counted (with={with_tokens}, without={without_tokens})"
    );
}

#[test]
fn test_estimate_counts_tool_result_output() {
    let big_output = json!({ "data": large_json_string(4_000) });
    let tool_msg = Message::tool_result("call_1", big_output, false);
    let tokens = estimate_message_tokens(&tool_msg);

    // Old estimator would have returned exactly PER_MESSAGE_OVERHEAD (10).
    assert!(
        tokens > PER_MESSAGE_OVERHEAD + 900,
        "tool-result JSON should be counted, got {tokens}"
    );
}

#[test]
fn test_estimate_counts_blocks_content() {
    let msg = Message::multimodal(
        MessageRole::User,
        vec![
            ContentBlock::text("A".repeat(400)),
            ContentBlock::text("B".repeat(400)),
        ],
    );
    let tokens = estimate_message_tokens(&msg);
    // Old estimator returned 0 content for Blocks; with ~800 chars it should be ~200 tokens.
    assert!(
        tokens > 150,
        "text inside Blocks must be counted, got {tokens}"
    );
}

#[test]
fn test_estimate_image_block_has_fixed_cost() {
    let huge_base64 = "A".repeat(100_000);
    let msg = Message::multimodal(
        MessageRole::User,
        vec![ContentBlock::image_base64("image/png", huge_base64)],
    );
    let tokens = estimate_message_tokens(&msg);
    // Image cost is flat — not proportional to base64 length.
    let lower = PER_MESSAGE_OVERHEAD + IMAGE_TOKEN_COST;
    let upper = PER_MESSAGE_OVERHEAD + IMAGE_TOKEN_COST + 10;
    assert!(
        (lower..=upper).contains(&tokens),
        "image cost should be fixed near IMAGE_TOKEN_COST, got {tokens}"
    );
}

#[test]
fn test_context_window_constant() {
    // Verify the context window is set correctly
    assert!(CONTEXT_WINDOW > 0);
    assert_eq!(CONTEXT_WINDOW, 128_000);
}

#[test]
fn test_compression_threshold_constant() {
    // Threshold should be between 0 and 1
    assert!(COMPRESSION_THRESHOLD > 0.0);
    assert!(COMPRESSION_THRESHOLD <= 1.0);
    assert_eq!(COMPRESSION_THRESHOLD, 0.60);
}

#[test]
fn test_keep_recent_messages_constant() {
    // Should keep a reasonable number of recent messages
    assert!(KEEP_RECENT_MESSAGES > 0);
    assert!(KEEP_RECENT_MESSAGES < 100);
    assert_eq!(KEEP_RECENT_MESSAGES, 15);
}
