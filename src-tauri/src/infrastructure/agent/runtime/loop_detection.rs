//! Loop detection utilities for agent runtime.
//!
//! Provides detection of repetitive tool call patterns to prevent
/// infinite loops during agent execution.
use crate::core::llm::entities::ToolCall;
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
    "agent_narrate",
    "agent_plan_subtasks",
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

/// State-aware variant of [`should_skip_loop_detection`].
///
/// During the final-summary branch the runtime sets
/// `current_allowed_tool_names = Some(Vec::new())` to express
/// "no tools may be called this turn". When that state is active,
/// even tools normally on the [`LOOP_SKIP_TOOLS`] whitelist (e.g.
/// `agent_narrate`) must NOT be exempt from loop detection: a model
/// that keeps re-emitting the same narration call across consecutive
/// final-summary iterations would otherwise burn the entire step
/// budget undetected.
///
/// Returns `true` (skip detection) only when the legacy whitelist
/// applies AND the runtime has not signalled an empty allow-list.
#[must_use]
pub fn should_skip_loop_detection_in_state(
    tool_name: &str,
    allowed_tool_names: Option<&[String]>,
) -> bool {
    if matches!(allowed_tool_names, Some(allowed) if allowed.is_empty()) {
        return false;
    }
    should_skip_loop_detection(tool_name)
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
    detect_repetitive_loop_in_state(tool_call, recent_calls, max_repetitions, None)
}

