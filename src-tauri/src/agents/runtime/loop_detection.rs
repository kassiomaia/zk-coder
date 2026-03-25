//! Loop detection utilities for agent runtime.
//!
//! Provides detection of repetitive tool call patterns to prevent
/// infinite loops during agent execution.

use crate::llm::ToolCall;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Maximum same-action repetitions before detecting a loop.
pub const MAX_REPETITIONS: u32 = 3;

/// Tools that should be skipped during loop detection.
///
/// These are agent metadata tools designed to be called multiple times
/// as part of normal agent workflow.
pub const LOOP_SKIP_TOOLS: &[&str] = &[
    "agent_advance_to_next_subtask",
    "agent_emit_status",
    "agent_mark_subtask_status",
];

/// Creates a hash of the tool call arguments for comparison.
pub fn hash_tool_arguments(arguments: &serde_json::Value) -> u64 {
    let mut hasher = DefaultHasher::new();
    arguments.hash(&mut hasher);
    hasher.finish()
}

/// Checks if a tool call should be excluded from loop detection.
pub fn should_skip_loop_detection(tool_name: &str) -> bool {
    LOOP_SKIP_TOOLS.contains(&tool_name)
}

/// Detects if a tool call is part of a repetitive loop.
///
/// # Arguments
///
/// * `tool_call` - The tool call to check
/// * `recent_calls` - History of recent tool call hashes
/// * `max_repetitions` - Maximum allowed repetitions before declaring a loop
///
/// # Returns
///
/// * `Some(count)` - The number of times this call has been seen (if at or above threshold)
/// * `None` - The call is not part of a loop
pub fn detect_repetitive_loop(
    tool_call: &ToolCall,
    recent_calls: &[(String, u64)],
    max_repetitions: u32,
) -> Option<usize> {
    // Skip loop detection for agent metadata tools
    if should_skip_loop_detection(&tool_call.name) {
        return None;
    }

    let args_hash = hash_tool_arguments(&tool_call.arguments);
    let tool_key = (tool_call.name.clone(), args_hash);

    // Count how many times this exact tool call has appeared recently
    let repeat_count = recent_calls.iter().filter(|x| **x == tool_key).count();

    if repeat_count >= max_repetitions as usize {
        Some(repeat_count)
    } else {
        None
    }
}

/// Adds tool calls to the recent call history, maintaining a bounded size.
///
/// # Arguments
///
/// * `tool_calls` - The tool calls to add
/// * `recent_calls` - The history to update
/// * `max_size` - Maximum size of the history (default: 10)
pub fn add_to_recent_calls(
    tool_calls: &[ToolCall],
    recent_calls: &mut Vec<(String, u64)>,
    max_size: usize,
) {
    for tool_call in tool_calls {
        let args_hash = hash_tool_arguments(&tool_call.arguments);
        recent_calls.push((tool_call.name.clone(), args_hash));
    }

    // Keep only recent tool calls
    if recent_calls.len() > max_size {
        recent_calls.drain(0..recent_calls.len() - max_size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::ToolCall;
    use serde_json::json;

    #[test]
    fn test_hash_tool_arguments_consistent() {
        let args1 = json!({"path": "test.txt", "content": "hello"});
        let args2 = json!({"path": "test.txt", "content": "hello"});
        let args3 = json!({"path": "test.txt", "content": "world"});

        let hash1 = hash_tool_arguments(&args1);
        let hash2 = hash_tool_arguments(&args2);
        let hash3 = hash_tool_arguments(&args3);

        assert_eq!(hash1, hash2, "Same arguments should produce same hash");
        assert_ne!(hash1, hash3, "Different arguments should produce different hash");
    }

    #[test]
    fn test_should_skip_loop_detection() {
        assert!(should_skip_loop_detection("agent_advance_to_next_subtask"));
        assert!(should_skip_loop_detection("agent_emit_status"));
        assert!(should_skip_loop_detection("agent_mark_subtask_status"));
        assert!(!should_skip_loop_detection("fs_write_file"));
        assert!(!should_skip_loop_detection("fs_read_file"));
    }

    #[test]
    fn test_detect_repetitive_loop_none() {
        let tool_call = ToolCall {
            id: "1".to_string(),
            name: "fs_write_file".to_string(),
            arguments: json!({"path": "test.txt"}),
        };

        let recent_calls = vec![];
        let result = detect_repetitive_loop(&tool_call, &recent_calls, 3);
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_repetitive_loop_at_threshold() {
        let tool_call = ToolCall {
            id: "1".to_string(),
            name: "fs_write_file".to_string(),
            arguments: json!({"path": "test.txt"}),
        };

        let args_hash = hash_tool_arguments(&tool_call.arguments);
        let recent_calls = vec![
            (tool_call.name.clone(), args_hash),
            (tool_call.name.clone(), args_hash),
            (tool_call.name.clone(), args_hash),
        ];

        let result = detect_repetitive_loop(&tool_call, &recent_calls, 3);
        assert!(result.is_some()); // Found 3 times, at threshold
        assert_eq!(result.unwrap(), 3);
    }

    #[test]
    fn test_detect_repetitive_loop_skips_metadata_tools() {
        let tool_call = ToolCall {
            id: "1".to_string(),
            name: "agent_emit_status".to_string(),
            arguments: json!({"message": "test"}),
        };

        let args_hash = hash_tool_arguments(&tool_call.arguments);
        let recent_calls = vec![
            (tool_call.name.clone(), args_hash),
            (tool_call.name.clone(), args_hash),
            (tool_call.name.clone(), args_hash),
            (tool_call.name.clone(), args_hash),
        ];

        let result = detect_repetitive_loop(&tool_call, &recent_calls, 3);
        assert!(result.is_none(), "Metadata tools should skip loop detection");
    }

    #[test]
    fn test_add_to_recent_calls_bounded() {
        let tool_calls = vec![
            ToolCall {
                id: "1".to_string(),
                name: "tool1".to_string(),
                arguments: json!({}),
            },
            ToolCall {
                id: "2".to_string(),
                name: "tool2".to_string(),
                arguments: json!({}),
            },
        ];

        let mut recent_calls = vec![];
        for _ in 0..6 {
            add_to_recent_calls(&tool_calls, &mut recent_calls, 5);
        }

        assert_eq!(recent_calls.len(), 5, "Should be bounded to max_size");
    }
}
