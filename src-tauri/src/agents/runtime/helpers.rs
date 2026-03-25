//! Helper functions for agent runtime.

use crate::agents::types::ActionType;

/// Derives the system prompt with optional subtask context.
pub fn derive_system_prompt(
    agent_system_prompt: &str,
    current_subtask: &Option<(super::types::SubtaskSummary, usize)>,
) -> String {
    match current_subtask {
        Some((summary, total)) => {
            if summary.status == crate::agents::task::TaskStatus::InProgress {
                format!(
                    "{}\n\n## Current Task ({}/{}): {}\n\n{}\n\nFocus on completing this task. \
                        When done, call agent_advance_to_next_subtask.",
                    agent_system_prompt,
                    summary.order + 1,
                    total,
                    summary.title,
                    summary.mode.instruction_text()
                )
            } else {
                agent_system_prompt.to_string()
            }
        }
        None => agent_system_prompt.to_string(),
    }
}

/// Infers the action type from tool name and arguments.
pub fn infer_action_type(tool_name: &str, arguments: &serde_json::Value) -> ActionType {
    let path = arguments
        .get("path")
        .or_else(|| arguments.get("file_path"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    match tool_name {
        name if name.contains("read") => ActionType::ReadFile { path },
        name if name.contains("write") || name.contains("create") => {
            let size = arguments
                .get("content")
                .and_then(|v| v.as_str())
                .map(|s| s.len() as u64)
                .unwrap_or(0);
            ActionType::WriteFile {
                path,
                size_bytes: size,
            }
        }
        name if name.contains("delete") || name.contains("remove") => {
            ActionType::DeleteFile { path }
        }
        name if name.contains("execute") || name.contains("command") || name.contains("shell") => {
            let command = arguments
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            ActionType::ExecuteCommand { command }
        }
        name if name.contains("fetch") || name.contains("http") || name.contains("web") => {
            let url = arguments
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            ActionType::WebRequest { url }
        }
        _ => ActionType::ReadFile { path },
    }
}

/// Generates a diff from two content strings.
pub fn generate_diff_from_content(path: &str, original: &str, current: &str) -> String {
    use std::path::Path;
    if let Some(diff) = crate::tools::fs::write_file::generate_diff(Path::new(path), original, current) {
        diff
    } else {
        // Fallback: simple diff generation
        let line_count = current.lines().count();
        let lines: String = current
            .lines()
            .map(|line| format!("+{}", line))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "--- a/{}\n+++ b/{}\n@@ -0,0 +1,{} @@\n{}",
            path, path, line_count, lines
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_action_type_read() {
        let action = infer_action_type(
            "read_workspace_file",
            &serde_json::json!({"path": "/tmp/test.rs"}),
        );
        match action {
            ActionType::ReadFile { path } => assert_eq!(path, "/tmp/test.rs"),
            _ => panic!("Expected ReadFile"),
        }
    }

    #[test]
    fn test_infer_action_type_write() {
        let action = infer_action_type(
            "write_file",
            &serde_json::json!({"path": "/tmp/out.rs", "content": "hello"}),
        );
        match action {
            ActionType::WriteFile { path, size_bytes } => {
                assert_eq!(path, "/tmp/out.rs");
                assert_eq!(size_bytes, 5);
            }
            _ => panic!("Expected WriteFile"),
        }
    }

    #[test]
    fn test_infer_action_type_execute() {
        let action = infer_action_type(
            "execute_command",
            &serde_json::json!({"command": "cargo build"}),
        );
        match action {
            ActionType::ExecuteCommand { command } => assert_eq!(command, "cargo build"),
            _ => panic!("Expected ExecuteCommand"),
        }
    }

    #[test]
    fn test_infer_action_type_unknown_defaults_to_read() {
        let action = infer_action_type("unknown_tool", &serde_json::json!({}));
        match action {
            ActionType::ReadFile { path } => assert_eq!(path, ""),
            _ => panic!("Expected ReadFile as default"),
        }
    }

    #[test]
    fn test_derive_system_prompt_without_subtask() {
        let base_prompt = "You are a helpful assistant.";
        let result = derive_system_prompt(base_prompt, &None);
        assert_eq!(result, base_prompt);
    }

    #[test]
    fn test_derive_system_prompt_with_subtask() {
        use crate::agents::runtime::SubtaskSummary;

        let base_prompt = "You are a helpful assistant.";
        let summary = SubtaskSummary {
            title: "Test task".to_string(),
            mode: crate::agents::task::SubTaskMode::Read,
            status: crate::agents::task::TaskStatus::InProgress,
            order: 0,
        };
        let result = derive_system_prompt(base_prompt, &Some((summary, 3)));
        assert!(result.contains("## Current Task (1/3): Test task"));
        assert!(result.contains("Focus on completing this task"));
    }

    #[test]
    fn test_generate_diff_from_content() {
        let original = "line1\nline2";
        let current = "line1\nline2\nline3";
        let diff = generate_diff_from_content("test.txt", original, current);
        assert!(diff.contains("test.txt"));
        assert!(diff.contains("+line3"));
    }
}