/// State-aware variant of [`detect_repetitive_loop`].
///
/// Behaves identically to [`detect_repetitive_loop`] when
/// `allowed_tool_names` is `None` or non-empty. When the caller
/// signals "no tools allowed this turn" via `Some(empty)`, the
/// metadata-tool whitelist is suppressed so repeated narration on
/// the final-summary branch is detected.
#[must_use]
pub fn detect_repetitive_loop_in_state(
    tool_call: &ToolCall,
    recent_calls: &[(String, u64)],
    max_repetitions: u32,
    allowed_tool_names: Option<&[String]>,
) -> Option<usize> {
    if should_skip_loop_detection_in_state(&tool_call.name, allowed_tool_names) {
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
        assert_ne!(
            hash1, hash3,
            "Different arguments should produce different hash"
        );
    }

    #[test]
    fn test_hash_tool_arguments_order_matters() {
        let args1 = json!({"path": "test.txt", "content": "hello"});
        let args2 = json!({"content": "hello", "path": "test.txt"});

        // JSON objects with different key orders should hash differently
        // because serde_json preserves insertion order
        let hash1 = hash_tool_arguments(&args1);
        let hash2 = hash_tool_arguments(&args2);

        // These might be the same or different depending on hash implementation
        // but the function should be deterministic
        let hash1_again = hash_tool_arguments(&args1);
        assert_eq!(hash1, hash1_again, "Hash should be deterministic");
    }

    #[test]
    fn test_hash_tool_arguments_empty() {
        let args = json!({});
        let hash = hash_tool_arguments(&args);
        // Empty object should produce a hash
        assert_ne!(hash, 0, "Empty arguments should still produce a hash");
    }

    #[test]
    fn test_hash_tool_arguments_nested() {
        let args1 = json!({"config": {"setting": "value"}, "path": "test.txt"});
        let args2 = json!({"config": {"setting": "value"}, "path": "test.txt"});
        let args3 = json!({"config": {"setting": "other"}, "path": "test.txt"});

        let hash1 = hash_tool_arguments(&args1);
        let hash2 = hash_tool_arguments(&args2);
        let hash3 = hash_tool_arguments(&args3);

        assert_eq!(hash1, hash2, "Same nested args should hash identically");
        assert_ne!(
            hash1, hash3,
            "Different nested values should hash differently"
        );
    }

    #[test]
    fn test_should_skip_loop_detection() {
        assert!(should_skip_loop_detection("agent_advance_to_next_subtask"));
        assert!(should_skip_loop_detection("agent_emit_status"));
        assert!(should_skip_loop_detection("agent_mark_subtask_status"));
        assert!(should_skip_loop_detection("agent_plan_subtasks"));
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
    fn test_detect_repetitive_loop_below_threshold() {
        let tool_call = ToolCall {
            id: "1".to_string(),
            name: "fs_write_file".to_string(),
            arguments: json!({"path": "test.txt"}),
        };

        let args_hash = hash_tool_arguments(&tool_call.arguments);
        let recent_calls = vec![
            (tool_call.name.clone(), args_hash),
            (tool_call.name.clone(), args_hash),
        ];

        let result = detect_repetitive_loop(&tool_call, &recent_calls, 3);
        assert!(result.is_none(), "Should not detect loop below threshold");
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
    fn test_detect_repetitive_loop_above_threshold() {
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
            (tool_call.name.clone(), args_hash),
            (tool_call.name.clone(), args_hash),
        ];

        let result = detect_repetitive_loop(&tool_call, &recent_calls, 3);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), 5, "Should report actual count");
    }

    #[test]
    fn test_detect_repetitive_loop_different_arguments() {
        let tool_call = ToolCall {
            id: "1".to_string(),
            name: "fs_write_file".to_string(),
            arguments: json!({"path": "test.txt"}),
        };

        let different_hash = hash_tool_arguments(&json!({"path": "other.txt"}));
        let recent_calls = vec![
            ("fs_write_file".to_string(), different_hash),
            ("fs_write_file".to_string(), different_hash),
            ("fs_write_file".to_string(), different_hash),
        ];

        let result = detect_repetitive_loop(&tool_call, &recent_calls, 3);
        assert!(
            result.is_none(),
            "Should not detect loop with different arguments"
        );
    }

    #[test]
    fn test_detect_repetitive_loop_different_tools() {
        let tool_call = ToolCall {
            id: "1".to_string(),
            name: "fs_write_file".to_string(),
            arguments: json!({"path": "test.txt"}),
        };

        let args_hash = hash_tool_arguments(&tool_call.arguments);
        let recent_calls = vec![
            ("fs_read_file".to_string(), args_hash),
            ("fs_read_file".to_string(), args_hash),
            ("fs_read_file".to_string(), args_hash),
        ];

        let result = detect_repetitive_loop(&tool_call, &recent_calls, 3);
        assert!(
            result.is_none(),
            "Should not detect loop with different tool names"
        );
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
        assert!(
            result.is_none(),
            "Metadata tools should skip loop detection"
        );
    }

    #[test]
    fn test_detect_repetitive_loop_all_skip_tools() {
        for tool in LOOP_SKIP_TOOLS {
            let tool_call = ToolCall {
                id: "1".to_string(),
                name: tool.to_string(),
                arguments: json!({"test": "data"}),
            };

            let args_hash = hash_tool_arguments(&tool_call.arguments);
            let recent_calls = vec![
                (tool.to_string(), args_hash),
                (tool.to_string(), args_hash),
                (tool.to_string(), args_hash),
                (tool.to_string(), args_hash),
            ];

            let result = detect_repetitive_loop(&tool_call, &recent_calls, 3);
            assert!(
                result.is_none(),
                "Tool '{}' should be skipped from loop detection",
                tool
            );
        }
    }

    #[test]
    fn test_add_to_recent_calls_empty() {
        let tool_calls = vec![];
        let mut recent_calls = vec![];
        add_to_recent_calls(&tool_calls, &mut recent_calls, 10);
        assert_eq!(recent_calls.len(), 0);
    }

    #[test]
    fn test_add_to_recent_calls_single() {
        let tool_calls = vec![ToolCall {
            id: "1".to_string(),
            name: "test_tool".to_string(),
            arguments: json!({"arg": "value"}),
        }];

        let mut recent_calls = vec![];
        add_to_recent_calls(&tool_calls, &mut recent_calls, 10);

        assert_eq!(recent_calls.len(), 1);
        assert_eq!(recent_calls[0].0, "test_tool");
    }

    #[test]
    fn test_add_to_recent_calls_multiple() {
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
        add_to_recent_calls(&tool_calls, &mut recent_calls, 10);

        assert_eq!(recent_calls.len(), 2);
        assert_eq!(recent_calls[0].0, "tool1");
        assert_eq!(recent_calls[1].0, "tool2");
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

    #[test]
    fn test_add_to_recent_calls_keeps_most_recent() {
        let tool_calls = vec![ToolCall {
            id: "1".to_string(),
            name: "tool".to_string(),
            arguments: json!({}),
        }];

        let mut recent_calls = vec![];
        for i in 0..10 {
            add_to_recent_calls(&tool_calls, &mut recent_calls, 5);
            // Each addition should have the same hash since arguments are identical
        }

        assert_eq!(recent_calls.len(), 5);
        // All entries should be the same tool
        for entry in &recent_calls {
            assert_eq!(entry.0, "tool");
        }
    }

    #[test]
    fn test_add_to_recent_calls_preserves_order() {
        let tool_calls = vec![
            ToolCall {
                id: "1".to_string(),
                name: "tool_a".to_string(),
                arguments: json!({}),
            },
            ToolCall {
                id: "2".to_string(),
                name: "tool_b".to_string(),
                arguments: json!({}),
            },
        ];

        let mut recent_calls = vec![];
        add_to_recent_calls(&tool_calls, &mut recent_calls, 10);

        // Check order is preserved
        assert_eq!(recent_calls[0].0, "tool_a");
        assert_eq!(recent_calls[1].0, "tool_b");
    }

    #[test]
    fn test_max_repetitions_constant() {
        assert_eq!(MAX_REPETITIONS, 3, "Default max repetitions should be 3");
    }

    #[test]
    fn state_aware_skip_honors_whitelist_when_allow_list_is_none() {
        // No allow-list signal → legacy whitelist behavior is preserved.
        for tool in LOOP_SKIP_TOOLS {
            assert!(
                should_skip_loop_detection_in_state(tool, None),
                "{tool} must remain whitelisted when allow-list is None"
            );
        }
        assert!(!should_skip_loop_detection_in_state("fs_read_file", None));
    }

    #[test]
    fn state_aware_skip_honors_whitelist_when_allow_list_is_non_empty() {
        // Restricted-but-non-empty allow-list (regular subtask mode) →
        // legacy whitelist behavior is preserved. The empty-allow-list
        // signal is what disables the whitelist; any non-empty list is
        // a normal subtask and must not affect narration loop detection.
        let allowed = vec!["fs_read_file".to_string(), "search_grep".to_string()];
        for tool in LOOP_SKIP_TOOLS {
            assert!(
                should_skip_loop_detection_in_state(tool, Some(&allowed)),
                "{tool} must remain whitelisted under a non-empty allow-list"
            );
        }
    }

    #[test]
    fn state_aware_skip_suppresses_whitelist_when_allow_list_is_empty() {
        // Final-summary branch signal: `Some(empty)` means no tool may
        // be called this turn. Loop detection must apply to *every*
        // tool — including agent metadata tools — so a model that
        // keeps re-emitting `agent_narrate` is caught instead of
        // burning the step budget unnoticed.
        let empty: Vec<String> = Vec::new();
        for tool in LOOP_SKIP_TOOLS {
            assert!(
                !should_skip_loop_detection_in_state(tool, Some(&empty)),
                "{tool} must NOT be skipped when the allow-list is Some(empty)"
            );
        }
    }

    #[test]
    fn detect_repetitive_loop_in_state_catches_narration_under_empty_allow_list() {
        // Replays the exact final-summary failure mode: `agent_narrate`
        // with identical args fired three times in a row while the
        // runtime is in the empty-allow-list state.
        let tool_call = ToolCall {
            id: "1".to_string(),
            name: "agent_narrate".to_string(),
            arguments: json!({"message": "summarizing"}),
        };
        let args_hash = hash_tool_arguments(&tool_call.arguments);
        let recent_calls = vec![
            (tool_call.name.clone(), args_hash),
            (tool_call.name.clone(), args_hash),
            (tool_call.name.clone(), args_hash),
        ];
        let empty: Vec<String> = Vec::new();

        // Legacy detector still skips (proves we did not break it).
        assert!(detect_repetitive_loop(&tool_call, &recent_calls, 3).is_none());
        // State-aware detector with empty allow-list catches the loop.
        let result =
            detect_repetitive_loop_in_state(&tool_call, &recent_calls, 3, Some(&empty));
        assert_eq!(
            result,
            Some(3),
            "narration loop must be detected under empty allow-list"
        );
    }

    #[test]
    fn detect_repetitive_loop_in_state_matches_legacy_for_non_empty_allow_list() {
        // Sanity: Some(non-empty) must behave exactly like None for the
        // detector — the empty-allow-list signal is the only state that
        // changes behavior.
        let tool_call = ToolCall {
            id: "1".to_string(),
            name: "agent_narrate".to_string(),
            arguments: json!({"message": "checkpoint"}),
        };
        let args_hash = hash_tool_arguments(&tool_call.arguments);
        let recent_calls = vec![
            (tool_call.name.clone(), args_hash),
            (tool_call.name.clone(), args_hash),
            (tool_call.name.clone(), args_hash),
        ];
        let allowed = vec!["agent_narrate".to_string()];
        assert!(
            detect_repetitive_loop_in_state(&tool_call, &recent_calls, 3, Some(&allowed))
                .is_none(),
            "non-empty allow-list must preserve legacy whitelist skip"
        );
    }

    #[test]
    fn test_loop_skip_tools_list_complete() {
        // Verify the skip list contains expected tools
        assert!(LOOP_SKIP_TOOLS.contains(&"agent_advance_to_next_subtask"));
        assert!(LOOP_SKIP_TOOLS.contains(&"agent_emit_status"));
        assert!(LOOP_SKIP_TOOLS.contains(&"agent_mark_subtask_status"));
        assert!(LOOP_SKIP_TOOLS.contains(&"agent_plan_subtasks"));

        // Verify length is as expected
        assert_eq!(LOOP_SKIP_TOOLS.len(), 5);
    }
}
