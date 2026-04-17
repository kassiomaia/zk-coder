//! Request-shape stability hashes (M1.2).
//!
//! A trace consumer should be able to answer "why did two adjacent LLM
//! requests differ?" without reconstructing the full prompt. The three hashes
//! defined here — `system_prompt_hash`, `tool_catalog_hash`, and
//! `message_prefix_hash` — decompose the prepared request into the three
//! surfaces where cache-invalidating drift tends to originate:
//!
//! - **system prompt**: the top-of-prompt text, which Manus/Anthropic both
//!   call out as the most cache-sensitive surface.
//! - **tool catalog**: the serialized tool definitions. M1.1 sorts these by
//!   name, so the hash is deterministic as long as the catalog content is
//!   unchanged.
//! - **message prefix**: the messages the model has already consumed —
//!   everything except the final (freshly-appended) message. When a
//!   conversation advances by appending one new tool/user/assistant message,
//!   this hash should stay constant; if it drifts, something earlier in the
//!   context was mutated.
//!
//! All three return hex-encoded SHA-256 digests. Stability across runs is
//! what matters here — the digest is never compared with anything outside
//! this codebase, so a short hex string is sufficient.

use sha2::{Digest, Sha256};

use crate::core::llm::entities::{ChatMessage, ToolDefinition};

/// Hex-encoded SHA-256 of the system prompt text.
#[must_use]
pub fn system_prompt_hash(system_prompt: &str) -> String {
    let digest = Sha256::digest(system_prompt.as_bytes());
    format!("{digest:x}")
}

/// Hex-encoded SHA-256 of the sorted tool catalog.
///
/// Relies on the M1.1 guarantee that `ToolRegistry::definitions()` returns
/// tool definitions sorted by name, so the serialization order is canonical
/// without a re-sort here. Passing a non-canonical slice will still produce a
/// deterministic digest, but the digest will differ from the canonical order
/// — that is the signal the measurement harness wants to surface.
#[must_use]
pub fn tool_catalog_hash(tools: &[ToolDefinition]) -> String {
    let mut hasher = Sha256::new();
    for tool in tools {
        hasher.update(tool.name.as_bytes());
        hasher.update(b"\x1f");
        hasher.update(tool.description.as_bytes());
        hasher.update(b"\x1f");
        let schema = serde_json::to_string(&tool.input_schema)
            .unwrap_or_else(|_| String::from("schema_serialization_failed"));
        hasher.update(schema.as_bytes());
        hasher.update(b"\x1e");
    }
    let digest = hasher.finalize();
    format!("{digest:x}")
}

/// Hex-encoded SHA-256 of the message prefix — everything except the last
/// message.
///
/// The prefix is the cache-sensitive span. A stable prefix hash across
/// adjacent steps means the provider-side prompt cache can stay warm; a
/// changing prefix hash is a direct signal that something other than the new
/// tail message was mutated.
///
/// When the message list is empty or has a single entry, returns the digest
/// of the empty byte sequence — there is no prefix to hash.
#[must_use]
pub fn message_prefix_hash(messages: &[ChatMessage]) -> String {
    if messages.len() <= 1 {
        let digest = Sha256::digest(b"");
        return format!("{digest:x}");
    }
    let prefix = &messages[..messages.len() - 1];
    let mut hasher = Sha256::new();
    for message in prefix {
        let serialized = serde_json::to_string(message)
            .unwrap_or_else(|_| String::from("message_serialization_failed"));
        hasher.update(serialized.as_bytes());
        hasher.update(b"\x1e");
    }
    let digest = hasher.finalize();
    format!("{digest:x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::llm::entities::{MessageContent, MessageRole};
    use serde_json::json;

    fn tool(name: &str, description: &str) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: description.to_string(),
            input_schema: json!({"type": "object"}),
        }
    }

    fn user_message(content: &str) -> ChatMessage {
        ChatMessage {
            role: MessageRole::User,
            content: MessageContent::Text(content.to_string()),
            tool_calls: Vec::new(),
            tool_result: None,
        }
    }

    #[test]
    fn system_prompt_hash_is_deterministic() {
        let first = system_prompt_hash("You are a helpful agent.");
        let second = system_prompt_hash("You are a helpful agent.");
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
    }

    #[test]
    fn system_prompt_hash_differs_when_prompt_changes() {
        let a = system_prompt_hash("You are a helpful agent.");
        let b = system_prompt_hash("You are a thorough agent.");
        assert_ne!(a, b);
    }

    #[test]
    fn tool_catalog_hash_is_stable_for_identical_input() {
        let catalog = vec![tool("fs_read_file", "Read a file"), tool("fs_write_file", "Write a file")];
        let first = tool_catalog_hash(&catalog);
        let second = tool_catalog_hash(&catalog);
        assert_eq!(first, second);
    }

    #[test]
    fn tool_catalog_hash_changes_when_a_tool_is_added() {
        let base = vec![tool("fs_read_file", "Read a file")];
        let extended = vec![
            tool("fs_read_file", "Read a file"),
            tool("fs_write_file", "Write a file"),
        ];
        assert_ne!(tool_catalog_hash(&base), tool_catalog_hash(&extended));
    }

    #[test]
    fn tool_catalog_hash_changes_when_a_description_changes() {
        let before = vec![tool("fs_read_file", "Read a file")];
        let after = vec![tool("fs_read_file", "Read file contents")];
        assert_ne!(tool_catalog_hash(&before), tool_catalog_hash(&after));
    }

    #[test]
    fn tool_catalog_hash_is_sensitive_to_order() {
        // Canonical (sorted) order from M1.1 produces a different hash from
        // an intentionally reversed slice — this is the signal the
        // measurement harness wants to observe if a downstream caller ever
        // reshuffles the catalog.
        let canonical = vec![
            tool("agent_emit_status", "Emit status"),
            tool("fs_read_file", "Read a file"),
        ];
        let reversed = vec![
            tool("fs_read_file", "Read a file"),
            tool("agent_emit_status", "Emit status"),
        ];
        assert_ne!(tool_catalog_hash(&canonical), tool_catalog_hash(&reversed));
    }

    #[test]
    fn message_prefix_hash_is_empty_digest_for_zero_or_one_messages() {
        let empty_digest = format!("{:x}", Sha256::digest(b""));
        assert_eq!(message_prefix_hash(&[]), empty_digest);
        assert_eq!(message_prefix_hash(&[user_message("first")]), empty_digest);
    }

    #[test]
    fn message_prefix_hash_is_stable_when_only_last_message_changes() {
        let shared_prefix = vec![user_message("Hello"), user_message("World")];
        let mut variant_a = shared_prefix.clone();
        variant_a.push(user_message("tail A"));
        let mut variant_b = shared_prefix.clone();
        variant_b.push(user_message("tail B"));

        assert_eq!(message_prefix_hash(&variant_a), message_prefix_hash(&variant_b));
    }

    #[test]
    fn message_prefix_hash_changes_when_an_earlier_message_changes() {
        let original = vec![
            user_message("Hello"),
            user_message("World"),
            user_message("tail"),
        ];
        let mutated = vec![
            user_message("Hello"),
            user_message("Universe"),
            user_message("tail"),
        ];
        assert_ne!(message_prefix_hash(&original), message_prefix_hash(&mutated));
    }
}
